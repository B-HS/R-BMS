import * as React from 'react'

import { ChevronLeftIcon, ChevronRightIcon, MoreHorizontalIcon } from 'lucide-react'

import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
const Pagination = ({ className, ...props }: React.ComponentProps<'nav'>) => {
    return (
        <nav
            role='navigation'
            aria-label='페이지 이동'
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
    text = '이전',
    ...props
}: React.ComponentProps<typeof PaginationLink> & {
    text?: string
}) => {
    return (
        <PaginationLink aria-label='이전 페이지' size='default' className={cn('pl-1.5!', className)} {...props}>
            <ChevronLeftIcon data-icon='inline-start' />
            <span className='hidden sm:block'>{text}</span>
        </PaginationLink>
    )
}
const PaginationNext = ({
    className,
    text = '다음',
    ...props
}: React.ComponentProps<typeof PaginationLink> & {
    text?: string
}) => {
    return (
        <PaginationLink aria-label='다음 페이지' size='default' className={cn('pr-1.5!', className)} {...props}>
            <span className='hidden sm:block'>{text}</span>
            <ChevronRightIcon data-icon='inline-end' />
        </PaginationLink>
    )
}
const PaginationEllipsis = ({ className, ...props }: React.ComponentProps<'span'>) => {
    return (
        <span
            aria-hidden
            data-slot='pagination-ellipsis'
            className={cn("flex size-8 items-center justify-center [&_svg:not([class*='size-'])]:size-4", className)}
            {...props}>
            <MoreHorizontalIcon />
            <span className='sr-only'>더 많은 페이지</span>
        </span>
    )
}
export { Pagination, PaginationContent, PaginationEllipsis, PaginationItem, PaginationLink, PaginationNext, PaginationPrevious }
