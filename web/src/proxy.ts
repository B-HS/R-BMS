import { NextResponse, type NextRequest } from 'next/server'
import { getSessionCookie } from 'better-auth/cookies'
import { buildLoginHref } from '@shared/constants/auth-route'

/** Sends a request without a session cookie to the login page with a real redirect; the page still re-checks the session itself. */
export const proxy = (request: NextRequest) => {
    if (getSessionCookie(request)) return NextResponse.next()
    return NextResponse.redirect(new URL(buildLoginHref(`${request.nextUrl.pathname}${request.nextUrl.search}`), request.url))
}

export const config = {
    matcher: ['/settings/:path*', '/admin/:path*', '/folders/:path*'],
}
