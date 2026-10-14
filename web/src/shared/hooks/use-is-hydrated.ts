'use client'

import { useSyncExternalStore } from 'react'

const subscribeToHydration = () => () => undefined
const getHydratedSnapshot = () => true
const getHydratingSnapshot = () => false

/** Returns false while React hydrates server HTML and true afterwards, so client-only values never change the hydrated markup. */
export const useIsHydrated = () => useSyncExternalStore(subscribeToHydration, getHydratedSnapshot, getHydratingSnapshot)
