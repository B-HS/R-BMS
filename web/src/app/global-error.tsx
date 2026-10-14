'use client'

import '@app/globals.css'

const GlobalErrorPage = ({ retry }: { error: Error & { digest?: string }; retry: () => void }) => (
    <html lang='ko' className='dark' data-theme='dark'>
        <body className='bg-background text-foreground flex min-h-dvh flex-col items-center justify-center gap-3 p-6 text-center font-sans antialiased'>
            <title>오류 | R-BMS</title>
            <p className='rbms-micro-label'>R-BMS</p>
            <h1 className='text-sm font-semibold'>문제가 생겼습니다</h1>
            <p className='text-muted-foreground text-xs'>화면을 그리지 못했습니다. 잠시 후 다시 시도해 주세요.</p>
            <button
                type='button'
                onClick={() => retry()}
                className='border-border hover:bg-muted focus-visible:ring-ring/50 h-7 border px-2.5 text-xs font-medium outline-none focus-visible:ring-3'>
                다시 시도
            </button>
        </body>
    </html>
)

export default GlobalErrorPage
