import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'

export const LoadErrorState: FC<{ title: string }> = ({ title }) => (
    <div className='bg-card flex flex-col items-center justify-center gap-6 p-3 text-center text-balance'>
        <TriangleAlert className='text-destructive size-6' aria-hidden />
        <div className='flex flex-col gap-1'>
            <p className='text-sm font-medium'>{title}</p>
            <p className='text-muted-foreground text-xs'>요청이 실패했습니다. 잠시 후 다시 시도하세요.</p>
        </div>
    </div>
)
