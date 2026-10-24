import * as React from 'react'

import { ChevronLeftIcon, ChevronRightIcon, MoreHorizontalIcon } from 'lucide-react'
import { useTranslations } from 'next-intl'

import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
const Pagination = ({ className, ...props }: React.ComponentProps<'nav'>) => {
    const t = useTranslations('pages.pagination')
    return (
        <nav
            role='navigation'
            aria-label={t('navigation')}
            data-slot='pagination'
            className={cn('mx-auto flex w-full justify-center', className)}
            {...props}
        />
    )
}
const PaginationContent = ({ className, ...props }: React.ComponentProps<'ul'>) => {
    return <ul data-slot='pagination-content' className={cn('flex items-center gap-0.5', className)} {...props} />
}
const PaginationItem = ({ ...props }: React.ComponentProps<'li'>) => {
    return <li data-slot='pagination-item' {...props} />
}
type PaginationLinkProps = {
    isActive?: boolean
} & Pick<React.ComponentProps<typeof Button>, 'size'> &
    React.ComponentProps<'a'>
const PaginationLink = ({ className, isActive, size = 'icon', ...props }: PaginationLinkProps) => {
    return (
        <Button asChild variant={isActive ? 'outline' : 'ghost'} size={size} className={cn(className)}>
            <a aria-current={isActive ? 'page' : undefined} data-slot='pagination-link' data-active={isActive} {...props} />
        </Button>
    )
}
const PaginationPrevious = ({
    className,
    text,
    ...props
}: React.ComponentProps<typeof PaginationLink> & {
    text?: string
}) => {
    const t = useTranslations('pages.pagination')
    return (
        <PaginationLink aria-label={t('previous')} size='default' className={cn('pl-1.5!', className)} {...props}>
            <ChevronLeftIcon data-icon='inline-start' />
            <span className='hidden sm:block'>{text ?? t('previousShort')}</span>
        </PaginationLink>
    )
}
const PaginationNext = ({
    className,
    text,
    ...props
}: React.ComponentProps<typeof PaginationLink> & {
    text?: string
}) => {
    const t = useTranslations('pages.pagination')
    return (
        <PaginationLink aria-label={t('next')} size='default' className={cn('pr-1.5!', className)} {...props}>
            <span className='hidden sm:block'>{text ?? t('nextShort')}</span>
            <ChevronRightIcon data-icon='inline-end' />
        </PaginationLink>
    )
}
const PaginationEllipsis = ({ className, ...props }: React.ComponentProps<'span'>) => {
    const t = useTranslations('pages.pagination')
    return (
        <span
            aria-hidden
            data-slot='pagination-ellipsis'
            className={cn("flex size-8 items-center justify-center [&_svg:not([class*='size-'])]:size-4", className)}
            {...props}>
            <MoreHorizontalIcon />
            <span className='sr-only'>{t('more')}</span>
        </span>
    )
}
export { Pagination, PaginationContent, PaginationEllipsis, PaginationItem, PaginationLink, PaginationNext, PaginationPrevious }
