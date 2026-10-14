import { useId, type FC, type PropsWithChildren, type ReactNode } from 'react'
import { cn } from '@shared/lib/cn'

type DashboardPanelProps = PropsWithChildren<{
    title: string
    badge?: ReactNode
    actions?: ReactNode
    className?: string
    bodyClassName?: string
}>

export const DashboardPanel: FC<DashboardPanelProps> = ({ title, badge, actions, className, bodyClassName, children }) => {
    const headingId = useId()

    return (
        <section aria-labelledby={headingId} className={cn('bg-card flex min-h-0 min-w-0 flex-col overflow-hidden', className)}>
            <header className='flex h-9 shrink-0 items-center justify-between gap-2 pr-1.5 pl-3'>
                <div className='flex min-w-0 items-center gap-2'>
                    <h2 id={headingId} className='truncate text-sm font-semibold'>
                        {title}
                    </h2>
                    {badge}
                </div>
                {actions ? <div className='flex shrink-0 items-center gap-0.5'>{actions}</div> : null}
            </header>
            <div className={cn('flex min-h-0 min-w-0 flex-1 flex-col px-3 pb-3', bodyClassName)}>{children}</div>
        </section>
    )
}
