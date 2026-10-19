import { describe, expect, test } from 'bun:test'
import { getAiProvider } from '@server/service/shared/ai-provider'
import { anthropicAdapter } from '@server/service/shared/ai-provider/anthropic'
import { ollamaAdapter } from '@server/service/shared/ai-provider/ollama'
import { openaiAdapter } from '@server/service/shared/ai-provider/openai'
import { AI_PROVIDER_IDS, aiProviderIdSchema } from '@server/dto/ai'
import { AI_STRICTNESS } from '@server/service/shared/ai-provider/types'

describe('getAiProvider', () => {
    test('공급자 id 마다 자기 어댑터를 돌려준다', () => {
        expect(getAiProvider('anthropic')).toBe(anthropicAdapter)
        expect(getAiProvider('openai')).toBe(openaiAdapter)
        expect(getAiProvider('ollama')).toBe(ollamaAdapter)
    })

    test.each([...AI_PROVIDER_IDS])('%s 어댑터는 계약의 메서드를 모두 갖고 자기 id 를 알린다', (id) => {
        const adapter = getAiProvider(id)

        expect(adapter.id).toBe(id)
        expect(typeof adapter.validateKey).toBe('function')
        expect(typeof adapter.listModels).toBe('function')
        expect(typeof adapter.generateStructured).toBe('function')
    })

    test('형식 보장은 Anthropic·OpenAI 가 native, Ollama 가 best_effort 다', () => {
        expect(getAiProvider('anthropic').strictness).toBe(AI_STRICTNESS.NATIVE)
        expect(getAiProvider('openai').strictness).toBe(AI_STRICTNESS.NATIVE)
        expect(getAiProvider('ollama').strictness).toBe(AI_STRICTNESS.BEST_EFFORT)
    })

    test('알 수 없는 공급자 id 는 스키마가 거절한다', () => {
        expect(aiProviderIdSchema.safeParse('gemini').success).toBe(false)
        expect(aiProviderIdSchema.safeParse('anthropic').success).toBe(true)
    })
})
