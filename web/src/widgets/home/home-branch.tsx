import 'server-only'
import { Suspense } from 'react'
import { headers } from 'next/headers'
import { connection } from 'next/server'
import { getSessionCookie } from 'better-auth/cookies'
import { getTranslations } from 'next-intl/server'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { readSessionUser } from '@widgets/auth/session-gate'
import { HomeContent } from '@widgets/home/home-content'
import { HomeIntroPanel } from '@widgets/home/home-intro-panel'
import { MemberDashboardContent } from '@widgets/home/member-dashboard-content'
import { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_FULL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'

const readSignedInUser = async () => {
    try {
        const requestHeaders = await headers()
        if (!getSessionCookie(new Headers({ cookie: requestHeaders.get('cookie') ?? '' }))) return null
        return await readSessionUser()
    } catch {
        return null
    }
}

/** Draws the signed-in dashboard for a visitor with a live session and the public home for anyone else, including a visitor whose session can not be read. */
export const HomeBranch = async () => {
    await connection()
    const user = await readSignedInUser()
    if (user) return <MemberDashboardContent user={user} />

    const t = await getTranslations('pages.route.home')

    return (
        <>
            <HomeIntroPanel />
            <Suspense
                fallback={
                    <>
                        <DashboardPanel title={t('serverStatus')} className={DASHBOARD_FILL_PANEL_CLASS_NAME}>
                            <DashboardPanelLoading label={t('serverStatusLoading')} />
                        </DashboardPanel>
                        <DashboardPanel title={t('recentActivity')} className={DASHBOARD_FULL_PANEL_CLASS_NAME}>
                            <DashboardPanelLoading label={t('recentActivityLoading')} className='min-h-64' />
                        </DashboardPanel>
                    </>
                }>
                <HomeContent />
            </Suspense>
        </>
    )
}
