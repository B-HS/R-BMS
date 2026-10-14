import type { Metadata } from 'next'
import { SITE_DESCRIPTION, SITE_LOCALE, SITE_NAME } from '@shared/constants/site'

type PageMetadataInput = {
    title: string
    description?: string
    path?: string
    index?: boolean
}

/** Builds one page's metadata: the title joins the site template, and `index: false` keeps mirrored or private pages out of search. */
export const buildPageMetadata = ({ title, description = SITE_DESCRIPTION, path, index = true }: PageMetadataInput): Metadata => ({
    title,
    description,
    alternates: path ? { canonical: path } : undefined,
    openGraph: { type: 'website', siteName: SITE_NAME, locale: SITE_LOCALE, title: `${title} | ${SITE_NAME}`, description, url: path },
    twitter: { card: 'summary', title: `${title} | ${SITE_NAME}`, description },
    robots: index ? undefined : { index: false, follow: false },
})
