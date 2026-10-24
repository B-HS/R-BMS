import type {
    AiChartCandidate,
    AiCredentialOutput,
    AiFailureKind,
    AiGoalKind,
    AiGoalOutput,
    AiGoalParams,
    AiGoalPlanView,
    AiInsight,
    AiJudgeHabitView,
    AiPracticeFolderCreateInput,
    AiPracticeFolderResult,
    AiPracticeFolderView,
    AiPracticeSourceInput,
    AiRecommendView,
    AiWeeklyView,
    AiModelOutput,
    AiOperatorLimitsOutput,
    AiOperatorLimitsPutInput,
    AiProviderId,
    AiRecordQueryAnswerOutput,
    AiRecordQueryField,
    AiRecordQueryFilter,
    AiRecordQueryIssue,
    AiRecordQueryOp,
    AiRecordQueryRowOutput,
    AiRecordQueryRunInput,
    AiUsageSummaryOutput,
    AiUserSettingOutput,
    AiUserSettingPutInput,
} from '@server/dto/ai'
import type {
    AiCompareMode,
    AiCompareQuery,
    AiCompareReason,
    AiCompareView,
    AiCoachView,
    AiGaugeAdviceView,
    AiGaugeName,
    AiOptionName,
    AiReportView,
    AiShareSettingOutput,
} from '@server/dto/ai-play'
import { ApiError } from '@shared/lib/fetch'

export type { AiFailureKind, AiProviderId }

export type AiCredential = AiCredentialOutput
export type AiModel = AiModelOutput
export type AiUsageSummary = AiUsageSummaryOutput
export type AiTokenTotals = AiUsageSummary['today']
export type AiUserSetting = AiUserSettingOutput
export type AiUserSettingInput = AiUserSettingPutInput
export type AiOperatorLimits = AiOperatorLimitsOutput
export type AiOperatorLimitsInput = AiOperatorLimitsPutInput
export type AiRecordQueryAnswer = AiRecordQueryAnswerOutput
export type AiRecordQueryPlan = AiRecordQueryRunInput
export type AiRecordQueryRow = AiRecordQueryRowOutput
export type { AiRecordQueryField, AiRecordQueryFilter, AiRecordQueryIssue, AiRecordQueryOp }
export type {
    AiChartCandidate,
    AiGoalKind,
    AiGoalParams,
    AiGoalPlanView,
    AiInsight,
    AiJudgeHabitView,
    AiPracticeFolderCreateInput,
    AiPracticeFolderResult,
    AiPracticeFolderView,
    AiPracticeSourceInput,
    AiRecommendView,
    AiWeeklyView,
}
export type { AiCoachView, AiCompareMode, AiCompareQuery, AiCompareReason, AiCompareView, AiGaugeAdviceView, AiGaugeName, AiOptionName, AiReportView }
export type AiShareSetting = AiShareSettingOutput
export type AiGoal = AiGoalOutput
/** How a stored or fresh AI answer was made, without the answer itself. */
export type AiInterpretationMeta = Omit<NonNullable<AiRecommendView['ai']>, 'output'>

export const AI_PROVIDER_ORDER = ['anthropic', 'openai', 'ollama'] as const satisfies readonly AiProviderId[]

export const AI_PROVIDER_LABEL = {
    anthropic: 'Anthropic (Claude)',
    openai: 'OpenAI',
    ollama: 'Ollama Cloud',
} satisfies Record<AiProviderId, string>

export const AI_PROVIDER_NOTE = {
    anthropic: 'Claude API 키(Console·Platform)를 등록합니다. 구독 로그인은 지원하지 않습니다.',
    openai: 'OpenAI API 키를 등록합니다. 모델 목록에서 원하는 모델을 직접 고릅니다(codex 계열도 목록에 보입니다).',
    ollama: 'Ollama Cloud API 키를 등록합니다. 출력 형식을 보장하지 않아 해석이 실패할 수 있고, 그때는 다시 요청해야 합니다.',
} satisfies Record<AiProviderId, string>

