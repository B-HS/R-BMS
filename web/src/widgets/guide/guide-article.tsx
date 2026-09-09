import type { FC } from 'react'
import Link from 'next/link'
import { GUIDE_ADVANCED_SECTIONS, GUIDE_INTRO, GUIDE_SECTIONS, type GuideSection } from '@widgets/guide/guide-sections'

const GuideSectionBlock: FC<{ section: GuideSection }> = ({ section }) => (
    <section id={section.id} className='flex flex-col gap-2'>
        <h2 className='text-xl font-semibold tracking-tight'>{section.title}</h2>
        <p className='text-muted-foreground text-sm leading-7'>{section.body}</p>
        {section.steps && (
            <ol className='text-muted-foreground flex list-decimal flex-col gap-1 pl-5 text-sm leading-7'>
                {section.steps.map((step) => (
                    <li key={step}>{step}</li>
                ))}
            </ol>
        )}
        {section.rows && (
            <dl className='border-border flex flex-col gap-2 border-l pl-4 text-sm leading-7'>
                {section.rows.map((row) => (
                    <div key={row.label} className='flex flex-col gap-0.5 sm:flex-row sm:gap-3'>
                        <dt className='text-foreground shrink-0 font-mono text-2xs sm:w-52 sm:text-sm'>{row.label}</dt>
                        <dd className='text-muted-foreground'>{row.description}</dd>
                    </div>
                ))}
            </dl>
        )}
        {section.code && <pre className='bg-muted text-foreground text-2xs overflow-x-auto p-3 font-mono lg:text-sm'>{section.code}</pre>}
    </section>
)

export const GuideArticle: FC = () => (
    <>
        <header className='bg-background/60 border-border sticky top-0 z-50 flex h-12 items-center border-b px-4 backdrop-blur-xs'>
            <span className='font-mono text-sm'>rbms IR · 연동 가이드</span>
        </header>
        <main className='mx-auto flex w-full max-w-3xl flex-col gap-6 px-4 pt-6 pb-20'>
            <h1 className='text-3xl font-extrabold tracking-tight'>게임 안에서 IR 연동하기</h1>
            <p className='text-muted-foreground text-sm leading-7'>{GUIDE_INTRO}</p>
            {GUIDE_SECTIONS.map((section) => (
                <GuideSectionBlock key={section.id} section={section} />
            ))}
            <h2 className='text-2xl font-bold tracking-tight'>고급 (Advanced)</h2>
            {GUIDE_ADVANCED_SECTIONS.map((section) => (
                <GuideSectionBlock key={section.id} section={section} />
            ))}
            <nav aria-label='다음 단계' className='flex flex-wrap gap-4 text-sm'>
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
        </main>
    </>
)
