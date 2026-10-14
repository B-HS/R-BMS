'use client'

import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'
import { ShellPage } from '@widgets/shell/shell-page'

const ErrorPage = ({ retry }: { error: Error & { digest?: string }; retry: () => void }) => (
    <ShellPage title='오류'>
        <EmptyState
            title='문제가 생겼습니다'
            description='잠시 후 다시 시도해 주세요.'
            className='min-h-64'
            action={
                <Button variant='outline' size='sm' onClick={() => retry()}>
                    다시 시도
                </Button>
            }
        />
    </ShellPage>
)

export default ErrorPage
