import type { FC } from 'react'
import { PanelCard } from '@features/shell/panel-card'

export const AiPrincipleNote: FC = () => (
    <PanelCard title='AI 해석의 원칙'>
        <ul className='text-muted-foreground flex list-disc flex-col gap-1 pl-4 text-xs leading-5 break-keep'>
            <li>숫자, 표, 그래프는 서버가 계산한 결과입니다. 같은 기록이면 언제나 같은 값이 나옵니다.</li>
            <li>문장 해석만 AI 가 만들며, 어느 공급자와 모델이 만들었는지 함께 표시합니다.</li>
            <li>AI 는 직접 요청할 때만 호출됩니다. 호출은 내 키와 사용 한도를 쓰고, 같은 입력은 저장된 결과를 다시 보여 줍니다.</li>
        </ul>
    </PanelCard>
)
