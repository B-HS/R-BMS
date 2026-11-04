import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { eq, like } from 'drizzle-orm'
import { composeFolder } from '@server/compose/folder'
import { composeTableSnapshot } from '@server/compose/table-snapshot'
import { createTrainingSectionsOf } from '@server/compose/trace'
import { createTrainingChartDb } from '@server/compose/training'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { playTrace, replay, score, trainingChart, trainingFolder, userFolder } from '@server/db/schema'
import { TRAINING_LIMIT } from '@server/dto/training'
import { trainingSpecSchema } from '@server/service/domain/training/spec'
import type { TrainingChartRow } from '@server/service/domain/training/training.service'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 훈련곡 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'u7tc'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const NOW = Date.UTC(2026, 10, 4, 12)
const GOLDEN = join(import.meta.dirname, '..', '..', '..', 'contract', 'training-chart', 'basic-160.spec.json')
const SPEC = trainingSpecSchema.parse(JSON.parse(readFileSync(GOLDEN, 'utf8')).spec)

const hashOf = (algorithm: 'md5' | 'sha256', value: string) => createHash(algorithm).update(`${PREFIX}:${value}`).digest('hex')

const idOf = (index: number) => `tc_e7e7${index.toString(16).padStart(28, '0')}`

const rowOf = (index: number, createdAt = NOW + index): TrainingChartRow => ({
    id: idOf(index),
    md5: hashOf('md5', `chart-${index}`),
    sha256: hashOf('sha256', `chart-${index}`),
    title: 'R-BMS Training · 2026-11-04 · PUSH',
    playLevel: 9,
    bpm: SPEC.base_bpm,
    noteCount: 300,
    lengthMs: 58_000,
    generatorVersion: SPEC.generator_version,
    spec: SPEC,
    request: {
        focus_key: 'auto',
        intensity: 'push',
        plan: {
            description: 'd',
            base_bpm: 160,
            level: 0.5,
            shape: 'flat',
            sections: [
                {
                    focus_key: 'warmup',
                    kind: 'stream',
                    weight: 1,
                    lane_hint: 'all',
                    chord_size: 1,
                    scratch_ratio: 0,
                    ln_ratio: 0,
                    bpm_ratio: 1,
                    intent: 'i',
                },
            ],
            advice: 'a',
        },
        advice: 'a',
        description: 'd',
    },
    profileSnapshot: { profile_version: 1, fingerprint: 'f'.repeat(64), computed_at: NOW, targets: {} },
    provider: 'anthropic',
    model: 'claude-test',
    createdAt,
})

const BMS = new TextEncoder().encode('#TITLE test\n')

