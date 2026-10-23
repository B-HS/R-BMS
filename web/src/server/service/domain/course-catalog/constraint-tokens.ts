import { asciiLowercase } from '@server/service/domain/course-catalog/course-hash'

type ConstraintSpelling = { token: string; referenceName: string; group: number }

/** The fourteen course constraints in declaration order: the wire token, the name the reference implementation writes, and the exclusive group (`CourseConstraint`). */
const CONSTRAINTS: readonly ConstraintSpelling[] = [
    { token: 'grade', referenceName: 'CLASS', group: 0 },
    { token: 'grade_mirror', referenceName: 'MIRROR', group: 0 },
    { token: 'grade_random', referenceName: 'RANDOM', group: 0 },
    { token: 'no_speed', referenceName: 'NO_SPEED', group: 1 },
    { token: 'no_good', referenceName: 'NO_GOOD', group: 2 },
    { token: 'no_great', referenceName: 'NO_GREAT', group: 2 },
    { token: 'gauge_lr2', referenceName: 'GAUGE_LR2', group: 3 },
    { token: 'gauge_5k', referenceName: 'GAUGE_5KEYS', group: 3 },
    { token: 'gauge_7k', referenceName: 'GAUGE_7KEYS', group: 3 },
    { token: 'gauge_9k', referenceName: 'GAUGE_9KEYS', group: 3 },
    { token: 'gauge_24k', referenceName: 'GAUGE_24KEYS', group: 3 },
    { token: 'ln', referenceName: 'LN', group: 4 },
    { token: 'cn', referenceName: 'CN', group: 4 },
    { token: 'hcn', referenceName: 'HCN', group: 4 },
]

const CONSTRAINT_GROUP_COUNT = 5

/** The long note types in the order the client numbers them: 0 LN, 1 CN, 2 HCN. */
const LONG_NOTE_TOKENS = ['ln', 'cn', 'hcn']

const findConstraint = (spelling: string) => {
    const lowered = asciiLowercase(spelling)
    return CONSTRAINTS.find((constraint) => constraint.token === lowered || asciiLowercase(constraint.referenceName) === lowered)
}

/**
 * The constraint tokens a course carries, as the client keeps them (`Course::validate`): either spelling is read ignoring
 * ASCII case, a spelling nobody knows is dropped, and only the first constraint of each exclusive group is kept, listed in
 * group order.
 */
export const canonicalConstraints = (raw: readonly string[]) => {
    const slots: (string | undefined)[] = Array.from({ length: CONSTRAINT_GROUP_COUNT }, () => undefined)
    for (const spelling of raw) {
        const constraint = findConstraint(spelling)
        if (constraint && slots[constraint.group] === undefined) slots[constraint.group] = constraint.token
    }
    return slots.flatMap((token) => token ?? [])
}

/** The long note type a course forces: 0 LN, 1 CN, 2 HCN, and 0 when it names none. */
export const lntypeOfConstraints = (tokens: readonly string[]) =>
    Math.max(
        0,
        LONG_NOTE_TOKENS.findIndex((token) => tokens.includes(token)),
    )
