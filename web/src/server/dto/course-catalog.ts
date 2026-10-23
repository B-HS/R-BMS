import { z } from 'zod'
import { MD5_LENGTH, SHA256_LENGTH } from '@server/dto/common'

export const CATALOG_ID_MAX_LENGTH = 64
export const CATALOG_NAME_MAX_LENGTH = 255
export const CATALOG_URL_MAX_LENGTH = 512
export const DAN_LABEL_MAX_LENGTH = 64

export const COURSE_SOURCE_KINDS = ['bundled', 'table'] as const

export type CourseSourceKind = (typeof COURSE_SOURCE_KINDS)[number]

export const DAN_SYSTEM_IDS = ['genoside2018_sp', 'genoside2018_dp', 'stella', 'satellite'] as const

export type DanSystemId = (typeof DAN_SYSTEM_IDS)[number]

export const danSystemIdSchema = z.enum(DAN_SYSTEM_IDS)

const MD5_PATTERN = new RegExp(`^[0-9a-fA-F]{${MD5_LENGTH}}$`)
const SHA256_PATTERN = new RegExp(`^[0-9a-fA-F]{${SHA256_LENGTH}}$`)
const RETRIEVED_ON_PATTERN = /^\d{4}-\d{2}-\d{2}$/
const BUNDLED_FILE_SCHEMA_VERSION = 1

const lowercaseHash = (value: string) => value.toLowerCase()

const catalogIdSchema = z.string().min(1).max(CATALOG_ID_MAX_LENGTH)
const catalogNameSchema = z.string().max(CATALOG_NAME_MAX_LENGTH)
const catalogUrlSchema = z.string().max(CATALOG_URL_MAX_LENGTH)

/** The course file of a bundled course set, as `web/data/courses/<set>/courses.json` holds it (written by `rbms-cli course-snapshot`). */
export const bundledCoursesFileSchema = z.object({
    schema: z.literal(BUNDLED_FILE_SCHEMA_VERSION),
    source: z.object({
        maker: catalogNameSchema,
        page_url: catalogUrlSchema,
        file_url: catalogUrlSchema,
        retrieved_on: z.string().regex(RETRIEVED_ON_PATTERN),
        file_bytes: z.number().int().min(0),
        file_sha256: z.string().regex(SHA256_PATTERN).transform(lowercaseHash),
        entry_name: z.string(),
        entry_sha256: z.string().regex(SHA256_PATTERN).transform(lowercaseHash),
        entry_hash_prefix: z.string(),
        entry_type: z.number().int(),
        constraint_sources: z.array(
            z.object({ url: z.string(), sha256: z.string().regex(SHA256_PATTERN).transform(lowercaseHash), retrieved_on: z.string() }),
        ),
        title_source: z.string(),
    }),
    groups: z.array(
        z.object({
            id: catalogIdSchema,
            name: catalogNameSchema,
            keys: z.number().int().min(0),
            courses: z.array(
                z.object({
                    course_hash: z.string().min(1),
                    name: catalogNameSchema,
                    charts: z.array(
                        z.object({
                            md5: z.string().regex(MD5_PATTERN).transform(lowercaseHash),
                            sha256: z.union([z.literal(''), z.string().regex(SHA256_PATTERN).transform(lowercaseHash)]).default(''),
                            title: z.string().max(CATALOG_NAME_MAX_LENGTH).default(''),
                        }),
                    ),
                    constraint: z.array(z.string()),
                    trophy: z.array(z.object({ name: z.string(), missrate: z.number(), scorerate: z.number() })),
                    release: z.boolean(),
                }),
            ),
        }),
    ),
})

export type BundledCoursesFile = z.infer<typeof bundledCoursesFileSchema>

export const courseCatalogSourceOutputSchema = z.object({
    id: z.string(),
    kind: z.enum(COURSE_SOURCE_KINDS),
    name: z.string(),
    maker: z.string(),
    page_url: z.string(),
    file_url: z.string(),
    retrieved_at: z.number().int(),
    file_sha256: z.string(),
    detail: z.record(z.string(), z.unknown()),
})

export const courseCatalogDanOutputSchema = z.object({
    system: danSystemIdSchema,
    system_name: z.string(),
    label: z.string(),
    rank: z.number().int(),
})

export const courseCatalogCourseOutputSchema = z.object({
    course_hash: z.string(),
    name: z.string(),
    stage_count: z.number().int(),
    active: z.boolean(),
    dan: courseCatalogDanOutputSchema.nullable(),
})

export const courseCatalogGroupOutputSchema = z.object({
    id: z.string(),
    name: z.string(),
    keys: z.number().int(),
    source: courseCatalogSourceOutputSchema,
    courses: z.array(courseCatalogCourseOutputSchema),
})

export const courseCatalogOutputSchema = z.object({ groups: z.array(courseCatalogGroupOutputSchema) })

export const courseCatalogEntryOutputSchema = z.object({
    course_hash: z.string(),
    name: z.string(),
    active: z.boolean(),
    group: z.object({ id: z.string(), name: z.string(), keys: z.number().int() }),
    source: courseCatalogSourceOutputSchema,
    dan: courseCatalogDanOutputSchema.nullable(),
    charts: z.array(z.object({ position: z.number().int(), md5: z.string(), sha256: z.string(), title: z.string() })),
})

export type CourseCatalogSourceOutput = z.infer<typeof courseCatalogSourceOutputSchema>
export type CourseCatalogDanOutput = z.infer<typeof courseCatalogDanOutputSchema>
export type CourseCatalogOutput = z.infer<typeof courseCatalogOutputSchema>
export type CourseCatalogEntryOutput = z.infer<typeof courseCatalogEntryOutputSchema>
