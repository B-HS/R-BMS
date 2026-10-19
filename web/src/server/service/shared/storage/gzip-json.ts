import { promisify } from 'node:util'
import { gunzip, gzip } from 'node:zlib'

const gzipAsync = promisify(gzip)
const gunzipAsync = promisify(gunzip)

/** JSON text gzip-compressed for a binary column. */
export const gzipJsonText = async (text: string) => gzipAsync(Buffer.from(text, 'utf8'))

/** The value a `gzipJsonText` blob holds, parsed but not validated: the reader narrows it with its own schema. */
export const gunzipJson = async (bytes: Uint8Array): Promise<unknown> => JSON.parse((await gunzipAsync(bytes)).toString('utf8'))
