import type { FC } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { cn } from '@shared/lib/cn'

type PagerProps = {
    page: number
    totalPages: number
    onPageChange: (page: number) => void
}

const CONTROL_CLASS = 'text-muted-foreground hover:text-foreground inline-flex size-7 items-center justify-center transition-opacity'

export const Pager: FC<PagerProps> = ({ page, totalPages, onPageChange }) => (
    <nav aria-label='페이지 이동' className='flex items-center justify-end gap-2 pt-2'>
        <button
            type='button'
            aria-label='이전 페이지'
            onClick={() => onPageChange(page - 1)}
            className={cn(CONTROL_CLASS, page <= 1 && 'pointer-events-none opacity-50')}>
            <ChevronLeft className='size-4' aria-hidden />
        </button>
        <span className='text-muted-foreground text-2xs tabular-nums'>
            {page} / {totalPages}
        </span>
        <button
            type='button'
            aria-label='다음 페이지'
            onClick={() => onPageChange(page + 1)}
            className={cn(CONTROL_CLASS, page >= totalPages && 'pointer-events-none opacity-50')}>
            <ChevronRight className='size-4' aria-hidden />
        </button>
    </nav>
)
