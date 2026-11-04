import { TRAINING_SECTION_SEED_STRIDE } from '@server/service/domain/training/training-constants'

const MULBERRY32_INCREMENT = 0x6d2b79f5
const MULBERRY32_SHIFTS = { FIRST: 15, SECOND: 7, THIRD: 14 } as const
const MULBERRY32_ODD = 61
const UINT32_RANGE = 2 ** 32

/** A random source: each call gives a number in [0, 1). */
export type RandomSource = () => number

/** The mulberry32 generator: a 32-bit state, the same sequence for the same seed on every runtime (integer arithmetic only). */
export const mulberry32 = (seed: number): RandomSource => {
    let state = seed >>> 0
    return () => {
        state = (state + MULBERRY32_INCREMENT) >>> 0
        let mixed = Math.imul(state ^ (state >>> MULBERRY32_SHIFTS.FIRST), state | 1)
        mixed ^= mixed + Math.imul(mixed ^ (mixed >>> MULBERRY32_SHIFTS.SECOND), mixed | MULBERRY32_ODD)
        return ((mixed ^ (mixed >>> MULBERRY32_SHIFTS.THIRD)) >>> 0) / UINT32_RANGE
    }
}

/** The generator of one section: seeded apart from the others, so changing one section's place or content leaves the rest as they were. */
export const sectionRandomOf = (seed: number, index: number) => mulberry32((seed ^ Math.imul(index, TRAINING_SECTION_SEED_STRIDE)) >>> 0)

/** An integer in [0, count) drawn from `random`. */
export const randomIndex = (random: RandomSource, count: number) => Math.min(count - 1, Math.floor(random() * count))

/** One item of a non-empty list drawn from `random`. */
export const randomItem = <T>(random: RandomSource, items: readonly T[]) => items[randomIndex(random, items.length)]
