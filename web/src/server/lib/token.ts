import { createHash, randomBytes, randomUUID } from 'node:crypto'

const TOKEN_BYTES = 32
export const TOKEN_PREFIX = 'rbms_'

export const generateApiToken = () => `${TOKEN_PREFIX}${randomBytes(TOKEN_BYTES).toString('base64url')}`

export const hashApiToken = (plain: string) => createHash('sha256').update(plain).digest('hex')

export const parseBearerToken = (request: Request) => {
    const header = request.headers.get('authorization')
    if (!header) return null
    const [scheme, ...rest] = header.split(' ')
    if (scheme.toLowerCase() !== 'bearer') return null
    const value = rest.join(' ').trim()
    return value.length > 0 ? value : null
}

export const newId = (prefix: string) => `${prefix}${randomUUID().replaceAll('-', '').slice(0, 30)}`