export const AI_FORM_LIMIT = {
    API_KEY_MIN_LENGTH: 8,
    API_KEY_MAX_LENGTH: 512,
    DAILY_CALLS_MAX: 1000,
    CONCURRENT_RUNS_MAX: 5,
    MONTHLY_TOKEN_LIMIT_MAX: 1_000_000_000_000,
    MODEL_MAX_LENGTH: 128,
} as const

/** Mirrors the server's AI error codes the screens react to (pinned by the contract test). */
export const AI_ERROR_CODE = {
    CREDENTIAL_NOT_FOUND: 'AI_CREDENTIAL_NOT_FOUND',
    CREDENTIAL_INVALID: 'AI_CREDENTIAL_INVALID',
    PROVIDER_FORBIDDEN: 'AI_PROVIDER_FORBIDDEN',
    MODEL_UNAVAILABLE: 'AI_MODEL_UNAVAILABLE',
} as const

const AI_SETTINGS_ERROR_CODES = new Set<string>(Object.values(AI_ERROR_CODE))

/**
 * What a screen shows for a failed AI call: the server's sentence (`null` when the failure is not an API error, so the screen words it), the seconds the provider asked to wait (kept in production
 * details), and whether the fix is in the AI settings (no key, a rejected or forbidden key, a missing or retired model).
 */
export const toAiFailureView = (error: unknown) => {
    if (!(error instanceof ApiError)) return { message: null, retryAfterSeconds: null, needsSettings: false, code: null }
    const retryAfter = error.details?.retry_after_seconds
    return {
        message: error.message,
        retryAfterSeconds: typeof retryAfter === 'number' && retryAfter > 0 ? retryAfter : null,
        needsSettings: AI_SETTINGS_ERROR_CODES.has(error.code) || error.details?.reason === 'model_not_selected',
        code: error.code,
    }
}

/** Picks the credential AI features run with, by the same rule the server uses: the chosen default provider, else the earliest registered key. */
export const selectActiveAiCredential = (credentials: readonly AiCredential[], defaultProvider: AiProviderId | null) =>
    credentials.find((credential) => credential.provider === defaultProvider) ??
    credentials.toSorted((left, right) => left.created_at - right.created_at)[0] ??
    null

/** Mirrors the server's AI_RECORD_QUERY_ALLOWED_OPS (pinned by the contract test): the operators each filter field takes. */
export const AI_RECORD_QUERY_FIELD_OPS = {
    level: ['eq', 'ne', 'in', 'gte', 'lte'],
    lamp: ['eq', 'ne', 'in', 'gte', 'lte'],
    ex_rate: ['gte', 'lte'],
    bp: ['eq', 'gte', 'lte'],
    played_at: ['gte', 'lte'],
    random: ['eq', 'ne', 'in'],
    gauge: ['eq', 'ne', 'in'],
    scratch_auto: ['eq'],
    title: ['contains'],
    table: ['eq'],
} as const satisfies Record<AiRecordQueryField, readonly AiRecordQueryOp[]>

export const AI_RECORD_QUERY_FIELD_LABEL = {
    level: '표 레벨',
    lamp: '램프',
    ex_rate: 'EX 비율(%)',
    bp: 'BP',
    played_at: '친 날짜',
    random: '배치 옵션',
    gauge: '게이지',
    scratch_auto: 'SCRATCH AUTO',
    title: '곡 제목',
    table: '난이도표',
} satisfies Record<AiRecordQueryField, string>

export const AI_RECORD_QUERY_VALUE_HINT = {
    level: '예: 12 또는 10,11,12',
    lamp: '예: Hard 또는 Easy,Normal',
    ex_rate: '예: 90 또는 85.5',
    bp: '예: 10',
    played_at: '예: 2026-10-01',
    random: '예: Random 또는 Off,Mirror',
    gauge: '예: Hard 또는 Easy,Normal',
    scratch_auto: 'true 또는 false',
    title: '제목에 들어간 글자',
    table: '예: insane1',
} satisfies Record<AiRecordQueryField, string>

export const AI_RECORD_QUERY_OP_LABEL = {
    eq: '같음',
    ne: '아님',
    gte: '이상',
    lte: '이하',
    in: '중 하나',
    contains: '포함',
} satisfies Record<AiRecordQueryOp, string>

