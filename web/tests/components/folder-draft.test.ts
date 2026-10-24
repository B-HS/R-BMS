import { describe, expect, test } from 'bun:test'
import { createTranslator } from 'next-intl'
import { FOLDER_LIMIT, type FolderDetail, type FolderItem } from '../../src/entities/folder.type'
import { MESSAGES } from '../../src/shared/i18n/messages'
import {
    appendFolderItems,
    buildLevelLabelByMd5,
    countFolderDraftChanges,
    describeFolderDraftChanges,
    moveFolderItem,
    summariseFolderConflict,
    toFolderDraft,
    toReplaceItems,
} from '../../src/widgets/folder/folder-draft'

const changesTranslator = (locale: keyof typeof MESSAGES) =>
    createTranslator({ locale, messages: MESSAGES[locale], namespace: 'browse.folder.changes' })

const md5Of = (seed: number) => seed.toString(16).padStart(32, '0')

const item = (seed: number, overrides: Partial<FolderItem> = {}): FolderItem => ({
    md5: md5Of(seed),
    sha256: '',
    title: `곡 ${seed}`,
    artist: `작곡 ${seed}`,
    note: '',
    ...overrides,
})

const folder = (items: FolderItem[], overrides: Partial<FolderDetail> = {}): FolderDetail => ({
    id: 'uf_000000000000000000000000000001',
    name: '연습',
    description: '',
    is_public: false,
    position: 0,
    item_count: items.length,
    updated_at: 1000,
    items,
    ...overrides,
})

describe('폴더 편집 초안의 변경 세기', () => {
    test('같은 내용이면 변경이 0건이다', () => {
        const base = toFolderDraft(folder([item(1), item(2)]))
        expect(countFolderDraftChanges(base, { ...base }).total).toBe(0)
    })

    test('추가·빼기·순서·메모·이름을 따로 센다', () => {
        const base = toFolderDraft(folder([item(1), item(2), item(3)]))
        const draft = { ...base, name: '새 이름', items: [item(3), item(1, { note: '느림' }), item(4)] }
        const changes = countFolderDraftChanges(base, draft)
        expect(changes).toMatchObject({ added: 1, removed: 1, notes: 1, isReordered: true, isRenamed: true, isDescriptionChanged: false })
        expect(changes.total).toBe(5)
        expect(describeFolderDraftChanges(changes, changesTranslator('ko'))).toBe('추가 1 · 빼기 1 · 순서 · 메모 1 · 이름')
        expect(describeFolderDraftChanges(changes, changesTranslator('en'))).toBe('Added 1 · Removed 1 · Order · Notes 1 · Name')
        expect(describeFolderDraftChanges(changes, changesTranslator('ja'))).toBe('追加 1 · 削除 1 · 順序 · メモ 1 · 名前')
    })

    test('빠진 곡 때문에 남은 곡의 상대 순서가 같으면 순서 변경으로 세지 않는다', () => {
        const base = toFolderDraft(folder([item(1), item(2), item(3)]))
        const changes = countFolderDraftChanges(base, { ...base, items: [item(1), item(3)] })
        expect(changes.isReordered).toBe(false)
        expect(changes.total).toBe(1)
    })
})

describe('곡 옮기기', () => {
    const items = [item(1), item(2), item(3), item(4)]

    test('위·아래·맨 위·맨 아래로 옮긴다', () => {
        expect(moveFolderItem(items, 2, 'up').map((value) => value.md5)).toEqual([1, 3, 2, 4].map(md5Of))
        expect(moveFolderItem(items, 1, 'down').map((value) => value.md5)).toEqual([1, 3, 2, 4].map(md5Of))
        expect(moveFolderItem(items, 3, 'top').map((value) => value.md5)).toEqual([4, 1, 2, 3].map(md5Of))
        expect(moveFolderItem(items, 0, 'bottom').map((value) => value.md5)).toEqual([2, 3, 4, 1].map(md5Of))
    })

    test('끝에서 더 옮기면 그대로 둔다', () => {
        expect(moveFolderItem(items, 0, 'up')).toBe(items)
        expect(moveFolderItem(items, 3, 'down')).toBe(items)
    })
})

describe('곡 넣기', () => {
    test('이미 든 곡과 같은 요청 안의 중복은 건너뛴다', () => {
        const result = appendFolderItems([item(1)], [item(1), item(2), item(2)])
        expect(result.added).toBe(1)
        expect(result.skipped).toBe(2)
        expect(result.items.map((value) => value.md5)).toEqual([md5Of(1), md5Of(2)])
    })

    test('폴더 한도에서 멈춘다', () => {
        const full = Array.from({ length: FOLDER_LIMIT.ITEMS_PER_FOLDER - 1 }, (_, index) => item(index + 1))
        const result = appendFolderItems(full, [item(900000), item(900001)])
        expect(result.added).toBe(1)
        expect(result.items).toHaveLength(FOLDER_LIMIT.ITEMS_PER_FOLDER)
    })
})

describe('저장 본문', () => {
    test('저장된 곡은 md5 와 메모만, 새 곡은 글자와 올바른 sha256 을 함께 보낸다', () => {
        const base = toFolderDraft(folder([item(1)]))
        const longTitle = 'あ'.repeat(FOLDER_LIMIT.TEXT_COPY_MAX + 10)
        const draft = {
            ...base,
            items: [item(1, { note: '메모' }), item(2, { title: longTitle, sha256: 'xyz' }), item(3, { sha256: 'a'.repeat(64) })],
        }
        const body = toReplaceItems(base, draft)
        expect(body[0]).toEqual({ md5: md5Of(1), note: '메모' })
        expect(body[1]).toEqual({ md5: md5Of(2), title: 'あ'.repeat(FOLDER_LIMIT.TEXT_COPY_MAX), artist: '작곡 2', note: '' })
        expect(body[2]).toMatchObject({ md5: md5Of(3), sha256: 'a'.repeat(64) })
    })
})

describe('충돌 요약', () => {
    test('서버에만 있는 곡과 화면에만 있는 곡, 이름 차이를 알려 준다', () => {
        const draft = toFolderDraft(folder([item(1), item(2), item(5)]))
        const server = folder([item(2), item(1), item(3)], { name: '다른 이름', updated_at: 2000 })
        expect(summariseFolderConflict(draft, server)).toEqual({
            serverName: '다른 이름',
            serverItemCount: 3,
            serverUpdatedAt: 2000,
            onlyMine: 1,
            onlyServer: 1,
            isOrderDifferent: true,
            isNameDifferent: true,
            isDescriptionDifferent: false,
        })
    })
})

describe('레벨 표시', () => {
    test('여러 표에 같은 곡이 있으면 앞의 표를 쓴다', () => {
        const labels = buildLevelLabelByMd5([
            { id: 'a', name: 'A', symbol: '★', levels: [{ level: '12', entries: [{ md5: md5Of(1), title: '', artist: '' }] }] },
            { id: 'b', name: 'B', symbol: '▼', levels: [{ level: '3', entries: [{ md5: md5Of(1), title: '', artist: '' }] }] },
        ])
        expect(labels.get(md5Of(1))).toBe('★12')
    })
})
