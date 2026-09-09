export const readParam = (params: Record<string, string | string[] | undefined>, key: string) => {
    const raw = params[key]
    if (Array.isArray(raw)) return raw[0]
    return raw
}

export const readNumberParam = (params: Record<string, string | string[] | undefined>, key: string, fallback: number) => {
    const raw = readParam(params, key)
    if (raw === undefined) return fallback
    const parsed = Number(raw)
    return Number.isFinite(parsed) ? parsed : fallback
}

export const readListParam = (params: Record<string, string | string[] | undefined>, key: string) => {
    const raw = readParam(params, key)
    if (!raw) return []
    return raw.split(',').filter(Boolean)
}

export const serializeSearchParams = (values: Record<string, string | number | string[] | undefined | null>) => {
    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(values)) {
        if (value === undefined || value === null || value === '') continue
        if (Array.isArray(value)) {
            if (value.length === 0) continue
            search.set(key, value.join(','))
            continue
        }
        search.set(key, String(value))
    }
    return search.toString()
}

export const toggleListValue = (list: string[], value: string) => (list.includes(value) ? list.filter((item) => item !== value) : [...list, value])
