export const QUERY_KEY = {
    AUTH: {
        ALL: ['auth'],
        SESSION: ['auth', 'session'],
        ME: ['auth', 'me'],
    },
    CHART: {
        ALL: ['chart'],
        META: (hash: string) => ['chart', 'meta', hash],
        RANKING: (hash: string) => ['chart', 'ranking', hash],
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
    },
    TABLE: {
        ALL: ['table'],
        LIST: ['table', 'list'],
        DETAIL: (tableId: string) => ['table', 'detail', tableId],
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
    ADMIN: {
        ALL: ['admin'],
        BUILDS: ['admin', 'builds'],
    },
    SYSTEM: {
        HEALTH: ['system', 'health'],
        VERSION: ['system', 'version'],
    },
} as const
