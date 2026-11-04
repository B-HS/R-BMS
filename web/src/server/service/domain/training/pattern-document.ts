import {
    CHART_PATTERN_FORMAT,
    CHART_PATTERN_FORMAT_VERSION,
    CHART_PATTERN_MODE,
    CHART_PATTERN_NOTE_KIND,
    chartPatternDocumentSchema,
    type ChartPatternNoteKind,
} from '@server/dto/chart-pattern'
import { trainingTotalOf } from '@server/service/domain/training/bms-writer'
import { trainingBpmChangesOf, trainingBpmDefinitionsOf, trainingTimelineOf, type WrittenLine } from '@server/service/domain/training/chart-timing'
import type { TrainingSpec } from '@server/service/domain/training/spec'
import {
    TRAINING_EMPTY_TOKEN,
    TRAINING_LANE_CHANNELS,
    TRAINING_LN_CHANNELS,
    TRAINING_PARSER_VERSION,
    TRAINING_RANK,
} from '@server/service/domain/training/training-constants'

const NEUTRAL_SCROLL = 1

type Placed = { position: number; lane: number; kind: ChartPatternNoteKind }

const laneByChannel = (channels: readonly string[]) => new Map(channels.map((channel, lane) => [channel, lane]))
const LANE_OF_CHANNEL = laneByChannel(TRAINING_LANE_CHANNELS)
const LANE_OF_LN_CHANNEL = laneByChannel(TRAINING_LN_CHANNELS)

/**
 * The pattern document of a training chart as `rbms-cli chart-pattern` would write it from the chart's bytes (plan 3.10): positions
 * read from the written lines with the token count they were written with, times on the client's timeline, the long-note objects
 * of a lane paired in time order (head, then end). The golden fixtures of `contract/training-chart/` hold the two sides to one
 * another. `declared_ln` is `ln` when the chart has a long note and null when it has none, as the client reads it.
 */
export const buildTrainingPatternDocument = (params: { spec: TrainingSpec; lines: readonly WrittenLine[]; md5: string; sha256: string }) => {
    const { spec, lines } = params
    const timeline = trainingTimelineOf(lines, spec.base_bpm, trainingBpmDefinitionsOf(trainingBpmChangesOf(spec)))
    const tokensOf = (line: WrittenLine) =>
        line.tokens.flatMap((token, index) => (token === TRAINING_EMPTY_TOKEN ? [] : [line.measure + index / line.resolution]))
    const normals = lines.flatMap((line) => {
        const lane = LANE_OF_CHANNEL.get(line.channel)
        return lane === undefined ? [] : tokensOf(line).map((position): Placed => ({ position, lane, kind: CHART_PATTERN_NOTE_KIND.NORMAL }))
    })
    const longObjects = lines.flatMap((line) => {
        const lane = LANE_OF_LN_CHANNEL.get(line.channel)
        return lane === undefined ? [] : tokensOf(line).map((position) => ({ position, lane }))
    })
    const longs = [...Map.groupBy(longObjects, (object) => object.lane).values()].flatMap((objects) =>
        objects
            .toSorted((a, b) => a.position - b.position)
            .map((object, order): Placed => ({
                ...object,
                kind: order % 2 === 0 ? CHART_PATTERN_NOTE_KIND.LONG_HEAD : CHART_PATTERN_NOTE_KIND.LONG_END,
            })),
    )
    const placed = [...normals, ...longs]
        .map((note) => ({ ...note, timeUs: timeline.timeAt(note.position) }))
        .toSorted((a, b) => a.timeUs - b.timeUs || a.lane - b.lane)
    const playable = placed.filter((note) => note.kind !== CHART_PATTERN_NOTE_KIND.LONG_END).length
    return chartPatternDocumentSchema.parse({
        format: CHART_PATTERN_FORMAT,
        version: CHART_PATTERN_FORMAT_VERSION,
        parser_version: TRAINING_PARSER_VERSION,
        md5: params.md5,
        sha256: params.sha256,
        mode: CHART_PATTERN_MODE,
        declared_ln: longs.length > 0 ? 'ln' : null,
        has_undefined_ln: false,
        uses_control_flow: false,
        init_bpm: spec.base_bpm,
        total: trainingTotalOf(playable),
        rank: TRAINING_RANK,
        play_level: spec.play_level,
        playable_notes: playable,
        notes: {
            time_us: placed.map((note) => note.timeUs),
            lane: placed.map((note) => note.lane),
            kind: placed.map((note) => note.kind),
            section: placed.map((note) => note.position),
        },
        tempo: timeline.tempo.map((point) => ({ t_us: point.t_us, bpm: point.bpm, stop_us: 0, scroll: NEUTRAL_SCROLL })),
    })
}
