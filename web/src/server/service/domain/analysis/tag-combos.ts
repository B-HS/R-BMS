import type { PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { cellRecordOf, cellsByKey, type CellNote } from '@server/service/domain/analysis/play-cells'

/** The tags whose pairs are counted (plan 2026-11-04 2.2.6), in PATTERN_TAGS order: every tag but `scratch_solo`, `ln_head`, `ln_end` and `mine_near`. */
export const COMBO_TAGS = [
    'chord_2',
    'chord_3',
    'chord_4plus',
    'jack',
    'trill',
    'stair',
    'stream',
    'scratch_chord',
    'scratch_mixed',
    'with_scratch',
    'scratch_run',
    'ln_hold_hit',
    'ln_release_then_hit',
    'soflan',
    'dense',
] as const satisfies readonly PatternTag[]
export type ComboTag = (typeof COMBO_TAGS)[number]

/** The key of a tag pair, the earlier tag of PATTERN_TAGS first: `a+b`. */
export const comboKeyOf = (first: ComboTag, second: ComboTag) => `${first}+${second}`

/** Every pair key in order: by the first tag, then the second. */
export const COMBO_KEYS = COMBO_TAGS.flatMap((first, index) => COMBO_TAGS.slice(index + 1).map((second) => comboKeyOf(first, second)))

/** The combo tags of a note's tags, in COMBO_TAGS order. */
export const comboTagsOf = (tags: readonly PatternTag[]) => COMBO_TAGS.filter((tag) => tags.includes(tag))

/** Every pair of a note's combo tags. */
export const comboKeysOf = (comboTags: readonly ComboTag[]) =>
    comboTags.flatMap((first, index) => comboTags.slice(index + 1).map((second) => comboKeyOf(first, second)))

/** A note that took a judgement, with its tags. */
export type ComboNote = CellNote & { tags: readonly PatternTag[] }

/**
 * The judgements of each pair of combo tags that share a note (`by_combo`, pairs with notes only, COMBO_KEYS order) and of the notes
 * whose only combo tag is one tag (`by_tag_solo`, COMBO_TAGS order), so a tag alone is told from the tag together with another.
 */
export const computeTagCombos = (notes: readonly ComboNote[]) => {
    const tagsOfNotes = notes.map((note) => comboTagsOf(note.tags))
    const pairs = cellsByKey(notes.flatMap((note, index) => comboKeysOf(tagsOfNotes[index]).map((key) => [key, note] as const)))
    const solos = cellsByKey(notes.flatMap((note, index) => (tagsOfNotes[index].length === 1 ? [[tagsOfNotes[index][0], note] as const] : [])))
    return { by_combo: cellRecordOf(COMBO_KEYS, pairs), by_tag_solo: cellRecordOf(COMBO_TAGS, solos) }
}
