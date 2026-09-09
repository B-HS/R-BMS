import { getEnv } from '@server/lib/env'
import type { ServerCapabilitiesOutput, ServerInfoOutput } from '@server/dto/system'

const COMMIT_SHORT_LENGTH = 7

export const SERVER_NAME = 'rbms-ir'
export const SERVER_VERSION = '0.1.0'
export const IR_COMPAT = 'superset-1'
export const IR_API_VERSION = 1

export const SERVER_CAPABILITIES: ServerCapabilitiesOutput = {
    ranking: true,
    player_best: true,
    rivals: true,
    courses: true,
    replays: true,
    tables: true,
    settings_sync: true,
    accounts: true,
    lr2ir_compat: false,
}

export const getServerInfo = (): ServerInfoOutput => ({
    name: SERVER_NAME,
    version: getEnv().SERVER_VERSION ?? SERVER_VERSION,
    ir_compat: IR_COMPAT,
    capabilities: SERVER_CAPABILITIES,
})

export const getVersionInfo = () => {
    const env = getEnv()
    return {
        api_version: IR_API_VERSION,
        server: env.SERVER_VERSION ?? SERVER_VERSION,
        commit: env.SERVER_COMMIT ?? env.VERCEL_GIT_COMMIT_SHA?.slice(0, COMMIT_SHORT_LENGTH) ?? null,
    }
}
