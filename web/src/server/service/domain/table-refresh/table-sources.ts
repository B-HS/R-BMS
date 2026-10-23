const BYTES_PER_MIB = 1024 * 1024

export const TABLE_HEADER_MAX_BYTES = BYTES_PER_MIB
export const TABLE_DATA_MAX_BYTES = 8 * BYTES_PER_MIB
export const TABLE_FETCH_TIMEOUT_MS = 20_000
export const TABLE_FETCH_USER_AGENT = 'R-BMS table sync (+https://bms.hyuns.uk)'

export type TableSourceDefinition = {
    id: 'stella-st' | 'stella-sl'
    expectedName: string
    symbol: string
    sourceName: string
    maker: string
    pageUrl: string
    headerUrl: string
    allowedHost: string
    keys: number
    groupPosition: number
}

/**
 * The difficulty tables this server copies once a day. The ids match the default-table ids of the client, and `allowedHost` is the
 * only host a header's `data_url` may point to, so a changed header can never send this server to another site.
 */
export const REMOTE_TABLES = [
    {
        id: 'stella-st',
        expectedName: 'Stella',
        symbol: 'st',
        sourceName: 'Stella',
        maker: 'stellabms.xyz',
        pageUrl: 'https://stellabms.xyz/st/table.html',
        headerUrl: 'https://stellabms.xyz/st/header.json',
        allowedHost: 'stellabms.xyz',
        keys: 7,
        groupPosition: 2,
    },
    {
        id: 'stella-sl',
        expectedName: 'Satellite',
        symbol: 'sl',
        sourceName: 'Satellite',
        maker: 'stellabms.xyz',
        pageUrl: 'https://stellabms.xyz/sl/table.html',
        headerUrl: 'https://stellabms.xyz/sl/header.json',
        allowedHost: 'stellabms.xyz',
        keys: 7,
        groupPosition: 3,
    },
] as const satisfies readonly TableSourceDefinition[]
