import type { FC } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'

type ShellPageLoadingProps = {
    title: string
    description?: string
}

export const ShellPageLoading: FC<ShellPageLoadingProps> = ({ title, description }) => (
    <ShellPage title={title} description={description}>
        <PanelSkeleton height='page' label={`${title} 불러오는 중`} />
    </ShellPage>
)
