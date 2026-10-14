import type { FC } from 'react'
import { CircleAlert } from 'lucide-react'

export const QueryErrorState: FC<{ message: string }> = ({ message }) => (
    <div className='bg-card flex min-h-32 flex-col items-center justify-center gap-3 p-6 text-center text-balance'>
        <CircleAlert className='size-4 shrink-0' aria-hidden />
        <div role='alert' className='flex flex-col gap-1'>
            <p className='text-sm font-medium'>검색식을 해석할 수 없습니다</p>
            <p className='text-muted-foreground font-mono text-xs'>{message}</p>
        </div>
    </div>
)
