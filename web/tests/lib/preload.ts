import { mock } from 'bun:test'

mock.module('server-only', () => ({}))

process.env.ALLOW_GUEST = 'false'
process.env.REQUIRE_BUILD_HASH = 'false'
