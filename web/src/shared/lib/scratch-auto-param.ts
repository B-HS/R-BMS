import { DEFAULT_INCLUDE_SCRATCH_AUTO, INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'
import { readBooleanParam, writeBooleanParam } from '@shared/lib/search-params'

export const readIncludeScratchAutoParam = (params: Record<string, string | string[] | undefined>) =>
    readBooleanParam(params, INCLUDE_SCRATCH_AUTO_PARAM, DEFAULT_INCLUDE_SCRATCH_AUTO)

export const writeIncludeScratchAutoParam = (include: boolean) => writeBooleanParam(include, DEFAULT_INCLUDE_SCRATCH_AUTO)
