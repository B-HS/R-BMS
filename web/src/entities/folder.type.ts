import { z } from 'zod'

export const FOLDER_LIMIT = {
    FOLDERS_PER_USER: 50,
    ITEMS_PER_FOLDER: 2000,
    ITEMS_PER_USER: 20000,
    ITEMS_PER_REQUEST: 500,
    NAME_MAX: 64,
    DESCRIPTION_MAX: 255,
    TEXT_COPY_MAX: 200,
    NOTE_MAX: 100,
} as const

export const FOLDER_ERROR_CODE = {
    NOT_FOUND: 'IR_FOLDER_NOT_FOUND',
    NAME_TAKEN: 'IR_FOLDER_NAME_TAKEN',
    LIMIT_EXCEEDED: 'IR_FOLDER_LIMIT_EXCEEDED',
    CONFLICT: 'IR_FOLDER_CONFLICT',
} as const

export type FolderSummary = {
    id: string
    name: string
    description: string
    is_public: boolean
    position: number
    item_count: number
    updated_at: number
}

export type FolderItem = {
    md5: string
    sha256: string
    title: string
    artist: string
    note: string
}

export type FolderDetail = FolderSummary & { items: FolderItem[] }

export type FolderItemChange = {
    added: number
    removed: number
    skipped: number
    item_count: number
    updated_at: number
}

export type FolderItemInput = Pick<FolderItem, 'md5'> & Partial<Omit<FolderItem, 'md5'>>

export type FolderTableEntry = Pick<FolderItem, 'md5' | 'title' | 'artist'>

type FolderTableLevel = { level: string; entries: FolderTableEntry[] }

/** Compact copy of a bundled difficulty table that the folder screens search and import from without another request. */
export type FolderTableIndex = { id: string; name: string; symbol: string; levels: FolderTableLevel[] }

const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f]/
const MD5_PATTERN = /^[0-9a-f]{32}$/
const FOLDER_ID_PATTERN = /^uf_[0-9a-f]{30}$/
const SHA256_PATTERN = /^[0-9a-f]{64}$/

export const isFolderId = (value: string) => FOLDER_ID_PATTERN.test(value)

export const isFolderMd5 = (value: string) => MD5_PATTERN.test(value)

const isFolderSha256 = (value: string) => SHA256_PATTERN.test(value)

const clipFolderText = (value: string) => [...value].slice(0, FOLDER_LIMIT.TEXT_COPY_MAX).join('')

/** Builds the request item for a chart that is not in the folder yet, keeping only a valid sha256 and clipping copied text to the server limit. */
export const toNewFolderItemInput = (item: Pick<FolderItem, 'md5'> & Partial<FolderItem>) => ({
    md5: item.md5,
    ...(item.sha256 && isFolderSha256(item.sha256) ? { sha256: item.sha256 } : {}),
    title: clipFolderText(item.title ?? ''),
    artist: clipFolderText(item.artist ?? ''),
    note: item.note ?? '',
})

export const FOLDER_FORM_ERROR = {
    CONTROL_CHARACTER: 'controlCharacter',
    NAME_TOO_LONG: 'nameTooLong',
    NAME_REQUIRED: 'nameRequired',
    DESCRIPTION_TOO_LONG: 'descriptionTooLong',
    NOTE_TOO_LONG: 'noteTooLong',
} as const

export type FolderFormError = (typeof FOLDER_FORM_ERROR)[keyof typeof FOLDER_FORM_ERROR]

export const isFolderFormError = (value: string): value is FolderFormError => Object.values<string>(FOLDER_FORM_ERROR).includes(value)

const singleLineText = (max: number, tooLongError: FolderFormError) =>
    z
        .string()
        .transform((value) => value.normalize('NFC').trim())
        .refine((value) => !CONTROL_CHARACTERS.test(value), FOLDER_FORM_ERROR.CONTROL_CHARACTER)
        .refine((value) => value.length <= max, tooLongError)

/**
 * Mirrors the server's folder name and description rules so the form rejects what the API would reject.
 * An issue's message is a `FOLDER_FORM_ERROR` code that the form words in the current language.
 */
export const folderMetaFormSchema = z.object({
    name: singleLineText(FOLDER_LIMIT.NAME_MAX, FOLDER_FORM_ERROR.NAME_TOO_LONG).refine(
        (value) => value.length >= 1,
        FOLDER_FORM_ERROR.NAME_REQUIRED,
    ),
    description: singleLineText(FOLDER_LIMIT.DESCRIPTION_MAX, FOLDER_FORM_ERROR.DESCRIPTION_TOO_LONG),
})

export const folderNoteFormSchema = singleLineText(FOLDER_LIMIT.NOTE_MAX, FOLDER_FORM_ERROR.NOTE_TOO_LONG)

export type FolderMetaInput = z.infer<typeof folderMetaFormSchema>

const FOLDER_ERROR_MESSAGE: Record<string, string> = {
    [FOLDER_ERROR_CODE.NOT_FOUND]: '폴더를 찾을 수 없습니다. 다른 곳에서 지웠을 수 있습니다.',
    [FOLDER_ERROR_CODE.NAME_TAKEN]: '같은 이름의 폴더가 이미 있습니다.',
    [FOLDER_ERROR_CODE.LIMIT_EXCEEDED]: `한도를 넘었습니다. 폴더는 ${FOLDER_LIMIT.FOLDERS_PER_USER}개, 폴더당 ${FOLDER_LIMIT.ITEMS_PER_FOLDER.toLocaleString('ko-KR')}곡, 모두 합쳐 ${FOLDER_LIMIT.ITEMS_PER_USER.toLocaleString('ko-KR')}곡까지 둘 수 있습니다.`,
    RATE_LIMITED: '요청이 너무 잦습니다. 잠시 후 다시 시도하세요.',
}

const FALLBACK_ERROR_MESSAGE = '요청을 처리하지 못했습니다. 잠시 후 다시 시도하세요.'

export const getFolderErrorCode = (error: unknown) =>
    typeof error === 'object' && error !== null && 'code' in error && typeof error.code === 'string' ? error.code : ''

/** Picks the user-facing sentence for a folder API failure from its error code, falling back to the server message. */
export const toFolderErrorMessage = (error: unknown) => {
    const known = FOLDER_ERROR_MESSAGE[getFolderErrorCode(error)]
    if (known) return known
    return error instanceof Error && error.message ? error.message : FALLBACK_ERROR_MESSAGE
}
