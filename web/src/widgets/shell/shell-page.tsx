import type { FC, PropsWithChildren, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'
import { ScrollContainer } from '@shared/ui/scroll-container'
import { ShellPageHeader } from '@widgets/shell/shell-page-header'

type ShellPageProps = PropsWithChildren<{
    title: string
    description?: string
    actions?: ReactNode
    contentClassName?: string
}>

export const ShellPage: FC<ShellPageProps> = ({ title, description, actions, contentClassName, children }) => (
    <section className='rbms-page'>
        <ShellPageHeader title={title} description={description} actions={actions} />
        <ScrollContainer className={cn('flex flex-col gap-px pb-12 *:shrink-0', contentClassName)}>{children}</ScrollContainer>
    </section>
)
