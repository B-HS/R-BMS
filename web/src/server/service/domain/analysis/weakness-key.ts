import { z } from 'zod'
import { BPM_BANDS, HAND_KEYS, SP7_LANE_NAMES } from '@server/service/domain/analysis/analysis-constants'
import { PATTERN_TAGS, type PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { COMBO_TAGS, comboKeyOf, type ComboTag } from '@server/service/domain/analysis/tag-combos'

/** The longest weakness key a request may carry: the longest key (`combo:ln_release_then_hit+scratch_mixed`, 39) with room to spare. */
export const WEAKNESS_KEY_MAX_LENGTH = 64

/** What a weakness key names (plan 2026-11-04 2.5.5): the cell kinds of the weakness profile. */
export const WEAKNESS_KINDS = [
    'tag',
    'solo',
    'combo',
    'lane',
    'hand',
    'timing',
    'bpm',
    'soflan',
    'fatigue',
    'density',
    'bp_context',
    'gauge_context',
] as const
export type WeaknessKind = (typeof WEAKNESS_KINDS)[number]

/** The soflan phases a key may name: the phases after an event compared with the phase before it. */
export const SOFLAN_KEY_PHASES = ['after', 'settled'] as const
export type SoflanKeyPhase = (typeof SOFLAN_KEY_PHASES)[number]

/** The density cells a key may name: the density limit curve and sustained against burst density. */
export const DENSITY_KEY_PARTS = ['limit', 'sustained'] as const
export type DensityKeyPart = (typeof DENSITY_KEY_PARTS)[number]

/** The lane names a key may carry: keys 1 to 7 and `S`, the scratch. */
export type LaneName = (typeof SP7_LANE_NAMES)[number]

const KEY_SEPARATOR = ':'
const COMBO_SEPARATOR = '+'
const FATIGUE_KEY = 'fatigue'

/** Builds every key of the grammar, so a key is written in one place and read back by `parseWeaknessKey`. */
export const WEAKNESS_KEY = {
    tag: (tag: PatternTag) => `tag:${tag}`,
    solo: (tag: ComboTag) => `solo:${tag}`,
    combo: (first: ComboTag, second: ComboTag) => `combo:${comboKeyOf(first, second)}`,
    lane: (lane: LaneName) => `lane:${lane}`,
    hand: (hand: (typeof HAND_KEYS)[number]) => `hand:${hand}`,
    timing: (tag: PatternTag) => `timing:${tag}`,
    bpm: (band: (typeof BPM_BANDS)[number]) => `bpm:${band}`,
    soflan: (phase: SoflanKeyPhase) => `soflan:${phase}`,
    fatigue: () => FATIGUE_KEY,
    density: (part: DensityKeyPart) => `density:${part}`,
    bpContext: (tag: PatternTag) => `bp_context:${tag}`,
    gaugeContext: (tag: PatternTag) => `gauge_context:${tag}`,
} as const

const isOneOf = <T extends string>(values: readonly T[], value: string): value is T => (values as readonly string[]).includes(value)

const tagKey = (kind: WeaknessKind, argument: string, tags: readonly string[]) =>
    isOneOf(tags, argument) && isOneOf(PATTERN_TAGS, argument) ? { kind, tags: [argument], arg: null } : null

const argumentKey = (kind: WeaknessKind, argument: string, allowed: readonly string[]) =>
    isOneOf(allowed, argument) ? { kind, tags: [] as PatternTag[], arg: argument } : null

const comboKey = (argument: string) => {
    const parts = argument.split(COMBO_SEPARATOR)
    const [first, second] = parts
    if (parts.length !== 2 || first === undefined || second === undefined) return null
    if (!isOneOf(COMBO_TAGS, first) || !isOneOf(COMBO_TAGS, second)) return null
    if (PATTERN_TAGS.indexOf(first) >= PATTERN_TAGS.indexOf(second)) return null
    return { kind: 'combo' as const, tags: [first, second], arg: null }
}

/**
 * Reads a weakness key of the grammar of plan 2026-11-04 2.5.5 into its kind, the pattern tags it names and its other argument (a
 * lane, a hand, a BPM band, a soflan phase or a density part). `tag:`, `timing:`, `bp_context:` and `gauge_context:` take any of
 * PATTERN_TAGS; `solo:` takes one of COMBO_TAGS, the tags `by_tag_solo` keeps; `combo:` takes two COMBO_TAGS, the earlier of
 * PATTERN_TAGS first. Anything else, or a key longer than WEAKNESS_KEY_MAX_LENGTH, is null.
 */
export const parseWeaknessKey = (value: string): { kind: WeaknessKind; tags: PatternTag[]; arg: string | null } | null => {
    if (value.length > WEAKNESS_KEY_MAX_LENGTH) return null
    if (value === FATIGUE_KEY) return { kind: 'fatigue', tags: [], arg: null }
    const separator = value.indexOf(KEY_SEPARATOR)
    if (separator < 0) return null
    const prefix = value.slice(0, separator)
    const argument = value.slice(separator + 1)
    if (prefix === 'tag' || prefix === 'timing' || prefix === 'bp_context' || prefix === 'gauge_context')
        return tagKey(prefix, argument, PATTERN_TAGS)
    if (prefix === 'solo') return tagKey(prefix, argument, COMBO_TAGS)
    if (prefix === 'combo') return comboKey(argument)
    if (prefix === 'lane') return argumentKey(prefix, argument, SP7_LANE_NAMES)
    if (prefix === 'hand') return argumentKey(prefix, argument, HAND_KEYS)
    if (prefix === 'bpm') return argumentKey(prefix, argument, BPM_BANDS)
    if (prefix === 'soflan') return argumentKey(prefix, argument, SOFLAN_KEY_PHASES)
    if (prefix === 'density') return argumentKey(prefix, argument, DENSITY_KEY_PARTS)
    return null
}

export type ParsedWeaknessKey = NonNullable<ReturnType<typeof parseWeaknessKey>>

/** A weakness key from outside (a request body or query): the grammar of `parseWeaknessKey`, at most WEAKNESS_KEY_MAX_LENGTH long. */
export const weaknessKeySchema = z
    .string()
    .max(WEAKNESS_KEY_MAX_LENGTH)
    .refine((value) => parseWeaknessKey(value) !== null)

/** The kinds whose key picks charts by their own features (tag shares, pairs, NPS, BPM band, soflan); lanes, hands and timing move with options. */
const FOLDER_SELECTABLE_KINDS = ['tag', 'solo', 'combo', 'density', 'bpm', 'soflan'] as const satisfies readonly WeaknessKind[]

/** Whether a practice folder can pick charts for this kind of weakness (plan 2.5.5 `selectable_for_folder`). */
export const isFolderSelectableKind = (kind: WeaknessKind) => isOneOf(FOLDER_SELECTABLE_KINDS, kind)
