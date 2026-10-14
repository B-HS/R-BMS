import { describe, expect, mock, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'

const SIGNED_IN_SESSION = {
    data: { user: { id: 'user-1', name: 'Alice', email: 'alice@example.test', loginId: 'alice', role: 'user' } },
    isPending: false,
}

mock.module('@shared/lib/auth-client', () => ({
    authClient: { useSession: () => SIGNED_IN_SESSION, $store: { notify: () => undefined } },
}))

const { useSessionUser } = await import('../../src/entities/auth.query')

const SessionProbe = () => {
    const { user, isPending } = useSessionUser()
    return <span>{isPending ? 'pending' : (user?.name ?? 'signed-out')}</span>
}

describe('useSessionUser', () => {
    test('세션 조회가 이미 끝났어도 서버 렌더와 하이드레이션에서는 대기 상태를 그린다', () => {
        expect(renderToStaticMarkup(<SessionProbe />)).toBe('<span>pending</span>')
    })
})
