'use client'
import { useEffect } from 'react'

export const useUnsavedChangesWarning = (isEnabled: boolean) => {
    useEffect(() => {
        if (!isEnabled) return

        const handleBeforeUnload = (event: BeforeUnloadEvent) => event.preventDefault()

        window.addEventListener('beforeunload', handleBeforeUnload)

        return () => window.removeEventListener('beforeunload', handleBeforeUnload)
    }, [isEnabled])
}
