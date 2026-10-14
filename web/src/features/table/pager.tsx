import type { FC } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'

type PagerProps = {
    page: number
    totalPages: number
    onPageChange: (page: number) => void
}

const CONTROL_CLASS =
    'text-muted-foreground hover:text-foreground relative inline-flex size-7 items-center justify-center transition-opacity disabled:pointer-events-none disabled:opacity-50 pointer-coarse:after:absolute pointer-coarse:after:-inset-2'

export const Pager: FC<PagerProps> = ({ page, totalPages, onPageChange }) => (
    <nav aria-label='페이지 이동' className='flex items-center justify-end gap-2 pt-2'>
        <button type='button' aria-label='이전 페이지' disabled={page <= 1} onClick={() => onPageChange(page - 1)} className={CONTROL_CLASS}>
            <ChevronLeft className='size-4' aria-hidden />
        </button>
        <span className='text-muted-foreground text-2xs tabular-nums'>
            {page} / {totalPages}
        </span>
        <button type='button' aria-label='다음 페이지' disabled={page >= totalPages} onClick={() => onPageChange(page + 1)} className={CONTROL_CLASS}>
            <ChevronRight className='size-4' aria-hidden />
        </button>
    </nav>
)
