import type { FC } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useTranslations } from 'next-intl'

type PagerProps = {
    page: number
    totalPages: number
    onPageChange: (page: number) => void
}

const CONTROL_CLASS =
    'text-muted-foreground hover:text-foreground relative inline-flex size-7 items-center justify-center transition-opacity disabled:pointer-events-none disabled:opacity-50 pointer-coarse:after:absolute pointer-coarse:after:-inset-2'

export const Pager: FC<PagerProps> = ({ page, totalPages, onPageChange }) => {
    const t = useTranslations('pages.pagination')

    return (
        <nav aria-label={t('navigation')} className='flex items-center justify-end gap-2 pt-2'>
            <button type='button' aria-label={t('previous')} disabled={page <= 1} onClick={() => onPageChange(page - 1)} className={CONTROL_CLASS}>
                <ChevronLeft className='size-4' aria-hidden />
            </button>
            <span className='text-muted-foreground text-2xs tabular-nums'>
                {page} / {totalPages}
            </span>
            <button
                type='button'
                aria-label={t('next')}
                disabled={page >= totalPages}
                onClick={() => onPageChange(page + 1)}
                className={CONTROL_CLASS}>
                <ChevronRight className='size-4' aria-hidden />
            </button>
        </nav>
    )
}
