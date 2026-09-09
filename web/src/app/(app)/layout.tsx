import { Suspense } from 'react'
import { NavRail } from '@widgets/shell/nav-rail'
import { MobileNav } from '@widgets/shell/mobile-nav'
import { FilterPanel } from '@widgets/shell/filter-panel'

const AppLayout = ({ children }: LayoutProps<'/'>) => (
    <div data-surface='app' className='bg-background flex h-dvh min-h-0 w-full flex-col gap-px md:flex-row'>
        <Suspense fallback={<div className='bg-sidebar h-12 shrink-0 md:hidden' />}>
            <MobileNav />
        </Suspense>
        <Suspense fallback={<div className='bg-sidebar hidden w-64 shrink-0 md:block' />}>
            <NavRail />
        </Suspense>
        <main className='bg-background flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto'>
            <div className='flex flex-col gap-px'>{children}</div>
        </main>
        <Suspense fallback={null}>
            <FilterPanel />
        </Suspense>
    </div>
)

export default AppLayout
