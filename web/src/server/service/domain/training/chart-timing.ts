import {
    MICROS_PER_MEASURE_AT_ONE_BPM,
    TRAINING_BPM_CHANNEL,
    TRAINING_EMPTY_TOKEN,
    TRAINING_TOKEN_LENGTH,
    TRAINING_TOKEN_RADIX,
} from '@server/service/domain/training/training-constants'

/** One data line as the writer wrote it: its measure, channel, the token count N it was written with and the tokens (`i/N` apart). */
export type WrittenLine = { measure: number; channel: string; resolution: number; tokens: string[] }

/** Where the tempo changes: the first measure of a section whose tempo differs from the one before. */
export type TrainingBpmChange = { measure: number; bpm: number }

/** A `#BPMxx` definition: its two-character id and its tempo. */
export type TrainingBpmDefinition = { id: string; bpm: number }

/** A two-character base-36 token (`01`, `02`, ... `0A`), as `#BPMxx`, `#WAVxx` and the channel 08 objects use. */
export const trainingTokenOf = (value: number) => value.toString(TRAINING_TOKEN_RADIX).toUpperCase().padStart(TRAINING_TOKEN_LENGTH, '0')

/** The tempo changes of a spec: every section start where the tempo differs from what was in effect (the base tempo first). */
export const trainingBpmChangesOf = (spec: { base_bpm: number; sections: readonly { measure_from: number; bpm: number }[] }) => {
    let current = spec.base_bpm
    return spec.sections.flatMap((section): TrainingBpmChange[] => {
        if (section.bpm === current) return []
        current = section.bpm
        return [{ measure: section.measure_from, bpm: section.bpm }]
    })
}

/** The `#BPMxx` definitions of the changes: each tempo once, numbered from 01 in ascending order of tempo. */
export const trainingBpmDefinitionsOf = (changes: readonly TrainingBpmChange[]) =>
    [...new Set(changes.map((change) => change.bpm))]
        .toSorted((a, b) => a - b)
        .map((bpm, index): TrainingBpmDefinition => ({ id: trainingTokenOf(index + 1), bpm }))

/** The last index of a sorted array whose value is at most `value`, or -1. */
const lastIndexAtMost = (sorted: readonly number[], value: number) => {
    let low = 0
    let high = sorted.length - 1
    let found = -1
    while (low <= high) {
        const middle = (low + high) >> 1
        if (sorted[middle] <= value) {
            found = middle
            low = middle + 1
        } else {
            high = middle - 1
        }
    }
    return found
}

/**
 * The timeline the client's parser builds from these lines (`to_model` and `assign_times`, rbms-chart; 02 §4.5): a point at the
 * start of every measure from 0 to the last written one and at every token that is not `00`, at `measure + i/N` as a double. Times
 * add up `trunc(240_000_000 × (position − previous) / bpm)` with the tempo of the previous point, a channel 08 token setting the
 * tempo of its own point. `timeAt` gives the time of any position: one past the last point is reached from the last point.
 */
export const trainingTimelineOf = (lines: readonly WrittenLine[], initBpm: number, definitions: readonly TrainingBpmDefinition[]) => {
    const bpmOfToken = new Map(definitions.map((definition) => [definition.id, definition.bpm]))
    const lastMeasure = lines.reduce((last, line) => Math.max(last, line.measure), 0)
    const positionSet = new Set(Array.from({ length: lastMeasure + 1 }, (_, measure) => measure))
    const tempoAt = new Map<number, number>()
    for (const line of lines) {
        line.tokens.forEach((token, index) => {
            if (token === TRAINING_EMPTY_TOKEN) return
            const position = line.measure + index / line.resolution
            positionSet.add(position)
            const bpm = line.channel === TRAINING_BPM_CHANNEL ? bpmOfToken.get(token) : undefined
            if (bpm !== undefined && bpm > 0) tempoAt.set(position, bpm)
        })
    }
    const positions = [...positionSet].toSorted((a, b) => a - b)
    const times: number[] = []
    const bpms: number[] = []
    positions.forEach((position, index) => {
        const previousBpm = bpms[index - 1] ?? initBpm
        const time =
            index === 0 ? 0 : times[index - 1] + Math.trunc((MICROS_PER_MEASURE_AT_ONE_BPM * (position - positions[index - 1])) / previousBpm)
        times.push(time)
        bpms.push(tempoAt.get(position) ?? previousBpm)
    })
    const timeAt = (position: number) => {
        const at = lastIndexAtMost(positions, position)
        if (at < 0) return 0
        if (positions[at] === position) return times[at]
        return times[at] + Math.trunc((MICROS_PER_MEASURE_AT_ONE_BPM * (position - positions[at])) / bpms[at])
    }
    const tempo = positions.flatMap((position, index) =>
        tempoAt.has(position) && bpms[index] !== (bpms[index - 1] ?? initBpm) ? [{ t_us: times[index], bpm: bpms[index] }] : [],
    )
    return { timeAt, tempo }
}