describe.skipIf(!hasDatabase)('훈련곡 저장 (임시 MySQL)', () => {
    const cleanup = async () => {
        const db = getDb()
        await db.delete(playTrace).where(like(playTrace.userId, `${PREFIX}_%`))
        await db.delete(replay).where(like(replay.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        await cleanup()
        await getDb()
            .insert(user)
            .values([OWNER, OTHER].map((id) => ({ id, name: id, email: `${id}@u7.invalid`, loginId: id })))
    })

    beforeEach(async () => {
        const db = getDb()
        await db.delete(trainingChart).where(like(trainingChart.userId, `${PREFIX}_%`))
        await db.delete(trainingFolder).where(like(trainingFolder.userId, `${PREFIX}_%`))
        await db.delete(userFolder).where(like(userFolder.userId, `${PREFIX}_%`))
    })

    afterAll(cleanup)

    test('19곡에서 동시에 두 곡을 저장하면 정확히 하나만 들어가고 다른 하나는 limit 이다', async () => {
        const store = createTrainingChartDb(getDb())
        for (let index = 0; index < TRAINING_LIMIT.CHARTS_PER_USER - 1; index += 1) {
            expect(await store.insertChart({ ...rowOf(index), userId: OWNER, bms: BMS })).toBe('inserted')
        }
        const outcomes = await Promise.all([
            store.insertChart({ ...rowOf(100), userId: OWNER, bms: BMS }),
            store.insertChart({ ...rowOf(101), userId: OWNER, bms: BMS }),
        ])
        expect(outcomes.toSorted()).toEqual(['inserted', 'limit'])
        expect(await store.countCharts(OWNER)).toBe(TRAINING_LIMIT.CHARTS_PER_USER)
        expect(await store.insertChart({ ...rowOf(200), userId: OTHER, bms: BMS })).toBe('inserted')
    })

    test('목록은 최신순이고 상세·지우기는 주인만, 받기는 id 만으로 원 바이트를 준다', async () => {
        const store = createTrainingChartDb(getDb())
        await store.insertChart({ ...rowOf(1), userId: OWNER, bms: BMS })
        await store.insertChart({ ...rowOf(2), userId: OWNER, bms: BMS })
        expect((await store.listCharts(OWNER)).map((row) => row.id)).toEqual([idOf(2), idOf(1)])
        expect(await store.findChart({ id: idOf(1), userId: OWNER })).toEqual(rowOf(1))
        expect(await store.findChart({ id: idOf(1), userId: OTHER })).toBeNull()
        expect(Buffer.from((await store.findDownload(idOf(1)))?.bms ?? []).equals(Buffer.from(BMS))).toBe(true)
        expect(await store.deleteChart({ id: idOf(1), userId: OTHER })).toBe(false)
        expect(await store.deleteChart({ id: idOf(1), userId: OWNER })).toBe(true)
        expect(await store.findDownload(idOf(1))).toBeNull()
    })

    test('폴더 연결은 먼저 잡은 쪽이 이기고, 폴더를 지우면 연결도 사라진다', async () => {
        const db = getDb()
        const store = createTrainingChartDb(db)
        const [first, second] = [`uf_${'1'.repeat(30)}`, `uf_${'2'.repeat(30)}`]
        await db.insert(userFolder).values([
            { id: first, userId: OWNER, name: 'u7 a' },
            { id: second, userId: OWNER, name: 'u7 b' },
        ])
        expect(await store.claimFolderLink({ userId: OWNER, folderId: first, createdAt: NOW })).toBe(first)
        expect(await store.claimFolderLink({ userId: OWNER, folderId: second, createdAt: NOW })).toBe(first)
        expect(await store.findFolderLink(OWNER)).toBe(first)
        await db.delete(userFolder).where(eq(userFolder.id, first))
        expect(await store.findFolderLink(OWNER)).toBeNull()
    })

    test('판 집계·판 목록·지울 때 남는 개수를 sha256 과 주인으로 센다', async () => {
        const db = getDb()
        const store = createTrainingChartDb(db)
        const row = rowOf(5)
        await store.insertChart({ ...row, userId: OWNER, bms: BMS })
        const scoreOf = (name: string, userId: string, exScore: number, playedAt: number) => ({
            id: `${PREFIX}_${name}`,
            userId,
            chartSha256: row.sha256,
            chartMd5: row.md5,
            exScore,
            maxExScore: 600,
            notes: 300,
            minbp: 4,
            playedAt,
            random: 'MIRROR',
        })
        await db.insert(score).values([scoreOf('a', OWNER, 300, NOW + 10), scoreOf('b', OWNER, 450, NOW + 20), scoreOf('c', OTHER, 600, NOW + 30)])
        await db.insert(replay).values({ id: `${PREFIX}_r1`, userId: OWNER, chartSha256: row.sha256, scoreId: `${PREFIX}_a` })
        await db.insert(playTrace).values({
            id: `${PREFIX}_t1`,
            scoreId: `${PREFIX}_a`,
            userId: OWNER,
            chartSha256: row.sha256,
            chartMd5: row.md5,
            playedAt: NOW + 10,
            formatVersion: 1,
            noteCount: 300,
            byteSize: 10,
            storedSize: 10,
            data: Buffer.from('x'),
            createdAt: NOW,
        })
        expect(await store.listPlayStats({ userId: OWNER, sha256s: [row.sha256] })).toEqual([
            { sha256: row.sha256, plays: 2, bestExRatio: 0.75, lastPlayedAt: NOW + 20 },
        ])
        expect(
            (await store.listPlays({ userId: OWNER, sha256: row.sha256, limit: 1 })).map((play) => [play.scoreId, play.maxExScore, play.random]),
        ).toEqual([[`${PREFIX}_b`, 600, 'MIRROR']])
        expect(await store.countKept({ userId: OWNER, sha256: row.sha256 })).toEqual({ scores: 2, traces: 1, replays: 1 })
        expect(await store.countKept({ userId: OTHER, sha256: row.sha256 })).toEqual({ scores: 1, traces: 0, replays: 0 })
    })

    test('판 분석은 sha256 으로 훈련곡 구간을 찾고, 지운 뒤에는 null 이다', async () => {
        const db = getDb()
        const store = createTrainingChartDb(db)
        const row = rowOf(7)
        await store.insertChart({ ...row, userId: OWNER, bms: BMS })
        const sectionsOf = createTrainingSectionsOf(db)
        expect(await sectionsOf(row.sha256)).toEqual(
            SPEC.sections.map((section) => ({
                index: section.index,
                kind: section.kind,
                focus_key: section.focus_key,
                from_us: section.from_us,
                to_us: section.to_us,
            })),
        )
        await store.deleteChart({ id: row.id, userId: OWNER })
        expect(await sectionsOf(row.sha256)).toBeNull()
    })

    test('폴더 항목은 그 폴더 주인이 만든 훈련곡의 id 를 함께 읽는다', async () => {
        const db = getDb()
        const store = createTrainingChartDb(db)
        const row = rowOf(9)
        await store.insertChart({ ...row, userId: OWNER, bms: BMS })
        const { folderService } = composeFolder(db, {
            tableSnapshotService: composeTableSnapshot(db).tableSnapshotService,
            lookupTableText: () => null,
        })
        const items = [{ md5: row.md5, sha256: row.sha256, title: row.title, artist: 'R-BMS', note: '' }]
        const download = `https://example.test/api/training/${row.id}/chart.zip`
        for (const userId of [OWNER, OTHER]) {
            const created = await folderService.create({ userId, input: { name: 'u7 training', description: '' } })
            if (!created.ok) return expect.unreachable(created.reason)
            await folderService.addItems({ userId, folderId: created.value.id, items })
            const detail = await folderService.get({ userId, folderId: created.value.id, origin: 'https://example.test' })
            expect(detail?.items[0]?.download_url).toBe(userId === OWNER ? download : undefined)
        }
    })
})
