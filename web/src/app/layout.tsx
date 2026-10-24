import type { Metadata, Viewport } from 'next'
import { NextIntlClientProvider } from 'next-intl'
import { getLocale, getTranslations } from 'next-intl/server'
import '@app/globals.css'
import { Providers } from '@app/providers'
import { OPEN_GRAPH_LOCALE, SITE_NAME, SITE_URL } from '@shared/constants/site'
import { AppShell } from '@widgets/shell/app-shell'

export const generateMetadata = async (): Promise<Metadata> => {
    const [locale, t] = await Promise.all([getLocale(), getTranslations('pages.site')])
    const description = t('description')
    return {
        metadataBase: new URL(SITE_URL),
        applicationName: SITE_NAME,
        title: { template: `%s | ${SITE_NAME}`, default: SITE_NAME },
        description,
        openGraph: { type: 'website', siteName: SITE_NAME, locale: OPEN_GRAPH_LOCALE[locale], title: SITE_NAME, description, url: '/' },
        twitter: { card: 'summary', title: SITE_NAME, description },
    }
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
