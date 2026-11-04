import { createHash } from 'node:crypto'
import { weaknessProfileViewSchema, type WeaknessProfileView } from '@server/dto/ai-play'
import {
    PROFILE_DIGEST_CONTEXT_TOP,
    PROFILE_DIGEST_TIMING_TOP,
    PROFILE_MAX_PLAYS,
    PROFILE_RECENT_DAYS,
    PROFILE_VERSION,
    PROFILE_WINDOW_DAYS,
    PROFILE_WINDOW_KEY,
} from '@server/service/domain/ai/play-feature.constants'
import { EVIDENCE } from '@server/service/domain/analysis/sample'
import { computeWeaknessProfile, measureProfileKeys } from '@server/service/domain/analysis/weakness-profile'
import { CURRENT_ANALYSIS_VERSIONS } from '@server/service/domain/trace/trace-analysis'
import type { TraceService } from '@server/service/domain/trace/trace.service'

const DAY_MS = 86_400_000
const TAG_KEY_PREFIX = 'tag:'
const ISO_DATE_LENGTH = 10

/** What a cached profile is checked against: the analyses of the window, the newest of them and of the plays, the training charts. */
export type ProfileFingerprintInputs = { analyses: number; latestComputedAt: number | null; latestPlayedAt: number | null; trainingCharts: number }

/** A cached profile (`play_profile`). */
export type PlayProfileRow = { profileVersion: number; fingerprint: string; data: unknown; computedAt: number }

export type WeaknessProfileServiceDb = {
    /**
     * Cheap counts over the account's traced plays (autoplay left out) played at `sinceMs` or later: how many have a stored analysis,
     * the latest `computed_at` of those and the latest `played_at`, and how many training charts the account keeps.
     */
    readFingerprintInputs: (params: { userId: string; sinceMs: number }) => Promise<ProfileFingerprintInputs>
    findProfile: (params: { userId: string; windowKey: string }) => Promise<PlayProfileRow | null>
    /** Inserts or replaces the account's profile of the window. */
    saveProfile: (row: PlayProfileRow & { userId: string; windowKey: string }) => Promise<void>
}

type WeaknessProfileServiceDeps = {
    db: WeaknessProfileServiceDb
    plays: Pick<TraceService, 'analyzedPlaysInWindow'>
    now: () => number
}

const dayStartOf = (at: number) => Math.floor(at / DAY_MS) * DAY_MS

/** The first instant of the profile window: the start of the UTC day PROFILE_WINDOW_DAYS days ago, so a window holds whole days. */
export const profileWindowStartOf = (now: number) => dayStartOf(now) - PROFILE_WINDOW_DAYS * DAY_MS

/**
 * The fingerprint of a profile's input (plan 2026-11-04 2.5.1): the profile and analysis versions, the window's first UTC day and
 * the fingerprint counts. A new or recomputed analysis, a new play, a day passing or a training chart made or removed changes it.
 */
export const profileFingerprintOf = (inputs: ProfileFingerprintInputs, sinceMs: number) =>
    createHash('sha256')
        .update(
            JSON.stringify([
                PROFILE_VERSION,
                CURRENT_ANALYSIS_VERSIONS,
                new Date(sinceMs).toISOString().slice(0, ISO_DATE_LENGTH),
                inputs.analyses,
                inputs.latestComputedAt,
                inputs.latestPlayedAt,
                inputs.trainingCharts,
            ]),
        )
        .digest('hex')

const byMagnitude = (left: { value: number | null }, right: { value: number | null }) => Math.abs(right.value ?? 0) - Math.abs(left.value ?? 0)

/**
 * What the AI features are given of the profile (plan 2026-11-04 2.6): the window, the density limits, the weaknesses with a sample
 * of `low` or more and the strengths (all `strong`), the strong timing tags straying furthest, the failure and gauge-drop contexts
 * with the strongest evidence and lift, and the hands; the pattern tags the profile could not call either way by name only, so they
 * cannot be pointed at. Every item points at its profile key.
 */
