import type {
    AiCredentialOutput,
    AiFailureKind,
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

export const AI_FAILURE_LABEL = {
    invalid_key: '키가 거부됨',
    forbidden: '키 권한 또는 모델 접근을 확인하세요',
    model_unavailable: '모델이 없거나 은퇴함',
    quota: '잔액 또는 지출 한도 도달',
    rate_limited: '요청 한도 초과',
    unavailable: '공급자 연결 실패',
    refused: '답변 거절',
    truncated: '출력이 잘림',
    invalid_output: '출력 형식 오류',
    bad_request: '요청 형식 오류',
} satisfies Record<AiFailureKind, string>

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

const AI_FAILURE_FALLBACK_MESSAGE = 'AI 요청을 처리하지 못했습니다. 잠시 뒤 다시 시도해 주세요.'

/**
 * What a screen shows for a failed AI call: the server's sentence, the seconds the provider asked to wait (kept in production
 * details), and whether the fix is in the AI settings (no key, a rejected or forbidden key, a missing or retired model).
 */
export const toAiFailureView = (error: unknown) => {
    if (!(error instanceof ApiError)) return { message: AI_FAILURE_FALLBACK_MESSAGE, retryAfterSeconds: null, needsSettings: false, code: null }
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
