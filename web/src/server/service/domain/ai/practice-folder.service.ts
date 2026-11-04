import {
    AI_PRACTICE_LIMIT,
    aiPracticeFolderOutputSchema,
    type AiChartCandidate,
    type AiEffort,
    type AiPracticeFolderCreateInput,
    type AiPracticeFolderOutput,
    type AiPracticeFolderResult,
    type AiPracticeFolderView,
    type AiPracticeSourceInput,
} from '@server/dto/ai'
import { FOLDER_LIMIT } from '@server/dto/folder'
import {
    AI_OUTPUT_LANGUAGE_NAME,
    ASSIST_GRADE_RULE,
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    cleanLine,
    fitsLine,
    notEnoughData,
    readStoredInterpretation,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import type { AiGoalService } from '@server/service/domain/ai/goal-plan.service'
import type { AiRecommendService, TableFeatureDeps } from '@server/service/domain/ai/recommend.service'
import { hashAiInput, type AiResultService } from '@server/service/domain/ai/result.service'
import { FOLDER_CANDIDATES, SCORE_FEATURE_ANALYSIS_VERSION } from '@server/service/domain/ai/score-feature.constants'
import { practicePoolOf, rankWeaknessCharts, readTableProgress, toCandidate } from '@server/service/domain/ai/table-progress'
import type { WeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { PATTERN_TAGS, type PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { WEAKNESS_KEY } from '@server/service/domain/analysis/weakness-key'
import type { FolderService } from '@server/service/domain/folder/folder.service'
import type { Locale } from '@shared/i18n/locale'

export const AI_PRACTICE_FEATURE = 'practice_folder'
export const AI_PRACTICE_PROMPT_VERSION = 3
export const AI_PRACTICE_MAX_OUTPUT_TOKENS = 8_000
export const AI_PRACTICE_EFFORT = 'low' satisfies AiEffort

const ISO_DATE_LENGTH = 10

/** The folder name used when the AI's name does not fit, in the language the AI was asked to write in. */
export const AI_PRACTICE_DEFAULT_SUFFIX = {
    ko: { recommend: '다음 도전', tag: '패턴 연습', goal: '목표', weakness: '약점 연습' },
    en: { recommend: 'Next challenges', tag: 'Pattern practice', goal: 'Goal', weakness: 'Weakness practice' },
    ja: { recommend: '次の挑戦', tag: 'パターン練習', goal: '目標', weakness: '弱点練習' },
} as const satisfies Record<Locale, Record<AiPracticeSourceInput['source'], string>>

export const buildAiPracticeSystemPrompt = (language: Locale) =>
    [
        'You build one practice folder of charts for a rhythm game (BMS) player from charts the server already chose.',
        DIGEST_IS_DATA_RULE,
        "`source` says why these charts were chosen (next challenges, a pattern tag to practise, a goal, or a weakness of the player's profile in `source.weakness`, whose charts train it most) and `candidates` lists them.",
        `Choose at most \`limits.max_items\` charts from \`candidates\` only, by \`md5\`, in the order to play them, each with a ${AI_OUTPUT_LANGUAGE_NAME[language]} \`memo\` of at most \`limits.memo_max\` characters on one line.`,
        `\`name_suffix\` is a short ${AI_OUTPUT_LANGUAGE_NAME[language]} folder name of at most \`limits.name_suffix_max\` characters; \`description\` one ${AI_OUTPUT_LANGUAGE_NAME[language]} sentence of at most \`limits.description_max\` characters.`,
        'Chart `tags` describe the chart layout with no lane option (layout "original").',
        ASSIST_GRADE_RULE,
        DIGEST_KEY_RULE,
    ].join('\n')

export const aiPracticeSpecOf = (language: Locale) =>
    ({
        feature: AI_PRACTICE_FEATURE,
        analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
        promptVersion: AI_PRACTICE_PROMPT_VERSION,
        system: buildAiPracticeSystemPrompt(language),
        schemaName: 'practice_folder',
        schema: aiPracticeFolderOutputSchema,
        maxOutputTokens: AI_PRACTICE_MAX_OUTPUT_TOKENS,
        effort: AI_PRACTICE_EFFORT,
    }) satisfies AiFeatureSpec<typeof aiPracticeFolderOutputSchema>

export type AiPracticeFolderServiceDb = {
    findLinkedFolderId: (params: { userId: string; previewHash: string }) => Promise<string | null>
    /**
     * Links the preview to the folder unless it is already linked, and returns the folder id the preview is linked to afterwards:
     * `folderId` when this call made the link, the earlier folder's id when another request got there first.
     */
    claimLink: (params: { userId: string; previewHash: string; folderId: string; createdAt: number }) => Promise<string>
}

type AiPracticeFolderServiceDeps = TableFeatureDeps & {
    db: AiPracticeFolderServiceDb
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
    recommend: Pick<AiRecommendService, 'buildDigest'>
    goals: Pick<AiGoalService, 'remainingCandidates'>
    folders: Pick<FolderService, 'list' | 'get' | 'create' | 'replaceItems' | 'remove'>
    /** The player's weakness profile, the only way to the `weakness` source; without it that source has no data. */
    profile?: Pick<WeaknessProfileService, 'view'>
    now: () => number
}

type ResolvedSource = {
    tableId: string
    tag: string | null
    goalId: string | null
    focusKey: string | null
    weakness: { key: string; kind: string; value: number | null; unit: string; evidence: string } | null
    label: string
    candidates: AiChartCandidate[]
    needsPatterns: boolean
}

export type AiPracticeSourceFailure =
    | { reason: 'table_not_found' }
    | { reason: 'goal_not_found' }
    | { reason: 'invalid_source'; field: 'tag' | 'goal_id' | 'focus_key' }
    | { reason: 'not_enough_data'; need: string }

const isPatternTag = (tag: string): tag is PatternTag => (PATTERN_TAGS as readonly string[]).includes(tag)

/**
 * Keeps the folder to the candidates (plan 5.6): an item of another md5 or a memo that does not fit is dropped, never cut, and of
 * the items that pass, a chart's first one is kept and its repeats are dropped; a name or description that does not fit falls back to the default name and no description. No item left rejects it.
 */
export const refinePracticeOutput = (
    output: AiPracticeFolderOutput,
    params: { candidates: readonly { md5: string }[]; source: AiPracticeSourceInput['source']; language: Locale },
) => {
    const allowed = new Set(params.candidates.map((candidate) => candidate.md5))
    const items = output.items
        .filter((item) => allowed.has(item.md5) && (item.memo.trim() === '' || fitsLine(item.memo, AI_PRACTICE_LIMIT.MEMO_MAX)))
        .filter((item, index, valid) => valid.findIndex((other) => other.md5 === item.md5) === index)
        .slice(0, AI_PRACTICE_LIMIT.ITEMS_MAX)
        .map((item) => ({ md5: item.md5, memo: item.memo.trim() }))
    if (items.length === 0) return null
    return {
        name_suffix: fitsLine(output.name_suffix, AI_PRACTICE_LIMIT.NAME_SUFFIX_MAX)
            ? output.name_suffix.trim()
            : AI_PRACTICE_DEFAULT_SUFFIX[params.language][params.source],
        description: fitsLine(output.description, AI_PRACTICE_LIMIT.DESCRIPTION_MAX) ? output.description.trim() : '',
        items,
    }
}

export const practiceSubjectKey = (source: {
    source: string
    tableId: string
    tag: string | null
    goalId: string | null
    focusKey?: string | null
}) => ['src', source.source, source.goalId ?? source.tableId, source.tag ?? source.focusKey ?? ''].join(':')

const NO_SOURCE_DETAIL = { tag: null, goalId: null, focusKey: null, weakness: null } as const

export const createAiPracticeFolderService = (deps: AiPracticeFolderServiceDeps) => {
    /**
     * The charts of the practice pool (frontier −2 to +1 levels, short of a FULL COMBO) that train one weakness key most, as
     * candidates (plan 5.6 for a tag, plan 2026-11-04 2.6 feature 6 for any chart-picking weakness).
     */
    const fromKey = async (params: { userId: string; tableId?: string; key: string; limitNps: number | null }) => {
        const table = deps.loadTable(params.tableId)
        if (!table) return null
        const progress = readTableProgress(table, (await deps.loadRecords({ tableId: table.id, userId: params.userId })) ?? {})
        const pool = practicePoolOf(progress)
        const load = await deps.loadChartTags(pool.map((chart) => chart.md5))
        const candidates = rankWeaknessCharts({ pool, key: params.key, load, limitNps: params.limitNps, limit: FOLDER_CANDIDATES }).map(({ chart }) =>
            toCandidate(chart, load.shares),
        )
        return { tableId: table.id, candidates, needsPatterns: load.shares.size === 0 }
    }

    /**
     * The `weakness` source: the key must be one of the profile's weaknesses (else not enough data, `weakness_profile`) and one a
     * folder can pick charts for (else not enough data, `candidates`), both checked before any provider is called.
     */
    const fromWeakness = async (
        userId: string,
        input: AiPracticeSourceInput,
        focusKey: string,
    ): Promise<({ ok: true } & ResolvedSource) | ({ ok: false } & AiPracticeSourceFailure)> => {
        const profile = deps.profile ? await deps.profile.view({ userId }) : null
        const item = profile?.weaknesses.find((weakness) => weakness.key === focusKey)
        if (!profile || !item) return { ok: false, reason: 'not_enough_data', need: 'weakness_profile' }
        if (!item.selectable_for_folder) return { ok: false, reason: 'not_enough_data', need: 'candidates' }
        const picked = await fromKey({ userId, tableId: input.table, key: focusKey, limitNps: profile.density.limit_nps_ex })
        if (!picked) return { ok: false, reason: 'table_not_found' }
        const weakness = { key: item.key, kind: item.kind, value: item.value, unit: item.unit, evidence: item.evidence }
        return { ok: true, ...picked, ...NO_SOURCE_DETAIL, focusKey, weakness, label: focusKey }
    }

    const resolveSource = async (
        userId: string,
        input: AiPracticeSourceInput,
    ): Promise<({ ok: true } & ResolvedSource) | ({ ok: false } & AiPracticeSourceFailure)> => {
        if (input.source === 'goal') {
            if (!input.goal_id) return { ok: false, reason: 'invalid_source', field: 'goal_id' }
            const goal = await deps.goals.remainingCandidates({ userId, goalId: input.goal_id, limit: FOLDER_CANDIDATES })
            if (!goal) return { ok: false, reason: 'goal_not_found' }
            return {
                ok: true,
                tableId: goal.tableId,
                ...NO_SOURCE_DETAIL,
                goalId: input.goal_id,
                label: goal.label,
                candidates: goal.candidates,
                needsPatterns: false,
            }
        }
        if (input.source === 'tag') {
            if (!input.tag || !isPatternTag(input.tag)) return { ok: false, reason: 'invalid_source', field: 'tag' }
            const tagged = await fromKey({ userId, tableId: input.table, key: WEAKNESS_KEY.tag(input.tag), limitNps: null })
            if (!tagged) return { ok: false, reason: 'table_not_found' }
            return { ok: true, ...tagged, ...NO_SOURCE_DETAIL, tag: input.tag, label: input.tag }
        }
        if (input.source === 'weakness') {
            if (!input.focus_key) return { ok: false, reason: 'invalid_source', field: 'focus_key' }
            return fromWeakness(userId, input, input.focus_key)
        }
        const built = await deps.recommend.buildDigest({ userId, tableId: input.table, limit: FOLDER_CANDIDATES })
        if (!built) return { ok: false, reason: 'table_not_found' }
        return { ok: true, tableId: built.table.id, ...NO_SOURCE_DETAIL, label: '', candidates: built.digest.candidates, needsPatterns: false }
    }

    const folderRoom = async (userId: string) => {
        const used = (await deps.folders.list(userId)).length
        return { used, limit: FOLDER_LIMIT.FOLDERS_PER_USER, can_create: used < FOLDER_LIMIT.FOLDERS_PER_USER }
    }

    const digestOf = (input: AiPracticeSourceInput, resolved: ResolvedSource) => ({
        source: { kind: input.source, focus_tag: resolved.tag, goal: resolved.goalId ? resolved.label : null, weakness: resolved.weakness },
        candidates: resolved.candidates,
        limits: {
            max_items: AI_PRACTICE_LIMIT.ITEMS_MAX,
            name_suffix_max: AI_PRACTICE_LIMIT.NAME_SUFFIX_MAX,
            memo_max: AI_PRACTICE_LIMIT.MEMO_MAX,
            description_max: AI_PRACTICE_LIMIT.DESCRIPTION_MAX,
        },
    })

    const noticeOf = (resolved: ResolvedSource) => {
        if (resolved.needsPatterns) return { need: 'chart_patterns' }
        return resolved.candidates.length === 0 ? { need: 'candidates' } : null
    }

    const subjectOf = (input: AiPracticeSourceInput, resolved: ResolvedSource) =>
        practiceSubjectKey({
            source: input.source,
            tableId: resolved.tableId,
            tag: resolved.tag,
            goalId: resolved.goalId,
            focusKey: resolved.focusKey,
        })

    const createFolder = async (userId: string, name: string, description: string) => {
        const created = await deps.folders.create({ userId, input: { name, description } })
        if (created.ok || created.reason !== 'name_taken') return created
        const dated = `${name} ${new Date(deps.now()).toISOString().slice(0, ISO_DATE_LENGTH)}`
        return deps.folders.create({ userId, input: { name: dated, description } })
    }

    return {
        /** The candidates for one source, the folder room left and the latest stored AI preview. No provider is called. */
        view: async (params: { userId: string; input: AiPracticeSourceInput; language: Locale }) => {
            const resolved = await resolveSource(params.userId, params.input)
            if (!resolved.ok) return resolved
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: aiPracticeSpecOf(params.language),
                userId: params.userId,
                subjectKey: subjectOf(params.input, resolved),
                digest: digestOf(params.input, resolved),
            })
            const value = {
                source: params.input.source,
                table_id: resolved.tableId,
                tag: resolved.tag,
                goal_id: resolved.goalId,
                focus_key: resolved.focusKey,
                candidates: resolved.candidates,
                folders: await folderRoom(params.userId),
                notice: noticeOf(resolved),
                ai,
            } satisfies AiPracticeFolderView
            return { ok: true, value } as const
        },

        /**
         * Asks the user's AI for a folder preview. Nothing is created here: the folder is made only when the user presses create on
         * the preview. With the folder limit reached or no candidate there is no call.
         */
        preview: async (params: { userId: string; input: AiPracticeSourceInput; language: Locale }) => {
            const resolved = await resolveSource(params.userId, params.input)
            if (!resolved.ok) return resolved
            const notice = noticeOf(resolved)
            if (notice) return notEnoughData(notice.need)
            const room = await folderRoom(params.userId)
            if (!room.can_create) return { ok: false, reason: 'folder_limit', used: room.used, limit: room.limit } as const
            return runAiFeature(deps.runner, {
                ...aiPracticeSpecOf(params.language),
                userId: params.userId,
                subjectKey: subjectOf(params.input, resolved),
                digest: digestOf(params.input, resolved),
                refine: (output) =>
                    refinePracticeOutput(output, { candidates: resolved.candidates, source: params.input.source, language: params.language }),
            })
        },

        /**
         * Makes the folder the user confirmed, as `AI 연습 · <name>` (a taken name gets today's UTC date after it), with the
         * confirmed charts that are still candidates of the source. The same preview pressed again, or pressed twice at once,
         * replaces that folder's charts instead of making another one: the link of the preview is claimed with a unique key
         * right after the folder is made, and the request that loses the claim removes the folder it made and fills the winner's.
         */
        create: async (params: { userId: string; input: AiPracticeFolderCreateInput }) => {
            const resolved = await resolveSource(params.userId, params.input)
            if (!resolved.ok) return resolved
            const byMd5 = new Map(resolved.candidates.map((candidate) => [candidate.md5, candidate]))
            const items = params.input.items
                .filter((item, index, all) => all.findIndex((other) => other.md5 === item.md5) === index)
                .flatMap((item) => {
                    const candidate = byMd5.get(item.md5)
                    return candidate
                        ? [
                              {
                                  md5: item.md5,
                                  sha256: '',
                                  title: candidate.title,
                                  artist: candidate.artist,
                                  note: cleanLine(item.memo, FOLDER_LIMIT.NOTE_MAX),
                              },
                          ]
                        : []
                })
            if (items.length === 0) return { ok: false, reason: 'no_items' } as const
            const name = `${AI_PRACTICE_LIMIT.NAME_PREFIX}${cleanLine(params.input.name_suffix, AI_PRACTICE_LIMIT.NAME_SUFFIX_MAX)}`.normalize('NFC')
            const description = cleanLine(params.input.description, AI_PRACTICE_LIMIT.DESCRIPTION_MAX)
            const link = {
                userId: params.userId,
                previewHash: hashAiInput({
                    source: params.input.source,
                    table: resolved.tableId,
                    tag: resolved.tag,
                    goal: resolved.goalId,
                    ...(resolved.focusKey ? { focus: resolved.focusKey } : {}),
                    name,
                    description,
                    items: items.map((item) => [item.md5, item.note]),
                }),
            }

            const fill = async (folder: { id: string; name: string }, outcome: AiPracticeFolderResult['outcome']) => {
                const replaced = await deps.folders.replaceItems({ userId: params.userId, folderId: folder.id, input: { items } })
                if (!replaced.ok) return replaced
                const value = {
                    outcome,
                    folder_id: folder.id,
                    name: folder.name,
                    item_count: replaced.value.item_count,
                } satisfies AiPracticeFolderResult
                return { ok: true, value } as const
            }

            const fillLinked = async (folderId: string | null) => {
                const linked = folderId ? await deps.folders.get({ userId: params.userId, folderId }) : null
                return linked ? fill(linked, 'updated') : null
            }

            const existing = await fillLinked(await deps.db.findLinkedFolderId(link))
            if (existing) return existing
            const created = await createFolder(params.userId, name, description)
            if (!created.ok) return (await fillLinked(await deps.db.findLinkedFolderId(link))) ?? created
            const claimedId = await deps.db.claimLink({ ...link, folderId: created.value.id, createdAt: deps.now() })
            if (claimedId === created.value.id) return fill(created.value, 'created')
            await deps.folders.remove({ userId: params.userId, folderId: created.value.id })
            return (await fillLinked(claimedId)) ?? ({ ok: false, reason: 'not_found' } as const)
        },
    }
}

export type AiPracticeFolderService = ReturnType<typeof createAiPracticeFolderService>
