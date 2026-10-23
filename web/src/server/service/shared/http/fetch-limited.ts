const REDIRECT_STATUS_MIN = 300
const REDIRECT_STATUS_MAX = 399
const TIMEOUT_ERROR_NAME = 'TimeoutError'

export type FetchLimits = { maxBytes: number; timeoutMs: number; userAgent: string }

export type FetchFailureReason = 'network' | 'timeout' | 'too_large' | 'redirect'

export type FetchedBytes = { ok: true; status: number; bytes: Uint8Array; contentType: string } | { ok: false; reason: FetchFailureReason }

export type FetchBytes = (url: string, limits: FetchLimits) => Promise<FetchedBytes>

type FetchImplementation = (url: string, init: RequestInit) => Promise<Response>

const concatChunks = (chunks: Uint8Array[], total: number) => {
    const bytes = new Uint8Array(total)
    chunks.reduce((offset, chunk) => {
        bytes.set(chunk, offset)
        return offset + chunk.byteLength
    }, 0)
    return bytes
}

const readLimited = async (body: ReadableStream<Uint8Array> | null, maxBytes: number) => {
    if (!body) return new Uint8Array(0)
    const reader = body.getReader()
    const chunks: Uint8Array[] = []
    let total = 0
    let result = await reader.read()
    while (!result.done) {
        total += result.value.byteLength
        if (total > maxBytes) {
            await reader.cancel()
            return null
        }
        chunks.push(result.value)
        result = await reader.read()
    }
    return concatChunks(chunks, total)
}

const declaredLength = (response: Response) => {
    const header = response.headers.get('content-length')
    return header === null ? null : Number(header)
}

/**
 * Downloads one URL for the daily table copy. A redirect is never followed (the answer must come from the host that was asked), the body
 * is read only up to `maxBytes`, and the whole request, body included, ends after `timeoutMs`. A failure is returned, never thrown.
 */
export const fetchLimited = async (url: string, limits: FetchLimits, fetchImplementation: FetchImplementation = fetch): Promise<FetchedBytes> => {
    try {
        const response = await fetchImplementation(url, {
            redirect: 'manual',
            signal: AbortSignal.timeout(limits.timeoutMs),
            headers: { 'user-agent': limits.userAgent, accept: 'application/json' },
            cache: 'no-store',
        })
        if (response.status >= REDIRECT_STATUS_MIN && response.status <= REDIRECT_STATUS_MAX) {
            await response.body?.cancel()
            return { ok: false, reason: 'redirect' }
        }
        const declared = declaredLength(response)
        if (declared !== null && declared > limits.maxBytes) {
            await response.body?.cancel()
            return { ok: false, reason: 'too_large' }
        }
        const bytes = await readLimited(response.body, limits.maxBytes)
        if (bytes === null) return { ok: false, reason: 'too_large' }
        return { ok: true, status: response.status, bytes, contentType: response.headers.get('content-type') ?? '' }
    } catch (error) {
        return { ok: false, reason: error instanceof Error && error.name === TIMEOUT_ERROR_NAME ? 'timeout' : 'network' }
    }
}
