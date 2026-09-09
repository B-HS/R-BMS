import type { CourseMeta } from '@entities/course.type'

export type TableChartRef = {
    md5: string
    sha256: string
}

export type TableFolder = {
    name: string
    charts: TableChartRef[]
}

export type TableData = {
    id: string
    name: string
    url: string | null
    folders: TableFolder[]
    courses: CourseMeta[]
    extra: Record<string, unknown>
}
