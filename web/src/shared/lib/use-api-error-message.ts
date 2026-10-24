import { useMessages, useTranslations } from 'next-intl'
import { ApiError } from '@shared/lib/fetch'

const HTTP_ERROR_CODE = 'HTTP_ERROR'

/**
 * Returns a function that turns a failed request into a sentence in the screen's language. The API keeps answering with its code
 * and Korean `message` (the player shows that message as is); the web reads the code instead: a known code is read from the
 * `server.error` catalog, a response with no API envelope reads `server.httpError` with its status, and a code the catalog does
 * not know keeps the server's own message. Anything that is not an `ApiError` gives `null`, so the caller keeps its own fallback.
 */
export const useApiErrorMessage = () => {
    const t = useTranslations('server')
    const errorMessages = useMessages().server.error
    const isKnownCode = (code: string): code is keyof typeof errorMessages => Object.hasOwn(errorMessages, code)

    return (error: unknown) => {
        if (!(error instanceof ApiError)) return null
        if (error.code === HTTP_ERROR_CODE) return t('httpError', { status: error.status })
        return isKnownCode(error.code) ? t(`error.${error.code}`) : error.message
    }
}
