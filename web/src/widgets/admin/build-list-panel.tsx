'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
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

    const { data, isLoading, isError } = useClientBuildList()
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
                    toast.success('빌드를 등록했습니다.')
                },
            },
        )
    }

    const toggleTrusted = (row: ClientBuild) => {
        registerBuild.mutate(
            { ...row, trusted: !row.trusted },
            { onSuccess: () => toast.success(row.trusted ? '빌드를 untrusted 로 바꿨습니다.' : '빌드를 trusted 로 바꿨습니다.') },
        )
    }

    const columns: DataTableColumn<ClientBuild>[] = [
        { key: 'sha256', label: 'sha256', width: 128, mono: true, cell: (row) => formatHash(row.sha256) },
        { key: 'version', label: '버전', flex: true, cell: (row) => row.version },
        { key: 'platform', label: '플랫폼', width: 160, cell: (row) => row.platform },
        { key: 'channel', label: '채널', width: 96, cell: (row) => <MetaBadge>{row.channel}</MetaBadge> },
        { key: 'trusted', label: '신뢰', width: 80, cell: (row) => (row.trusted ? 'trusted' : 'untrusted') },
        { key: 'note', label: '메모', width: 160, cell: (row) => row.note || '—' },
        { key: 'released_at', label: '등록', width: 144, align: 'right', cell: (row) => (row.released_at ? formatDateTime(row.released_at) : '—') },
        {
            key: 'action',
            label: '',
            width: 104,
            align: 'right',
            cell: (row) => (
                <Button variant='ghost' size='sm' disabled={registerBuild.isPending} onClick={() => toggleTrusted(row)}>
                    {row.trusted ? '신뢰 해제' : '신뢰 부여'}
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard
                title='클라이언트 빌드 allowlist'
                action={
                    <div className='flex items-center gap-2'>
                        <Link href='/settings' className='text-muted-foreground text-xs underline'>
                            설정으로
                        </Link>
                        <Button size='sm' onClick={() => setOpen(true)}>
                            빌드 등록
                        </Button>
                    </div>
                }
                contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton />}
                {!isLoading && isError && <LoadErrorState title='빌드 목록을 불러오지 못했습니다' />}
                {!isLoading && !isError && rows.length === 0 && <EmptyState title='등록된 빌드가 없습니다' />}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable
                        columns={columns}
                        rows={rows}
                        rowKey={(row) => row.sha256}
                        rowMuted={(row) => !row.trusted}
                        caption='클라이언트 빌드 allowlist'
                    />
                )}
                <PanelFootnote>allowlist 에 없거나 untrusted 인 빌드가 제출하면 UNKNOWN_BUILD 플래그가 붙고 unranked 처리됩니다.</PanelFootnote>
            </PanelCard>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title='클라이언트 빌드 등록'
                description='릴리스 바이너리의 sha256 을 allowlist 에 추가합니다.'
                submitLabel='등록'
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
                    <FieldDescription>64자 hex 문자열입니다.</FieldDescription>
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-version'>버전</FieldLabel>
                    <Input id='build-version' value={draft.version} onChange={(event) => setDraft({ ...draft, version: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-platform'>플랫폼</FieldLabel>
                    <Input id='build-platform' value={draft.platform} onChange={(event) => setDraft({ ...draft, platform: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-channel'>채널</FieldLabel>
                    <Input id='build-channel' value={draft.channel} onChange={(event) => setDraft({ ...draft, channel: event.target.value })} />
                </Field>
                <Field>
                    <FieldLabel htmlFor='build-note'>메모</FieldLabel>
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
                    <FieldDescription>끄면 이 빌드의 제출이 unranked 로 기록됩니다.</FieldDescription>
                </Field>
            </FormDialog>
        </>
    )
}
