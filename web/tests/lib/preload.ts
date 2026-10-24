import { mock } from 'bun:test'
import * as ReactDomServer from 'react-dom/server'
import { withIntl } from '../support/intl'

mock.module('server-only', () => ({}))

type MarkupRenderer = (...args: Parameters<typeof ReactDomServer.renderToStaticMarkup>) => string

const wrapInIntl =
    (render: MarkupRenderer): MarkupRenderer =>
    (ui, options) =>
        render(withIntl(ui), options)

mock.module('react-dom/server', () => ({
    ...ReactDomServer,
    renderToStaticMarkup: wrapInIntl(ReactDomServer.renderToStaticMarkup),
    renderToString: wrapInIntl(ReactDomServer.renderToString),
}))

process.env.ALLOW_GUEST = 'false'
process.env.REQUIRE_BUILD_HASH = 'false'
process.env.DATABASE_URL ??= 'mysql://rbms:rbms@127.0.0.1:1/rbms_unreachable'
