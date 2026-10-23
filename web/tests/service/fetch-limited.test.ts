import { describe, expect, test } from 'bun:test'
import { fetchLimited, type FetchLimits } from '@server/service/shared/http/fetch-limited'

const LIMITS: FetchLimits = { maxBytes: 16, timeoutMs: 1000, userAgent: 'rbms-test' }
const URL_ASKED = 'https://stellabms.xyz/st/header.json'
const LETTER_A = 0x61

const streamOf = (...chunks: Uint8Array[]) =>
    new ReadableStream<Uint8Array>({
        start: (controller) => {
            chunks.forEach((chunk) => controller.enqueue(chunk))
            controller.close()
        },
    })

const bytesOf = (length: number) => new Uint8Array(length).fill(LETTER_A)

const answering = (response: Response) => {
    const requests: { url: string; init: RequestInit }[] = []
    const fetchImplementation = async (url: string, init: RequestInit) => {
        requests.push({ url, init })
        return response
    }
    return { fetchImplementation, requests }
}

describe('fetchLimited', () => {
    test('200 응답의 본문과 상태, 형식을 돌려준다', async () => {
        const { fetchImplementation } = answering(new Response('{"a":1}', { headers: { 'content-type': 'application/json' } }))
        const result = await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)
        expect(result.ok && result.status).toBe(200)
        expect(result.ok && new TextDecoder().decode(result.bytes)).toBe('{"a":1}')
        expect(result.ok && result.contentType).toBe('application/json')
    })

    test('요청은 리다이렉트를 따르지 않고 전용 User-Agent 와 캐시 없음을 단다', async () => {
        const { fetchImplementation, requests } = answering(new Response('{}'))
        await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)
        expect(requests).toHaveLength(1)
        expect(requests[0].url).toBe(URL_ASKED)
        expect(requests[0].init.redirect).toBe('manual')
        expect(requests[0].init.cache).toBe('no-store')
        expect(requests[0].init.signal).toBeInstanceOf(AbortSignal)
        expect(new Headers(requests[0].init.headers).get('user-agent')).toBe('rbms-test')
    })

    test('3xx 응답은 따르지 않고 redirect 로 돌려준다', async () => {
        const { fetchImplementation } = answering(new Response(null, { status: 302, headers: { location: 'https://evil.example/' } }))
        expect(await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)).toEqual({ ok: false, reason: 'redirect' })
    })

    test('404 같은 응답은 실패가 아니라 상태와 함께 돌려준다', async () => {
        const { fetchImplementation } = answering(new Response('nope', { status: 404 }))
        const result = await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)
        expect(result.ok && result.status).toBe(404)
    })

    test('선언된 길이가 상한을 넘으면 본문을 읽지 않고 too_large', async () => {
        const { fetchImplementation } = answering(new Response(streamOf(bytesOf(4)), { headers: { 'content-length': '1000' } }))
        expect(await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)).toEqual({ ok: false, reason: 'too_large' })
    })

    test('길이를 알리지 않아도 읽는 도중 상한을 넘으면 멈추고 too_large', async () => {
        const { fetchImplementation } = answering(new Response(streamOf(bytesOf(10), bytesOf(10))))
        expect(await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)).toEqual({ ok: false, reason: 'too_large' })
    })

    test('상한과 같은 크기는 받는다', async () => {
        const { fetchImplementation } = answering(new Response(streamOf(bytesOf(8), bytesOf(8))))
        const result = await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)
        expect(result.ok && result.bytes.byteLength).toBe(LIMITS.maxBytes)
    })

    test('본문이 없는 응답은 빈 바이트이다', async () => {
        const { fetchImplementation } = answering(new Response(null, { status: 200 }))
        const result = await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)
        expect(result.ok && result.bytes.byteLength).toBe(0)
    })

    test('연결 오류는 network, 제한 시간 초과는 timeout 이고 던지지 않는다', async () => {
        const refused = async () => Promise.reject(new TypeError('fetch failed'))
        const timedOut = async () => Promise.reject(new DOMException('The operation timed out.', 'TimeoutError'))
        expect(await fetchLimited(URL_ASKED, LIMITS, refused)).toEqual({ ok: false, reason: 'network' })
        expect(await fetchLimited(URL_ASKED, LIMITS, timedOut)).toEqual({ ok: false, reason: 'timeout' })
    })

    test('본문을 읽는 도중 끊기면 network', async () => {
        const broken = new ReadableStream<Uint8Array>({ start: (controller) => controller.error(new TypeError('terminated')) })
        const { fetchImplementation } = answering(new Response(broken))
        expect(await fetchLimited(URL_ASKED, LIMITS, fetchImplementation)).toEqual({ ok: false, reason: 'network' })
    })
})
