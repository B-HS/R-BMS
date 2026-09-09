import { bigint, boolean, double, json, mysqlTable, text, timestamp, varchar } from 'drizzle-orm/mysql-core'

export const user = mysqlTable('user', {
    id: varchar('id', { length: 36 }).primaryKey(),
    name: text('name').notNull(),
    email: varchar('email', { length: 255 }).notNull().unique(),
    emailVerified: boolean('email_verified')
        .$defaultFn(() => false)
        .notNull(),
    image: text('image'),
    createdAt: timestamp('created_at', { fsp: 3 })
        .$defaultFn(() => new Date())
        .notNull(),
    updatedAt: timestamp('updated_at', { fsp: 3 })
        .$defaultFn(() => new Date())
        .notNull(),
    loginId: varchar('login_id', { length: 64 }).notNull().unique(),
    rank: varchar('rank', { length: 32 }).default('').notNull(),
    rankPoints: double('rank_points').default(0).notNull(),
    totalPlays: bigint('total_plays', { mode: 'number' }).default(0).notNull(),
    role: varchar('role', { length: 16 }).default('user').notNull(),
    allowGuestMerge: boolean('allow_guest_merge').default(false).notNull(),
    extra: json('extra'),
})

export const session = mysqlTable('session', {
    id: varchar('id', { length: 36 }).primaryKey(),
    expiresAt: timestamp('expires_at', { fsp: 3 }).notNull(),
    token: varchar('token', { length: 255 }).notNull().unique(),
    createdAt: timestamp('created_at', { fsp: 3 }).notNull(),
    updatedAt: timestamp('updated_at', { fsp: 3 }).notNull(),
    ipAddress: text('ip_address'),
    userAgent: text('user_agent'),
    userId: varchar('user_id', { length: 36 })
        .notNull()
        .references(() => user.id, { onDelete: 'cascade' }),
})

export const account = mysqlTable('account', {
    id: varchar('id', { length: 36 }).primaryKey(),
    accountId: varchar('account_id', { length: 255 }).notNull(),
    providerId: varchar('provider_id', { length: 255 }).notNull(),
    userId: varchar('user_id', { length: 36 })
        .notNull()
        .references(() => user.id, { onDelete: 'cascade' }),
    accessToken: text('access_token'),
    refreshToken: text('refresh_token'),
    idToken: text('id_token'),
    accessTokenExpiresAt: timestamp('access_token_expires_at', { fsp: 3 }),
    refreshTokenExpiresAt: timestamp('refresh_token_expires_at', { fsp: 3 }),
    scope: text('scope'),
    password: text('password'),
    createdAt: timestamp('created_at', { fsp: 3 }).notNull(),
    updatedAt: timestamp('updated_at', { fsp: 3 }).notNull(),
})

export const verification = mysqlTable('verification', {
    id: varchar('id', { length: 36 }).primaryKey(),
    identifier: varchar('identifier', { length: 255 }).notNull(),
    value: text('value').notNull(),
    expiresAt: timestamp('expires_at', { fsp: 3 }).notNull(),
    createdAt: timestamp('created_at', { fsp: 3 }).$defaultFn(() => new Date()),
    updatedAt: timestamp('updated_at', { fsp: 3 }).$defaultFn(() => new Date()),
})
