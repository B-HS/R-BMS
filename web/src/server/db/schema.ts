import {
    bigint,
    boolean,
    char,
    double,
    float,
    index,
    int,
    json,
    mediumtext,
    mysqlTable,
    primaryKey,
    tinyint,
    uniqueIndex,
    varchar,
    timestamp,
} from 'drizzle-orm/mysql-core'
import { user } from '@server/db/auth-schema'

export const chart = mysqlTable(
    'chart',
    {
        sha256: char('sha256', { length: 64 }).primaryKey(),
        md5: char('md5', { length: 32 }),
        title: varchar('title', { length: 255 }).default('').notNull(),
        subtitle: varchar('subtitle', { length: 255 }).default('').notNull(),
        genre: varchar('genre', { length: 255 }).default('').notNull(),
        artist: varchar('artist', { length: 255 }).default('').notNull(),
        subartist: varchar('subartist', { length: 255 }).default('').notNull(),
        level: int('level'),
        total: double('total'),
        mode: varchar('mode', { length: 16 }).default('').notNull(),
        lntype: int('lntype').default(0).notNull(),
        judge: int('judge').default(0).notNull(),
        minbpm: int('minbpm').default(0).notNull(),
        maxbpm: int('maxbpm').default(0).notNull(),
        notes: int('notes').default(0).notNull(),
        hasLn: boolean('has_ln').default(false).notNull(),
        hasCn: boolean('has_cn').default(false).notNull(),
        hasHcn: boolean('has_hcn').default(false).notNull(),
        hasMine: boolean('has_mine').default(false).notNull(),
        hasRandom: boolean('has_random').default(false).notNull(),
        hasStop: boolean('has_stop').default(false).notNull(),
        url: varchar('url', { length: 512 }),
        appendurl: varchar('appendurl', { length: 512 }),
        extra: json('extra'),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
        updatedAt: timestamp('updated_at', { fsp: 3 })
            .defaultNow()
            .$onUpdate(() => new Date())
            .notNull(),
    },
    (table) => [uniqueIndex('chart_md5_uniq').on(table.md5), index('chart_title_idx').on(table.title)],
)

