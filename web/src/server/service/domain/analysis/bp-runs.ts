import {
    BP_RUN_BUCKETS,
    BP_RUN_CONTEXT_MIN,
    BP_RUN_MIN_REPORT,
    BP_RUN_TOP,
    CONTEXT_TOP_TAGS,
    NPS_SUMMARY_DECIMALS,
    PRECEDING_NOTES,
    RECOVERY_STREAK,
} from '@server/service/domain/analysis/analysis-constants'
import type { PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { countTagsOf, roundToDecimals, tagCountRecordOf, topTagsOf, type Grade } from '@server/service/domain/analysis/play-cells'

const MICROS_PER_MILLI = 1000

/** A note that took a judgement, as the BP runs read it: its time, judgement, tags and local NPS. */
export type RunNote = { timeUs: number; grade: Grade; tags: readonly PatternTag[]; nps2: number }

const isBreak = (grade: Grade) => grade === 'bad' || grade === 'poor'

const isClean = (grade: Grade) => grade === 'pgreat' || grade === 'great'

/** The bucket names of BP_RUN_BUCKETS: a single length where the next edge follows at once, a range otherwise, the last open. */
export const BP_RUN_BUCKET_KEYS = BP_RUN_BUCKETS.map((edge, index) => {
    if (index === BP_RUN_BUCKETS.length - 1) return `${edge}+`
    const last = BP_RUN_BUCKETS[index + 1] - 1
    return last === edge ? String(edge) : `${edge}-${last}`
})

const bucketKeyOf = (length: number) => BP_RUN_BUCKET_KEYS[BP_RUN_BUCKETS.filter((edge) => length >= edge).length - 1]

/**
 * The runs of BAD and POOR in a play (plan 2026-11-04 2.2.4) from its judged notes in time then lane order (an empty POOR is not a
 * note and does not take part): a histogram of run lengths, the longest run, the BP_RUN_TOP longest runs of BP_RUN_MIN_REPORT notes
 * or more (earlier first between equals) with the local NPS at their first note, how many judged notes after their end it took to
 * come RECOVERY_STREAK PGREAT or GREAT in a row (null when it never did) and the tags of the PRECEDING_NOTES notes before them, and the
 * tag counts over the notes before every run of BP_RUN_CONTEXT_MIN notes or more, each note counted once.
 */
export const computeBpRuns = (notes: readonly RunNote[]) => {
    const runs: { start: number; end: number }[] = []
    let start = -1
    notes.forEach((note, index) => {
        if (isBreak(note.grade)) {
            if (start < 0) start = index
            return
        }
        if (start >= 0) runs.push({ start, end: index - 1 })
        start = -1
    })
    if (start >= 0) runs.push({ start, end: notes.length - 1 })
    const lengthOf = (run: { start: number; end: number }) => run.end - run.start + 1
    const precedingOf = (runStart: number) => notes.slice(Math.max(0, runStart - PRECEDING_NOTES), runStart)

    const histogram = Object.fromEntries(BP_RUN_BUCKET_KEYS.map((key) => [key, 0]))
    for (const run of runs) histogram[bucketKeyOf(lengthOf(run))] += 1

    const recoveryOf = (end: number) => {
        let streak = 0
        for (let index = end + 1; index < notes.length; index += 1) {
            streak = isClean(notes[index].grade) ? streak + 1 : 0
            if (streak >= RECOVERY_STREAK) return index - end
        }
        return null
    }

    const contextIndexes = new Set(
        runs
            .filter((run) => lengthOf(run) >= BP_RUN_CONTEXT_MIN)
            .flatMap((run) => Array.from({ length: Math.min(PRECEDING_NOTES, run.start) }, (_, offset) => run.start - 1 - offset)),
    )
    const contextCounts = countTagsOf([...contextIndexes].map((index) => notes[index].tags))

    return {
        histogram,
        longest: runs.reduce((longest, run) => Math.max(longest, lengthOf(run)), 0),
        runs: runs
            .filter((run) => lengthOf(run) >= BP_RUN_MIN_REPORT)
            .toSorted((a, b) => lengthOf(b) - lengthOf(a) || a.start - b.start)
            .slice(0, BP_RUN_TOP)
            .map((run) => ({
                from_ms: Math.floor(notes[run.start].timeUs / MICROS_PER_MILLI),
                to_ms: Math.floor(notes[run.end].timeUs / MICROS_PER_MILLI),
                length: lengthOf(run),
                start_nps: roundToDecimals(notes[run.start].nps2, NPS_SUMMARY_DECIMALS),
                recovery_notes: recoveryOf(run.end),
                preceding_tags: topTagsOf(countTagsOf(precedingOf(run.start).map((note) => note.tags)), CONTEXT_TOP_TAGS),
            })),
        context_tags: tagCountRecordOf(contextCounts),
        context_notes: contextIndexes.size,
    }
}

export type BpRuns = ReturnType<typeof computeBpRuns>
