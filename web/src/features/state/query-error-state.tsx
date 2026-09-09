import type { FC } from 'react'
import { CircleAlert } from 'lucide-react'

export const QueryErrorState: FC<{ message: string }> = ({ message }) => (
    <div className='bg-card flex flex-col items-center justify-center gap-6 p-3 text-center text-balance'>
        <CircleAlert className='size-6' aria-hidden />
        <div className='flex flex-col gap-1'>
            <p className='text-sm font-medium'>검색식을 해석할 수 없습니다</p>
            <p className='text-muted-foreground font-mono text-xs'>{message}</p>
        </div>
    </div>
)
