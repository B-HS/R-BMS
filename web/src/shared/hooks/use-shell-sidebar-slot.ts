'use client'

import { useContext } from 'react'

import { ShellSidebarSlotContext } from '@shared/providers/shell-sidebar-slot-context'

export const useShellSidebarSlot = () => {
    const context = useContext(ShellSidebarSlotContext)

    if (!context) {
        throw new Error('AppShell is missing.')
    }

    return context
}
