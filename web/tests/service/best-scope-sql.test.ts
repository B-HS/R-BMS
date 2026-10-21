import { describe, expect, test } from 'bun:test'
import { MySqlDialect } from 'drizzle-orm/mysql-core'
import { chartBest, courseBest } from '@server/db/schema'
import { isBeatenBy, isRankedAheadOf, listOrder, rankInList } from '@server/compose/best-scope'

const dialect = new MySqlDialect()
const RUN = { clear: 3, assistGrade: 2, exScore: 100, minbp: 5 }

describe('최고 기록 비교 SQL (램프 > 어시스트 등급 > EX > BP)', () => {
    test('isBeatenBy 는 첫 번째로 다른 키가 가르고 BP 만 작은 쪽이 이긴다', () => {
        const query = dialect.sqlToQuery(isBeatenBy(chartBest, RUN)!)
        expect(query.sql).toBe(
            '(? > `chart_best`.`clear` or ' +
                '(? = `chart_best`.`clear` and ? > `chart_best`.`assist_grade`) or ' +
                '(? = `chart_best`.`clear` and ? = `chart_best`.`assist_grade` and ? > `chart_best`.`ex_score`) or ' +
                '(? = `chart_best`.`clear` and ? = `chart_best`.`assist_grade` and ? = `chart_best`.`ex_score` and ? < `chart_best`.`minbp`))',
        )
        expect(query.params).toEqual([3, 3, 2, 3, 2, 100, 3, 2, 100, 5])
    })

    test('isRankedAheadOf 는 같은 키들에서 행이 위인 경우만 센다', () => {
        const query = dialect.sqlToQuery(isRankedAheadOf(courseBest, RUN)!)
        expect(query.sql).toBe(
            '(`course_best`.`clear` > ? or ' +
                '(`course_best`.`clear` = ? and `course_best`.`assist_grade` > ?) or ' +
                '(`course_best`.`clear` = ? and `course_best`.`assist_grade` = ? and `course_best`.`ex_score` > ?) or ' +
                '(`course_best`.`clear` = ? and `course_best`.`assist_grade` = ? and `course_best`.`ex_score` = ? and `course_best`.`minbp` < ?))',
        )
        expect(query.params).toEqual([3, 3, 2, 3, 2, 100, 3, 2, 100, 5])
    })

    test('rank() 와 목록 순서도 같은 키 순서다', () => {
        expect(dialect.sqlToQuery(rankInList(chartBest)).sql).toBe(
            'rank() over (order by `chart_best`.`clear` desc, `chart_best`.`assist_grade` desc, `chart_best`.`ex_score` desc, `chart_best`.`minbp` asc)',
        )
        const order = listOrder(chartBest).map((column) => dialect.sqlToQuery(column).sql)
        expect(order).toEqual([
            '`chart_best`.`clear` desc',
            '`chart_best`.`assist_grade` desc',
            '`chart_best`.`ex_score` desc',
            '`chart_best`.`minbp` asc',
            '`chart_best`.`updated_at` asc',
            '`chart_best`.`user_id` asc',
        ])
    })
})
