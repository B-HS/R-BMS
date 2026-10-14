import type { MetadataRoute } from 'next'
import { ROBOTS_DISALLOW_PATHS, SITE_URL } from '@shared/constants/site'

const robots = (): MetadataRoute.Robots => ({
    rules: { userAgent: '*', allow: '/', disallow: ROBOTS_DISALLOW_PATHS },
    sitemap: `${SITE_URL}/sitemap.xml`,
})

export default robots
