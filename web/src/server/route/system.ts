import { connection } from 'next/server'
import { irRaw } from '@server/lib/api-response'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { getServerInfo, getVersionInfo } from '@server/service/domain/system/capabilities'

export const createHealthRoute = () =>
    withErrorHandling(async () => {
        await connection()
        return irRaw(getServerInfo())
    })

export const createVersionRoute = () =>
    withErrorHandling(async () => {
        await connection()
        return irRaw(getVersionInfo())
    })
