'use client'

import type { FC, PropsWithChildren } from 'react'
import { createPortal } from 'react-dom'
import { useIsHydrated } from '@shared/hooks/use-is-hydrated'
import { useShellSidebarSlot } from '@shared/hooks/use-shell-sidebar-slot'

export const ShellSidebarPortal: FC<PropsWithChildren> = ({ children }) => {
    const { element } = useShellSidebarSlot()
    const isHydrated = useIsHydrated()

    if (!isHydrated || !element) return null

    return createPortal(children, element)
}
