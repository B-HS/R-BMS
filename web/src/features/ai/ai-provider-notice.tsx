import type { FC } from 'react'
import { ShieldCheck } from 'lucide-react'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'

export const AiProviderNotice: FC = () => (
    <Alert role='note'>
        <ShieldCheck aria-hidden />
        <AlertTitle>키 보관과 비용 안내</AlertTitle>
        <AlertDescription className='break-keep'>
            <p>키는 이 서버에 암호화해 보관하고 AI 호출에만 씁니다. 공급자 콘솔에서 언제든 취소할 수 있습니다.</p>
            <p>AI 호출은 직접 요청할 때만 일어나며 비용은 내 공급자 계정에 청구됩니다. 공급자 콘솔에서 지출 한도를 걸어 두기를 권합니다.</p>
        </AlertDescription>
    </Alert>
)
