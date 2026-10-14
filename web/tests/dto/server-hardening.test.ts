import { describe, expect, test } from 'bun:test'
import { accountSchema, loginSchema } from '@server/dto/auth'
import { PLAYER_SORTS, playerLeaderboardQuerySchema } from '@server/dto/fe'
import { toLeaderboardItem } from '@server/service/domain/fe/fe.service'
import { toScoreRecord } from '@server/service/domain/score/score.service'

const account = (id: string) => ({ id, password: 'password-1234', email: 'a@example.com' })

describe('가입 아이디의 서버 검증', () => {
    test('영문, 숫자, 하이픈, 밑줄만 받는다', () => {
        for (const id of ['alice', 'Alice_01', 'a-b', 'x', '1234']) expect(accountSchema.safeParse(account(id)).success).toBe(true)
    })

    test('공백, 슬래시, 한글, 점, 빈 값, 65자를 거부한다', () => {
        for (const id of ['a b', 'a/b', '한글', 'a.b', '', 'a'.repeat(65), '../x', 'a?b', 'a%2Fb']) {
            expect(accountSchema.safeParse(account(id)).success).toBe(false)
        }
    })

    test('guest 는 대소문자와 상관없이 예약어이다', () => {
        for (const id of ['guest', 'Guest', 'GUEST']) expect(accountSchema.safeParse(account(id)).success).toBe(false)
        expect(accountSchema.safeParse(account('guest2')).success).toBe(true)
    })

    test('이미 있는 계정이 로그인할 수 있도록 로그인 스키마는 형식을 묻지 않는다', () => {
        expect(loginSchema.safeParse({ id: 'legacy id', password: 'x' }).success).toBe(true)
    })
})

describe('리더보드 정렬', () => {
    test('기본은 cleared 이고 새 값을 그대로 받는다', () => {
        expect(playerLeaderboardQuerySchema.parse({}).sort).toBe('cleared')
        for (const sort of PLAYER_SORTS) expect(playerLeaderboardQuerySchema.parse({ sort }).sort).toBe(sort)
    })

    test('옛 값 rank_points 와 total_plays 는 실제 값 정렬로 바꿔 받는다', () => {
        expect(playerLeaderboardQuerySchema.parse({ sort: 'rank_points' }).sort).toBe('cleared')
        expect(playerLeaderboardQuerySchema.parse({ sort: 'total_plays' }).sort).toBe('plays')
    })

    test('모르는 값은 거부한다', () => {
        expect(playerLeaderboardQuerySchema.safeParse({ sort: 'elo' }).success).toBe(false)
    })

    test('응답 항목에 클리어한 차트 수와 플레이한 차트 수가 실린다', () => {
        const item = toLeaderboardItem({
            id: 'u1',
            loginId: 'alice',
            name: 'Alice',
            rank: '',
            rankPoints: 0,
            totalPlays: 0,
            role: 'user',
            plays: 12,
            clearedCharts: 7,
            playedCharts: 9,
        })
        expect(item).toMatchObject({ id: 'alice', total_plays: 12, cleared_charts: 7, played_charts: 9 })
    })
})

describe('순위 항목의 replay_id', () => {
    const row = {
        scoreId: 's1',
        loginId: 'alice',
        playerName: 'Alice',
        clear: 5,
        exScore: 10,
        maxCombo: 5,
        minbp: 1,
        playedAt: 1,
        lntype: 0,
        option: 0,
        totalNotes: 10,
        judge: null,
    }

    test('리플레이가 붙은 점수는 extra.replay_id 로 알린다', () => {
        expect(toScoreRecord({ ...row, replayId: 'rp_1' }, 1).extra).toEqual({ replay_id: 'rp_1' })
    })

    test('붙지 않았거나 모르면 extra 는 비어 있다', () => {
        expect(toScoreRecord({ ...row, replayId: null }, 1).extra).toEqual({})
        expect(toScoreRecord(row, 1).extra).toEqual({})
    })
})
