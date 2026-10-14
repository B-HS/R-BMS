import { createContext } from 'react'

export type ShellSidebarSlotContextValue = {
    element: HTMLElement | null
}

export const ShellSidebarSlotContext = createContext<ShellSidebarSlotContextValue | null>(null)
