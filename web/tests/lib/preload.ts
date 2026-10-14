import { mock } from 'bun:test'

mock.module('server-only', () => ({}))

process.env.ALLOW_GUEST = 'false'
process.env.REQUIRE_BUILD_HASH = 'false'
process.env.DATABASE_URL ??= 'mysql://rbms:rbms@127.0.0.1:1/rbms_unreachable'
