import { describe, expect, test } from 'bun:test'
import {
    WEAKNESS_KEY,
    WEAKNESS_KEY_MAX_LENGTH,
    WEAKNESS_KINDS,
    isFolderSelectableKind,
    parseWeaknessKey,
    weaknessKeySchema,
    type WeaknessKind,
} from '@server/service/domain/analysis/weakness-key'

describe('약점 키 문법(기획 2.5.5)', () => {
    test('13 가지 꼴을 종류·태그·인자로 나눈다', () => {
        expect([
            parseWeaknessKey('tag:trill'),
            parseWeaknessKey('solo:jack'),
            parseWeaknessKey('combo:chord_2+trill'),
            parseWeaknessKey('lane:S'),
            parseWeaknessKey('hand:shared'),
            parseWeaknessKey('timing:stream'),
            parseWeaknessKey('bpm:180-209'),
            parseWeaknessKey('soflan:settled'),
            parseWeaknessKey('fatigue'),
            parseWeaknessKey('density:limit'),
            parseWeaknessKey('density:sustained'),
            parseWeaknessKey('bp_context:scratch_run'),
            parseWeaknessKey('gauge_context:mine_near'),
        ]).toEqual([
            { kind: 'tag', tags: ['trill'], arg: null },
            { kind: 'solo', tags: ['jack'], arg: null },
            { kind: 'combo', tags: ['chord_2', 'trill'], arg: null },
            { kind: 'lane', tags: [], arg: 'S' },
            { kind: 'hand', tags: [], arg: 'shared' },
            { kind: 'timing', tags: ['stream'], arg: null },
            { kind: 'bpm', tags: [], arg: '180-209' },
            { kind: 'soflan', tags: [], arg: 'settled' },
            { kind: 'fatigue', tags: [], arg: null },
            { kind: 'density', tags: [], arg: 'limit' },
            { kind: 'density', tags: [], arg: 'sustained' },
            { kind: 'bp_context', tags: ['scratch_run'], arg: null },
            { kind: 'gauge_context', tags: ['mine_near'], arg: null },
        ])
    })

    test('키를 만드는 함수와 읽는 함수는 서로 맞는다', () => {
        const keys = [
            WEAKNESS_KEY.tag('ln_end'),
            WEAKNESS_KEY.solo('dense'),
            WEAKNESS_KEY.combo('ln_release_then_hit', 'dense'),
            WEAKNESS_KEY.lane('7'),
            WEAKNESS_KEY.hand('scratch'),
            WEAKNESS_KEY.timing('jack'),
            WEAKNESS_KEY.bpm('<120'),
            WEAKNESS_KEY.soflan('after'),
            WEAKNESS_KEY.fatigue(),
            WEAKNESS_KEY.density('limit'),
            WEAKNESS_KEY.bpContext('trill'),
            WEAKNESS_KEY.gaugeContext('stair'),
        ]
        expect(keys.every((key) => parseWeaknessKey(key) !== null)).toBe(true)
        expect(WEAKNESS_KEY.combo('ln_release_then_hit', 'dense')).toBe('combo:ln_release_then_hit+dense')
    })

    test('모르는 접두사·태그·레인·손·띠, 순서가 거꾸로이거나 같은 조합, 조합 대상이 아닌 태그는 거절한다', () => {
        for (const key of [
            'tag:trills',
            'tag:',
            'tags:trill',
            'trill',
            'solo:mine_near',
            'solo:ln_head',
            'combo:trill+chord_2',
            'combo:trill+trill',
            'combo:trill',
            'combo:trill+chord_2+jack',
            'combo:ln_head+trill',
            'lane:8',
            'lane:s',
            'hand:both',
            'bpm:120',
            'soflan:before',
            'density:peak',
            'fatigue:1',
            'timing:',
            'bp_context:unknown',
            '',
        ]) {
            expect(parseWeaknessKey(key), key).toBeNull()
            expect(weaknessKeySchema.safeParse(key).success, key).toBe(false)
        }
    })

    test('가장 긴 키는 상한 안이고 상한을 넘는 문자열은 거절한다', () => {
        const longest = 'combo:scratch_mixed+ln_release_then_hit'
        expect(longest.length).toBeLessThanOrEqual(WEAKNESS_KEY_MAX_LENGTH)
        expect(weaknessKeySchema.safeParse(longest).success).toBe(true)
        expect(weaknessKeySchema.safeParse('combo:ln_release_then_hit+scratch_mixed').success).toBe(false)
        const tooLong = `tag:trill${' '.repeat(WEAKNESS_KEY_MAX_LENGTH)}`
        expect(parseWeaknessKey(tooLong)).toBeNull()
        expect(weaknessKeySchema.safeParse(tooLong).success).toBe(false)
        expect(weaknessKeySchema.safeParse(42).success).toBe(false)
    })

    test('연습 폴더로 곡을 고를 수 있는 종류는 곡 특징으로 정해지는 것뿐이다', () => {
        const selectable = ['tag', 'solo', 'combo', 'density', 'bpm', 'soflan'] satisfies WeaknessKind[]
        const optionBound = ['lane', 'hand', 'timing', 'fatigue', 'bp_context', 'gauge_context'] satisfies WeaknessKind[]
        expect(selectable.every(isFolderSelectableKind)).toBe(true)
        expect(optionBound.some(isFolderSelectableKind)).toBe(false)
        expect([...selectable, ...optionBound].toSorted()).toEqual([...WEAKNESS_KINDS].toSorted())
    })
})
