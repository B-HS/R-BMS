'use client'

import { useRef, type RefObject } from 'react'

const readFocusedElement = () =>
    document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null

/** Returns Radix dialog focus handlers that send focus back to the opener (or `returnFocusRef`) when a state-driven dialog closes. */
export const useDialogFocusReturn = (returnFocusRef?: RefObject<HTMLElement | null>) => {
    const openerRef = useRef<HTMLElement | null>(null)

    return {
        onOpenAutoFocus: () => {
            openerRef.current = readFocusedElement()
        },
        onCloseAutoFocus: (event: Event) => {
            const target = returnFocusRef?.current ?? openerRef.current
            if (!target?.isConnected) return
            event.preventDefault()
            target.focus()
        },
    }
}
