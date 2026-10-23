import { createHash } from 'node:crypto'
import { createCourseCatalogService } from '@server/service/domain/course-catalog/course-catalog.service'
import type { CatalogCourse, CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { createMemoryCourseCatalogDb } from './course-catalog-memory-db'

export const GENOSIDE_RETRIEVED_AT = Date.UTC(2026, 9, 23)
export const STELLA_RETRIEVED_AT = Date.UTC(2026, 9, 23, 15, 5)
export const GENOSIDE_FILE_SHA256 = 'c426719dcf13c279fe003fcd80258fd625c3b796e7c5adda21abec1655df65ca'
export const STELLA_HEADER_SHA256 = '951f0558a27817f88ccac02e383db529158ff5613d4553cfeb683c06120bbb4c'
export const GENOSIDE_PAGE_URL = 'https://nekokan.dyndns.info/~lobsak/genocide/grade.html'
export const STELLA_PAGE_URL = 'https://stellabms.xyz/st/table.html'
export const CONSTRAINT_SOURCE_URL = 'https://miraiscarlet.github.io/bms/table/genocide_normal/header_normal.json'
export const GENOSIDE_FIRST_NAME = 'GENOSIDE 2018 段位認定 初段'
export const GENOSIDE_FIRST_TITLE = 'Agito'
export const STELLA_TRAINING_NAME = 'STELLA技能訓練 test'

const md5Of = (seed: string) => createHash('md5').update(seed).digest('hex')

const courseOf = (params: { name: string; titles: string[]; dan: CatalogCourse['dan'] }): CatalogCourse => ({
    courseHash: createHash('sha256').update(params.name).digest('hex'),
    name: params.name,
    lntype: 0,
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    trophy: [{ name: 'silvermedal', scorerate: 70, smissrate: 5 }],
    charts: params.titles.map((title) => ({ md5: md5Of(`${params.name}/${title}`), sha256: '', title })),
    dan: params.dan,
})

export const GENOSIDE_FIRST = courseOf({
    name: GENOSIDE_FIRST_NAME,
    titles: [GENOSIDE_FIRST_TITLE, 'Beat Cannon', 'ZetsueN', 'Scaffold'],
    dan: { system: 'genoside2018_sp', label: '初段', rank: 1 },
})
export const GENOSIDE_DP_FIRST = courseOf({
    name: 'GENOSIDE 2018 DP段位認定 初段',
    titles: ['Dp one', 'Dp two', 'Dp three', 'Dp four'],
    dan: { system: 'genoside2018_dp', label: '初段', rank: 1 },
})
export const STELLA_FIRST = courseOf({
    name: 'Stella Skill Simulator 4th st0',
    titles: ['St one', 'St two', 'St three', 'St four'],
    dan: { system: 'stella', label: 'st0', rank: 0 },
})
export const STELLA_TRAINING = courseOf({ name: STELLA_TRAINING_NAME, titles: ['Training one'], dan: null })

export const GENOSIDE_DEFINITION: CatalogDefinition = {
    source: {
        id: 'genoside-2018',
        kind: 'bundled',
        name: 'GENOSIDE 2018 段位認定',
        maker: 'GENOCIDE',
        pageUrl: GENOSIDE_PAGE_URL,
        fileUrl: 'http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip',
        retrievedAt: GENOSIDE_RETRIEVED_AT,
        fileSha256: GENOSIDE_FILE_SHA256,
        detail: { constraint_sources: [{ url: CONSTRAINT_SOURCE_URL, sha256: 'a'.repeat(64), retrieved_on: '2026-10-23' }] },
    },
    groups: [
        { id: 'genoside-2018-sp', sourceId: 'genoside-2018', name: 'GENOSIDE 2018 段位認定', keys: 7, position: 0, courses: [GENOSIDE_FIRST] },
        { id: 'genoside-2018-dp', sourceId: 'genoside-2018', name: 'GENOSIDE 2018 DP段位認定', keys: 14, position: 1, courses: [GENOSIDE_DP_FIRST] },
    ],
}

export const STELLA_DEFINITION: CatalogDefinition = {
    source: {
        id: 'stella-st',
        kind: 'table',
        name: 'Stella',
        maker: 'stellabms.xyz',
        pageUrl: STELLA_PAGE_URL,
        fileUrl: 'https://stellabms.xyz/st/header.json',
        retrievedAt: STELLA_RETRIEVED_AT,
        fileSha256: STELLA_HEADER_SHA256,
        detail: { data_url: 'https://stellabms.xyz/st/score.json', data_sha256: 'b'.repeat(64) },
    },
    groups: [{ id: 'stella-st', sourceId: 'stella-st', name: 'Stella', keys: 7, position: 2, courses: [STELLA_FIRST, STELLA_TRAINING] }],
}

/** The catalog service over the groups of GENOSIDE 2018 (SP and DP) and Stella, registered the way a sync registers them. */
export const createCatalogFixture = async () => {
    const service = createCourseCatalogService({ db: createMemoryCourseCatalogDb().db })
    await service.applySync(GENOSIDE_DEFINITION)
    await service.applySync(STELLA_DEFINITION)
    return service
}
