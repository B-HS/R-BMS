import { afterEach, describe, expect, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import {
    FOLDER_LIMIT as SERVER_FOLDER_LIMIT,
    folderCreateSchema,
    type FolderDetailOutput,
    type FolderItemChangeOutput,
} from '../../src/server/dto/folder'
import { folderContainingQueryOptions, folderDetailQueryOptions, folderListQueryOptions } from '../../src/entities/folder.query'
import {
    FOLDER_LIMIT,
    folderMetaFormSchema,
    isFolderId,
    toFolderErrorMessage,
    toNewFolderItemInput,
    type FolderDetail,
    type FolderItemChange,
} from '../../src/entities/folder.type'
import { removeUserScopedQueries } from '../../src/entities/auth.query'
import { QUERY_KEY } from '../../src/shared/constants/query-key'
import { ApiError } from '../../src/shared/lib/fetch'

const detailFromServer: FolderDetail = {} as FolderDetailOutput
const detailToServer: FolderDetailOutput = {} as FolderDetail
const changeFromServer: FolderItemChange = {} as FolderItemChangeOutput

const originalFetch = globalThis.fetch

const capture = (body: string) => {
    const calls: string[] = []
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
        calls.push(String(input))
        return new Response(body, { status: 200, headers: { 'content-type': 'application/json' } })
    }, originalFetch)
    return calls
}

const run = async <T>(options: { queryFn?: unknown }) => (options.queryFn as () => Promise<T>)()

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('폴더 화면과 서버의 계약', () => {
    test('화면의 한도가 서버 한도와 같다', () => {
        expect([detailFromServer, detailToServer, changeFromServer]).toHaveLength(3)
        for (const [key, value] of Object.entries(FOLDER_LIMIT)) expect(SERVER_FOLDER_LIMIT[key as keyof typeof SERVER_FOLDER_LIMIT]).toBe(value)
    })

    test('이름·설명 검증이 서버와 같은 값을 받고 같은 값을 거절한다', () => {
        const samples = [
            { name: '연습 12', description: '' },
            { name: '  앞뒤 공백  ', description: '설명' },
            { name: '', description: '' },
            { name: '   ', description: '' },
            { name: 'a'.repeat(FOLDER_LIMIT.NAME_MAX), description: '' },
            { name: 'a'.repeat(FOLDER_LIMIT.NAME_MAX + 1), description: '' },
            { name: '줄\n바꿈', description: '' },
            { name: '설명 초과', description: 'b'.repeat(FOLDER_LIMIT.DESCRIPTION_MAX + 1) },
        ]
        for (const sample of samples) {
            const client = folderMetaFormSchema.safeParse(sample)
            const server = folderCreateSchema.safeParse(sample)
            expect({ sample, ok: client.success }).toEqual({ sample, ok: server.success })
            if (client.success && server.success) expect(client.data).toEqual(server.data)
        }
    })

    test('새 곡 입력은 서버 스키마를 통과한다', () => {
        const input = toNewFolderItemInput({ md5: 'a'.repeat(32), sha256: 'not-a-hash', title: 'x'.repeat(500), artist: '' })
        expect(input).not.toHaveProperty('sha256')
        expect(input.title).toHaveLength(FOLDER_LIMIT.TEXT_COPY_MAX)
    })

    test('폴더 id 모양을 서버와 같게 본다', () => {
        expect(isFolderId('uf_623ec6e8ddc745d58514a68517281c')).toBe(true)
        expect(isFolderId('uf_XYZ')).toBe(false)
    })

    test('에러 코드마다 사용자 문구를 고르고 모르는 코드는 서버 문구를 쓴다', () => {
        expect(toFolderErrorMessage(new ApiError('IR_FOLDER_NAME_TAKEN', 'x', 409))).toBe('같은 이름의 폴더가 이미 있습니다.')
        expect(toFolderErrorMessage(new ApiError('SOMETHING', '서버 문구', 500))).toBe('서버 문구')
    })
})

describe('폴더 쿼리가 실제 경로를 부른다', () => {
    test('목록·상세·포함 폴더는 봉투를 벗겨 돌려준다', async () => {
        const calls = capture('{"success":true,"data":[]}')
        expect(await run<unknown[]>(folderListQueryOptions())).toEqual([])
        await run(folderDetailQueryOptions('uf_623ec6e8ddc745d58514a68517281c'))
        await run(folderContainingQueryOptions('a'.repeat(32)))
        expect(calls[0]).toEndWith('/api/fe/me/folders')
        expect(calls[1]).toEndWith('/api/fe/me/folders/uf_623ec6e8ddc745d58514a68517281c')
        expect(calls[2]).toEndWith(`/api/fe/me/folders/containing/${'a'.repeat(32)}`)
    })
})

describe('로그아웃 때 사용자 묶음 캐시를 지운다', () => {
    test('폴더·내 기록·세션·토큰은 지우고 공개 자료는 남긴다', () => {
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.FOLDER.LIST, [])
        queryClient.setQueryData(QUERY_KEY.FOLDER.CONTAINING('a'.repeat(32)), [])
        queryClient.setQueryData(QUERY_KEY.TABLE_SNAPSHOT.RECORDS('insane1', 'user-1'), {})
        queryClient.setQueryData(QUERY_KEY.TOKEN.LIST, [])
        queryClient.setQueryData(QUERY_KEY.STATS.SUMMARY, {})
        removeUserScopedQueries(queryClient)
        const keys = queryClient
            .getQueryCache()
            .getAll()
            .map((query) => query.queryKey)
        expect(keys).toEqual([QUERY_KEY.STATS.SUMMARY])
    })
})
