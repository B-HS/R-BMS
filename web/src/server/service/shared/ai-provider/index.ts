import { anthropicAdapter } from '@server/service/shared/ai-provider/anthropic'
import { ollamaAdapter } from '@server/service/shared/ai-provider/ollama'
import { openaiAdapter } from '@server/service/shared/ai-provider/openai'
import type { AiProviderId } from '@server/dto/ai'
import type { AiProviderAdapter } from '@server/service/shared/ai-provider/types'

const AI_PROVIDER_ADAPTERS = {
    anthropic: anthropicAdapter,
    openai: openaiAdapter,
    ollama: ollamaAdapter,
} satisfies Record<AiProviderId, AiProviderAdapter>

export const getAiProvider = (id: AiProviderId): AiProviderAdapter => AI_PROVIDER_ADAPTERS[id]
