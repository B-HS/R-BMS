export type CourseChartRef = {
    md5: string
    sha256: string
}

export type CourseTrophy = {
    name: string
    scorerate: number
    smissrate: number
}

export type CourseMeta = {
    course_hash: string
    name: string
    lntype: number
    charts: CourseChartRef[]
    constraint: string[]
    trophy: CourseTrophy[]
    extra: Record<string, unknown>
}
