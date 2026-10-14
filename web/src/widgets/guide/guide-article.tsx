import type { FC } from 'react'
import Link from 'next/link'
import { GUIDE_ADVANCED_SECTIONS, GUIDE_INTRO, GUIDE_SECTIONS, type GuideSection } from '@widgets/guide/guide-sections'

const GuideSectionBlock: FC<{ section: GuideSection }> = ({ section }) => (
    <section id={section.id} className='flex scroll-mt-3 flex-col gap-2'>
        <h2 className='text-base font-semibold tracking-tight'>{section.title}</h2>
        <p className='text-muted-foreground'>{section.body}</p>
        {section.steps && (
            <ol className='text-muted-foreground flex list-decimal flex-col gap-1 pl-5'>
                {section.steps.map((step) => (
                    <li key={step}>{step}</li>
                ))}
            </ol>
        )}
        {section.rows && (
            <dl className='border-border flex flex-col border text-xs leading-5'>
                {section.rows.map((row) => (
                    <div key={row.label} className='border-border flex flex-col gap-0.5 border-b p-2 last:border-b-0 sm:flex-row sm:gap-3'>
                        <dt className='text-foreground shrink-0 font-mono sm:w-52'>{row.label}</dt>
                        <dd className='text-muted-foreground'>{row.description}</dd>
                    </div>
                ))}
            </dl>
        )}
        {section.code && <pre className='bg-muted text-foreground overflow-x-auto p-3 font-mono text-xs'>{section.code}</pre>}
    </section>
)

export const GuideArticle: FC = () => (
    <article className='flex w-full max-w-3xl flex-col gap-8 p-3 pb-12 text-sm leading-6 break-keep'>
        <p className='text-muted-foreground'>{GUIDE_INTRO}</p>
        {GUIDE_SECTIONS.map((section) => (
            <GuideSectionBlock key={section.id} section={section} />
        ))}
        <p className='rbms-micro-label border-border border-t pt-3'>고급 (Advanced)</p>
        {GUIDE_ADVANCED_SECTIONS.map((section) => (
            <GuideSectionBlock key={section.id} section={section} />
        ))}
        <nav aria-label='다음 단계' className='flex flex-wrap gap-4'>
            <Link href='/signup' className='underline'>
                계정 만들기
            </Link>
            <Link href='/login' className='underline'>
                로그인
            </Link>
            <Link href='/settings' className='underline'>
                API 토큰 발급
            </Link>
        </nav>
    </article>
)
