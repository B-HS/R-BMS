import { describe, expect, test } from 'bun:test'
import { z } from 'zod'
import { isAppError } from '@server/lib/error'
import { parseLimitedJsonBody, readLimitedText } from '@server/lib/parse-request'

const MAX_BYTES = 1024

const streamRequest = (chunkSize: number, chunkCount: number) => {
    const chunk = new TextEncoder().encode('x'.repeat(chunkSize))
    let sent = 0
    const body = new ReadableStream<Uint8Array>({
        pull(controller) {
            sent += 1
            controller.enqueue(chunk)
            if (sent >= chunkCount) controller.close()
        },
    })
    return new Request('http://localhost:3000/api/scores', { method: 'POST', body, duplex: 'half' } as RequestInit)
}

const captureError = async (run: () => Promise<unknown>) => {
    try {
        await run()
        return null
    } catch (error) {
        return isAppError(error) ? error : null
    }
}

describe('readLimitedText', () => {
    test('content-length 가 없어도 실제 바이트 수로 제한을 건다', async () => {
        const error = await captureError(() => readLimitedText(streamRequest(512, 4), MAX_BYTES))
        expect(error?.code).toBe('IR_PAYLOAD_TOO_LARGE')
        expect(error?.statusCode).toBe(413)
    })

    test('제한 이내 스트림은 전체 본문을 이어붙여 돌려준다', async () => {
        const text = await readLimitedText(streamRequest(256, 2), MAX_BYTES)
        expect(text).toHaveLength(512)
    })

    test('content-length 가 제한을 넘으면 본문을 읽기 전에 거부한다', async () => {
        const request = new Request('http://localhost:3000/api/scores', {
            method: 'POST',
            headers: { 'content-length': String(MAX_BYTES + 1) },
        })
        const error = await captureError(() => readLimitedText(request, MAX_BYTES))
        expect(error?.code).toBe('IR_PAYLOAD_TOO_LARGE')
    })
})

describe('parseLimitedJsonBody', () => {
    const schema = z.object({ id: z.string() })

    test('정상 JSON 은 스키마로 파싱한다', async () => {
        const request = new Request('http://localhost:3000/api/scores', { method: 'POST', body: JSON.stringify({ id: 'alice' }) })
        expect(await parseLimitedJsonBody(request, schema, MAX_BYTES)).toEqual({ id: 'alice' })
    })

    test('깨진 JSON 은 VALIDATION_ERROR 다', async () => {
        const request = new Request('http://localhost:3000/api/scores', { method: 'POST', body: '{' })
        const error = await captureError(() => parseLimitedJsonBody(request, schema, MAX_BYTES))
        expect(error?.code).toBe('VALIDATION_ERROR')
        expect(error?.statusCode).toBe(400)
    })
})
