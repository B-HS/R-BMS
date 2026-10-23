import type { Metadata, Viewport } from 'next'
import { NextIntlClientProvider } from 'next-intl'
import { getLocale } from 'next-intl/server'
import '@app/globals.css'
import { Providers } from '@app/providers'
import { SITE_DESCRIPTION, SITE_LOCALE, SITE_NAME, SITE_URL } from '@shared/constants/site'
import { AppShell } from '@widgets/shell/app-shell'

export const metadata: Metadata = {
    metadataBase: new URL(SITE_URL),
    applicationName: SITE_NAME,
    title: { template: `%s | ${SITE_NAME}`, default: SITE_NAME },
    description: SITE_DESCRIPTION,
    openGraph: { type: 'website', siteName: SITE_NAME, locale: SITE_LOCALE, title: SITE_NAME, description: SITE_DESCRIPTION, url: '/' },
    twitter: { card: 'summary', title: SITE_NAME, description: SITE_DESCRIPTION },
}

export const instant = false

export const viewport: Viewport = {
    themeColor: [
        { media: '(prefers-color-scheme: dark)', color: '#000000' },
        { media: '(prefers-color-scheme: light)', color: '#e2e2e2' },
    ],
}

const RootLayout = async ({ children }: LayoutProps<'/'>) => (
    <html lang={await getLocale()} suppressHydrationWarning>
        <body className='bg-background text-foreground min-h-dvh font-sans antialiased md:h-dvh md:overflow-hidden'>
            <NextIntlClientProvider>
                <Providers>
                    <AppShell>{children}</AppShell>
                </Providers>
            </NextIntlClientProvider>
        </body>
    </html>
)

export default RootLayout
