import { describe, expect, test } from 'bun:test'
import { join } from 'node:path'
import { REFRESH_MODE, exitCodeFor, formatReport, parseRefreshMode } from '../../scripts/refresh-tables'
import type { TableRefreshReport } from '@server/service/domain/table-refresh/table-refresh.service'

const WEB_DIR = join(import.meta.dirname, '..', '..')
const FAILED_EXIT_CODE = 1
const USAGE_EXIT_CODE = 2

const reportOf = (overrides: Partial<TableRefreshReport> = {}): TableRefreshReport => ({
    tableId: 'stella-st',
    outcome: 'saved',
    reason: null,
    entries: 2350,
    added: 3,
    removed: 1,
    releveled: 2,
    courses: null,
    awarded: null,
    ...overrides,
})

describe('refresh-tables 스크립트', () => {
    test('count 와 apply-large 만 알아듣고 나머지는 사용법 오류이다', () => {
        expect(parseRefreshMode('count')).toBe(REFRESH_MODE.COUNT)
        expect(parseRefreshMode('apply-large')).toBe(REFRESH_MODE.APPLY_LARGE)
        for (const argument of [undefined, '', 'apply', 'COUNT', '--force']) expect(parseRefreshMode(argument)).toBeNull()
    })

    test('한 줄에 표 id, 고정 코드, 개수만 찍는다', () => {
        expect(formatReport(reportOf())).toBe('stella-st saved - entries=2350 added=3 removed=1 releveled=2')
        expect(
            formatReport(reportOf({ outcome: 'failed', reason: 'fetch_failed', entries: null, added: null, removed: null, releveled: null })),
        ).toBe('stella-st failed fetch_failed entries=- added=- removed=- releveled=-')
    })

    test('받지 못한 표가 있으면 어느 모드에서도 실패 코드 1 이다', () => {
        const reports = [reportOf(), reportOf({ tableId: 'stella-sl', outcome: 'failed', reason: 'fetch_failed' })]
        expect(exitCodeFor(REFRESH_MODE.COUNT, reports)).toBe(1)
        expect(exitCodeFor(REFRESH_MODE.APPLY_LARGE, reports)).toBe(1)
    })

    test('거절은 count 에서는 숫자를 보여 줄 뿐 0 이고, apply-large 에서는 1 이다', () => {
        const reports = [reportOf({ outcome: 'rejected', reason: 'shrank' })]
        expect(exitCodeFor(REFRESH_MODE.COUNT, reports)).toBe(0)
        expect(exitCodeFor(REFRESH_MODE.APPLY_LARGE, reports)).toBe(1)
    })

    test('저장됐거나 그대로인 표만 있으면 0 이다', () => {
        const reports = [reportOf(), reportOf({ tableId: 'stella-sl', outcome: 'unchanged' })]
        expect(exitCodeFor(REFRESH_MODE.COUNT, reports)).toBe(0)
        expect(exitCodeFor(REFRESH_MODE.APPLY_LARGE, reports)).toBe(0)
    })
})

describe('refresh-tables 실행', () => {
    const run = (...args: string[]) =>
        Bun.spawnSync([process.execPath, '--no-env-file', 'scripts/refresh-tables.ts', ...args], {
            cwd: WEB_DIR,
            env: { PATH: process.env.PATH ?? '', DATABASE_URL: '' },
        })

    test('알 수 없는 인자는 사용법을 찍고 종료 코드 2 이다', () => {
        const result = run('everything')
        expect(result.exitCode).toBe(USAGE_EXIT_CODE)
        expect(result.stderr.toString()).toContain('count')
    })

    test('접속 정보가 없으면 아무것도 받지 않고 종료 코드 1 이다', () => {
        const result = run('count')
        expect(result.exitCode).toBe(FAILED_EXIT_CODE)
        expect(result.stdout.toString()).toBe('')
    })
})