export const score = mysqlTable(
    'score',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        userId: varchar('user_id', { length: 36 }).references(() => user.id, { onDelete: 'set null' }),
        guestName: varchar('guest_name', { length: 64 }),
        chartSha256: char('chart_sha256', { length: 64 }).notNull(),
        chartMd5: char('chart_md5', { length: 32 }),
        mode: varchar('mode', { length: 16 }).default('').notNull(),
        lntype: int('lntype').default(0).notNull(),
        clear: tinyint('clear').default(0).notNull(),
        epg: int('epg').default(0).notNull(),
        lpg: int('lpg').default(0).notNull(),
        egr: int('egr').default(0).notNull(),
        lgr: int('lgr').default(0).notNull(),
        egd: int('egd').default(0).notNull(),
        lgd: int('lgd').default(0).notNull(),
        ebd: int('ebd').default(0).notNull(),
        lbd: int('lbd').default(0).notNull(),
        epr: int('epr').default(0).notNull(),
        lpr: int('lpr').default(0).notNull(),
        ems: int('ems').default(0).notNull(),
        lms: int('lms').default(0).notNull(),
        pgreat: int('pgreat').default(0).notNull(),
        great: int('great').default(0).notNull(),
        good: int('good').default(0).notNull(),
        bad: int('bad').default(0).notNull(),
        poor: int('poor').default(0).notNull(),
        miss: int('miss').default(0).notNull(),
        fast: int('fast').default(0).notNull(),
        slow: int('slow').default(0).notNull(),
        combobreak: int('combobreak').default(0).notNull(),
        emptyPoor: int('empty_poor').default(0).notNull(),
        avgjudge: bigint('avgjudge', { mode: 'number' }).default(0).notNull(),
        exScore: int('ex_score').default(0).notNull(),
        maxExScore: int('max_ex_score').default(0).notNull(),
        maxCombo: int('max_combo').default(0).notNull(),
        notes: int('notes').default(0).notNull(),
        passnotes: int('passnotes').default(0).notNull(),
        minbp: int('minbp').default(0).notNull(),
        gaugeValue: float('gauge_value').default(0).notNull(),
        gauge: tinyint('gauge').default(0).notNull(),
        option: bigint('option', { mode: 'number' }).default(0).notNull(),
        random: varchar('random', { length: 16 }).default('Off').notNull(),
        randomP2: varchar('random_p2', { length: 16 }),
        scratchLeft: boolean('scratch_left').default(false).notNull(),
        scratchAuto: boolean('scratch_auto').default(false).notNull(),
        seed: bigint('seed', { mode: 'number' }).default(0).notNull(),
        hispeed: double('hispeed').default(0).notNull(),
        constant: boolean('constant').default(false).notNull(),
        greenNumber: int('green_number'),
        lift: float('lift').default(0).notNull(),
        laneCover: float('lane_cover').default(0).notNull(),
        assist: int('assist').default(0).notNull(),
        judgeRate: int('judge_rate').default(100).notNull(),
        offsetMs: int('offset_ms').default(0).notNull(),
        autoOffset: boolean('auto_offset').default(false).notNull(),
        totalOverride: double('total_override').default(0).notNull(),
        autoplay: boolean('autoplay').default(false).notNull(),
        inputDevice: varchar('input_device', { length: 32 }).default('').notNull(),
        judgeAlgorithm: varchar('judge_algorithm', { length: 16 }).default('').notNull(),
        rule: varchar('rule', { length: 16 }).default('').notNull(),
        skin: varchar('skin', { length: 64 }).default('').notNull(),
        client: varchar('client', { length: 64 }).default('').notNull(),
        clientBuildSha256: char('client_build_sha256', { length: 64 }),
        clientPlatform: varchar('client_platform', { length: 32 }),
        ranked: boolean('ranked').default(true).notNull(),
        flags: json('flags'),
        verified: boolean('verified').default(false).notNull(),
        replayId: varchar('replay_id', { length: 36 }),
        playedAt: bigint('played_at', { mode: 'number' }).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
        extra: json('extra'),
    },
    (table) => [
        uniqueIndex('score_idempotent_uniq').on(table.userId, table.chartSha256, table.playedAt),
        index('score_ranking_idx').on(table.chartSha256, table.ranked, table.clear, table.exScore),
        index('score_player_idx').on(table.userId, table.playedAt),
        index('score_build_idx').on(table.clientBuildSha256),
        index('score_recent_idx').on(table.playedAt),
    ],
)

export const chartBest = mysqlTable(
    'chart_best',
    {
        chartSha256: char('chart_sha256', { length: 64 }).notNull(),
        userId: varchar('user_id', { length: 36 }).notNull(),
        scoreId: varchar('score_id', { length: 36 }).notNull(),
        clear: tinyint('clear').default(0).notNull(),
        exScore: int('ex_score').default(0).notNull(),
        minbp: int('minbp').default(0).notNull(),
        maxCombo: int('max_combo').default(0).notNull(),
        updatedAt: timestamp('updated_at', { fsp: 3 })
            .defaultNow()
            .$onUpdate(() => new Date())
            .notNull(),
    },
    (table) => [
        primaryKey({ columns: [table.chartSha256, table.userId] }),
        index('chart_best_rank_idx').on(table.chartSha256, table.clear, table.exScore),
    ],
)

