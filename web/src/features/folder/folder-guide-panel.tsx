import type { FC } from 'react'
import Link from 'next/link'
import { BookOpen } from 'lucide-react'

export const FolderGuidePanel: FC = () => (
    <section aria-labelledby='folder-guide-title' className='flex flex-col gap-2 px-3 py-3 group-data-[collapsible=icon]:hidden'>
        <h2 id='folder-guide-title' className='rbms-micro-label'>
            R-BMS 에서 보기
        </h2>
        <p className='text-muted-foreground text-xs leading-relaxed'>
            R-BMS 에 로그인하면 선곡 화면의 MY FOLDERS 아래에 이 폴더들이 같은 순서로 나타납니다. R-BMS 는 읽기만 하므로 고치는 일은 여기서 합니다.
        </p>
        <Link href='/guide#account' className='text-foreground flex items-center gap-1.5 text-xs font-medium hover:underline'>
            <BookOpen className='size-3.5' aria-hidden />
            연결 방법 보기
        </Link>
    </section>
)
