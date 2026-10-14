'use client'

import { useEffect, useRef, useState, type CSSProperties, type FC, type PropsWithChildren } from 'react'

type TableDeferredMountProps = PropsWithChildren<{
    itemCount: number
    isInitiallyMounted: boolean
}>

const CARD_HEIGHT_PX = 56
const GRID_GAP_PX = 1
const GRID_COLUMNS = [2, 3, 4, 5, 6] as const
const PRELOAD_MARGIN_PX = 1200
const SCROLLABLE_OVERFLOW = ['auto', 'scroll']

const gridHeight = (itemCount: number, columns: number) => {
    const rows = Math.ceil(itemCount / columns)
    return rows * CARD_HEIGHT_PX + Math.max(rows - 1, 0) * GRID_GAP_PX
}

const findScrollRoot = (element: HTMLElement) => {
    const viewport = element.closest('[data-slot="scroll-container"]')?.firstElementChild
    if (!(viewport instanceof HTMLElement)) return null
    return SCROLLABLE_OVERFLOW.includes(getComputedStyle(viewport).overflowY) ? viewport : null
}

/** Holds a level's card grid back until it nears the visible area, keeping the exact grid height so jumps and the scrollbar stay true. */
export const TableDeferredMount: FC<TableDeferredMountProps> = ({ itemCount, isInitiallyMounted, children }) => {
    const placeholderRef = useRef<HTMLDivElement>(null)

    const [isMounted, setIsMounted] = useState(isInitiallyMounted)

    const placeholderStyle = Object.fromEntries(
        GRID_COLUMNS.map((columns) => [`--grid-height-${columns}`, `${gridHeight(itemCount, columns)}px`]),
    ) as CSSProperties

    useEffect(() => {
        const placeholder = placeholderRef.current
        if (isMounted || !placeholder) return
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries.some((entry) => entry.isIntersecting)) setIsMounted(true)
            },
            { root: findScrollRoot(placeholder), rootMargin: `${PRELOAD_MARGIN_PX}px 0px` },
        )
        observer.observe(placeholder)
        return () => observer.disconnect()
    }, [isMounted])

    if (isMounted) return children

    return (
        <div
            ref={placeholderRef}
            aria-hidden='true'
            style={placeholderStyle}
            className='bg-card h-(--grid-height-2) sm:h-(--grid-height-3) lg:h-(--grid-height-4) xl:h-(--grid-height-5) 2xl:h-(--grid-height-6)'
        />
    )
}
