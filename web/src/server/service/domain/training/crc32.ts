const CRC32_POLYNOMIAL = 0xedb88320
const TABLE_SIZE = 256
const BITS_PER_BYTE = 8
const BYTE_MASK = 0xff
const ALL_BITS = 0xffffffff

const CRC32_TABLE = Array.from({ length: TABLE_SIZE }, (_, byte) => {
    let value = byte
    for (let bit = 0; bit < BITS_PER_BYTE; bit += 1) value = value & 1 ? CRC32_POLYNOMIAL ^ (value >>> 1) : value >>> 1
    return value >>> 0
})

/** The CRC-32 (IEEE 802.3, the one zip uses) of `bytes`, as an unsigned 32-bit number. */
export const crc32 = (bytes: Uint8Array) => {
    let crc = ALL_BITS
    for (const byte of bytes) crc = CRC32_TABLE[(crc ^ byte) & BYTE_MASK] ^ (crc >>> BITS_PER_BYTE)
    return (crc ^ ALL_BITS) >>> 0
}
