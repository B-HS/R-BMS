import { describe, expect, test } from 'bun:test'
import { assembleTable, folderIdFor } from '@server/service/domain/table/table.service'

const TABLE = { id: 'insane', name: '発狂BMS難易度表', url: null, extra: null }

describe('assembleTable', () => {
    test('폴더를 position 순으로 정렬하고 자기 차트만 담는다', () => {
        const table = assembleTable({
            table: TABLE,
            folders: [
                { id: 'f2', name: '★2', position: 1 },
                { id: 'f1', name: '★1', position: 0 },
            ],
            charts: [
                { folderId: 'f2', chartSha256: 'b'.repeat(64), md5: null },
                { folderId: 'f1', chartSha256: 'a'.repeat(64), md5: 'c'.repeat(32) },
            ],
            courses: [],
        })
        expect(table.folders.map((folder) => folder.name)).toEqual(['★1', '★2'])
        expect(table.folders[0].charts).toEqual([{ md5: 'c'.repeat(32), sha256: 'a'.repeat(64) }])
        expect(table.folders[1].charts).toEqual([{ md5: '', sha256: 'b'.repeat(64) }])
    })

    test('폴더가 없으면 빈 목록과 기본 extra 를 낸다', () => {
        const table = assembleTable({ table: TABLE, folders: [], charts: [], courses: [] })
        expect(table.folders).toEqual([])
        expect(table.extra).toEqual({})
    })
})

describe('folderIdFor', () => {
    test('표 id 와 위치로 폴더 id 를 만든다', () => {
        expect(folderIdFor('insane', 3)).toBe('insane#3')
    })
})
