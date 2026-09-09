'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { Button } from '@shared/ui/button'
import { authClient } from '@shared/lib/auth-client'
import type { SessionUser } from '@entities/auth.type'

const FIELD_CLASS = 'flex items-center justify-between gap-4 py-1'

export const ProfilePanel: FC<{ user: SessionUser }> = ({ user }) => {
    const router = useRouter()

    const signOut = async () => {
        await authClient.signOut()
        toast.success('로그아웃했습니다.')
        router.replace('/login')
        router.refresh()
    }

    return (
        <PanelCard
            title='프로필'
            action={
                <div className='flex items-center gap-2'>
                    {user.role === 'admin' && (
                        <Link href='/admin/builds' className='text-muted-foreground text-xs underline'>
                            빌드 관리
                        </Link>
                    )}
                    <Button variant='ghost' size='sm' onClick={signOut}>
                        로그아웃
                    </Button>
                </div>
            }
            contentClassName='flex flex-col gap-2'>
            <dl className='flex flex-col text-xs'>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>플레이어 ID</dt>
                    <dd className='font-mono'>{user.login_id}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>표시 이름</dt>
                    <dd>{user.name}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>이메일</dt>
                    <dd>{user.email}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>역할</dt>
                    <dd>
                        <MetaBadge>{user.role}</MetaBadge>
                    </dd>
                </div>
            </dl>
            <PanelFootnote>플레이어 ID 는 네이티브 클라이언트의 `--player` 값과 동일합니다.</PanelFootnote>
        </PanelCard>
    )
}
