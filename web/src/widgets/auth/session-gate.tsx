import 'server-only'
import type { PropsWithChildren } from 'react'
import { headers } from 'next/headers'
import { redirect } from 'next/navigation'
import type { SessionUser } from '@entities/auth.type'

/** Resolves the better-auth session on the server from the incoming request cookies. */
export const readSessionUser = async (): Promise<SessionUser | null> => {
    const { getAuth } = await import('@server/lib/auth')
    const session = await getAuth().api.getSession({ headers: await headers() })
    if (!session) return null
    const user = session.user
    return {
        id: user.id,
        login_id: typeof user.loginId === 'string' ? user.loginId : user.id,
        name: user.name,
        email: user.email,
        role: typeof user.role === 'string' ? user.role : 'user',
    }
}

type SessionGateProps = PropsWithChildren<{ next: string; requireAdmin?: boolean }>

export const SessionGate = async ({ next, requireAdmin, children }: SessionGateProps) => {
    const user = await readSessionUser()
    if (!user) redirect(`/login?next=${encodeURIComponent(next)}`)
    if (requireAdmin && user.role !== 'admin') redirect('/')
    return <>{children}</>
}
