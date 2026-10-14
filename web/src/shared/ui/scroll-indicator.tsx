'use client'

import type { FC, RefObject } from 'react'
import { useScrollThumb } from '@shared/hooks/use-scroll-thumb'
import { cn } from '@shared/lib/cn'

type ScrollIndicatorProps = {
    targetRef?: RefObject<HTMLElement | null>
    className?: string
}

export const ScrollIndicator: FC<ScrollIndicatorProps> = ({ targetRef, className }) => {
    const { heightPercent, topPercent, isScrollable, isVisible } = useScrollThumb(targetRef)

    if (!isScrollable) return null

    return (
        <div
            aria-hidden='true'
            data-slot='scroll-indicator'
            className={cn('pointer-events-none inset-y-0 right-0 z-(--z-scroll-indicator) w-0.75', targetRef ? 'absolute' : 'fixed', className)}>
            <div
                className={cn(
                    'absolute right-0 w-0.75 bg-foreground/50 transition-opacity duration-200 motion-reduce:transition-none',
                    isVisible ? 'opacity-100' : 'opacity-0',
                )}
                style={{ height: heightPercent + '%', top: topPercent + '%' }}
            />
        </div>
    )
}
