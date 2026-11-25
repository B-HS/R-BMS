import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { parsePurgeCommand } from '../../scripts/chart-blocklist-purge'
import {
    checkBlocklist,
    deriveSourceName,
    isChartFileName,
    mergeSource,
    parseBlocklistCommand,
    renderDataModule,
    scanFolder,
} from '../../scripts/chart-blocklist'

const CHART_TEXT = '#TITLE sample\n#00111:01\n'
const OTHER_TEXT = '#TITLE other\n'
const DECOMPOSED_NAME = 'ユミル'.normalize('NFD')

const entry = (seed: string, path: string) => ({ md5: seed.repeat(32), sha256: seed.repeat(64), path })

describe('parseBlocklistCommand', () => {
    test('add 는 폴더 여럿과 --publish 를 받고 bun run 의 -- 를 건너뛴다', () => {
        expect(parseBlocklistCommand(['add', '--', '/a', '/b', '--publish'])).toEqual({
            kind: 'add',
            folders: ['/a', '/b'],
            id: null,
            label: null,
            shouldPublish: true,
        })
        expect(parseBlocklistCommand(['check'])).toEqual({ kind: 'check' })
    })

    test('--id·--label 은 폴더 하나일 때만, id 는 kebab-case 만 받는다', () => {
        expect(parseBlocklistCommand(['add', '/a', '--id', 'iidx-20-tricoro', '--label', ' beatmania IIDX 20 tricoro '])).toMatchObject({
            id: 'iidx-20-tricoro',
            label: 'beatmania IIDX 20 tricoro',
        })
        expect(parseBlocklistCommand(['add', '/a', '/b', '--id', 'x'])).toBeNull()
        expect(parseBlocklistCommand(['add', '/a', '--id', 'IIDX 20'])).toBeNull()
        expect(parseBlocklistCommand(['add'])).toBeNull()
        expect(parseBlocklistCommand(['add', '/a', '--force'])).toBeNull()
        expect(parseBlocklistCommand(['check', 'x'])).toBeNull()
        expect(parseBlocklistCommand([])).toBeNull()
    })
})

describe('deriveSourceName', () => {
    test('IIDX 폴더 이름에서 버전과 부제를 뽑는다', () => {
        expect(deriveSourceName('Beatmania IIDX18 ~Resort Anthem~')).toEqual({
            id: 'iidx-18-resort-anthem',
            label: 'beatmania IIDX 18 Resort Anthem',
        })
        expect(deriveSourceName('Beatmania IIDX19 ~Lincle~')).toEqual({ id: 'iidx-19-lincle', label: 'beatmania IIDX 19 Lincle' })
        expect(deriveSourceName('beatmania IIDX 25 CANNON BALLERS')).toEqual({
            id: 'iidx-25-cannon-ballers',
            label: 'beatmania IIDX 25 CANNON BALLERS',
        })
        expect(deriveSourceName('beatmania IIDX 9th style')).toEqual({ id: 'iidx-9-style', label: 'beatmania IIDX 9th style' })
    })

    test('IIDX 가 아닌 이름은 이름 그대로를 kebab-case 로, 만들 수 없으면 null', () => {
        expect(deriveSourceName('Pop n Music 20 fantasia')).toEqual({ id: 'pop-n-music-20-fantasia', label: 'Pop n Music 20 fantasia' })
        expect(deriveSourceName('ユミル')).toBeNull()
    })
})

describe('isChartFileName', () => {
    test('클라이언트가 읽는 확장자만 대소문자 없이 고르고 숨김 파일은 뺀다', () => {
        expect(['a.bms', 'a.BME', 'a.bml', 'a.pms', 'a.bmson'].every(isChartFileName)).toBe(true)
        expect(['a.ogg', 'a.dtx', 'a.txt', '._a.bms', '.a.bme'].some(isChartFileName)).toBe(false)
    })
})

describe('scanFolder', () => {
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
        const folder = `song ${DECOMPOSED_NAME.normalize('NFC')}`
        expect(await scanFolder(root)).toEqual([
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

describe('mergeSource', () => {
    const other = { id: 'other', label: 'Other', charts: [entry('a', 'x/a.bms')] }
    const existing = { id: 'pack', label: 'Pack (first)', charts: [entry('b', 'p/b.bms')] }

    test('다른 목록에 있는 차트는 빼고, 같은 id 의 목록에 합치며 그 이름을 지킨다', () => {
        const scanned = [entry('a', 'p/a.bms'), entry('b', 'p/b.bms'), entry('c', 'p/c.bms')]
        const merged = mergeSource({ id: 'pack', label: 'Pack (second)', scanned, sources: [other, existing] })
        expect(merged).toEqual({
            source: { id: 'pack', label: 'Pack (first)', charts: [entry('b', 'p/b.bms'), entry('c', 'p/c.bms')] },
            added: 1,
            alreadyListed: 1,
        })
    })

    test('md5 만 다른 목록에 있어도 이미 있는 차트로 본다', () => {
        const scanned = [{ ...entry('d', 'p/d.bms'), md5: 'a'.repeat(32) }]
        expect(mergeSource({ id: 'new', label: 'New', scanned, sources: [other] })).toMatchObject({ added: 0, alreadyListed: 1 })
    })
})

describe('renderDataModule·checkBlocklist', () => {
    let dir = ''

    beforeAll(async () => {
        dir = await mkdtemp(join(tmpdir(), 'chart-blocklist-module-'))
        await mkdir(join(dir, 'lists'))
        await writeFile(
            join(dir, 'lists', 'iidx-20-tricoro.json'),
            JSON.stringify({ id: 'iidx-20-tricoro', label: 'T', charts: [entry('a', 'a.bms')] }),
        )
    })

    afterAll(async () => {
        await rm(dir, { recursive: true, force: true })
    })

    test('목록마다 import 하나와 배열 한 칸을 id 순으로 쓴다', async () => {
        const text = await renderDataModule(['iidx-20-tricoro', 'iidx-18-resort-anthem'])
        expect(text).toContain("import iidx18ResortAnthem from '../../../../../data/chart-blocklist/iidx-18-resort-anthem.json'")
        expect(text).toContain('CHART_BLOCKLIST_RAW_SOURCES: readonly unknown[] = [iidx18ResortAnthem, iidx20Tricoro]')
    })

    test('모듈이 파일 목록과 다르면 알리고 같으면 빈 목록이다', async () => {
        const modulePath = join(dir, 'module.ts')
        await writeFile(modulePath, await renderDataModule([]))
        expect(await checkBlocklist(join(dir, 'lists'), modulePath)).toHaveLength(1)
        await writeFile(modulePath, await renderDataModule(['iidx-20-tricoro']))
        expect(await checkBlocklist(join(dir, 'lists'), modulePath)).toEqual([])
    })

    test('저장소의 목록 모듈은 생성기가 쓰는 것과 같다', async () => {
        expect(await checkBlocklist()).toEqual([])
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
