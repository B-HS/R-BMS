import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { ASSIST_GRADES, CLEAR_LAMPS, GAUGE_TYPES, clearLampToId, gaugeTypeToId, type AssistGrade, type GaugeType } from '@server/dto/common'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'

const LIGHT_ASSIST = clearLampToId('LightAssistEasy')

const GRADE_OF_GAUGE: Record<GaugeType, AssistGrade> = {
    AssistEasy: 0,
    Easy: 1,
    Normal: 2,
    Hard: 3,
    ExHard: 4,
    Hazard: 4,
    Class: 2,
    ExClass: 3,
    ExHardClass: 4,
}

describe('deriveAssistGrade', () => {
    test('LIGHT ASSIST 클리어는 끝난 게이지로 강등 없이 클리어했을 때의 게이지 램프를 접은 등급이다', () => {
        for (const gauge of GAUGE_TYPES) {
            expect([gauge, deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId(gauge) })]).toEqual([gauge, GRADE_OF_GAUGE[gauge]])
        }
    })

    test('ASSIST EASY 게이지의 클리어는 등급 없는 LIGHT ASSIST 이다', () => {
        expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('AssistEasy') })).toBe(0)
    })

    test('HAZARD 는 EXHARD 와 같은 최고 등급이고 코스 게이지는 같은 종류의 램프 등급이다', () => {
        expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('Hazard') })).toBe(4)
        expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('Class') })).toBe(
            deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('Normal') }),
        )
        expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('ExClass') })).toBe(
            deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('Hard') }),
        )
        expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('ExHardClass') })).toBe(
            deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('ExHard') }),
        )
    })

    test('LIGHT ASSIST 가 아닌 램프는 어떤 게이지여도 0 이다 (ASSIST 포함)', () => {
        for (const lamp of CLEAR_LAMPS.filter((candidate) => candidate !== 'LightAssistEasy')) {
            for (const gauge of GAUGE_TYPES)
                expect([lamp, gauge, deriveAssistGrade({ clear: clearLampToId(lamp), gauge: gaugeTypeToId(gauge) })]).toEqual([lamp, gauge, 0])
        }
    })

    test('게이지를 모르는 행은 0 이다', () => {
        for (const gauge of [-1, GAUGE_TYPES.length, 99, 1.5, Number.NaN]) expect(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge })).toBe(0)
    })

    test('등급은 0 에서 4 까지이고 램프 id 와 와이어 이름은 11종 그대로다', () => {
        expect([...ASSIST_GRADES]).toEqual([0, 1, 2, 3, 4])
        expect(CLEAR_LAMPS).toHaveLength(11)
        expect(CLEAR_LAMPS.indexOf('LightAssistEasy')).toBe(3)
    })
})

describe('마이그레이션 0011 의 백필 SQL', () => {
    const migration = readFileSync(new URL('../../drizzle/0011_assist_grade.sql', import.meta.url), 'utf8')
    const [backfill] = migration.split('--> statement-breakpoint').slice(-1)
    const gaugeCases = [...backfill.matchAll(/WHEN (\d+) THEN (\d+)/g)].map(([, gauge, grade]) => [Number(gauge), Number(grade)] as const)

    test('chart_best 를 그 행이 가리키는 score 로 채우고 LIGHT ASSIST 가 아닌 행은 0 으로 되돌린다', () => {
        expect(backfill).toContain('UPDATE `chart_best`')
        expect(backfill).toContain('`score`.`id` = `best`.`score_id`')
        expect(backfill).toContain('CASE WHEN `score`.`clear` = 3 THEN CASE `score`.`gauge`')
        expect(backfill).toContain('ELSE 0 END ELSE 0 END')
        expect(backfill).not.toContain('WHERE')
    })

    test('SQL 의 게이지별 등급이 deriveAssistGrade 와 같고 SET 은 assist_grade 하나뿐이다', () => {
        expect(gaugeCases.map(([gauge]) => gauge)).toEqual(GAUGE_TYPES.map((_, id) => id).filter((id) => id > 0))
        for (const [gauge, grade] of gaugeCases) expect([gauge, grade]).toEqual([gauge, deriveAssistGrade({ clear: LIGHT_ASSIST, gauge })])
        expect(backfill.match(/SET /g)).toHaveLength(1)
        expect(backfill.match(/`best`\.`[a-z_]+` =/g)).toEqual(['`best`.`assist_grade` ='])
    })
})

describe('운영 DB 의 어시스트 등급 보정 SQL', () => {
    const [migrationBackfill = ''] = readFileSync(new URL('../../drizzle/0011_assist_grade.sql', import.meta.url), 'utf8')
        .split('--> statement-breakpoint')
        .slice(-1)
    const migrationCase = migrationBackfill.match(/SET `best`\.`assist_grade` = (CASE .+ END);/)?.[1] ?? ''
    const backfill = readFileSync(new URL('../../scripts/assist-grade.backfill.sql', import.meta.url), 'utf8')
    const count = readFileSync(new URL('../../scripts/assist-grade.count.sql', import.meta.url), 'utf8')
    const caseExpressions = (sql: string) => [...sql.matchAll(/CASE WHEN .+? END ELSE 0 END/g)].map(([found]) => found)

    test('등급 식은 0011 의 백필 식과 같고 보정은 바뀔 행만 건드리며 세기는 같은 식으로 센다', () => {
        expect(migrationCase).not.toBe('')
        expect(caseExpressions(backfill)).toEqual([migrationCase, migrationCase])
        expect(caseExpressions(count)).toEqual([migrationCase])
        expect(backfill).toContain('UPDATE `chart_best` AS `best`')
        expect(backfill).toContain('INNER JOIN `score` ON `score`.`id` = `best`.`score_id`')
        expect(backfill).toContain('WHERE `best`.`assist_grade` <> CASE')
        expect(backfill.match(/SET /g)).toHaveLength(1)
    })

    test('식의 게이지별 등급이 deriveAssistGrade 와 같다', () => {
        const gaugeCases = [...migrationCase.matchAll(/WHEN (\d+) THEN (\d+)/g)].map(([, gauge, grade]) => [Number(gauge), Number(grade)] as const)
        expect(gaugeCases.map(([gauge]) => gauge)).toEqual(GAUGE_TYPES.map((_, id) => id).filter((id) => id > 0))
        for (const [gauge, grade] of gaugeCases) expect(grade).toBe(deriveAssistGrade({ clear: LIGHT_ASSIST, gauge }))
        expect(migrationCase).toStartWith(`CASE WHEN \`score\`.\`clear\` = ${LIGHT_ASSIST} THEN CASE \`score\`.\`gauge\``)
    })
})
