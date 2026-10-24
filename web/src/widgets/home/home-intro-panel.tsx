import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { Button } from '@shared/ui/button'

const HIGHLIGHT_KEYS = ['scores', 'rivals', 'sync'] as const

export const HomeIntroPanel: FC = () => {
    const t = useTranslations('browse.home.intro')

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_WIDE_PANEL_CLASS_NAME} bodyClassName='gap-3'>
            <p className='text-xs leading-5 break-keep'>{t('description')}</p>
            <ul className='text-muted-foreground flex list-disc flex-col gap-1 pl-4 text-xs leading-5 break-keep'>
                {HIGHLIGHT_KEYS.map((key) => (
                    <li key={key}>{t(`highlights.${key}`)}</li>
                ))}
            </ul>
            <div className='mt-auto flex flex-wrap gap-1'>
                <Button asChild size='sm'>
                    <Link href='/guide'>{t('guideLink')}</Link>
                </Button>
                <Button asChild variant='outline' size='sm'>
                    <Link href='/tables/insane1'>{t('insaneLink')}</Link>
                </Button>
                <Button asChild variant='outline' size='sm'>
                    <Link href='/search'>{t('searchLink')}</Link>
                </Button>
            </div>
        </DashboardPanel>
    )
}