export const replay = mysqlTable(
    'replay',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        userId: varchar('user_id', { length: 36 }),
        chartSha256: char('chart_sha256', { length: 64 }).notNull(),
        scoreId: varchar('score_id', { length: 36 }),
        format: varchar('format', { length: 32 }).default('').notNull(),
        mode: varchar('mode', { length: 16 }).default('').notNull(),
        random: varchar('random', { length: 16 }).default('Off').notNull(),
        randomP2: varchar('random_p2', { length: 16 }),
        seed: bigint('seed', { mode: 'number' }).default(0).notNull(),
        lntype: int('lntype').default(0).notNull(),
        offsetMs: int('offset_ms').default(0).notNull(),
        judgeRate: int('judge_rate').default(100).notNull(),
        scratchAuto: boolean('scratch_auto').default(false).notNull(),
        constant: boolean('constant').default(false).notNull(),
        gauge: tinyint('gauge').default(0).notNull(),
        clientBuildSha256: char('client_build_sha256', { length: 64 }),
        eventCount: int('event_count').default(0).notNull(),
        durationUs: bigint('duration_us', { mode: 'number' }).default(0).notNull(),
        size: int('size').default(0).notNull(),
        storageKey: varchar('storage_key', { length: 255 }),
        data: mediumtext('data'),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
    },
    (table) => [index('replay_chart_idx').on(table.chartSha256), index('replay_score_idx').on(table.scoreId)],
)

export const apiToken = mysqlTable(
    'api_token',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        userId: varchar('user_id', { length: 36 })
            .notNull()
            .references(() => user.id, { onDelete: 'cascade' }),
        tokenHash: varchar('token_hash', { length: 255 }).notNull(),
        label: varchar('label', { length: 64 }).default('').notNull(),
        lastUsedAt: bigint('last_used_at', { mode: 'number' }),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
        revoked: boolean('revoked').default(false).notNull(),
    },
    (table) => [uniqueIndex('api_token_hash_uniq').on(table.tokenHash), index('api_token_user_idx').on(table.userId)],
)

export const rival = mysqlTable(
    'rival',
    {
        userId: varchar('user_id', { length: 36 }).notNull(),
        rivalId: varchar('rival_id', { length: 36 }).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
    },
    (table) => [primaryKey({ columns: [table.userId, table.rivalId] })],
)

export const settingBlob = mysqlTable(
    'setting_blob',
    {
        userId: varchar('user_id', { length: 36 }).notNull(),
        name: varchar('name', { length: 64 }).notNull(),
        format: varchar('format', { length: 16 }).default('').notNull(),
        content: mediumtext('content').notNull(),
        size: int('size').default(0).notNull(),
        updatedAt: bigint('updated_at', { mode: 'number' }).default(0).notNull(),
    },
    (table) => [primaryKey({ columns: [table.userId, table.name] })],
)

export const course = mysqlTable('course', {
    courseHash: varchar('course_hash', { length: 128 }).primaryKey(),
    name: varchar('name', { length: 255 }).default('').notNull(),
    lntype: int('lntype').default(0).notNull(),
    constraint: json('constraint'),
    trophy: json('trophy'),
    extra: json('extra'),
    createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
})

export const courseChart = mysqlTable(
    'course_chart',
    {
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
        position: int('position').notNull(),
        chartSha256: char('chart_sha256', { length: 64 }).notNull(),
    },
    (table) => [primaryKey({ columns: [table.courseHash, table.position] })],
)

export const courseScore = mysqlTable(
    'course_score',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        userId: varchar('user_id', { length: 36 }),
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
        clear: tinyint('clear').default(0).notNull(),
        epg: int('epg').default(0).notNull(),
        lpg: int('lpg').default(0).notNull(),
        egr: int('egr').default(0).notNull(),
        lgr: int('lgr').default(0).notNull(),
        egd: int('egd').default(0).notNull(),
        lgd: int('lgd').default(0).notNull(),
        ebd: int('ebd').default(0).notNull(),
        lbd: int('lbd').default(0).notNull(),
        epr: int('epr').default(0).notNull(),
        lpr: int('lpr').default(0).notNull(),
        ems: int('ems').default(0).notNull(),
        lms: int('lms').default(0).notNull(),
        exScore: int('ex_score').default(0).notNull(),
        maxCombo: int('max_combo').default(0).notNull(),
        gaugeValue: float('gauge_value').default(0).notNull(),
        minbp: int('minbp').default(0).notNull(),
        trophy: varchar('trophy', { length: 32 }),
        ranked: boolean('ranked').default(true).notNull(),
        playedAt: bigint('played_at', { mode: 'number' }).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
        extra: json('extra'),
    },
    (table) => [
        uniqueIndex('course_score_idempotent_uniq').on(table.userId, table.courseHash, table.playedAt),
        index('course_score_rank_idx').on(table.courseHash, table.ranked, table.clear, table.exScore),
    ],
)

