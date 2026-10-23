import { connection } from 'next/server'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { CourseCatalogPanel } from '@widgets/course/course-catalog-panel'
import { courseCatalogQueryOptions } from '@entities/course.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readCourseCatalog } from '@server/service/public-reads'

export const CourseCatalogContent = async () => {
    await connection()
    const queryClient = getQueryClient()
    await queryClient.prefetchQuery({ ...courseCatalogQueryOptions(), queryFn: readCourseCatalog })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <CourseCatalogPanel />
        </HydrationBoundary>
    )
}
