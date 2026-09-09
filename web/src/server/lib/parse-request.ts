import type { z } from 'zod'
import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'

const parseJsonText = (text: string): unknown => {
    try {
        return JSON.parse(text)
    } catch {
        return null
    }
}

export const parseJsonBody = async <T>(request: Request, schema: z.ZodType<T>): Promise<T> => {
    const body = await request.json().catch(() => null)
    const parsed = schema.safeParse(body)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { issues: parsed.error.issues })
    return parsed.data
}

export const parseSearchParams = <T>(request: Request, schema: z.ZodType<T>): T => {
    const params = Object.fromEntries(new URL(request.url).searchParams.entries())
    const parsed = schema.safeParse(params)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { issues: parsed.error.issues })
    return parsed.data
}

export const guardBodySize = (request: Request, maxBytes: number) => {
    const length = Number(request.headers.get('content-length') ?? 0)
    if (length > maxBytes) throw createAppError(ERROR_CODE.IR_PAYLOAD_TOO_LARGE, { maxBytes })
}

export const readLimitedText = async (request: Request, maxBytes: number) => {
    guardBodySize(request, maxBytes)
    const body = request.body
    if (!body) return await request.text()
    const reader = body.getReader()
    const chunks: Uint8Array[] = []
    let size = 0
    let chunk = await reader.read()
    while (!chunk.done) {
        size += chunk.value.byteLength
        if (size > maxBytes) {
            await reader.cancel()
            throw createAppError(ERROR_CODE.IR_PAYLOAD_TOO_LARGE, { maxBytes })
        }
        chunks.push(chunk.value)
        chunk = await reader.read()
    }
    const merged = new Uint8Array(size)
    let offset = 0
    for (const part of chunks) {
        merged.set(part, offset)
        offset += part.byteLength
    }
    return new TextDecoder().decode(merged)
}

export const parseLimitedJsonBody = async <T>(request: Request, schema: z.ZodType<T>, maxBytes: number): Promise<T> => {
    const text = await readLimitedText(request, maxBytes)
    const parsed = schema.safeParse(parseJsonText(text))
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { issues: parsed.error.issues })
    return parsed.data
}
