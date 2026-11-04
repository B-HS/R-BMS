import { createHash } from 'node:crypto'
import type { TrainingSpec } from '@server/service/domain/training/spec'
import { createAppError } from '@server/lib/error'
import { trainingBpmChangesOf, trainingBpmDefinitionsOf, type WrittenLine } from '@server/service/domain/training/chart-timing'
import {
    BEATS_PER_MEASURE,
    TRAINING_ARTIST,
    TRAINING_BGM_CHANNEL,
    TRAINING_BPM_CHANNEL,
    TRAINING_COUNT_TOKEN,
    TRAINING_COUNT_WAV_NAME,
    TRAINING_EMPTY_TOKEN,
    TRAINING_GENRE,
    TRAINING_HIT_TOKEN,
    TRAINING_HIT_WAV_NAME,
    TRAINING_LANE_CHANNELS,
    TRAINING_LN_CHANNELS,
    TRAINING_LN_MODE,
    TRAINING_MAX_LINE_RESOLUTION,
    TRAINING_MAX_MEASURE,
    TRAINING_MEASURE_DIGITS,
    TRAINING_PLAYER,
    TRAINING_RANK,
    TRAINING_TOTAL_COEFFICIENTS,
    TRAINING_TOTAL_MIN,
} from '@server/service/domain/training/training-constants'

export const TRAINING_NOTE_KINDS = ['normal', 'ln_head', 'ln_end'] as const
export type TrainingNoteKind = (typeof TRAINING_NOTE_KINDS)[number]

/** One note of a training chart at `measure + num/den` (a fraction in lowest terms): lanes 0-6 are keys 1-7, 7 the scratch. */
export type TrainingNote = { measure: number; num: number; den: number; lane: number; kind: TrainingNoteKind }

const COUNT_IN_MEASURE = 0
const BPM_LINE_RESOLUTION = 1

/** The greatest common divisor of two whole numbers. */
export const greatestCommonDivisor = (a: number, b: number): number => (b === 0 ? a : greatestCommonDivisor(b, a % b))
const leastCommonMultiple = (a: number, b: number) => (a / greatestCommonDivisor(a, b)) * b

/** `#TOTAL` for a chart of `playableNotes` (plan 3.7): grows with the notes and never falls under TRAINING_TOTAL_MIN. */
export const trainingTotalOf = (playableNotes: number) =>
    Math.max(
        TRAINING_TOTAL_MIN,
        Math.round(
            (TRAINING_TOTAL_COEFFICIENTS.SCALE * playableNotes) /
                (TRAINING_TOTAL_COEFFICIENTS.PER_NOTE * playableNotes + TRAINING_TOTAL_COEFFICIENTS.OFFSET),
        ),
    )

/** Notes the player judges as notes: everything but an LN end (`#LNMODE 1` does not count its ends). */
export const countPlayableNotes = (notes: readonly TrainingNote[]) => notes.filter((note) => note.kind !== 'ln_end').length

const channelOf = (note: TrainingNote) => (note.kind === 'normal' ? TRAINING_LANE_CHANNELS[note.lane] : TRAINING_LN_CHANNELS[note.lane])

/**
 * The data lines of a chart (plan 3.7): the count-in clicks in measure 0, a channel 08 token where the tempo changes, and one line
 * per measure and channel of notes whose token count N is the least common multiple of its notes' denominators. Lines are in
 * measure order and, within a measure, in channel order.
 */
export const trainingLinesOf = (
    spec: { base_bpm: number; sections: readonly { measure_from: number; bpm: number }[] },
    notes: readonly TrainingNote[],
) => {
    const definitions = trainingBpmDefinitionsOf(trainingBpmChangesOf(spec))
    const idOfBpm = new Map(definitions.map((definition) => [definition.bpm, definition.id]))
    const countIn: WrittenLine = {
        measure: COUNT_IN_MEASURE,
        channel: TRAINING_BGM_CHANNEL,
        resolution: BEATS_PER_MEASURE,
        tokens: Array.from({ length: BEATS_PER_MEASURE }, () => TRAINING_COUNT_TOKEN),
    }
    const tempoLines = trainingBpmChangesOf(spec).map((change): WrittenLine => ({
        measure: change.measure,
        channel: TRAINING_BPM_CHANNEL,
        resolution: BPM_LINE_RESOLUTION,
        tokens: [idOfBpm.get(change.bpm) ?? TRAINING_EMPTY_TOKEN],
    }))
    const grouped = Map.groupBy(notes, (note) => `${note.measure}:${channelOf(note)}`)
    const noteLines = [...grouped.values()].map((group): WrittenLine => {
        const resolution = group.reduce((lcm, note) => leastCommonMultiple(lcm, note.den), 1)
        if (resolution > TRAINING_MAX_LINE_RESOLUTION) throw createAppError('INTERNAL_ERROR', { resolution })
        const tokens = Array.from({ length: resolution }, () => TRAINING_EMPTY_TOKEN)
        for (const note of group) tokens[(note.num * resolution) / note.den] = TRAINING_HIT_TOKEN
        return { measure: group[0].measure, channel: channelOf(group[0]), resolution, tokens }
    })
    const lines = [countIn, ...tempoLines, ...noteLines].toSorted((a, b) => a.measure - b.measure || (a.channel < b.channel ? -1 : 1))
    const lastMeasure = lines.at(-1)?.measure ?? COUNT_IN_MEASURE
    if (lastMeasure > TRAINING_MAX_MEASURE) throw createAppError('INTERNAL_ERROR', { measure: lastMeasure })
    return lines
}

const lineText = (line: WrittenLine) => `#${String(line.measure).padStart(TRAINING_MEASURE_DIGITS, '0')}${line.channel}:${line.tokens.join('')}`

/**
 * The BMS bytes of a synthesised chart (plan 3.7): the header in a fixed order, then the data lines, every line ending in `\n`, UTF-8
 * with no byte-order mark. The hashes are of these very bytes, as the client takes them of the file.
 */
export const writeTrainingBms = (spec: TrainingSpec, notes: readonly TrainingNote[]) => {
    const lines = trainingLinesOf(spec, notes)
    const total = trainingTotalOf(countPlayableNotes(notes))
    const definitions = trainingBpmDefinitionsOf(trainingBpmChangesOf(spec))
    const header = [
        `#PLAYER ${TRAINING_PLAYER}`,
        `#GENRE ${TRAINING_GENRE}`,
        `#TITLE ${spec.title}`,
        `#SUBTITLE ${spec.subtitle}`,
        `#ARTIST ${TRAINING_ARTIST}`,
        `#BPM ${spec.base_bpm}`,
        `#PLAYLEVEL ${spec.play_level}`,
        `#RANK ${TRAINING_RANK}`,
        `#TOTAL ${total}`,
        `#LNMODE ${TRAINING_LN_MODE}`,
        `#WAV${TRAINING_HIT_TOKEN} ${TRAINING_HIT_WAV_NAME}`,
        `#WAV${TRAINING_COUNT_TOKEN} ${TRAINING_COUNT_WAV_NAME}`,
        ...definitions.map((definition) => `#BPM${definition.id} ${definition.bpm}`),
    ]
    const text = [...header, ...lines.map(lineText)].map((line) => `${line}\n`).join('')
    const bytes = new TextEncoder().encode(text)
    return {
        text,
        bytes,
        lines,
        total,
        md5: createHash('md5').update(bytes).digest('hex'),
        sha256: createHash('sha256').update(bytes).digest('hex'),
    }
}
