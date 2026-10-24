'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { Button } from '@shared/ui/button'
import { LOGIN_PATH } from '@shared/constants/auth-route'
import { useSessionLogout } from '@entities/auth.query'
import type { SessionUser } from '@entities/auth.type'

const FIELD_CLASS = 'flex items-center justify-between gap-4 py-1'

export const ProfilePanel: FC<{ user: SessionUser }> = ({ user }) => {
    const t = useTranslations('settings.profilePanel')
    const router = useRouter()
    const logout = useSessionLogout()

    const handleSignOut = () =>
        logout.mutate(undefined, {
            onSuccess: () => {
                toast.success(t('signedOut'))
                router.replace(LOGIN_PATH)
                router.refresh()
            },
        })

    return (
        <PanelCard
            title={t('title')}
            action={
                <div className='flex items-center gap-2'>
                    {user.role === 'admin' && (
                        <>
                            <Link href='/admin/builds' className='text-muted-foreground text-xs underline'>
                                {t('builds')}
                            </Link>
                            <Link href='/admin/ai' className='text-muted-foreground text-xs underline'>
                                {t('aiLimits')}
                            </Link>
                        </>
                    )}
                    <Button variant='ghost' size='sm' disabled={logout.isPending} onClick={handleSignOut}>
                        {t('signOut')}
                    </Button>
                </div>
            }
            contentClassName='flex flex-col gap-2'>
            <dl className='flex flex-col text-xs'>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>{t('playerId')}</dt>
                    <dd className='font-mono'>{user.login_id}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>{t('displayName')}</dt>
                    <dd>{user.name}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>{t('email')}</dt>
                    <dd>{user.email}</dd>
                </div>
                <div className={FIELD_CLASS}>
                    <dt className='text-muted-foreground'>{t('role')}</dt>
                    <dd>
                        <MetaBadge>{user.role}</MetaBadge>
                    </dd>
                </div>
            </dl>
            <PanelFootnote>{t('footnote')}</PanelFootnote>
        </PanelCard>
    )
}
