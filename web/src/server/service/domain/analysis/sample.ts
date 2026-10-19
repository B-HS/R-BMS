import { MIN_TAG_NOTES, MIN_TAG_PLAYS, MIN_TIMING_NOTES, MIN_TIMING_PLAYS, WILSON_Z_95 } from '@server/service/domain/analysis/analysis-constants'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import { PATTERN_TAGS, SCRATCH_TAGS, type PatternTag } from '@server/service/domain/analysis/pattern-tags'
import type { Tally } from '@server/service/domain/analysis/play-metrics'

/** How far a claim about a tag may go (plan 4.6). The AI is given only this grade, never the interval. */
export const EVIDENCE = { STRONG: 'strong', UNCERTAIN: 'uncertain', INSUFFICIENT: 'insufficient' } as const
export type Evidence = (typeof EVIDENCE)[keyof typeof EVIDENCE]

export const TAG_LEANING = { WEAKNESS: 'weakness', STRENGTH: 'strength' } as const
export type TagLeaning = (typeof TAG_LEANING)[keyof typeof TAG_LEANING]

/** The Wilson score interval of `successes` out of `trials`; the whole range when there is no trial. */
export const wilsonInterval = (successes: number, trials: number, z = WILSON_Z_95) => {
    if (trials <= 0) return { low: 0, high: 1 }
    const share = successes / trials
    const zSquared = z * z
    const denominator = 1 + zSquared / trials
    const center = (share + zSquared / (2 * trials)) / denominator
    const margin = (z * Math.sqrt((share * (1 - share)) / trials + zSquared / (4 * trials * trials))) / denominator
    return { low: Math.max(0, center - margin), high: Math.min(1, center + margin) }
}

const EX_PER_NOTE = 2

/** One play's tallies, overall and by tag, with whether the scratch was played by SCRATCH AUTO. */
export type TagPlaySample = {
    scratchAuto: boolean
    overall: Pick<Tally, 'notes' | 'ex_score'>
    byTag: Partial<Record<PatternTag, Pick<Tally, 'notes' | 'ex_score'>>>
}

const isScratchTag = (tag: PatternTag) => (SCRATCH_TAGS as readonly PatternTag[]).includes(tag)

const gradeOf = (isEnough: boolean, isSeparated: boolean) => {
    if (!isEnough) return EVIDENCE.INSUFFICIENT
    return isSeparated ? EVIDENCE.STRONG : EVIDENCE.UNCERTAIN
}

const leaningOf = (grade: Evidence, residual: number) => {
    if (grade !== EVIDENCE.STRONG) return null
    return residual < 0 ? TAG_LEANING.WEAKNESS : TAG_LEANING.STRENGTH
}

const tagEvidenceOf = (plays: readonly TagPlaySample[], tag: PatternTag) => {
    const counted = plays.filter((play) => !(play.scratchAuto && isScratchTag(tag)))
    const samples = counted.flatMap((play) => {
        const tagTally = play.byTag[tag]
        return tagTally && tagTally.notes > 0 && play.overall.notes > 0 ? [{ tag: tagTally, overall: play.overall }] : []
    })
    const notes = samples.reduce((sum, sample) => sum + sample.tag.notes, 0)
    const exScore = samples.reduce((sum, sample) => sum + sample.tag.ex_score, 0)
    const baselineWeighted = samples.reduce(
        (sum, sample) => sum + (sample.tag.notes * sample.overall.ex_score) / (EX_PER_NOTE * sample.overall.notes),
        0,
    )
    const exRatio = notes > 0 ? exScore / (EX_PER_NOTE * notes) : 0
    const baseline = notes > 0 ? baselineWeighted / notes : 0
    const residual = exRatio - baseline
    const interval = wilsonInterval(exScore, EX_PER_NOTE * notes)
    const grade = gradeOf(notes >= MIN_TAG_NOTES && samples.length >= MIN_TAG_PLAYS, baseline < interval.low || baseline > interval.high)
    return {
        notes,
        plays: samples.length,
        auto_plays: plays.length - counted.length,
        ex_ratio: roundRatio(exRatio),
        baseline_ex_ratio: roundRatio(baseline),
        residual: roundRatio(residual),
        grade,
        leaning: leaningOf(grade, residual),
    }
}

export type TagEvidence = ReturnType<typeof tagEvidenceOf>

/**
 * A tag's weakness or strength across plays (plan 4.4 and 4.6), keyed by tag. The residual is, play by play, the tag notes' EX
 * ratio less the whole play's, averaged with the tag's notes as weights, so playing harder charts does not read as a weakness. It
 * is called a weakness or a strength only with MIN_TAG_NOTES notes over MIN_TAG_PLAYS plays (`insufficient` below that), and only
 * when the 95% Wilson interval of the tag's pooled EX ratio leaves out the note-weighted play baseline (`uncertain` when it does
 * not). A play under SCRATCH AUTO is left out of the scratch tags and counted in `auto_plays` instead.
 */
export const aggregateTagEvidence = (plays: readonly TagPlaySample[]) =>
    Object.fromEntries(PATTERN_TAGS.map((tag) => [tag, tagEvidenceOf(plays, tag)]))

/** Whether there is enough timing to suggest an offset number (plan 4.6): MIN_TIMING_PLAYS plays and MIN_TIMING_NOTES PGREAT and GREAT notes. */
export const timingEvidence = (sample: { plays: number; notes: number }) =>
    sample.plays >= MIN_TIMING_PLAYS && sample.notes >= MIN_TIMING_NOTES ? EVIDENCE.STRONG : EVIDENCE.INSUFFICIENT
