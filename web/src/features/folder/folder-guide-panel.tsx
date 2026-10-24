import type { FC } from 'react'
import Link from 'next/link'
import { BookOpen } from 'lucide-react'
import { useTranslations } from 'next-intl'

export const FolderGuidePanel: FC = () => {
    const t = useTranslations('pages.folder.guide')

    return (
        <section aria-labelledby='folder-guide-title' className='flex flex-col gap-2 px-3 py-3 group-data-[collapsible=icon]:hidden'>
            <h2 id='folder-guide-title' className='rbms-micro-label'>
                {t('title')}
            </h2>
            <p className='text-muted-foreground text-xs leading-relaxed'>{t('body')}</p>
            <Link href='/guide#account' className='text-foreground flex items-center gap-1.5 text-xs font-medium hover:underline'>
                <BookOpen className='size-3.5' aria-hidden />
                {t('link')}
            </Link>
        </section>
    )
}
