import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { parsePurgeCommand } from '../../scripts/chart-blocklist-purge'
import { buildBlocklistSource, isChartFileName, parseBlocklistArgs } from '../../scripts/chart-blocklist'

const CHART_TEXT = '#TITLE sample\n#00111:01\n'
const OTHER_TEXT = '#TITLE other\n'
const DECOMPOSED_NAME = 'ユミル'.normalize('NFD')

describe('parseBlocklistArgs', () => {
    test('id·이름·폴더 세 인자를 읽는다', () => {
        expect(parseBlocklistArgs(['iidx-18-resort-anthem', ' beatmania IIDX 18 ', '/x'])).toEqual({
            id: 'iidx-18-resort-anthem',
            label: 'beatmania IIDX 18',
            folder: '/x',
        })
    })

    test('인자가 모자라거나 남거나 id 가 kebab-case 가 아니면 null 이다', () => {
        expect(parseBlocklistArgs(['iidx', 'x'])).toBeNull()
        expect(parseBlocklistArgs(['iidx', 'x', '/x', 'extra'])).toBeNull()
        expect(parseBlocklistArgs(['IIDX 18', 'x', '/x'])).toBeNull()
    })
})

describe('isChartFileName', () => {
    test('클라이언트가 읽는 확장자만 대소문자 없이 고르고 숨김 파일은 뺀다', () => {
        expect(['a.bms', 'a.BME', 'a.bml', 'a.pms', 'a.bmson'].every(isChartFileName)).toBe(true)
        expect(['a.ogg', 'a.dtx', 'a.txt', '._a.bms', '.a.bme'].some(isChartFileName)).toBe(false)
    })
})

describe('buildBlocklistSource', () => {
    let root = ''

    beforeAll(async () => {
        root = await mkdtemp(join(tmpdir(), 'chart-blocklist-'))
        await mkdir(join(root, `song ${DECOMPOSED_NAME}`))
        await mkdir(join(root, '.hidden'))
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'b [HYPER].BME'), CHART_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'a [NORMAL].bms'), OTHER_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'copy.bml'), CHART_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, '01.ogg'), 'audio')
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, '._a [NORMAL].bms'), 'apple double')
        await writeFile(join(root, '.hidden', 'x.bms'), 'hidden')
    })

    afterAll(async () => {
        await rm(root, { recursive: true, force: true })
    })

    test('원본 바이트의 md5·sha256 을 path 순으로 담고 같은 파일은 앞 경로 하나만 남긴다', async () => {
        const source = await buildBlocklistSource({ id: 'sample', label: 'Sample', folder: root })
        const folder = `song ${DECOMPOSED_NAME.normalize('NFC')}`
        expect(source.charts).toEqual([
            {
                md5: createHash('md5').update(OTHER_TEXT).digest('hex'),
                sha256: createHash('sha256').update(OTHER_TEXT).digest('hex'),
                path: `${folder}/a [NORMAL].bms`,
            },
            {
                md5: createHash('md5').update(CHART_TEXT).digest('hex'),
                sha256: createHash('sha256').update(CHART_TEXT).digest('hex'),
                path: `${folder}/b [HYPER].BME`,
            },
        ])
    })
})

describe('parsePurgeCommand', () => {
    test('count 와 delete 하나만 받는다', () => {
        expect(parsePurgeCommand(['count'])).toBe('count')
        expect(parsePurgeCommand(['delete'])).toBe('delete')
        expect(parsePurgeCommand(['delete', 'now'])).toBeUndefined()
        expect(parsePurgeCommand(['drop'])).toBeUndefined()
        expect(parsePurgeCommand([])).toBeUndefined()
    })
})
