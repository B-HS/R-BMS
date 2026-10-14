import { NextResponse, type NextRequest } from 'next/server'
import { getSessionCookie } from 'better-auth/cookies'
import { buildLoginHref } from '@shared/constants/auth-route'
import { createDetailExistenceCheck, DETAIL_ROUTE_KIND } from '@server/service/detail-existence'

const NOT_FOUND_REWRITE_PATH = '/not-found-by-proxy'

const checkDetailExists = createDetailExistenceCheck(async () => {
    const { compose } = await import('@server/compose')
    const { chartService, courseService, replayService, findUserIdByLoginId } = compose()
    return {
        chart: async (hash) => (await chartService.findByHash(hash)) !== null,
        player: async (loginId) => (await findUserIdByLoginId(loginId)) !== null,
        course: courseService.exists,
        replay: replayService.exists,
    }
})

/**
 * Detail pages stream a 200 before their data is read, so a missing chart, player, course or replay can only get a real 404
 * if it is found missing here. An answer the lookup can not give lets the page render as before.
 */
const rewriteMissingDetail = async (request: NextRequest) => {
    if ((await checkDetailExists(request.nextUrl.pathname)) !== false) return NextResponse.next()
    return NextResponse.rewrite(new URL(NOT_FOUND_REWRITE_PATH, request.url))
}

const isDetailPath = (pathname: string) => Object.values(DETAIL_ROUTE_KIND).some((segment) => pathname.startsWith(`/${segment}/`))

/** Sends a request without a session cookie to the login page with a real redirect; the page still re-checks the session itself. */
export const proxy = async (request: NextRequest) => {
    if (isDetailPath(request.nextUrl.pathname)) return rewriteMissingDetail(request)
    if (getSessionCookie(request)) return NextResponse.next()
    return NextResponse.redirect(new URL(buildLoginHref(`${request.nextUrl.pathname}${request.nextUrl.search}`), request.url))
}

export const config = {
    matcher: [
        '/settings/:path*',
        '/admin/:path*',
        '/folders/:path*',
        '/charts/:hash',
        '/players/:playerId',
        '/courses/:courseHash',
        '/replays/:replayId',
    ],
}
