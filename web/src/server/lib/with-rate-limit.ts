import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'

export const RATE_LIMIT = {
    AUTH: { limit: 5, windowMs: 60_000 },
    SCORE_SUBMIT: { limit: 60, windowMs: 60_000 },
    REPLAY_UPLOAD: { limit: 10, windowMs: 60_000 },
    PUBLIC_READ: { limit: 120, windowMs: 60_000 },
} as const

const MAX_BUCKETS = 5000

type Bucket = { count: number; resetAt: number }

const buckets = new Map<string, Bucket>()

const prune = (now: number) => {
    for (const [key, bucket] of buckets) {
        if (bucket.resetAt <= now) buckets.delete(key)
    }
}

export const consumeRateLimit = (key: string, limit: number, windowMs: number, now = Date.now()) => {
    const existing = buckets.get(key)
    if (!existing || existing.resetAt <= now) {
        if (buckets.size >= MAX_BUCKETS) prune(now)
        buckets.set(key, { count: 1, resetAt: now + windowMs })
        return { allowed: true, remaining: limit - 1 }
    }
    if (existing.count >= limit) return { allowed: false, remaining: 0 }
    existing.count += 1
    return { allowed: true, remaining: limit - existing.count }
}

export const resetRateLimits = () => buckets.clear()

export const getClientIp = (request: Request) => {
    const forwarded = request.headers.get('x-forwarded-for')
    if (forwarded) return forwarded.split(',')[0].trim()
    return request.headers.get('x-real-ip') ?? 'unknown'
}

export const withRateLimit =
    (scope: string, { limit, windowMs }: { limit: number; windowMs: number }, resolveKey: (request: Request) => string = getClientIp) =>
    <TContext>(handler: (request: Request, context: TContext) => Promise<Response> | Response) =>
    async (request: Request, context: TContext) => {
        const { allowed } = consumeRateLimit(`${scope}:${resolveKey(request)}`, limit, windowMs)
        if (!allowed) throw createAppError(ERROR_CODE.RATE_LIMITED)
        return handler(request, context)
    }
