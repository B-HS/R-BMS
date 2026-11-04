import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { folderCreateSchema, folderDetailSchema, folderItemInputSchema } from '@server/dto/folder'
import { createFolderService } from '@server/service/domain/folder/folder.service'
import { createMemoryFolderDb } from '../support/folder-memory-db'

/**
 * `contract/server-responses/user-folder-detail.json` holds the two item shapes the server sends a client (plan 2026-11-04 5.2): a
 * training chart with `download_url` and an ordinary item without it. The Rust DTO test reads the same file.
 */
const FIXTURE_PATH = join(import.meta.dirname, '..', '..', '..', 'contract', 'server-responses', 'user-folder-detail.json')
const ORIGIN = 'https://example.test'

const readFixture = () => folderDetailSchema.parse(JSON.parse(readFileSync(FIXTURE_PATH, 'utf8')))

describe('폴더 상세 응답 계약 (contract/server-responses/user-folder-detail.json)', () => {
    test('픽스처는 folderDetailSchema 를 통과하고 첫 항목만 download_url 을 갖는다', () => {
        const fixture = readFixture()
        expect(fixture.items).toHaveLength(2)
        expect(fixture.items[0]?.download_url).toStartWith(`${ORIGIN}/api/training/tc_`)
        expect(fixture.items[1]).not.toHaveProperty('download_url')
    })

    test('메모리 db 로 만든 서비스 출력이 픽스처와 같은 모양이다', async () => {
        const fixture = readFixture()
        const [training, ordinary] = fixture.items
        const trainingId = /\/api\/training\/(tc_[0-9a-f]{32})\/chart\.zip$/.exec(training?.download_url ?? '')?.[1] ?? ''
        const userId = 'user-owner'
        const memory = createMemoryFolderDb([], () => [{ userId, md5: training?.md5 ?? '', id: trainingId }])
        const service = createFolderService({
            db: memory.db,
            newId: () => fixture.id,
            now: () => fixture.updated_at - 1,
            loadTableEntries: async () => null,
            lookupTableText: () => null,
        })
        const created = await service.create({ userId, input: folderCreateSchema.parse({ name: fixture.name, description: fixture.description }) })
        expect(created.ok).toBe(true)
        await service.addItems({
            userId,
            folderId: fixture.id,
            items: fixture.items.map((item) => folderItemInputSchema.parse(item)),
        })
        const detail = await service.get({ userId, folderId: fixture.id, origin: ORIGIN })
        expect(detail).toEqual(fixture)
        expect(folderDetailSchema.parse(detail).items.map((item) => item.download_url)).toEqual([training?.download_url, ordinary?.download_url])
    })
})