export const toProfileDigest = (profile: WeaknessProfileView) => {
    const cellsOf = (kinds: readonly string[]) => profile.cells.filter((cell) => kinds.includes(cell.kind))
    const evidenceRank = (evidence: string) => (evidence === EVIDENCE.STRONG ? 0 : 1)
    return {
        window: { key: 'profile.window', plays: profile.window.plays, charts: profile.window.charts, days: profile.window.days },
        density: {
            key: 'density:limit',
            comfort_nps: profile.density.comfort_nps,
            limit_nps_ex: profile.density.limit_nps_ex,
            limit_nps_bp: profile.density.limit_nps_bp,
            max_seen_nps: profile.density.max_seen_nps,
            peak_seen_2s: profile.density.peak_seen_2s,
            evidence: profile.density.evidence,
        },
        sustained_minus_burst: { key: 'density:sustained', ...profile.density.sustained_minus_burst },
        weaknesses: profile.weaknesses
            .filter((item) => item.sample_level !== 'insufficient')
            .map(({ key, kind, value, unit, evidence, sample_level, notes, charts, trend_delta }) => ({
                key,
                kind,
                value,
                unit,
                evidence,
                sample_level,
                notes,
                charts,
                trend_delta,
            })),
        strengths: profile.strengths.map(({ key, kind, value, unit, evidence }) => ({ key, kind, value, unit, evidence })),
        timing: cellsOf(['timing'])
            .filter((cell) => cell.evidence === EVIDENCE.STRONG)
            .toSorted(byMagnitude)
            .slice(0, PROFILE_DIGEST_TIMING_TOP)
            .map((cell) => ({
                key: cell.key,
                deviation_ms: cell.value,
                mean_ms: cell.timing?.mean_ms ?? null,
                sd_ms: cell.timing?.sd_ms ?? null,
                fast_share: cell.timing?.fast_share ?? null,
                notes: cell.notes,
            })),
        context: cellsOf(['bp_context', 'gauge_context'])
            .filter((cell) => cell.evidence !== EVIDENCE.INSUFFICIENT)
            .toSorted((left, right) => evidenceRank(left.evidence) - evidenceRank(right.evidence) || (right.value ?? 0) - (left.value ?? 0))
            .slice(0, PROFILE_DIGEST_CONTEXT_TOP)
            .map((cell) => ({ key: cell.key, lift: cell.value, notes: cell.notes, evidence: cell.evidence })),
        uncertain_tags: cellsOf(['tag']).flatMap((cell) => (cell.evidence === EVIDENCE.UNCERTAIN ? [cell.key.slice(TAG_KEY_PREFIX.length)] : [])),
        insufficient_tags: cellsOf(['tag']).flatMap((cell) =>
            cell.evidence === EVIDENCE.INSUFFICIENT ? [cell.key.slice(TAG_KEY_PREFIX.length)] : [],
        ),
        hands: cellsOf(['hand']).map((cell) => ({
            key: cell.key,
            ex_residual: cell.value,
            bp_rate: cell.bp_rate,
            mean_ms: cell.timing?.mean_ms ?? null,
            evidence: cell.evidence,
        })),
        notice: profile.notice,
    }
}

export type ProfileDigest = ReturnType<typeof toProfileDigest>

/**
 * The player's weakness profile (plan 2026-11-04 2.5): computed from the analysed plays of the last PROFILE_WINDOW_DAYS days and
 * kept in `play_profile` until the fingerprint of its input changes. A profile computed while some plays were left out as stale is
 * not kept. No provider is ever called.
 */
export const createWeaknessProfileService = (deps: WeaknessProfileServiceDeps) => {
    const current = async (userId: string) => {
        const now = deps.now()
        const sinceMs = profileWindowStartOf(now)
        const fingerprint = profileFingerprintOf(await deps.db.readFingerprintInputs({ userId, sinceMs }), sinceMs)
        const stored = await deps.db.findProfile({ userId, windowKey: PROFILE_WINDOW_KEY })
        if (stored && stored.profileVersion === PROFILE_VERSION && stored.fingerprint === fingerprint) {
            const parsed = weaknessProfileViewSchema.safeParse(stored.data)
            if (parsed.success) return { profile: parsed.data, fingerprint, computedAt: stored.computedAt, cached: true }
        }
        const { plays, stale } = await deps.plays.analyzedPlaysInWindow({ userId, sinceMs, limit: PROFILE_MAX_PLAYS })
        const profile = computeWeaknessProfile(plays, {
            fromMs: sinceMs,
            toMs: now,
            recentFromMs: dayStartOf(now) - PROFILE_RECENT_DAYS * DAY_MS,
            days: PROFILE_WINDOW_DAYS,
            stale,
        })
        const computedFingerprint = profileFingerprintOf(await deps.db.readFingerprintInputs({ userId, sinceMs }), sinceMs)
        if (stale === 0) {
            await deps.db.saveProfile({
                userId,
                windowKey: PROFILE_WINDOW_KEY,
                profileVersion: PROFILE_VERSION,
                fingerprint: computedFingerprint,
                data: profile,
                computedAt: now,
            })
        }
        return { profile, fingerprint: computedFingerprint, computedAt: now, cached: false }
    }

    return {
        /** The profile with the fingerprint and time it was computed at, for a snapshot that must name the profile it was taken from. */
        current: async (params: { userId: string }) => current(params.userId),

        /** The profile as the screen shows it (`GET /api/fe/me/ai/profile`). */
        view: async (params: { userId: string }) => (await current(params.userId)).profile,

        /** The profile as the AI features read it (`toProfileDigest`). */
        digest: async (params: { userId: string }) => toProfileDigest((await current(params.userId)).profile),

        /**
         * The value of each key over the plays played at `sinceMs` or later (at most PROFILE_MAX_PLAYS, never cached), null for a key
         * whose sample conditions those plays do not meet: what a training chart's after is measured with (plan 7.2 `BeforeAfter`).
         */
        measureFocus: async (params: { userId: string; keys: readonly string[]; sinceMs: number }) => {
            const now = deps.now()
            const { plays, stale } = await deps.plays.analyzedPlaysInWindow({
                userId: params.userId,
                sinceMs: params.sinceMs,
                limit: PROFILE_MAX_PLAYS,
            })
            const days = Math.max(1, Math.ceil((now - params.sinceMs) / DAY_MS))
            return measureProfileKeys(plays, { fromMs: params.sinceMs, toMs: now, recentFromMs: params.sinceMs, days, stale }, params.keys)
        },
    }
}

export type WeaknessProfileService = ReturnType<typeof createWeaknessProfileService>
