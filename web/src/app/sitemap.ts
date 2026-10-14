import type { MetadataRoute } from 'next'
import { SITEMAP_STATIC_PATHS, SITE_URL } from '@shared/constants/site'

const sitemap = (): MetadataRoute.Sitemap => SITEMAP_STATIC_PATHS.map((path) => ({ url: `${SITE_URL}${path === '/' ? '' : path}` }))

export default sitemap
