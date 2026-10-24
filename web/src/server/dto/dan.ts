import { z } from 'zod'
import { courseGaugeRuleSchema } from '@server/dto/course'
import { DAN_LABEL_MAX_LENGTH, danSystemIdSchema } from '@server/dto/course-catalog'

export const DAN_DISPLAY_MODES = ['auto', 'hidden', 'label'] as const

export const DAN_DISPLAY_BODY_MAX_BYTES = 1024

export type DanDisplayMode = (typeof DAN_DISPLAY_MODES)[number]

/** What the label of a dan earned with assist starts with where it is shown: `A-十段`, `A-st3`. */
export const ASSISTED_DAN_LABEL_PREFIX = 'A-'

/** The label a dan is shown with: its own label, after `A-` when it was earned with assist. */
export const danDisplayLabel = (dan: { label: string; assisted: boolean }) => (dan.assisted ? `${ASSISTED_DAN_LABEL_PREFIX}${dan.label}` : dan.label)

/**
 * Which dan of one system a player shows to others: `auto` the highest one acquired (and also the highest A-dan when it ranks
 * above it), `hidden` none, `label` the one named, the A-dan of that label when `assisted` is true.
 */
export const danDisplayInputSchema = z.discriminatedUnion('mode', [
    z.object({ system: danSystemIdSchema, mode: z.literal('auto') }),
    z.object({ system: danSystemIdSchema, mode: z.literal('hidden') }),
    z.object({
        system: danSystemIdSchema,
        mode: z.literal('label'),
        label: z.string().min(1).max(DAN_LABEL_MAX_LENGTH),
        assisted: z.boolean().default(false),
    }),
])

/**
 * A dan others see next to a player: the system, its display name, the label it is shown with (`A-` first for an A-dan) and
 * whether it was earned with assist. A list stored before `assisted` existed reads as regular dans.
 */
export const displayedDanSchema = z.object({ system: danSystemIdSchema, name: z.string(), label: z.string(), assisted: z.boolean().default(false) })

export const danDisplayOutputSchema = z.object({ mode: z.enum(DAN_DISPLAY_MODES), label: z.string().nullable(), assisted: z.boolean() })

export const acquiredDanOutputSchema = z.object({
    label: z.string(),
    assisted: z.boolean(),
    rank: z.number().int(),
    course_hash: z.string(),
    course_name: z.string(),
    course_gauge: courseGaugeRuleSchema,
    acquired_at: z.number().int(),
})

export const danSystemOutputSchema = z.object({
    system: danSystemIdSchema,
    name: z.string(),
    display: danDisplayOutputSchema,
    shown: z.array(displayedDanSchema),
    acquired: z.array(acquiredDanOutputSchema),
})

export const myDansOutputSchema = z.object({ systems: z.array(danSystemOutputSchema) })

export type DanDisplayInput = z.infer<typeof danDisplayInputSchema>
export type DisplayedDan = z.infer<typeof displayedDanSchema>
export type DanDisplayOutput = z.infer<typeof danDisplayOutputSchema>
export type AcquiredDanOutput = z.infer<typeof acquiredDanOutputSchema>
export type MyDansOutput = z.infer<typeof myDansOutputSchema>
