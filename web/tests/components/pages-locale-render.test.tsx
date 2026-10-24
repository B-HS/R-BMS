import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost/' })

afterAll(() => GlobalRegistrator.unregister())

const { page, unmountAll, mount } = await import('../support/dom-harness')
const { LoadErrorState } = await import('../../src/features/state/load-error-state')
const { Pager } = await import('../../src/features/table/pager')
const { AiSetupNotice } = await import('../../src/features/ai/ai-setup-notice')
const { TableChartRecord } = await import('../../src/features/table/table-chart-record')
const { FolderConflictNotice } = await import('../../src/features/folder/folder-conflict-notice')

type Locale = import('../../src/shared/i18n/locale').Locale

const CONFLICT_SUMMARY = {
    serverName: 'Practice',
    serverItemCount: 1,
    serverUpdatedAt: 1_757_000_000_000,
    onlyMine: 0,
    onlyServer: 0,
    isOrderDifferent: false,
    isNameDifferent: false,
    isDescriptionDifferent: false,
}

afterEach(() => unmountAll())

describe('웹 화면 문구는 고른 언어로 그려진다', () => {
    const loadErrorCases: [Locale, string, string][] = [
        ['ko', '요청이 실패했습니다. 잠시 후 다시 시도하세요.', '다시 시도'],
        ['en', 'The request failed. Try again in a moment.', 'Try again'],
        ['ja', 'リクエストに失敗しました。しばらくしてからもう一度お試しください。', 'もう一度試す'],
    ]

    test.each(loadErrorCases)('%s: 불러오기 실패 기본 설명과 다시 시도 버튼', async (locale, description, retry) => {
        await mount(<LoadErrorState title='title' onRetry={() => undefined} />, locale)

        expect(page.getByText(description)).toBeTruthy()
        expect(page.getByRole('button', { name: retry })).toBeTruthy()
    })

    const pagerCases: [Locale, string, string, string][] = [
        ['ko', '페이지 이동', '이전 페이지', '다음 페이지'],
        ['en', 'Page navigation', 'Previous page', 'Next page'],
        ['ja', 'ページ移動', '前のページ', '次のページ'],
    ]

    test.each(pagerCases)('%s: 페이지 이동 막대의 접근성 이름', async (locale, navigation, previous, next) => {
        await mount(<Pager page={2} totalPages={5} onPageChange={() => undefined} />, locale)

        expect(page.getByLabelText(navigation)).toBeTruthy()
        expect(page.getByLabelText(previous)).toBeTruthy()
        expect(page.getByLabelText(next)).toBeTruthy()
    })

    const setupCases: [Locale, string, string][] = [
        ['ko', '등록된 AI 키가 없습니다', 'AI 키 등록하기'],
        ['en', 'No AI key is registered', 'Register an AI key'],
        ['ja', '登録されたAIキーがありません', 'AIキーを登録する'],
    ]

    test.each(setupCases)('%s: AI 키가 없을 때의 안내와 설정 링크', async (locale, title, action) => {
        await mount(<AiSetupNotice reason='no_key' />, locale)

        expect(page.getByText(title)).toBeTruthy()
        expect(page.getByRole('link', { name: action }).getAttribute('href')).toBe('/settings?tab=ai')
    })

    const signedOutCases: [Locale, string, string][] = [
        ['ko', '로그인하면 내 램프와 점수가 표시됩니다.', '로그인'],
        ['en', 'Sign in to see your lamps and scores.', 'Sign in'],
        ['ja', 'ログインすると自分のランプとスコアが表示されます。', 'ログイン'],
    ]

    test.each(signedOutCases)('%s: 로그아웃 상태의 내 기록 칸은 로그인 링크를 건다', async (locale, prompt, signIn) => {
        await mount(<TableChartRecord record={undefined} isSignedIn={false} isPending={false} loginHref='/login?next=/tables' />, locale)

        expect(page.getByText(prompt)).toBeTruthy()
        expect(page.getByRole('link', { name: signIn }).getAttribute('href')).toBe('/login?next=/tables')
    })

    const conflictCases: [Locale, number, RegExp][] = [
        ['ko', 1_234, /고쳐졌고 1,234곡입니다\./],
        ['en', 1, /has 1 song\./],
        ['en', 1_234, /has 1,234 songs\./],
        ['ja', 1_234, /編集され、1,234曲です。/],
    ]

    test.each(conflictCases)('%s: 서버 폴더 곡 수 %d 는 언어의 수 규칙으로 쓴다', async (locale, serverItemCount, expected) => {
        await mount(
            <FolderConflictNotice
                summary={{ ...CONFLICT_SUMMARY, serverItemCount }}
                isSaving={false}
                onUseServer={() => undefined}
                onOverwrite={() => undefined}
            />,
            locale,
        )

        expect(page.getByText(expected)).toBeTruthy()
    })
})
