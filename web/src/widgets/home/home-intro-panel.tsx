import type { FC } from 'react'
import Link from 'next/link'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { Button } from '@shared/ui/button'

const HIGHLIGHTS = [
    '플레이한 차트의 점수와 램프가 계정에 쌓이고, 차트마다 순위가 매겨집니다.',
    '라이벌을 등록하면 게임 안의 순위 패널에서 바로 견줄 수 있습니다.',
    '설정과 키 배치를 서버에 올려 다른 기기에서 내려받습니다.',
]

export const HomeIntroPanel: FC = () => (
    <DashboardPanel title='R-BMS 점수 서버' className={DASHBOARD_WIDE_PANEL_CLASS_NAME} bodyClassName='gap-3'>
        <p className='text-xs leading-5 break-keep'>
            R-BMS 는 BMS 플레이어입니다. 이 사이트는 그 플레이어가 점수를 올리는 인터넷 랭킹 서버이고, 올라온 기록과 난이도표를 여기서 볼 수 있습니다.
        </p>
        <ul className='text-muted-foreground flex list-disc flex-col gap-1 pl-4 text-xs leading-5 break-keep'>
            {HIGHLIGHTS.map((highlight) => (
                <li key={highlight}>{highlight}</li>
            ))}
        </ul>
        <div className='mt-auto flex flex-wrap gap-1'>
            <Button asChild size='sm'>
                <Link href='/guide'>연결 방법 보기</Link>
            </Button>
            <Button asChild variant='outline' size='sm'>
                <Link href='/tables/insane1'>発狂BMS難易度表 보기</Link>
            </Button>
            <Button asChild variant='outline' size='sm'>
                <Link href='/search'>차트 검색</Link>
            </Button>
        </div>
    </DashboardPanel>
)
