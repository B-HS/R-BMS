'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { FormDialog } from '@features/dialog/form-dialog'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { Switch } from '@shared/ui/switch'
import { formatDateTime, formatHash } from '@shared/lib/format'
import { useClientBuildList, useRegisterClientBuild } from '@entities/build.query'
import type { ClientBuild, ClientBuildInput } from '@entities/build.type'

const SHA256_LENGTH = 64
const NOTE_MAX = 255
const EMPTY_BUILD: ClientBuildInput = { sha256: '', version: '', platform: '', channel: 'stable', trusted: true, note: '' }

export const BuildListPanel: FC = () => {
    const [open, setOpen] = useState(false)
    const [draft, setDraft] = useState(EMPTY_BUILD)

    const t = useTranslations('browse.admin.builds')
    const { data, isLoading, isError, isFetching, refetch } = useClientBuildList()
    const registerBuild = useRegisterClientBuild()

    const rows = data ?? []
    const isDraftValid = draft.sha256.length === SHA256_LENGTH && draft.version !== '' && draft.platform !== ''

    const submit = () => {
        registerBuild.mutate(
            { ...draft, released_at: Date.now() },
            {
                onSuccess: () => {
                    setDraft(EMPTY_BUILD)
                    setOpen(false)
                    toast.success(t('registered'))
                },
            },
        )
    }

    const toggleTrusted = (row: ClientBuild) => {
        registerBuild.mutate(
            { ...row, trusted: !row.trusted },
            { onSuccess: () => toast.success(row.trusted ? t('markedUntrusted') : t('markedTrusted')) },
        )
    }

    const columns: DataTableColumn<ClientBuild>[] = [
        { key: 'sha256', label: 'sha256', width: 128, mono: true, cell: (row) => formatHash(row.sha256) },
        { key: 'version', label: t('version'), flex: true, cell: (row) => row.version },
        { key: 'platform', label: t('platform'), width: 160, cell: (row) => row.platform },
        { key: 'channel', label: t('channel'), width: 96, cell: (row) => <MetaBadge>{row.channel}</MetaBadge> },
        { key: 'trusted', label: t('trust'), width: 80, cell: (row) => (row.trusted ? 'trusted' : 'untrusted') },
        { key: 'note', label: t('note'), width: 160, cell: (row) => row.note || '—' },
        {
            key: 'released_at',
            label: t('releasedAt'),
            width: 144,
            align: 'right',
            cell: (row) => (row.released_at ? formatDateTime(row.released_at) : '—'),
        },
        {
            key: 'action',
            label: '',
            width: 104,
            align: 'right',
            cell: (row) => (
                <Button variant='ghost' size='sm' disabled={registerBuild.isPending} onClick={() => toggleTrusted(row)}>
                    {row.trusted ? t('revokeTrust') : t('grantTrust')}
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard
                title={t('title')}
                action={
                    <div className='flex items-center gap-2'>
                        <Link href='/settings' className='text-muted-foreground text-xs underline'>
                            {t('toSettings')}
                        </Link>
                        <Button size='sm' onClick={() => setOpen(true)}>
                            {t('register')}
                        </Button>
                    </div>
                }
                contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton />}
                {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
                {!isLoading && !isError && rows.length === 0 && <EmptyState title={t('empty')} />}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.sha256} rowMuted={(row) => !row.trusted} caption={t('title')} />
                )}
                <PanelFootnote>{t('footnote')}</PanelFootnote>
            </PanelCard>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title={t('dialogTitle')}
                description={t('dialogDescription')}
                submitLabel={t('submit')}
                submitDisabled={!isDraftValid || registerBuild.isPending}
                onSubmit={submit}>
                <Field>
                    <FieldLabel htmlFor='build-sha256'>sha256</FieldLabel>
                    <Input
                        id='build-sha256'
                        maxLength={SHA256_LENGTH}
                        value={draft.sha256}
                        onChange={(event) => setDraft({ ...draft, sha256: event.target.value.trim() })}
                    />
                    <FieldDescription>{t('sha256Hint')}</FieldDescription>
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-version'>{t('version')}</FieldLabel>
                    <Input id='build-version' value={draft.version} onChange={(event) => setDraft({ ...draft, version: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-platform'>{t('platform')}</FieldLabel>
                    <Input id='build-platform' value={draft.platform} onChange={(event) => setDraft({ ...draft, platform: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-channel'>{t('channel')}</FieldLabel>
                    <Input id='build-channel' value={draft.channel} onChange={(event) => setDraft({ ...draft, channel: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-note'>{t('note')}</FieldLabel>
                    <Input
                        id='build-note'
                        maxLength={NOTE_MAX}
                        value={draft.note ?? ''}
                        onChange={(event) => setDraft({ ...draft, note: event.target.value })}
                    />
                </Field>
                <Field orientation='horizontal'>
                    <FieldLabel htmlFor='build-trusted'>trusted</FieldLabel>
                    <Switch id='build-trusted' checked={draft.trusted} onCheckedChange={(checked) => setDraft({ ...draft, trusted: checked })} />
                    <FieldDescription>{t('trustedHint')}</FieldDescription>
                </Field>
            </FormDialog>
        </>
    )
}