export const courseBest = mysqlTable(
    'course_best',
    {
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
        userId: varchar('user_id', { length: 36 }).notNull(),
        courseScoreId: varchar('course_score_id', { length: 36 }).notNull(),
        clear: tinyint('clear').default(0).notNull(),
        exScore: int('ex_score').default(0).notNull(),
        minbp: int('minbp').default(0).notNull(),
        maxCombo: int('max_combo').default(0).notNull(),
        updatedAt: timestamp('updated_at', { fsp: 3 })
            .defaultNow()
            .$onUpdate(() => new Date())
            .notNull(),
    },
    (table) => [primaryKey({ columns: [table.courseHash, table.userId] })],
)

export const difficultyTable = mysqlTable('difficulty_table', {
    id: varchar('id', { length: 64 }).primaryKey(),
    name: varchar('name', { length: 255 }).default('').notNull(),
    url: varchar('url', { length: 512 }),
    extra: json('extra'),
    createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
})

export const tableFolder = mysqlTable(
    'table_folder',
    {
        id: varchar('id', { length: 64 }).primaryKey(),
        tableId: varchar('table_id', { length: 64 }).notNull(),
        name: varchar('name', { length: 255 }).default('').notNull(),
        position: int('position').default(0).notNull(),
    },
    (table) => [index('table_folder_table_idx').on(table.tableId)],
)

export const tableChart = mysqlTable(
    'table_chart',
    {
        folderId: varchar('folder_id', { length: 64 }).notNull(),
        chartSha256: char('chart_sha256', { length: 64 }).notNull(),
    },
    (table) => [primaryKey({ columns: [table.folderId, table.chartSha256] })],
)

export const tableCourse = mysqlTable(
    'table_course',
    {
        tableId: varchar('table_id', { length: 64 }).notNull(),
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
    },
    (table) => [primaryKey({ columns: [table.tableId, table.courseHash] })],
)

export const clientBuild = mysqlTable('client_build', {
    sha256: char('sha256', { length: 64 }).primaryKey(),
    version: varchar('version', { length: 32 }).default('').notNull(),
    platform: varchar('platform', { length: 32 }).default('').notNull(),
    channel: varchar('channel', { length: 16 }).default('stable').notNull(),
    releasedAt: bigint('released_at', { mode: 'number' }).default(0).notNull(),
    trusted: boolean('trusted').default(true).notNull(),
    note: varchar('note', { length: 255 }).default('').notNull(),
    createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
})

export const submissionAudit = mysqlTable(
    'submission_audit',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        scoreId: varchar('score_id', { length: 36 }),
        userId: varchar('user_id', { length: 36 }),
        clientBuildSha256: char('client_build_sha256', { length: 64 }),
        clientPlatform: varchar('client_platform', { length: 32 }),
        ip: varchar('ip', { length: 64 }),
        userAgent: varchar('user_agent', { length: 255 }),
        accepted: boolean('accepted').default(false).notNull(),
        ranked: boolean('ranked').default(false).notNull(),
        flags: json('flags'),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
    },
    (table) => [index('submission_audit_build_idx').on(table.clientBuildSha256), index('submission_audit_created_idx').on(table.createdAt)],
)
