import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'

type ShellPageLoadingProps = {
    title: string
    description?: string
}

export const ShellPageLoading: FC<ShellPageLoadingProps> = ({ title, description }) => {
    const t = useTranslations('common.shell')

    return (
        <ShellPage title={title} description={description}>
            <PanelSkeleton height='page' label={t('pageLoading', { title })} />
        </ShellPage>
    )
}
