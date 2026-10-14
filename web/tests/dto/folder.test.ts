import { describe, expect, test } from 'bun:test'
import {
    FOLDER_LIMIT,
    folderContainingParamSchema,
    folderCreateSchema,
    folderIdSchema,
    folderImportSchema,
    folderItemInputSchema,
    folderItemsAddSchema,
    folderItemsReplaceSchema,
    folderOrderSchema,
    folderUpdateSchema,
} from '@server/dto/folder'

const MD5 = '0123456789abcdef0123456789abcdef'

describe('folderCreateSchema', () => {
    test('이름을 NFC 로 고르고 앞뒤 공백을 뗀다', () => {
        const parsed = folderCreateSchema.parse({ name: '  é  ' })
        expect(parsed.name).toBe('é')
        expect(parsed.description).toBe('')
    })

    test('빈 이름, 공백뿐인 이름, 65자 이름, 제어 문자가 든 이름을 거부한다', () => {
        expect(folderCreateSchema.safeParse({ name: '' }).success).toBe(false)
        expect(folderCreateSchema.safeParse({ name: '   ' }).success).toBe(false)
        expect(folderCreateSchema.safeParse({ name: 'a'.repeat(FOLDER_LIMIT.NAME_MAX + 1) }).success).toBe(false)
        expect(folderCreateSchema.safeParse({ name: 'a\nb' }).success).toBe(false)
        expect(folderCreateSchema.safeParse({ name: 'a'.repeat(FOLDER_LIMIT.NAME_MAX) }).success).toBe(true)
    })

    test('설명은 255자까지이고 줄바꿈을 거부한다', () => {
        expect(folderCreateSchema.safeParse({ name: 'x', description: 'd'.repeat(256) }).success).toBe(false)
        expect(folderCreateSchema.safeParse({ name: 'x', description: 'a\nb' }).success).toBe(false)
    })
})

describe('folderUpdateSchema', () => {
    test('이름과 설명 중 하나는 있어야 한다', () => {
        expect(folderUpdateSchema.safeParse({}).success).toBe(false)
        expect(folderUpdateSchema.safeParse({ description: '' }).success).toBe(true)
        expect(folderUpdateSchema.safeParse({ name: 'x' }).success).toBe(true)
    })
})

describe('folderIdSchema', () => {
    test('uf_ 와 소문자 16진 30자만 받는다', () => {
        expect(folderIdSchema.safeParse(`uf_${'a'.repeat(30)}`).success).toBe(true)
        expect(folderIdSchema.safeParse(`uf_${'A'.repeat(30)}`).success).toBe(false)
        expect(folderIdSchema.safeParse(`uf_${'a'.repeat(29)}`).success).toBe(false)
        expect(folderIdSchema.safeParse('../etc/passwd').success).toBe(false)
        expect(folderIdSchema.safeParse(`xx_${'a'.repeat(30)}`).success).toBe(false)
    })
})

describe('folderItemInputSchema', () => {
    test('md5 를 소문자로 고르고 모양이 틀리면 거부한다', () => {
        expect(folderItemInputSchema.parse({ md5: ` ${MD5.toUpperCase()} ` }).md5).toBe(MD5)
        expect(folderItemInputSchema.safeParse({ md5: 'xyz' }).success).toBe(false)
        expect(folderItemInputSchema.safeParse({ md5: MD5.slice(1) }).success).toBe(false)
    })

    test('sha256 은 비어 있거나 64자 16진이다', () => {
        expect(folderItemInputSchema.parse({ md5: MD5 }).sha256).toBe('')
        expect(folderItemInputSchema.safeParse({ md5: MD5, sha256: 'a'.repeat(64) }).success).toBe(true)
        expect(folderItemInputSchema.safeParse({ md5: MD5, sha256: 'a'.repeat(63) }).success).toBe(false)
    })

    test('긴 제목은 거부하지 않고 서비스가 자른다', () => {
        expect(folderItemInputSchema.safeParse({ md5: MD5, title: 't'.repeat(5000) }).success).toBe(true)
    })

    test('메모는 100자까지이다', () => {
        expect(folderItemInputSchema.safeParse({ md5: MD5, note: 'n'.repeat(101) }).success).toBe(false)
    })
})

describe('목록 스키마', () => {
    test('곡 넣기는 1~500개이다', () => {
        expect(folderItemsAddSchema.safeParse({ items: [] }).success).toBe(false)
        expect(folderItemsAddSchema.safeParse({ items: Array.from({ length: 501 }, () => ({ md5: MD5 })) }).success).toBe(false)
        expect(folderItemsAddSchema.safeParse({ items: Array.from({ length: 500 }, () => ({ md5: MD5 })) }).success).toBe(true)
    })

    test('통째로 바꾸기는 0~2000개이고 base_updated_at 은 선택이다', () => {
        expect(folderItemsReplaceSchema.safeParse({ items: [] }).success).toBe(true)
        expect(folderItemsReplaceSchema.safeParse({ items: [], base_updated_at: -1 }).success).toBe(false)
        expect(folderItemsReplaceSchema.safeParse({ items: Array.from({ length: 2001 }, () => ({ md5: MD5 })) }).success).toBe(false)
    })

    test('폴더 순서는 최대 50개이다', () => {
        const ids = Array.from({ length: 51 }, (_, n) => `uf_${n.toString(16).padStart(30, '0')}`)
        expect(folderOrderSchema.safeParse({ folder_ids: ids }).success).toBe(false)
        expect(folderOrderSchema.safeParse({ folder_ids: ids.slice(0, 50) }).success).toBe(true)
    })

    test('가져오기는 표 id 가 필수이고 레벨은 100개까지이다', () => {
        expect(folderImportSchema.safeParse({}).success).toBe(false)
        expect(folderImportSchema.safeParse({ table_id: 'insane1', levels: ['1', '2'] }).success).toBe(true)
        expect(folderImportSchema.safeParse({ table_id: 'insane1', levels: Array.from({ length: 101 }, () => '1') }).success).toBe(false)
    })

    test('경로의 md5 는 소문자 16진 32자이다', () => {
        expect(folderContainingParamSchema.safeParse({ md5: MD5 }).success).toBe(true)
        expect(folderContainingParamSchema.safeParse({ md5: 'order' }).success).toBe(false)
    })
})
