import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { buildGuideAdvancedSections, buildGuideSections } from '@widgets/guide/guide-sections'

export const GuideToc: FC = () => {
    const t = useTranslations('browse.guide')

    return (
        <nav aria-label={t('toc.label')} className='flex flex-col py-2'>
            <p className='rbms-micro-label px-3 pb-1'>{t('toc.title')}</p>
            <ul className='flex flex-col'>
                {[...buildGuideSections(t), ...buildGuideAdvancedSections(t)].map((section) => (
                    <li key={section.id}>
                        <a
                            href={`#${section.id}`}
                            className='text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-ring block truncate px-3 py-1 text-xs outline-none focus-visible:ring-2 focus-visible:ring-inset'>
                            {section.title}
                        </a>
                    </li>
                ))}
            </ul>
        </nav>
    )
}
