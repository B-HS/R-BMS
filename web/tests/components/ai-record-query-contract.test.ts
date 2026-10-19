import { describe, expect, test } from 'bun:test'
import {
    AI_RECORD_QUERY_ALLOWED_OPS,
    AI_RECORD_QUERY_ISSUE_REASONS,
    AI_RECORD_QUERY_LIMIT,
    AI_RECORD_QUERY_SORT_FIELDS,
} from '../../src/server/dto/ai'
import {
    AI_RECORD_QUERY_FIELD_LABEL,
    AI_RECORD_QUERY_FIELD_OPS,
    AI_RECORD_QUERY_FORM_LIMIT,
    AI_RECORD_QUERY_ISSUE_LABEL,
    AI_RECORD_QUERY_SORT_LABEL,
    AI_RECORD_QUERY_VALUE_HINT,
} from '../../src/entities/ai.type'

describe('기록 질문 화면과 서버의 계약', () => {
    test('항목별 허용 연산자 표가 서버와 같다', () => {
        expect(AI_RECORD_QUERY_FIELD_OPS).toEqual(AI_RECORD_QUERY_ALLOWED_OPS)
        expect(Object.keys(AI_RECORD_QUERY_FIELD_LABEL).toSorted()).toEqual(Object.keys(AI_RECORD_QUERY_ALLOWED_OPS).toSorted())
        expect(Object.keys(AI_RECORD_QUERY_VALUE_HINT).toSorted()).toEqual(Object.keys(AI_RECORD_QUERY_ALLOWED_OPS).toSorted())
    })

    test('정렬 기준·실패 이유·한도가 서버와 같다', () => {
        expect(Object.keys(AI_RECORD_QUERY_SORT_LABEL)).toEqual([...AI_RECORD_QUERY_SORT_FIELDS])
        expect(Object.keys(AI_RECORD_QUERY_ISSUE_LABEL).toSorted()).toEqual([...AI_RECORD_QUERY_ISSUE_REASONS].toSorted())
        expect(AI_RECORD_QUERY_FORM_LIMIT).toEqual({
            QUESTION_MAX_CHARS: AI_RECORD_QUERY_LIMIT.QUESTION_MAX_CHARS,
            MAX_FILTERS: AI_RECORD_QUERY_LIMIT.MAX_FILTERS,
            ROWS_MIN: AI_RECORD_QUERY_LIMIT.ROWS_MIN,
            ROWS_MAX: AI_RECORD_QUERY_LIMIT.ROWS_MAX,
        })
    })
})
