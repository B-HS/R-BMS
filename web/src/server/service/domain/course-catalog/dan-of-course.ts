import { INT_COLUMN_MAX } from '@server/dto/common'
import { DAN_LABEL_MAX_LENGTH, type DanSystemId } from '@server/dto/course-catalog'
import type { CatalogDan } from '@server/service/domain/course-catalog/course-catalog.type'

type DanSystemRule = {
    id: DanSystemId
    name: string
    groupId: string
    read: (courseName: string, indexInGroup: number) => Omit<CatalogDan, 'system'> | null
}

const GENOSIDE_SP_NAME_PREFIX = 'GENOSIDE 2018 段位認定 '
const GENOSIDE_DP_NAME_PREFIX = 'GENOSIDE 2018 DP段位認定 '
const STELLA_NAME = /^Stella Skill Simulator \S+ (st)(\d+)$/
const SATELLITE_NAME = /^Satellite Skill Analyzer \S+ (sl)(\d+)$/

const isRankStorable = (rank: number) => Number.isSafeInteger(rank) && rank <= INT_COLUMN_MAX

const byPrefix = (prefix: string) => (courseName: string, indexInGroup: number) => {
    const label = courseName.startsWith(prefix) ? courseName.slice(prefix.length) : ''
    return label.length > 0 && label.length <= DAN_LABEL_MAX_LENGTH ? { label, rank: indexInGroup + 1 } : null
}

const byLevelLabel = (pattern: RegExp) => (courseName: string) => {
    const [, symbol, digits] = pattern.exec(courseName) ?? []
    const rank = Number(digits)
    return symbol !== undefined && isRankStorable(rank) ? { label: `${symbol}${rank}`, rank } : null
}

/** The dan systems and where each one's courses are: the group they belong to and how a course name gives its dan (label) and order (rank). */
const DAN_SYSTEM_RULES: readonly DanSystemRule[] = [
    { id: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', groupId: 'genoside-2018-sp', read: byPrefix(GENOSIDE_SP_NAME_PREFIX) },
    { id: 'genoside2018_dp', name: 'GENOSIDE 2018 DP', groupId: 'genoside-2018-dp', read: byPrefix(GENOSIDE_DP_NAME_PREFIX) },
    { id: 'stella', name: 'Stella', groupId: 'stella-st', read: byLevelLabel(STELLA_NAME) },
    { id: 'satellite', name: 'Satellite', groupId: 'stella-sl', read: byLevelLabel(SATELLITE_NAME) },
]

export const DAN_SYSTEMS = DAN_SYSTEM_RULES.map(({ id, name, groupId }) => ({ id, name, groupId }))

export const danSystemName = (system: DanSystemId) => DAN_SYSTEM_RULES.find((rule) => rule.id === system)?.name ?? system

/**
 * The dan a course of a group gives when it is cleared under the conditions, or null when the course is no dan course:
 * the GENOSIDE courses by their place in the group (the first is rank 1), Stella and Satellite by the level in the name
 * (`st0` to `st12`). The training courses of Stella (`STELLA技能訓練 ...`) give none.
 */
export const danOfCourse = (groupId: string, courseName: string, indexInGroup: number): CatalogDan | null => {
    const rule = DAN_SYSTEM_RULES.find((candidate) => candidate.groupId === groupId)
    const read = rule?.read(courseName, indexInGroup)
    return rule && read ? { system: rule.id, ...read } : null
}
