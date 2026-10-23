import 'server-only'
import { Suspense } from 'react'
import { notFound } from 'next/navigation'
import { connection } from 'next/server'
import { TableLevelSkeleton } from '@features/table/table-level-skeleton'
import { TableSourceAttribution } from '@features/table/table-source-attribution'
import { ShellPage } from '@widgets/shell/shell-page'
import { TableSnapshotSession } from '@widgets/table/table-snapshot-session'
import { SITE_URL } from '@shared/constants/site'
import { formatCount } from '@shared/lib/format'
import { CopyButton } from '@shared/ui/copy-button'
import { compose } from '@server/compose'
import { getSnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { groupTableByLevel } from '@server/service/domain/table-snapshot/table-snapshot.service'

const SKELETON_PREVIEW_CARDS = 12

export const TableSnapshotContent = async ({ tableId }: { tableId: string }) => {
    const bundledSnapshot = getSnapshotTable(tableId)
    if (!bundledSnapshot) await connection()
    const detail = bundledSnapshot ? groupTableByLevel(bundledSnapshot) : await compose().tableSnapshotService.getDetail(tableId)
    if (!detail) notFound()

    const { table, levels } = detail
    const tableUrl = `${SITE_URL}${table.standard_paths.page}`
    const levelRange = levels.length > 0 ? `${table.symbol}${levels[0].level} ~ ${table.symbol}${levels[levels.length - 1].level}` : ''

    return (
        <ShellPage
            title={table.name}
            description={`${levelRange} · ${formatCount(table.entry_count)}곡 · 갱신 ${table.last_update}`}
            actions={
                <CopyButton
                    value={tableUrl}
                    label='표 주소 복사'
                    copiedMessage='표 주소를 복사했습니다. R-BMS 의 난이도표 설정에 붙여 넣으세요.'
                    variant='ghost'
                    className='md:hidden'
                />
            }>
            {table.source && <TableSourceAttribution source={table.source} />}
            <Suspense
                fallback={
                    <TableLevelSkeleton
                        levels={levels.map((group) => ({ label: `${table.symbol}${group.level}`, count: group.entries.length }))}
                        previewCards={SKELETON_PREVIEW_CARDS}
                    />
                }>
                <TableSnapshotSession detail={detail} tableUrl={tableUrl} />
            </Suspense>
        </ShellPage>
    )
}
