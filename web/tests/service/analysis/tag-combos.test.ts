import { describe, expect, test } from 'bun:test'
import { PATTERN_TAGS } from '@server/service/domain/analysis/pattern-tags'
import { COMBO_KEYS, COMBO_TAGS, computeTagCombos, type ComboNote } from '@server/service/domain/analysis/tag-combos'

describe('태그 조합(2.2.6)', () => {
    test('조합 대상 15개는 PATTERN_TAGS 순서이고 쌍 키는 앞 태그가 먼저인 105개다', () => {
        expect(COMBO_TAGS).toHaveLength(15)
        expect(COMBO_TAGS.toSorted((a, b) => PATTERN_TAGS.indexOf(a) - PATTERN_TAGS.indexOf(b))).toEqual([...COMBO_TAGS])
        expect(COMBO_TAGS).not.toContain('scratch_solo')
        expect(COMBO_TAGS).not.toContain('ln_head')
        expect(COMBO_TAGS).not.toContain('ln_end')
        expect(COMBO_TAGS).not.toContain('mine_near')
        expect(COMBO_KEYS).toHaveLength(105)
        expect(COMBO_KEYS[0]).toBe('chord_2+chord_3')
        expect(COMBO_KEYS.at(-1)).toBe('soflan+dense')
    })

    test('노트마다 조합 대상 태그의 모든 쌍에 더하고, 조합 대상 태그가 하나뿐인 노트는 단독 칸에 더한다', () => {
        const notes: ComboNote[] = [
            { grade: 'pgreat', errUs: 500, tags: ['chord_2', 'trill', 'dense'] },
            { grade: 'bad', errUs: -30_000, tags: ['chord_2', 'dense', 'mine_near'] },
            { grade: 'great', errUs: 2000, tags: ['trill', 'ln_head'] },
            { grade: 'poor', errUs: 0, tags: ['trill'] },
            { grade: 'good', errUs: -9000, tags: ['scratch_solo', 'ln_head'] },
        ]
        const { by_combo, by_tag_solo } = computeTagCombos(notes)
        expect(Object.keys(by_combo)).toEqual(['chord_2+trill', 'chord_2+dense', 'trill+dense'])
        expect(by_combo['chord_2+trill']).toEqual({ notes: 1, ex_score: 2, bp: 0, fast: 0, slow: 0 })
        expect(by_combo['chord_2+dense']).toEqual({ notes: 2, ex_score: 2, bp: 1, fast: 0, slow: 1 })
        expect(by_combo['trill+dense']).toEqual({ notes: 1, ex_score: 2, bp: 0, fast: 0, slow: 0 })
        expect(by_tag_solo).toEqual({ trill: { notes: 2, ex_score: 1, bp: 1, fast: 1, slow: 0 } })
    })
})
