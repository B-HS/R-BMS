import { z } from 'zod'
import { displayedDanSchema } from '@server/dto/dan'
import type { AcquiredDanOutput, DanDisplayInput, DanDisplayOutput, DisplayedDan, MyDansOutput } from '@server/dto/dan'
import type { PlayerProfile } from '@entities/player.type'

export type { DisplayedDan }

export type DanDisplay = DanDisplayOutput
export type DanDisplayRequest = DanDisplayInput
export type AcquiredDan = AcquiredDanOutput
export type MyDans = MyDansOutput
export type DanSystem = MyDansOutput['systems'][number]

const displayedDanListSchema = z.array(displayedDanSchema)

/** The dans a player profile shows to others; a profile written before dans existed carries none. */
export const readDisplayedDans = (extra: PlayerProfile['extra']) => displayedDanListSchema.catch([]).parse(extra.dans)
