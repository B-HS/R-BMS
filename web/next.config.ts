import type { NextConfig } from 'next'

const CONTENT_SECURITY_POLICY = [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data: blob:",
    "font-src 'self' data:",
    "connect-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
].join('; ')

const SECURITY_HEADERS = [
    { key: 'X-Content-Type-Options', value: 'nosniff' },
    { key: 'X-Frame-Options', value: 'DENY' },
    { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
    { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=(), payment=(), usb=()' },
    ...(process.env.NODE_ENV === 'production' ? [{ key: 'Content-Security-Policy', value: CONTENT_SECURITY_POLICY }] : []),
]

const nextConfig: NextConfig = {
    reactCompiler: true,
    cacheComponents: true,
    serverExternalPackages: ['mysql2'],
    headers: async () => [{ source: '/:path*', headers: SECURITY_HEADERS }],
}

export default nextConfig
