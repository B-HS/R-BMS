type Timer = { at: number; order: number; resolve: () => void }

/**
 * A clock with no real waiting. `sleep` parks the caller; when every other task is blocked (the macrotask turn comes), time jumps to the
 * earliest sleeper and that one wakes. Code that only awaits promises resolved in microtasks therefore runs the same way every time.
 */
export const createVirtualClock = () => {
    let time = 0
    let order = 0
    let isTurnQueued = false
    const timers: Timer[] = []

    const queueTurn = () => {
        if (isTurnQueued) return
        isTurnQueued = true
        setImmediate(() => {
            isTurnQueued = false
            const next = timers.toSorted((left, right) => left.at - right.at || left.order - right.order)[0]
            if (!next) return
            timers.splice(timers.indexOf(next), 1)
            time = Math.max(time, next.at)
            next.resolve()
            if (timers.length > 0) queueTurn()
        })
    }

    return {
        now: () => time,
        sleep: (ms: number) =>
            new Promise<void>((resolve) => {
                order += 1
                timers.push({ at: time + Math.max(0, ms), order, resolve })
                queueTurn()
            }),
    }
}

export type VirtualClock = ReturnType<typeof createVirtualClock>
