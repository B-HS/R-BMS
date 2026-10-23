import type { BundledCoursesFile } from '@server/dto/course-catalog'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { canonicalConstraints, lntypeOfConstraints } from '@server/service/domain/course-catalog/constraint-tokens'
import { courseHashOf } from '@server/service/domain/course-catalog/course-hash'
import type { CatalogCourse, CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { danOfCourse } from '@server/service/domain/course-catalog/dan-of-course'

export const GENOSIDE_2018_SOURCE_ID = 'genoside-2018'
export const GENOSIDE_2018_SOURCE_NAME = 'GENOSIDE 2018 段位認定'

const UTC_MIDNIGHT = 'T00:00:00Z'

const invalidFile = (field: string, reason: string) => createAppError(ERROR_CODE.VALIDATION_ERROR, { field, reason })

const toRetrievedAt = (retrievedOn: string) => {
    const retrievedAt = Date.parse(`${retrievedOn}${UTC_MIDNIGHT}`)
    if (Number.isNaN(retrievedAt)) throw invalidFile('source.retrieved_on', 'not a calendar date')
    return retrievedAt
}

/**
 * The definition of the courses a bundled course file holds. Every course id is calculated again from the course's name and
 * charts and compared with the id the file carries, so a file written under a different id rule is refused before anything is
 * registered.
 */
export const bundledCatalog = (file: BundledCoursesFile): CatalogDefinition => {
    const { source } = file
    const { maker, page_url, file_url, retrieved_on, file_sha256, ...detail } = source
    const seenHashes = new Set<string>()
    return {
        source: {
            id: GENOSIDE_2018_SOURCE_ID,
            kind: 'bundled',
            name: GENOSIDE_2018_SOURCE_NAME,
            maker,
            pageUrl: page_url,
            fileUrl: file_url,
            retrievedAt: toRetrievedAt(retrieved_on),
            fileSha256: file_sha256,
            detail,
        },
        groups: file.groups.map((group, groupIndex) => ({
            id: group.id,
            sourceId: GENOSIDE_2018_SOURCE_ID,
            name: group.name,
            keys: group.keys,
            position: groupIndex,
            courses: group.courses.map((course, index): CatalogCourse => {
                const charts = course.charts.map((chart) => ({ md5: chart.md5, sha256: chart.sha256, title: chart.title }))
                const courseHash = courseHashOf({ name: course.name, charts })
                if (courseHash !== course.course_hash) throw invalidFile(`${group.id}[${index}].course_hash`, 'not the hash of the name and charts')
                if (seenHashes.has(courseHash)) throw invalidFile(`${group.id}[${index}].course_hash`, 'listed twice')
                seenHashes.add(courseHash)
                const constraint = canonicalConstraints(course.constraint)
                return {
                    courseHash,
                    name: course.name,
                    lntype: lntypeOfConstraints(constraint),
                    constraint,
                    trophy: course.trophy.map((rule) => ({ name: rule.name, scorerate: rule.scorerate, smissrate: rule.missrate })),
                    charts,
                    dan: danOfCourse(group.id, course.name, index),
                }
            }),
        })),
    }
}
