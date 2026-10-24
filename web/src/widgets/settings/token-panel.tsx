'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
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
import { CopyButton } from '@shared/ui/copy-button'
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

    const t = useTranslations('settings.tokenPanel')
    const { data, isLoading, isError, isFetching, refetch } = useApiTokenList()
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
                    toast.success(t('issued'))
                },
            },
        )
    }

    const submitRevoke = () => {
        if (!revokeTarget) return
        revoke.mutate(revokeTarget.id, {
            onSuccess: () => {
                setRevokeTarget(null)
                toast.success(t('revoked'))
            },
        })
    }

    const columns: DataTableColumn<ApiTokenRow>[] = [
        { key: 'label', label: t('columnLabel'), flex: true, cell: (row) => row.label || t('unnamed') },
        { key: 'id', label: t('columnId'), width: 160, mono: true, cell: (row) => row.id },
        { key: 'created_at', label: t('columnIssuedAt'), width: 144, align: 'right', cell: (row) => formatDateTime(row.created_at) },
        {
            key: 'last_used_at',
            label: t('columnLastUsedAt'),
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
                    {t('revoke')}
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard
                title={t('title')}
                action={
                    <Button size='sm' onClick={() => setIssueOpen(true)}>
                        {t('issue')}
                    </Button>
                }
                contentClassName='flex flex-col gap-2'>
                {issuedToken && (
                    <div className='border-border flex flex-col gap-2 border p-2'>
                        <p className='text-xs font-medium'>{t('issuedTitle')}</p>
                        <code className='font-mono text-xs break-all'>{issuedToken}</code>
                        <p className='text-warning text-xs'>{t('issuedWarning')}</p>
                        <div className='flex gap-1'>
                            <CopyButton value={issuedToken} label={t('copy')} showLabel />
                            <Button variant='ghost' size='sm' onClick={() => setIssuedToken('')}>
                                {t('close')}
                            </Button>
                        </div>
                    </div>
                )}
                {isLoading && <TableSkeleton rows={3} />}
                {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
                {!isLoading && !isError && rows.length === 0 && (
                    <EmptyState
                        title={t('emptyTitle')}
                        description={t('emptyDescription')}
                        action={
                            <Button variant='outline' size='sm' onClick={() => setIssueOpen(true)}>
                                {t('issue')}
                            </Button>
                        }
                    />
                )}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption={t('caption')} />
                )}
                <PanelFootnote>{t('footnote')}</PanelFootnote>
            </PanelCard>
            <FormDialog
                open={issueOpen}
                onOpenChange={setIssueOpen}
                title={t('dialogTitle')}
                description={t('dialogDescription')}
                submitLabel={t('dialogSubmit')}
                submitDisabled={issue.isPending}
                onSubmit={submitIssue}>
                <Field>
                    <FieldLabel htmlFor='token-label'>{t('fieldLabel')}</FieldLabel>
                    <Input id='token-label' maxLength={TOKEN_LABEL_MAX} value={label} onChange={(event) => setLabel(event.target.value)} />
                    <FieldDescription>{t('fieldDescription')}</FieldDescription>
                </Field>
            </FormDialog>
            <ConfirmAction
                open={revokeTarget !== null}
                onOpenChange={(open) => !open && setRevokeTarget(null)}
                title={t('revokeTitle')}
                description={t('revokeDescription', { name: revokeTarget?.label || revokeTarget?.id || '' })}
                confirmLabel={t('revokeConfirm')}
                confirmDisabled={revoke.isPending}
                onConfirm={submitRevoke}
            />
        </>
    )
}
