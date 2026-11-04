import type { FC, PropsWithChildren, ReactNode } from 'react'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'

type DashboardQueryStateProps = PropsWithChildren<{
    isPending: boolean
    isError: boolean
    isFetching: boolean
    onRetry: () => void
    loadingLabel: string
    errorMessage: string
    isEmpty?: boolean
    emptyMessage?: string
    emptyAction?: ReactNode
    loadingClassName?: string
}>

/** Chooses what a panel body shows: the skeleton while it has no data, the failure with a retry, the empty notice, or its content. */
export const DashboardQueryState: FC<DashboardQueryStateProps> = ({
    isPending,
    isError,
    isFetching,
    onRetry,
    loadingLabel,
    errorMessage,
    isEmpty = false,
    emptyMessage,
    emptyAction,
    loadingClassName,
    children,
}) => {
    if (isPending) return <DashboardPanelLoading label={loadingLabel} className={loadingClassName} />
    if (isError) return <DashboardPanelError message={errorMessage} isRetrying={isFetching} onRetry={onRetry} />
    if (isEmpty) return <DashboardPanelEmpty message={emptyMessage ?? ''}>{emptyAction}</DashboardPanelEmpty>
    return <>{children}</>
}
