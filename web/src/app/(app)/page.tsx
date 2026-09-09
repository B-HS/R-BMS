import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { HomeContent } from '@widgets/home/home-content'

const HomePage = () => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <HomeContent />
    </Suspense>
)

export default HomePage
