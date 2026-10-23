import { createHash } from 'node:crypto'

const HASH_SEPARATOR = '\n'
const UPPERCASE_ASCII_LETTER = /[A-Z]/g

/** Lowercases ASCII letters only, as the client does for chart hashes and constraint spellings (`make_ascii_lowercase`). */
export const asciiLowercase = (text: string) => text.replace(UPPERCASE_ASCII_LETTER, (letter) => letter.toLowerCase())

/**
 * The id a course is ranked under, the same SHA-256 the client computes (`Course::hash`): the name exactly as it is (no
 * trimming, no normalisation), then each chart's SHA-256 when it has one and its MD5 otherwise, every part followed by a
 * line feed. The hashes must already be lowercase.
 */
export const courseHashOf = (course: { name: string; charts: readonly { md5: string; sha256: string }[] }) => {
    const hash = createHash('sha256')
    hash.update(course.name)
    hash.update(HASH_SEPARATOR)
    for (const chart of course.charts) {
        hash.update(chart.sha256 === '' ? chart.md5 : chart.sha256)
        hash.update(HASH_SEPARATOR)
    }
    return hash.digest('hex')
}
