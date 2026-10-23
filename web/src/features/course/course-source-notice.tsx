import type { FC } from 'react'
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
    const hashPrefix = <span className='font-mono'>{source.file_sha256.slice(0, HASH_PREFIX_LENGTH)}</span>

    if (source.kind === 'table')
        return (
            <p className='text-muted-foreground text-xs'>
                출처 {source.name}({source.maker}) · {linkOf(source.page_url, '원본 표')} · 내용 받은 날 {formatDateTime(source.retrieved_at)} (KST) ·
                SHA-256 {hashPrefix}
            </p>
        )

    const fileName = safeUrlOf(source.file_url)?.pathname.split('/').at(-1) ?? ''
    const constraintSources = readConstraintSources(source)

    return (
        <div className='text-muted-foreground flex flex-col gap-0.5 text-xs'>
            <p>
                출처 {source.name} · 코스 정의 {source.maker} 공식 코스 파일({fileName}) ·{' '}
                {new Date(source.retrieved_at).toISOString().slice(0, ISO_DATE_LENGTH)} 받음 · SHA-256 {hashPrefix} ·{' '}
                {linkOf(source.page_url, '공식 페이지')}
            </p>
            {constraintSources.length > 0 && (
                <p>
                    제약·트로피{' '}
                    {constraintSources.map((constraintSource, index) => (
                        <span key={constraintSource.url}>
                            {index > 0 && ', '}
                            {linkOf(constraintSource.url, safeUrlOf(constraintSource.url)?.host ?? constraintSource.url)} (
                            {constraintSource.retrieved_on} 받음)
                        </span>
                    ))}
                </p>
            )}
            {keys >= DOUBLE_PLAY_KEYS && <p>DP 코스의 제약·트로피는 같은 단의 SP 값입니다(DP 값의 원천이 없습니다).</p>}
        </div>
    )
}
