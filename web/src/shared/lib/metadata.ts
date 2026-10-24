import type { Metadata } from 'next'
import { getLocale, getTranslations } from 'next-intl/server'
import { OPEN_GRAPH_LOCALE, SITE_NAME } from '@shared/constants/site'

type PageMetadataInput = {
    title: string
    description?: string
    path?: string
    index?: boolean
}

/**
 * Builds one page's metadata in the language of the request: the title joins the site template, the description defaults to the site's,
 * and `index: false` keeps mirrored or private pages out of search.
 */
export const buildPageMetadata = async ({ title, description, path, index = true }: PageMetadataInput): Promise<Metadata> => {
    const [locale, t] = await Promise.all([getLocale(), getTranslations('pages.site')])
    const resolvedDescription = description ?? t('description')
    return {
        title,
        description: resolvedDescription,
        alternates: path ? { canonical: path } : undefined,
        openGraph: {
            type: 'website',
            siteName: SITE_NAME,
            locale: OPEN_GRAPH_LOCALE[locale],
            title: `${title} | ${SITE_NAME}`,
            description: resolvedDescription,
            url: path,
        },
        twitter: { card: 'summary', title: `${title} | ${SITE_NAME}`, description: resolvedDescription },
        robots: index ? undefined : { index: false, follow: false },
    }
}