export const AI_RECORD_QUERY_SORT_LABEL = {
    played_at: '친 날짜',
    ex_rate: 'EX 비율',
    bp: 'BP',
    level: '레벨',
    lamp: '램프',
} satisfies Record<AiRecordQueryPlan['sort_field'], string>

export const AI_RECORD_QUERY_ISSUE_LABEL = {
    op_not_allowed: '이 항목에 쓸 수 없는 비교입니다',
    value_invalid: '값을 읽을 수 없습니다',
    too_many_filters: '조건이 너무 많습니다',
} satisfies Record<AiRecordQueryIssue['reason'], string>

/** Mirrors the server's AI_RECORD_QUERY_LIMIT values the form needs (pinned by the contract test). */
export const AI_RECORD_QUERY_FORM_LIMIT = {
    QUESTION_MAX_CHARS: 200,
    MAX_FILTERS: 8,
    ROWS_MIN: 1,
    ROWS_MAX: 100,
} as const

/** The hub tabs, kept in the `tab` search parameter. */
export const AI_HUB_TABS = ['query', 'recommend', 'goals', 'weekly', 'report', 'habit', 'gauge'] as const
export type AiHubTab = (typeof AI_HUB_TABS)[number]

export const AI_HUB_TAB_LABEL = {
    query: '기록에 묻기',
    recommend: '추천·연습 폴더',
    goals: '목표',
    weekly: '주간 요약',
    report: '플레이어 리포트',
    habit: '판정 습관',
    gauge: '게이지·옵션',
} satisfies Record<AiHubTab, string>

/** Why a feature shows numbers only and makes no AI call: what it still needs (the server's `notice.need`). */
export const AI_DATA_NEEDS = [
    'table_records',
    'candidates',
    'chart_patterns',
    'goal_reached',
    'remaining_charts',
    'table',
    'week_plays',
    'recent_plays',
    'traced_plays',
    'play_trace',
    'play_metrics',
    'comparable_play',
] as const

export type AiDataNeed = (typeof AI_DATA_NEEDS)[number]

export const isAiDataNeed = (need: string): need is AiDataNeed => AI_DATA_NEEDS.some((known) => known === need)

export const AI_RECOMMEND_KIND_LABEL = {
    failed: '실패한 곡',
    unplayed: '안 친 곡',
    lamp_up: '램프 올리기',
} satisfies Record<AiRecommendView['candidates'][number]['kind'], string>

export const AI_GOAL_KINDS = ['lamp_count_at_level', 'clear_all_level', 'ex_rate_on_chart'] as const satisfies readonly AiGoalKind[]

export const AI_OFFSET_DIRECTION_LABEL = {
    earlier: '노트가 먼저 보이도록 올리기',
    later: '노트가 늦게 보이도록 내리기',
    keep: '그대로 두기',
} satisfies Record<AiJudgeHabitView['offset']['direction'], string>

export const AI_GREEN_DIRECTION_LABEL = {
    faster: '더 빠르게(GREEN 낮게)',
    slower: '더 느리게(GREEN 높게)',
    keep: '그대로',
} satisfies Record<'faster' | 'slower' | 'keep', string>

export const AI_EVIDENCE_LABEL = {
    strong: '근거 충분',
    uncertain: '불확실',
    insufficient: '표본 부족',
} satisfies Record<AiJudgeHabitView['offset']['grade'], string>

/** Mirrors the server's PATTERN_TAGS (pinned by the contract test), named for the screen. */
export const AI_PATTERN_TAG_LABEL = {
    chord_2: '2키 동시치기',
    chord_3: '3키 동시치기',
    chord_4plus: '4키 이상 동시치기',
    jack: '연타',
    trill: '트릴',
    stair: '계단',
    stream: '연속 난타',
    scratch_solo: '단독 스크래치',
    scratch_chord: '스크래치 동시치기',
    scratch_mixed: '스크래치 섞임',
    with_scratch: '스크래치 곁 건반',
    scratch_run: '스크래치 연속',
    ln_head: 'LN 시작',
    ln_end: 'LN 끝',
    ln_hold_hit: 'LN 누른 채 치기',
    ln_release_then_hit: 'LN 뗀 뒤 치기',
    soflan: '소프란',
    dense: '고밀도',
    mine_near: '지뢰 근처',
} as const

