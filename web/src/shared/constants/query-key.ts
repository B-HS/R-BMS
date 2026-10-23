export const QUERY_KEY = {
    AUTH: {
        ALL: ['auth'],
        SESSION: ['auth', 'session'],
        ME: ['auth', 'me'],
    },
    CHART: {
        ALL: ['chart'],
        META: (hash: string) => ['chart', 'meta', hash],
        RANKING: (hash: string, params: Record<string, unknown>) => ['chart', 'ranking', hash, params],
        BEST: (hash: string, player: string) => ['chart', 'best', hash, player],
        SEARCH: (params: Record<string, unknown>) => ['chart', 'search', params],
        REPLAYS: (hash: string, params: Record<string, unknown>) => ['chart', 'replays', hash, params],
    },
    PLAYER: {
        ALL: ['player'],
        PROFILE: (playerId: string) => ['player', 'profile', playerId],
        SCORES: (playerId: string, params: Record<string, unknown>) => ['player', 'scores', playerId, params],
        RECENT: (playerId: string, params: Record<string, unknown>) => ['player', 'recent', playerId, params],
        STATS: (playerId: string) => ['player', 'stats', playerId],
        RIVALS: (playerId: string) => ['player', 'rivals', playerId],
    },
    SCORE: {
        ALL: ['score'],
        DETAIL: (scoreId: string) => ['score', 'detail', scoreId],
    },
    REPLAY: {
        ALL: ['replay'],
        DETAIL: (replayId: string) => ['replay', 'detail', replayId],
    },
    COURSE: {
        ALL: ['course'],
        META: (courseHash: string) => ['course', 'meta', courseHash],
        RANKING: (courseHash: string, params: Record<string, unknown>) => ['course', 'ranking', courseHash, params],
        BEST: (courseHash: string, player: string) => ['course', 'best', courseHash, player],
        CATALOG: ['course', 'catalog'],
        CATALOG_ENTRY: (courseHash: string) => ['course', 'catalog', courseHash],
    },
    DAN: {
        ALL: ['dan'],
        MINE: ['dan', 'mine'],
    },
    TABLE: {
        ALL: ['table'],
        LIST: ['table', 'list'],
        DETAIL: (tableId: string) => ['table', 'detail', tableId],
    },
    TABLE_SNAPSHOT: {
        RECORDS_ALL: ['table-snapshot', 'records'],
        RECORDS: (tableId: string, userId: string) => ['table-snapshot', 'records', tableId, userId],
    },
    FOLDER: {
        ALL: ['folder'],
        LIST: ['folder', 'list'],
        DETAIL: (folderId: string) => ['folder', 'detail', folderId],
        CONTAINING_ALL: ['folder', 'containing'],
        CONTAINING: (md5: string) => ['folder', 'containing', md5],
    },
    STATS: {
        ALL: ['stats'],
        SUMMARY: ['stats', 'summary'],
    },
    ACTIVITY: {
        ALL: ['activity'],
        RECENT: (params: Record<string, unknown>) => ['activity', 'recent', params],
    },
    LEADERBOARD: {
        ALL: ['leaderboard'],
        PLAYERS: (params: Record<string, unknown>) => ['leaderboard', 'players', params],
    },
    TOKEN: {
        ALL: ['token'],
        LIST: ['token', 'list'],
    },
    SETTING: {
        ALL: ['setting'],
        LIST: ['setting', 'list'],
        DETAIL: (playerId: string, key: string) => ['setting', 'detail', playerId, key],
    },
    AI: {
        ALL: ['ai'],
        CREDENTIALS: ['ai', 'credentials'],
        MODELS: (provider: string) => ['ai', 'models', provider],
        USAGE: ['ai', 'usage'],
        SETTINGS: ['ai', 'settings'],
        DIGEST_ALL: ['ai', 'digest'],
        DIGEST_FEATURE: (feature: string) => ['ai', 'digest', feature],
        DIGEST: (feature: string, params: Record<string, unknown>) => ['ai', 'digest', feature, params],
        GOALS: ['ai', 'goals'],
        SHARING: ['ai', 'sharing'],
    },
    ADMIN: {
        ALL: ['admin'],
        BUILDS: ['admin', 'builds'],
        AI_LIMITS: ['admin', 'ai-limits'],
    },
    SYSTEM: {
        HEALTH: ['system', 'health'],
        VERSION: ['system', 'version'],
    },
} as const
