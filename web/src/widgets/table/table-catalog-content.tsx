import 'server-only'
import { Suspense } from 'react'
import { connection } from 'next/server'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { TableListContent } from '@widgets/table/table-list-content'
import { TableSnapshotListItem } from '@widgets/table/table-snapshot-list-item'
import { SITE_URL } from '@shared/constants/site'
import { compose } from '@server/compose'

export const TableCatalogContent = async () => {
    await connection()
    const defaultTables = await compose().tableSnapshotService.list()

    return (
        <>
            <p className='text-muted-foreground bg-card px-3 py-2 text-xs leading-5 break-keep'>
                표 주소를 R-BMS 의 난이도표 설정에 넣으면 선곡 화면에 레벨별 폴더로 나옵니다. 기본 표는 R-BMS 를 처음 실행할 때 등록됩니다.
            </p>
            <section aria-labelledby='bundled-tables-heading' className='bg-card flex flex-col'>
                <h2 id='bundled-tables-heading' className='rbms-micro-label flex h-8 items-center px-3'>
                    기본 표
                </h2>
                <ul className='bg-border flex flex-col gap-px border-t'>
                    {defaultTables.map((table) => (
                        <TableSnapshotListItem key={table.id} table={table} tableUrl={`${SITE_URL}${table.standard_paths.page}`} />
                    ))}
                </ul>
            </section>
            <section aria-labelledby='server-tables-heading' className='bg-card flex flex-col'>
                <h2 id='server-tables-heading' className='rbms-micro-label flex h-8 items-center px-3'>
                    서버 표
                </h2>
                <Suspense fallback={<RowListSkeleton rows={1} label='서버 표 불러오는 중' />}>
                    <TableListContent />
                </Suspense>
            </section>
        </>
    )
}
