import { describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { bundledCoursesFileSchema } from '@server/dto/course-catalog'
import { asciiLowercase, courseHashOf } from '@server/service/domain/course-catalog/course-hash'

const COURSES_FILE = join(import.meta.dirname, '..', '..', 'data', 'courses', 'genoside-2018', 'courses.json')
const EXPECTED_COURSE_COUNT = 44
const NO_SHA256 = ''

const md5Charts = (md5s: string[]) => md5s.map((md5) => ({ md5, sha256: NO_SHA256 }))

const sha256Hex = (text: string) => createHash('sha256').update(text).digest('hex')

describe('courseHashOf', () => {
    test('코스 id 는 이름과 차트 md5 로 만든다 — GENOSIDE 初段', () => {
        const hash = courseHashOf({
            name: 'GENOSIDE 2018 段位認定 初段',
            charts: md5Charts([
                'ae11930223a8f24e91633f1a7864e1eb',
                'cb661ee05c67990012bfd12703ba656c',
                '328dbf235f3b6cb2791e71bfb62951f8',
                '7c6ca04aa14d07b2c081f4dd35432844',
            ]),
        })
        expect(hash).toBe('ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1')
    })

    test('Stella Skill Simulator 4th st0 의 id', () => {
        const hash = courseHashOf({
            name: 'Stella Skill Simulator 4th st0',
            charts: md5Charts([
                '349bc491ec40d5595412637d8a4c8d2e',
                'baee0a1921fc5041b44d7d87c7b5548d',
                '72b1ce4b2051bd2a396dfa11a2d785ee',
                '3a1661a3eaafa13f976e1010d5b87ca0',
            ]),
        })
        expect(hash).toBe('afce60f187e4ba4ab0519b54d9a0fcf04184e6b137c4826c4513be2d4f99f158')
    })

    test('Satellite Skill Analyzer 2nd sl0 의 id', () => {
        const hash = courseHashOf({
            name: 'Satellite Skill Analyzer 2nd sl0',
            charts: md5Charts([
                '9ee5524e4ed8a3e7033b4de1fedc35fd',
                '9498378a847b94b949d11a566a86d3c0',
                '4023045d9134dfc47e953bc430e5ef07',
                'bb5d0716c18b37f9687bbcea17fc646c',
            ]),
        })
        expect(hash).toBe('b327c69e3462aa0a0e07d60d0fe0b9e6873a8bd0be3336c1408901c842a59967')
    })

    test('sha256 이 있는 차트는 sha256 을 쓴다', () => {
        const md5 = 'a'.repeat(32)
        const sha256 = 'b'.repeat(64)
        const other = 'c'.repeat(32)
        const withSha256 = courseHashOf({ name: 'course', charts: [{ md5, sha256 }, ...md5Charts([other])] })
        expect(withSha256).toBe(sha256Hex(`course\n${sha256}\n${other}\n`))
        expect(withSha256).not.toBe(courseHashOf({ name: 'course', charts: md5Charts([md5, other]) }))
    })

    test('이름은 앞뒤 공백까지 원문 그대로 해시한다', () => {
        const md5 = 'a'.repeat(32)
        const padded = courseHashOf({ name: '  spaced name \n', charts: md5Charts([md5]) })
        expect(padded).toBe(sha256Hex(`  spaced name \n\n${md5}\n`))
        expect(padded).not.toBe(courseHashOf({ name: 'spaced name', charts: md5Charts([md5]) }))
    })

    test('차트 순서가 바뀌면 다른 id 다', () => {
        const [first, second] = ['a'.repeat(32), 'b'.repeat(32)]
        expect(courseHashOf({ name: 'order', charts: md5Charts([first, second]) })).not.toBe(
            courseHashOf({ name: 'order', charts: md5Charts([second, first]) }),
        )
    })

    test('저장소의 GENOSIDE 2018 파일 44개 코스 id 가 TS 계산과 모두 같다', () => {
        const file = bundledCoursesFileSchema.parse(JSON.parse(readFileSync(COURSES_FILE, 'utf8')))
        const courses = file.groups.flatMap((group) => group.courses)
        expect(courses).toHaveLength(EXPECTED_COURSE_COUNT)
        for (const course of courses) expect(courseHashOf({ name: course.name, charts: course.charts })).toBe(course.course_hash)
    })
})

describe('asciiLowercase', () => {
    test('ASCII 대문자만 소문자로 바꾼다', () => {
        expect(asciiLowercase('AbC-09_ΚΑΙ')).toBe('abc-09_ΚΑΙ')
    })

    test('켈빈 기호는 건드리지 않는다', () => {
        expect(asciiLowercase('GAUGE_5K')).toBe('gauge_5K')
    })
})
