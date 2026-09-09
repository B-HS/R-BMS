'use client'

import { useState, type FC } from 'react'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { FormDialog } from '@features/dialog/form-dialog'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { formatDateTime } from '@shared/lib/format'
import { useApiTokenList, useIssueApiToken, useRevokeApiToken } from '@entities/token.query'
import type { ApiTokenRow } from '@entities/token.type'

const TOKEN_LABEL_MAX = 40

export const TokenPanel: FC = () => {
    const [label, setLabel] = useState('')
    const [issueOpen, setIssueOpen] = useState(false)
    const [revokeTarget, setRevokeTarget] = useState<ApiTokenRow | null>(null)
    const [issuedToken, setIssuedToken] = useState('')

    const { data, isLoading, isError } = useApiTokenList()
    const issue = useIssueApiToken()
    const revoke = useRevokeApiToken()

    const rows = data ?? []

    const submitIssue = () => {
        issue.mutate(
            { label },
            {
                onSuccess: (created) => {
                    setIssuedToken(created.token)
                    setIssueOpen(false)
                    setLabel('')
                    toast.success('토큰을 발급했습니다. 이 화면을 벗어나면 다시 볼 수 없습니다.')
                },
            },
        )
    }

    const submitRevoke = () => {
        if (!revokeTarget) return
        revoke.mutate(revokeTarget.id, {
            onSuccess: () => {
                setRevokeTarget(null)
                toast.success('토큰을 폐기했습니다.')
            },
        })
    }

    const columns: DataTableColumn<ApiTokenRow>[] = [
        { key: 'label', label: '라벨', flex: true, cell: (row) => row.label || '(이름 없음)' },
        { key: 'id', label: '토큰 ID', width: 160, mono: true, cell: (row) => row.id },
        { key: 'created_at', label: '발급', width: 144, align: 'right', cell: (row) => formatDateTime(row.created_at) },
        {
            key: 'last_used_at',
            label: '마지막 사용',
            width: 144,
            align: 'right',
            cell: (row) => (row.last_used_at ? formatDateTime(row.last_used_at) : '—'),
        },
        {
            key: 'action',
            label: '',
            width: 72,
            align: 'right',
            cell: (row) => (
                <Button variant='ghost' size='sm' onClick={() => setRevokeTarget(row)}>
                    폐기
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard
                title='API 토큰'
                action={
                    <Button size='sm' onClick={() => setIssueOpen(true)}>
                        토큰 발급
                    </Button>
                }
                contentClassName='flex flex-col gap-2'>
                {issuedToken && (
                    <div className='bg-muted flex flex-col gap-1 p-2'>
                        <p className='text-xs font-medium'>발급된 토큰 (한 번만 표시됩니다)</p>
                        <code className='font-mono text-xs break-all'>{issuedToken}</code>
                        <Button variant='ghost' size='sm' className='self-start' onClick={() => setIssuedToken('')}>
                            숨기기
                        </Button>
                    </div>
                )}
                {isLoading && <TableSkeleton rows={3} />}
                {!isLoading && isError && <LoadErrorState title='토큰 목록을 불러오지 못했습니다' />}
                {!isLoading && !isError && rows.length === 0 && (
                    <EmptyState title='발급한 토큰이 없습니다' description='네이티브 클라이언트 연동에 토큰이 필요합니다.' />
                )}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption='발급한 API 토큰 목록' />
                )}
                <PanelFootnote>목록에는 유효한 토큰만 남습니다. 평문은 발급 직후 1회만 표시되므로 분실하면 새로 발급하세요.</PanelFootnote>
            </PanelCard>
            <FormDialog
                open={issueOpen}
                onOpenChange={setIssueOpen}
                title='API 토큰 발급'
                description='네이티브 클라이언트가 Bearer 인증에 사용할 토큰을 만듭니다.'
                submitLabel='발급'
                submitDisabled={issue.isPending}
                onSubmit={submitIssue}>
                <Field>
                    <FieldLabel htmlFor='token-label'>라벨</FieldLabel>
                    <Input id='token-label' maxLength={TOKEN_LABEL_MAX} value={label} onChange={(event) => setLabel(event.target.value)} />
                    <FieldDescription>기기 이름처럼 나중에 구분할 수 있는 이름을 적습니다.</FieldDescription>
                </Field>
            </FormDialog>
            <ConfirmAction
                open={revokeTarget !== null}
                onOpenChange={(open) => !open && setRevokeTarget(null)}
                title='토큰을 폐기할까요?'
                description={`${revokeTarget?.label || revokeTarget?.id} 토큰으로는 더 이상 제출할 수 없습니다.`}
                confirmLabel='폐기'
                confirmDisabled={revoke.isPending}
                onConfirm={submitRevoke}
            />
        </>
    )
}
