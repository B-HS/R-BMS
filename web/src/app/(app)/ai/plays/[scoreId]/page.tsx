import { Suspense } from 'react'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiPlayDetailContent } from '@widgets/ai/ai-play-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const metadata = buildPageMetadata({ title: '판 코치', description: '내 판 하나를 구간별로 보고 다른 판과 비교합니다.', index: false })

const AiPlayPage = ({ params }: PageProps<'/ai/plays/[scoreId]'>) => (
    <Suspense fallback={<ShellPageLoading title='판 코치' />}>
        <AiPlayDetailContent params={params} />
    </Suspense>
)

export default AiPlayPage
