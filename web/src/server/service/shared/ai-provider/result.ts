import type { z } from 'zod'
import { AI_FAILURE_KIND, type AiUsage } from '@server/dto/ai'
import type { AiFailureDetail, AiGenerateResult } from '@server/service/shared/ai-provider/types'
import { AI_SCHEMA_UNSUPPORTED_ERROR_TYPE, createFailureDetail, tryParseJson } from '@server/service/shared/ai-provider/http'

export const toFailedGenerate = (detail: AiFailureDetail, usage: AiUsage | null = null) => ({ ok: false as const, usage, ...detail })

type FinalizeStructuredOutputInput<TSchema extends z.ZodType> = {
    schema: TSchema
    text: string
    parseText?: (text: string) => ReturnType<typeof tryParseJson>
    usage: AiUsage
    requestId: string | null
    httpStatus: number
}

/**
 * Turns the provider's answer text into data. Whatever the provider guarantees, the text is parsed and
 * re-validated with the original Zod schema, so the length and range limits stripped from the
 * provider JSON Schema are enforced here.
 */
export const finalizeStructuredOutput = <TSchema extends z.ZodType>({
    schema,
    text,
    parseText = tryParseJson,
    usage,
    requestId,
    httpStatus,
}: FinalizeStructuredOutputInput<TSchema>): AiGenerateResult<z.infer<TSchema>> => {
    const parsed = parseText(text)
    const validated = parsed.ok ? schema.safeParse(parsed.value) : null
    if (!validated?.success) return toFailedGenerate(createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { requestId, httpStatus }), usage)
    return { ok: true, data: validated.data, usage, requestId }
}

export const toSchemaUnsupportedFailure = () =>
    toFailedGenerate(createFailureDetail(AI_FAILURE_KIND.BAD_REQUEST, { providerErrorType: AI_SCHEMA_UNSUPPORTED_ERROR_TYPE }))
