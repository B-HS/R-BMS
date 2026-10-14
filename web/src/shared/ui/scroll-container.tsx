'use client'

import { type ComponentProps, type FC, useRef } from 'react'
import { cn } from '@shared/lib/cn'
import { ScrollIndicator } from '@shared/ui/scroll-indicator'

const SCROLL_CONTAINER_CLASS_NAMES = {
    always: { container: 'min-h-0', viewport: 'min-h-0 overflow-y-auto' },
    desktop: { container: 'md:min-h-0', viewport: 'md:min-h-0 md:overflow-auto' },
} as const

type ScrollContainerProps = ComponentProps<'div'> & {
    variant?: keyof typeof SCROLL_CONTAINER_CLASS_NAMES
    containerClassName?: string
    indicatorClassName?: string
}

export const ScrollContainer: FC<ScrollContainerProps> = ({ variant = 'desktop', containerClassName, indicatorClassName, className, ...props }) => {
    const viewportRef = useRef<HTMLDivElement>(null)

    return (
        <div
            data-slot='scroll-container'
            className={cn('relative flex min-w-0 flex-1 flex-col', SCROLL_CONTAINER_CLASS_NAMES[variant].container, containerClassName)}>
            <div {...props} ref={viewportRef} className={cn('min-w-0 flex-1', SCROLL_CONTAINER_CLASS_NAMES[variant].viewport, className)} />
            <ScrollIndicator targetRef={viewportRef} className={indicatorClassName} />
        </div>
    )
}
