import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPage } from '@widgets/shell/shell-page'
import { TableCatalogContent } from '@widgets/table/table-catalog-content'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.tables')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/tables' })
}

const TablesPage = async () => {
    const t = await getTranslations('pages.route.tables')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<RowListSkeleton label={t('loading')} />}>
                <TableCatalogContent />
            </Suspense>
        </ShellPage>
    )
}

export default TablesPage
