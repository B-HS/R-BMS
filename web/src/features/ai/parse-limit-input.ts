const DIGITS_ONLY = /^\d+$/

export const parseLimitInput = (text: string, bounds: { min: number; max: number }) => {
    const digits = text.replaceAll(',', '').trim()
    if (!DIGITS_ONLY.test(digits)) return null
    const value = Number(digits)
    return value >= bounds.min && value <= bounds.max ? value : null
}
