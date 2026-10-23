import { describe, expect, test } from 'bun:test'
import { DAN_SYSTEMS, danOfCourse, danSystemName } from '@server/service/domain/course-catalog/dan-of-course'

const SP_GROUP = 'genoside-2018-sp'
const DP_GROUP = 'genoside-2018-dp'
const STELLA_GROUP = 'stella-st'
const SATELLITE_GROUP = 'stella-sl'
const SP_COURSE_COUNT = 22
const LAST_INDEX = SP_COURSE_COUNT - 1
const TOO_LONG_LABEL = 'x'.repeat(65)
const TOO_LARGE_LEVEL = '9'.repeat(20)

describe('danOfCourse', () => {
    test('GENOSIDE SP 初段 은 rank 1, Overjoy 는 rank 22', () => {
        expect(danOfCourse(SP_GROUP, 'GENOSIDE 2018 段位認定 初段', 0)).toEqual({ system: 'genoside2018_sp', label: '初段', rank: 1 })
        expect(danOfCourse(SP_GROUP, 'GENOSIDE 2018 段位認定 Overjoy', LAST_INDEX)).toEqual({
            system: 'genoside2018_sp',
            label: 'Overjoy',
            rank: SP_COURSE_COUNT,
        })
    })

    test('DP 접두사는 DP 체계다', () => {
        expect(danOfCourse(DP_GROUP, 'GENOSIDE 2018 DP段位認定 発狂皆伝', 20)).toEqual({ system: 'genoside2018_dp', label: '発狂皆伝', rank: 21 })
    })

    test('GENOSIDE 의 rank 는 이름이 아니라 묶음 안의 순서다', () => {
        expect(danOfCourse(SP_GROUP, 'GENOSIDE 2018 段位認定 二段', 5)?.rank).toBe(6)
    })

    test('Stella Skill Simulator 4th st12 는 st12 rank 12', () => {
        expect(danOfCourse(STELLA_GROUP, 'Stella Skill Simulator 4th st12', 12)).toEqual({ system: 'stella', label: 'st12', rank: 12 })
        expect(danOfCourse(STELLA_GROUP, 'Stella Skill Simulator 4th st0', 0)).toEqual({ system: 'stella', label: 'st0', rank: 0 })
    })

    test('5th 판 이름도 같은 label 을 준다', () => {
        expect(danOfCourse(STELLA_GROUP, 'Stella Skill Simulator 5th st5', 5)).toEqual({ system: 'stella', label: 'st5', rank: 5 })
    })

    test('Satellite Skill Analyzer 2nd sl3 은 sl3 rank 3', () => {
        expect(danOfCourse(SATELLITE_GROUP, 'Satellite Skill Analyzer 2nd sl3', 3)).toEqual({ system: 'satellite', label: 'sl3', rank: 3 })
    })

    test('STELLA技能訓練 코스는 단위가 없다', () => {
        expect(danOfCourse(STELLA_GROUP, 'STELLA技能訓練 4th st3', 13)).toBeNull()
    })

    test('다른 묶음의 같은 이름은 단위가 없다', () => {
        expect(danOfCourse('some-other-table', 'Stella Skill Simulator 4th st3', 3)).toBeNull()
        expect(danOfCourse(SATELLITE_GROUP, 'Stella Skill Simulator 4th st3', 3)).toBeNull()
        expect(danOfCourse(SP_GROUP, 'GENOSIDE 2018 DP段位認定 初段', 0)).toBeNull()
    })

    test('접두사만 있고 단 이름이 없으면 단위가 없다', () => {
        expect(danOfCourse(SP_GROUP, 'GENOSIDE 2018 段位認定 ', 0)).toBeNull()
    })

    test('단 이름이 label 열보다 길면 단위가 없다', () => {
        expect(danOfCourse(SP_GROUP, `GENOSIDE 2018 段位認定 ${TOO_LONG_LABEL}`, 0)).toBeNull()
    })

    test('열에 들어가지 않는 단계 숫자는 단위가 없다', () => {
        expect(danOfCourse(STELLA_GROUP, `Stella Skill Simulator 4th st${TOO_LARGE_LEVEL}`, 0)).toBeNull()
    })
})

describe('DAN_SYSTEMS', () => {
    test('네 체계와 그 묶음을 낸다', () => {
        expect(DAN_SYSTEMS).toEqual([
            { id: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', groupId: SP_GROUP },
            { id: 'genoside2018_dp', name: 'GENOSIDE 2018 DP', groupId: DP_GROUP },
            { id: 'stella', name: 'Stella', groupId: STELLA_GROUP },
            { id: 'satellite', name: 'Satellite', groupId: SATELLITE_GROUP },
        ])
        expect(danSystemName('satellite')).toBe('Satellite')
    })
})
