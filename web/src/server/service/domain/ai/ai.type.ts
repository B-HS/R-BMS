import type { AiProviderId, AiUsageLimitReason } from '@server/dto/ai'
import type { AiFailureDetail, AiProviderAdapter } from '@server/service/shared/ai-provider/types'

export type AiProviderResolver = (id: AiProviderId) => AiProviderAdapter

export type AiServiceFailure =
    | { reason: 'credential_not_found' }
    | { reason: 'credential_undecryptable'; cause: 'unknown_kek' | 'tampered' }
    | { reason: 'model_not_selected'; provider: AiProviderId }
    | { reason: 'usage_limit'; limit: AiUsageLimitReason }
    | { reason: 'provider'; provider: AiProviderId; detail: AiFailureDetail }
    | { reason: 'output_invalid'; provider: AiProviderId }
    | { reason: 'not_enough_data'; need: string }

export type AiServiceResult<T> = { ok: true; value: T } | ({ ok: false } & AiServiceFailure)