export type AiPatternTag = keyof typeof AI_PATTERN_TAG_LABEL

export type AiEvidenceGrade = keyof typeof AI_EVIDENCE_LABEL

export const isAiPatternTag = (tag: string): tag is AiPatternTag => tag in AI_PATTERN_TAG_LABEL

export const aiPatternTagLabel = (tag: string) => (isAiPatternTag(tag) ? AI_PATTERN_TAG_LABEL[tag] : tag)

/** Mirrors the server's AI_PRACTICE_LIMIT and AI_GOAL_LIMIT values the forms need (pinned by the contract test). */
export const AI_SCORE_FEATURE_FORM_LIMIT = {
    PRACTICE_NAME_PREFIX: 'AI 연습 · ',
    PRACTICE_NAME_SUFFIX_MAX: 40,
    GOAL_ACTIVE_MAX: 10,
    GOAL_COUNT_MAX: 2000,
    GOAL_EX_RATE_MAX: 100,
} as const

/** Mirrors the server's AI_GAUGE_NAMES (pinned by the contract test), named for the screen. */
export const AI_GAUGE_LABEL = {
    assist_easy: 'ASSIST EASY',
    easy: 'EASY',
    normal: 'NORMAL',
    hard: 'HARD',
    ex_hard: 'EX HARD',
    hazard: 'HAZARD',
    class: 'CLASS',
    ex_class: 'EX CLASS',
    ex_hard_class: 'EX HARD CLASS',
} satisfies Record<AiGaugeName, string>

/** Mirrors the server's AI_OPTION_NAMES (pinned by the contract test), named for the screen. */
export const AI_OPTION_LABEL = {
    off: 'OFF',
    mirror: 'MIRROR',
    random: 'RANDOM',
    r_random: 'R-RANDOM',
    s_random: 'S-RANDOM',
    spiral: 'SPIRAL',
    h_random: 'H-RANDOM',
    all_scratch: 'ALL-SCRATCH',
    converge: 'CONVERGE',
} satisfies Record<AiOptionName, string>

export const AI_GAUGE_OUTCOME_LABEL = {
    cleared: '통과',
    failed: '실패',
    unknown: '알 수 없음',
} satisfies Record<NonNullable<AiCoachView['analysis']>['gauge']['curves'][number]['outcome'], string>

export const AI_RUN_END_LABEL = {
    finished: '끝까지 친 판',
    gauge_empty: '게이지가 비어 끝난 판',
    left: '중간에 나간 판',
} satisfies Record<NonNullable<AiCoachView['analysis']>['run_end'], string>

export const AI_COMPARE_REASON_LABEL = {
    no_other_play: '이 곡의 다른 판이 없습니다.',
    note_layout_differs: 'LN 종류나 SCRATCH AUTO 가 달라 구간의 노트 수가 맞지 않습니다. 판정 합계만 비교합니다.',
    rival_not_sharing:
        '이 라이벌과는 구간 요약을 주고받지 않습니다. 서로 라이벌로 등록하고 상대가 공유를 켜야 합니다. 공개된 판정 합계만 비교합니다.',
    rival_no_trace: '이 라이벌의 이 곡 판에는 판별 데이터가 없습니다. 공개된 판정 합계만 비교합니다.',
    rival_no_record: '이 라이벌의 이 곡 공개 기록이 없어 맞댈 판이 없습니다.',
    not_a_rival: '내 라이벌 목록에 없는 플레이어입니다.',
} satisfies Record<AiCompareReason, string>

/** Mirrors the server's score id pattern of the AI play routes (pinned by the contract test). */
export const AI_SCORE_ID_PATTERN = /^[A-Za-z0-9_-]{1,36}$/

export const isAiScoreId = (value: string) => AI_SCORE_ID_PATTERN.test(value)

/** The coach page of one own play. */
export const aiPlayHref = (scoreId: string) => `/ai/plays/${encodeURIComponent(scoreId)}`
