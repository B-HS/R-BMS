import { describe, expect, test } from 'bun:test'
import { MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { LOCK_RETRY_ATTEMPTS, withLockRetry } from '@server/lib/with-lock-retry'

const queryErrorWith = (code: string) => Object.assign(new Error('Failed query'), { cause: Object.assign(new Error(code), { code }) })

const failingTimes = (failures: number, code: string) => {
    const calls: string[] = []
    const write = withLockRetry(async (value: string) => {
        calls.push(value)
        if (calls.length <= failures) throw queryErrorWith(code)
        return `${value}-stored`
    })
    return { calls, write }
}

describe('withLockRetry', () => {
    test('교착으로 되돌려진 쓰기는 같은 인자로 다시 해서 결과를 돌려준다', async () => {
        const { calls, write } = failingTimes(1, MYSQL_ERROR_CODE.LOCK_DEADLOCK)
        expect(await write('run')).toBe('run-stored')
        expect(calls).toEqual(['run', 'run'])
    })

    test('잠금 대기 초과도 다시 한다', async () => {
        const { calls, write } = failingTimes(2, MYSQL_ERROR_CODE.LOCK_WAIT_TIMEOUT)
        expect(await write('run')).toBe('run-stored')
        expect(calls).toHaveLength(3)
    })

    test('정한 횟수를 다 써도 교착이면 마지막 오류를 그대로 던진다', async () => {
        const { calls, write } = failingTimes(LOCK_RETRY_ATTEMPTS, MYSQL_ERROR_CODE.LOCK_DEADLOCK)
        const error = await write('run').catch((caught: unknown) => caught)
        expect(error).toMatchObject({ cause: { code: MYSQL_ERROR_CODE.LOCK_DEADLOCK } })
        expect(calls).toHaveLength(LOCK_RETRY_ATTEMPTS)
    })

    test('잠금 오류가 아니면 다시 하지 않는다', async () => {
        const { calls, write } = failingTimes(1, MYSQL_ERROR_CODE.DUPLICATE_ENTRY)
        await expect(write('run')).rejects.toMatchObject({ cause: { code: MYSQL_ERROR_CODE.DUPLICATE_ENTRY } })
        expect(calls).toHaveLength(1)
    })
})
