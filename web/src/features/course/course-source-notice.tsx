import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { TableExternalLink } from '@features/table/table-external-link'
import { formatDateTime } from '@shared/lib/format'
import { readConstraintSources, type CourseCatalogSource } from '@entities/course.type'

const HASH_PREFIX_LENGTH = 12
const DOUBLE_PLAY_KEYS = 14
const ISO_DATE_LENGTH = 'YYYY-MM-DD'.length
const ALLOWED_LINK_PROTOCOLS = new Set(['https:', 'http:'])

const safeUrlOf = (value: string) => {
    try {
        const url = new URL(value)
        return ALLOWED_LINK_PROTOCOLS.has(url.protocol) ? url : null
    } catch {
        return null
    }
}

const linkOf = (href: string, label: string) =>
    safeUrlOf(href) ? <TableExternalLink href={href} label={label} appearance='text' /> : <span>{label}</span>

type CourseSourceNoticeProps = {
    source: CourseCatalogSource
    keys: number
}

/**
 * Says where a course group comes from: who made it, the file or table it was read from, when it was received and the start of the
 * SHA-256 of what was received, so a viewer can tell the copy from the original.
 */
export const CourseSourceNotice: FC<CourseSourceNoticeProps> = ({ source, keys }) => {
    const t = useTranslations('pages.course.source')
    const hashPrefix = <span className='font-mono'>{source.file_sha256.slice(0, HASH_PREFIX_LENGTH)}</span>

    if (source.kind === 'table')
        return (
            <p className='text-muted-foreground text-xs'>
                {t('label')} {source.name}({source.maker}) · {linkOf(source.page_url, t('originalTable'))} ·{' '}
                {t('receivedOn', { date: formatDateTime(source.retrieved_at) })} · SHA-256 {hashPrefix}
            </p>
        )

    const fileName = safeUrlOf(source.file_url)?.pathname.split('/').at(-1) ?? ''
    const constraintSources = readConstraintSources(source)

    return (
        <div className='text-muted-foreground flex flex-col gap-0.5 text-xs'>
            <p>
                {t('label')} {source.name} · {t('officialFile', { maker: source.maker, file: fileName })} ·{' '}
                {t('received', { date: new Date(source.retrieved_at).toISOString().slice(0, ISO_DATE_LENGTH) })} · SHA-256 {hashPrefix} ·{' '}
                {linkOf(source.page_url, t('officialPage'))}
            </p>
            {constraintSources.length > 0 && (
                <p>
                    {t('constraints')}{' '}
                    {constraintSources.map((constraintSource, index) => (
                        <span key={constraintSource.url}>
                            {index > 0 && ', '}
                            {linkOf(constraintSource.url, safeUrlOf(constraintSource.url)?.host ?? constraintSource.url)}{' '}
                            {t('constraintReceived', { date: constraintSource.retrieved_on })}
                        </span>
                    ))}
                </p>
            )}
            {keys >= DOUBLE_PLAY_KEYS && <p>{t('dpNote')}</p>}
        </div>
    )
}
