import 'server-only'
import { headers } from 'next/headers'

/** Builds fetch init that forwards the incoming session cookie to a server-side prefetch. */
export const forwardedCookieInit = async () => {
    const cookie = (await headers()).get('cookie')
    return cookie ? { headers: { cookie } } : {}
}
