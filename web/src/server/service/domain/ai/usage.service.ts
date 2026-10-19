import type { AiUsage, AiUsageLimitReason } from '@server/dto/ai'
import type { AiEffectiveLimits } from '@server/service/domain/ai/settings.service'

const DAY_KEY_LENGTH = 10
const MONTH_KEY_LENGTH = 7
const FIRST_DAY_SUFFIX = '-01'
const LAST_DAY_SUFFIX = '-31'

/** How long a run holds its concurrency slot when it never releases it (the function died). Longer than any AI call may take. */
export const AI_RUN_HOLD_MS = 300_000

export type AiTokenTotals = {
    calls: number
    inputTokens: number
    outputTokens: number
    cachedInputTokens: number
}

export type AiRunHold = { userId: string; slot: number; runId: string; day: string }

export type AiUsageServiceDb = {
    sumUsage: (params: { userId: string; fromDay: string; toDay: string }) => Promise<AiTokenTotals>
    /** Adds one call to the day's row only while it stays under `limit`, in one conditional write. */
    reserveDailyCall: (params: { userId: string; day: string; limit: number }) => Promise<boolean>
    addTokens: (params: { userId: string; day: string; usage: AiUsage }) => Promise<void>
    /** Takes one concurrency slot if it is free or its previous hold expired at or before `now`, in one conditional write. */
    acquireSlot: (params: { userId: string; slot: number; runId: string; now: number; expiresAt: number }) => Promise<boolean>
    releaseSlot: (params: { userId: string; slot: number; runId: string }) => Promise<void>
}

type AiUsageServiceDeps = {
    db: AiUsageServiceDb
    now: () => number
    newRunId: () => string
}

export const utcDayKey = (at: number) => new Date(at).toISOString().slice(0, DAY_KEY_LENGTH)

export const utcMonthKey = (at: number) => new Date(at).toISOString().slice(0, MONTH_KEY_LENGTH)

const toTotalsOutput = (totals: AiTokenTotals) => ({
    calls: totals.calls,
    input_tokens: totals.inputTokens,
    output_tokens: totals.outputTokens,
    cached_input_tokens: totals.cachedInputTokens,
})

export const createAiUsageService = (deps: AiUsageServiceDeps) => {
    const sumMonth = (userId: string, month: string) =>
        deps.db.sumUsage({ userId, fromDay: `${month}${FIRST_DAY_SUFFIX}`, toDay: `${month}${LAST_DAY_SUFFIX}` })

    const release = (hold: AiRunHold) => deps.db.releaseSlot({ userId: hold.userId, slot: hold.slot, runId: hold.runId })

    /** Takes one of the day's calls for a run that already holds a slot; an error gives the slot back before it is thrown on. */
    const reserveDailyCallOrRelease = async (hold: AiRunHold, limit: number) => {
        try {
            return await deps.db.reserveDailyCall({ userId: hold.userId, day: hold.day, limit })
        } catch (error) {
            await Promise.allSettled([release(hold)])
            throw error
        }
    }

    const acquireAnySlot = async (userId: string, slots: number, runId: string, now: number) => {
        for (let slot = 0; slot < slots; slot += 1) {
            if (await deps.db.acquireSlot({ userId, slot, runId, now, expiresAt: now + AI_RUN_HOLD_MS })) return slot
        }
        return null
    }

    return {
        summary: async ({ userId, limits }: { userId: string; limits: AiEffectiveLimits }) => {
            const now = deps.now()
            const day = utcDayKey(now)
            const month = utcMonthKey(now)
            const [today, thisMonth] = await Promise.all([deps.db.sumUsage({ userId, fromDay: day, toDay: day }), sumMonth(userId, month)])
            return {
                day,
                month,
                today: toTotalsOutput(today),
                this_month: toTotalsOutput(thisMonth),
                limits: { daily_calls: limits.dailyCalls, concurrent_runs: limits.concurrentRuns, monthly_token_limit: limits.monthlyTokenLimit },
            }
        },

        /**
         * Claims what one provider call needs, in the order that wastes nothing: the monthly token cap (read only), a concurrency
         * slot, then one of the day's calls. A refused or failed step gives back what the earlier steps took.
         */
        reserve: async ({ userId, limits }: { userId: string; limits: AiEffectiveLimits }) => {
            const now = deps.now()
            const fail = (limit: AiUsageLimitReason) => ({ ok: false as const, limit })
            if (limits.monthlyTokenLimit !== null) {
                const month = await sumMonth(userId, utcMonthKey(now))
                if (month.inputTokens + month.outputTokens >= limits.monthlyTokenLimit) return fail('monthly_tokens')
            }
            const runId = deps.newRunId()
            const slot = await acquireAnySlot(userId, limits.concurrentRuns, runId, now)
            if (slot === null) return fail('concurrent_runs')
            const hold = { userId, slot, runId, day: utcDayKey(now) }
            if (!(await reserveDailyCallOrRelease(hold, limits.dailyCalls))) {
                await release(hold)
                return fail('daily_calls')
            }
            return { ok: true as const, hold }
        },

        record: async ({ hold, usage }: { hold: AiRunHold; usage: AiUsage }) => deps.db.addTokens({ userId: hold.userId, day: hold.day, usage }),

        release,
    }
}

export type AiUsageService = ReturnType<typeof createAiUsageService>
