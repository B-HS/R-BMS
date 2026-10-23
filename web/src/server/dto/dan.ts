import { z } from 'zod'
import { courseGaugeRuleSchema } from '@server/dto/course'
import { DAN_LABEL_MAX_LENGTH, danSystemIdSchema } from '@server/dto/course-catalog'

export const DAN_DISPLAY_MODES = ['auto', 'hidden', 'label'] as const

export const DAN_DISPLAY_BODY_MAX_BYTES = 1024

export type DanDisplayMode = (typeof DAN_DISPLAY_MODES)[number]

/** Which dan of one system a player shows to others: `auto` the highest one acquired, `hidden` none, `label` the one named. */
export const danDisplayInputSchema = z.discriminatedUnion('mode', [
    z.object({ system: danSystemIdSchema, mode: z.literal('auto') }),
    z.object({ system: danSystemIdSchema, mode: z.literal('hidden') }),
    z.object({ system: danSystemIdSchema, mode: z.literal('label'), label: z.string().min(1).max(DAN_LABEL_MAX_LENGTH) }),
])

/** A dan others see next to a player: the system, its display name and the label of the dan shown. */
export const displayedDanSchema = z.object({ system: danSystemIdSchema, name: z.string(), label: z.string() })

export const danDisplayOutputSchema = z.object({ mode: z.enum(DAN_DISPLAY_MODES), label: z.string().nullable() })

export const acquiredDanOutputSchema = z.object({
    label: z.string(),
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
    shown: z.string().nullable(),
    acquired: z.array(acquiredDanOutputSchema),
})

export const myDansOutputSchema = z.object({ systems: z.array(danSystemOutputSchema) })

export type DanDisplayInput = z.infer<typeof danDisplayInputSchema>
export type DisplayedDan = z.infer<typeof displayedDanSchema>
export type DanDisplayOutput = z.infer<typeof danDisplayOutputSchema>
export type AcquiredDanOutput = z.infer<typeof acquiredDanOutputSchema>
export type MyDansOutput = z.infer<typeof myDansOutputSchema>
