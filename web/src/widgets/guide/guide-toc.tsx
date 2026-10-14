import type { FC } from 'react'
import { GUIDE_ADVANCED_SECTIONS, GUIDE_SECTIONS } from '@widgets/guide/guide-sections'

export const GuideToc: FC = () => (
    <nav aria-label='가이드 목차' className='flex flex-col py-2'>
        <p className='rbms-micro-label px-3 pb-1'>목차</p>
        <ul className='flex flex-col'>
            {[...GUIDE_SECTIONS, ...GUIDE_ADVANCED_SECTIONS].map((section) => (
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
