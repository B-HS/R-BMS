import type { Metadata } from 'next'
import '@app/globals.css'
import { Providers } from '@app/providers'

export const metadata: Metadata = {
    title: 'rbms IR',
    description: 'rbms 인터넷 랭킹 서버',
}

const RootLayout = ({ children }: LayoutProps<'/'>) => (
    <html lang='ko' className='h-full' suppressHydrationWarning>
        <body className='flex min-h-full flex-col'>
            <Providers>{children}</Providers>
        </body>
    </html>
)

export default RootLayout
