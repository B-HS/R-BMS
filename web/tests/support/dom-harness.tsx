import { mock } from 'bun:test'
import { act, useLayoutEffect, type ReactNode } from 'react'
import { createRoot, type Root } from 'react-dom/client'

const skipLayoutEffect: typeof useLayoutEffect = () => undefined

/**
 * Radix picks `useLayoutEffect` or a no-op once, when its module first loads, by checking `document`. Bun runs every test file in one
 * process, so whichever file loads Radix first would decide for all of them. The mock makes the same check on every call instead,
 * which keeps server-rendering tests (no `document`) and DOM tests (happy-dom registered) correct in any file order.
 */
mock.module('@radix-ui/react-use-layout-effect', () => ({
    useLayoutEffect: (...args: Parameters<typeof useLayoutEffect>) => (globalThis.document ? useLayoutEffect : skipLayoutEffect)(...args),
}))

type Matcher = string | RegExp
type Scope = () => ParentNode
type RoleOptions = { name?: Matcher }

const POLL_INTERVAL_MS = 10
const WAIT_TIMEOUT_MS = 1000
const DEBUG_HTML_LIMIT = 1500
const HEADING_TAG = /^h[1-6]$/

const mounted: { root: Root; container: HTMLElement }[] = []

const actEnvironment = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }

export const runInAct = async (work: () => void | Promise<void>) => {
    const previous = actEnvironment.IS_REACT_ACT_ENVIRONMENT
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = true
    try {
        await act(async () => {
            await work()
        })
    } finally {
        actEnvironment.IS_REACT_ACT_ENVIRONMENT = previous
    }
}

export const mount = async (ui: ReactNode) => {
    const container = document.body.appendChild(document.createElement('div'))
    const root = createRoot(container)
    mounted.push({ root, container })
    await runInAct(() => root.render(ui))
}

export const unmountAll = async () => {
    for (const { root, container } of mounted.splice(0)) {
        await runInAct(() => root.unmount())
        container.remove()
    }
}

const normalize = (text: string | null | undefined) => (text ?? '').replace(/\s+/g, ' ').trim()

const matches = (value: string, matcher: Matcher) => (typeof matcher === 'string' ? value === matcher : matcher.test(value))

const describeMatcher = (matcher: Matcher) => (typeof matcher === 'string' ? JSON.stringify(matcher) : String(matcher))

const descendants = (scope: Scope) => [...scope().querySelectorAll('*')] as HTMLElement[]

const roleOf = (element: HTMLElement) => {
    const explicit = element.getAttribute('role')
    if (explicit) return explicit
    const tag = element.tagName.toLowerCase()
    if (tag === 'button') return 'button'
    if (tag === 'a' && element.hasAttribute('href')) return 'link'
    if (HEADING_TAG.test(tag)) return 'heading'
    if (tag === 'section' && (element.hasAttribute('aria-labelledby') || element.hasAttribute('aria-label'))) return 'region'
    return null
}

const nameOf = (element: HTMLElement) => {
    const labelledBy = element.getAttribute('aria-labelledby')
    if (labelledBy)
        return normalize(
            labelledBy
                .split(/\s+/)
                .map((id) => document.getElementById(id)?.textContent ?? '')
                .join(' '),
        )
    const ariaLabel = element.getAttribute('aria-label')
    if (ariaLabel) return normalize(ariaLabel)
    const nativeLabel = element.id ? document.querySelector(`label[for="${element.id}"]`) : null
    if (nativeLabel) return normalize(nativeLabel.textContent)
    return normalize(element.textContent)
}

const ownText = (element: HTMLElement) =>
    normalize(
        [...element.childNodes]
            .filter((node) => node.nodeType === Node.TEXT_NODE)
            .map((node) => node.textContent)
            .join(''),
    )

const labelledControls = (scope: Scope, matcher: Matcher) => {
    const byLabel = [...scope().querySelectorAll('label')]
        .filter((label) => matches(normalize(label.textContent), matcher))
        .flatMap((label) => {
            const target = label.htmlFor ? document.getElementById(label.htmlFor) : label.querySelector('input,select,textarea,button')
            return target ? [target as HTMLElement] : []
        })
    const byAriaLabel = descendants(scope).filter(
        (element) => element.hasAttribute('aria-label') && matches(normalize(element.getAttribute('aria-label')), matcher),
    )
    return [...new Set([...byLabel, ...byAriaLabel])]
}

const debugHtml = (scope: Scope) => {
    const root = scope()
    const html = root instanceof Element ? root.outerHTML : document.body.innerHTML
    return html.length > DEBUG_HTML_LIMIT ? `${html.slice(0, DEBUG_HTML_LIMIT)}...` : html
}

export const waitFor = async <T,>(read: () => T) => {
    const deadline = Date.now() + WAIT_TIMEOUT_MS
    for (;;) {
        try {
            return read()
        } catch (error) {
            if (Date.now() >= deadline) throw error
            await runInAct(() => new Promise<void>((resolve) => setTimeout(resolve, POLL_INTERVAL_MS)))
        }
    }
}

const queriesFor = (scope: Scope) => {
    const exactlyOne = (found: HTMLElement[], description: string) => {
        if (found.length === 1) return found[0]
        const reason = found.length === 0 ? 'Unable to find' : `Found ${found.length} elements for`
        throw new Error(`${reason} ${description}\n${debugHtml(scope)}`)
    }

    const allByRole = (role: string, options: RoleOptions = {}) =>
        descendants(scope).filter((element) => roleOf(element) === role && (options.name === undefined || matches(nameOf(element), options.name)))
    const allByText = (matcher: Matcher) => descendants(scope).filter((element) => matches(ownText(element), matcher))

    const roleDescription = (role: string, options: RoleOptions) =>
        `role=${role}${options.name === undefined ? '' : ` name=${describeMatcher(options.name)}`}`

    const getByRole = (role: string, options: RoleOptions = {}) => exactlyOne(allByRole(role, options), roleDescription(role, options))
    const getByText = (matcher: Matcher) => exactlyOne(allByText(matcher), `text=${describeMatcher(matcher)}`)
    const getByLabelText = (matcher: Matcher) => exactlyOne(labelledControls(scope, matcher), `label=${describeMatcher(matcher)}`)

    return {
        getByRole,
        queryByRole: (role: string, options: RoleOptions = {}) => allByRole(role, options)[0] ?? null,
        findByRole: (role: string, options: RoleOptions = {}) => waitFor(() => getByRole(role, options)),
        getAllByRole: allByRole,
        getByText,
        queryByText: (matcher: Matcher) => allByText(matcher)[0] ?? null,
        findByText: (matcher: Matcher) => waitFor(() => getByText(matcher)),
        getByLabelText,
        queryByLabelText: (matcher: Matcher) => labelledControls(scope, matcher)[0] ?? null,
        findByLabelText: (matcher: Matcher) => waitFor(() => getByLabelText(matcher)),
    }
}

export const page = queriesFor(() => document.body)

export const within = (element: HTMLElement) => queriesFor(() => element)

export const click = (element: HTMLElement) =>
    runInAct(() => void element.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })))

export const submit = (form: HTMLFormElement) => runInAct(() => void form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })))

export const keyDown = (element: HTMLElement, key: string) =>
    runInAct(() => void element.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })))

export const changeValue = (input: HTMLInputElement, value: string) =>
    runInAct(() => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value)
        input.dispatchEvent(new Event('input', { bubbles: true }))
    })
