import { describe, expect, test } from 'bun:test'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { z } from 'zod'

const WEB_DIR = join(import.meta.dirname, '..', '..')
const CRON_PATH = '/api/cron/tables'
const CRON_SCHEDULE = '0 15 * * *'
const CRON_ROUTE_FILE = join(WEB_DIR, 'src', 'app', 'api', 'cron', 'tables', 'route.ts')
const IGNORE_COMMAND = 'if [ "$VERCEL_ENV" != "production" ]; then exit 0; fi; git diff --quiet "$VERCEL_GIT_PREVIOUS_SHA" HEAD -- . || exit 1'

const vercelConfigSchema = z.object({
    framework: z.string(),
    regions: z.array(z.string()),
    crons: z.array(z.object({ path: z.string(), schedule: z.string() })),
    ignoreCommand: z.string(),
})

const config = vercelConfigSchema.parse(JSON.parse(readFileSync(join(WEB_DIR, 'vercel.json'), 'utf8')))

describe('vercel.json', () => {
    test('매일 UTC 15시 표 갱신 Cron 이 있고 그 경로의 라우트 파일이 있다', () => {
        expect(config.crons).toEqual([{ path: CRON_PATH, schedule: CRON_SCHEDULE }])
        expect(existsSync(CRON_ROUTE_FILE)).toBe(true)
    })

    test('Cron 라우트는 GET 을 내보내고 함수 시간 제한을 60초로 둔다', () => {
        const source = readFileSync(CRON_ROUTE_FILE, 'utf8')
        expect(source).toContain('export const GET =')
        expect(source).toContain('export const maxDuration = 60')
    })

    test('ignoreCommand 와 리전은 그대로이다', () => {
        expect(config.ignoreCommand).toBe(IGNORE_COMMAND)
        expect(config.framework).toBe('nextjs')
        expect(config.regions).toEqual(['icn1'])
    })
})
