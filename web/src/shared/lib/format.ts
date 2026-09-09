const HASH_PREVIEW_LENGTH = 8

export const formatCount = (value: number) => new Intl.NumberFormat('ko-KR').format(value)

export const formatHash = (value: string) => (value.length <= HASH_PREVIEW_LENGTH ? value : `${value.slice(0, HASH_PREVIEW_LENGTH)}…`)

export const DISPLAY_TIME_ZONE = 'Asia/Seoul'

const dateTimeFormat = new Intl.DateTimeFormat('ko-KR', {
    timeZone: DISPLAY_TIME_ZONE,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
})

export const formatDateTime = (epochMs: number) => {
    const date = new Date(epochMs)
    if (Number.isNaN(date.getTime())) return '—'
    const parts = Object.fromEntries(dateTimeFormat.formatToParts(date).map((part) => [part.type, part.value]))
    return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}`
}

export const formatPercent = (numerator: number, denominator: number) =>
    denominator === 0 ? '0.00%' : `${((numerator / denominator) * 100).toFixed(2)}%`
