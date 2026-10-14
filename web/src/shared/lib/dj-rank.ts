const RATE_STEPS = 27

const STEP_GRADE_LABELS = [
    'F-',
    'F-',
    'F',
    'F',
    'F+',
    'F+',
    'E-',
    'E',
    'E+',
    'D-',
    'D',
    'D+',
    'C-',
    'C',
    'C+',
    'B-',
    'B',
    'B+',
    'A-',
    'A',
    'A+',
    'AA-',
    'AA',
    'AA+',
    'AAA-',
    'AAA',
    'MAX-',
]

const PERFECT_GRADE = 'MAX'
const STRONG_GRADE_FIRST_STEP = 24
const MEDIUM_GRADE_FIRST_STEP = 18

/** Counts how many whole twenty-sevenths of the chart ceiling a score reached, the same integer test the player uses. */
export const getRateStep = (exScore: number, maxExScore: number) => {
    if (maxExScore <= 0 || exScore <= 0) return 0
    return Math.min(Math.floor((exScore * RATE_STEPS) / maxExScore), RATE_STEPS)
}

/** Names the DJ grade of a score with the player's twenty-seven step table (`F-` … `MAX-`, `MAX` for the ceiling). */
export const getDjRank = (exScore: number, maxExScore: number) => {
    const step = getRateStep(exScore, maxExScore)
    return step >= RATE_STEPS ? PERFECT_GRADE : STEP_GRADE_LABELS[step]
}

/** Picks the weight and tone a grade is written in: ranks carry no colour, only emphasis. */
export const getDjRankClassName = (exScore: number, maxExScore: number) => {
    const step = getRateStep(exScore, maxExScore)
    if (step >= STRONG_GRADE_FIRST_STEP) return 'font-semibold text-foreground'
    if (step >= MEDIUM_GRADE_FIRST_STEP) return 'font-medium text-foreground'
    return 'text-muted-foreground'
}
