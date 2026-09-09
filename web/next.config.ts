import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
    reactCompiler: true,
    cacheComponents: true,
    serverExternalPackages: ['mysql2'],
}

export default nextConfig
