import type { FC, ReactNode } from 'react'
import { SidebarTrigger } from '@shared/ui/sidebar'

type ShellPageHeaderProps = {
    title: string
    description?: string
    actions?: ReactNode
}

export const ShellPageHeader: FC<ShellPageHeaderProps> = ({ title, description, actions }) => (
    <header className='rbms-page-header'>
        <div className='flex min-w-0 items-center gap-2'>
            <SidebarTrigger aria-label='메뉴 열기' className='shrink-0 md:hidden' />
            <div className='min-w-0'>
                <h1 className='truncate text-sm font-semibold'>{title}</h1>
                {description ? <p className='text-muted-foreground hidden truncate text-xs sm:block'>{description}</p> : null}
            </div>
        </div>
        {actions ? <div className='flex shrink-0 items-center gap-1'>{actions}</div> : null}
    </header>
)
