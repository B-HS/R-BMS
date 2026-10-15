import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, like } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { course, courseChart, difficultyTable, tableChart, tableCourse, tableFolder } from '@server/db/schema'
import { INT_COLUMN_MAX } from '@server/dto/common'
import { COURSE_NAME_MAX_LENGTH, courseMetaUpsertSchema } from '@server/dto/course'
import {
    folderIdFor,
    TABLE_FOLDER_ID_MAX_LENGTH,
    TABLE_FOLDER_NAME_MAX_LENGTH,
    TABLE_NAME_MAX_LENGTH,
    TABLE_URL_MAX_LENGTH,
    tableUpsertSchema,
} from '@server/dto/table'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 관리자 메타 열 경계 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19a'
const COURSE_HASH = `${PREFIX}-course`
const TABLE_ID = PREFIX.padEnd(TABLE_FOLDER_ID_MAX_LENGTH - folderIdFor('', 0).length, 't')
const MULTI_BYTE_CHARACTER = '段'

const cleanUp = async () => {
    const db = getDb()
    await db.delete(courseChart).where(eq(courseChart.courseHash, COURSE_HASH))
    await db.delete(course).where(eq(course.courseHash, COURSE_HASH))
    await db.delete(tableChart).where(like(tableChart.folderId, `${PREFIX}%`))
    await db.delete(tableFolder).where(eq(tableFolder.tableId, TABLE_ID))
    await db.delete(tableCourse).where(eq(tableCourse.tableId, TABLE_ID))
    await db.delete(difficultyTable).where(eq(difficultyTable.id, TABLE_ID))
}

describe.skipIf(!hasDatabase)('관리자 코스 메타와 난이도표의 열 경계 (임시 MySQL)', () => {
    beforeAll(cleanUp)
    afterAll(cleanUp)

    test('스키마가 받는 가장 긴 코스 이름과 가장 큰 lntype 을 DB 가 그대로 저장한다', async () => {
        const name = MULTI_BYTE_CHARACTER.repeat(COURSE_NAME_MAX_LENGTH)
        const meta = courseMetaUpsertSchema.parse({ course_hash: COURSE_HASH, name, lntype: INT_COLUMN_MAX })
        const stored = await compose().courseService.upsertMeta(meta)
        expect(stored).toMatchObject({ course_hash: COURSE_HASH, name, lntype: INT_COLUMN_MAX })
    })

    test('스키마가 받는 가장 긴 표 id·이름·주소·폴더 이름을 DB 가 그대로 저장한다', async () => {
        const input = tableUpsertSchema.parse({
            id: TABLE_ID,
            name: MULTI_BYTE_CHARACTER.repeat(TABLE_NAME_MAX_LENGTH),
            url: 'u'.repeat(TABLE_URL_MAX_LENGTH),
            folders: [{ name: MULTI_BYTE_CHARACTER.repeat(TABLE_FOLDER_NAME_MAX_LENGTH) }],
        })
        const stored = await compose().tableService.upsert(input)
        expect(stored).toMatchObject({ id: TABLE_ID, name: input.name, url: input.url, folders: [{ name: input.folders[0].name }] })
    })
})
