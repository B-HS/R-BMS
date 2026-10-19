import type { FC } from 'react'
import { Badge } from '@shared/ui/badge'
import { formatDateTime } from '@shared/lib/format'
import { AI_FAILURE_LABEL, type AiCredential } from '@entities/ai.type'

const KEY_MASK = '••••'

export const AiCredentialStatus: FC<{ credential: AiCredential }> = ({ credential }) => {
    const errorKind = credential.last_error_kind
    const isWarning = errorKind === 'forbidden'
    const hasValidation = (errorKind === null || isWarning) && credential.last_validated_at !== null

    return (
        <dl className='grid grid-cols-[auto_1fr] items-center gap-x-4 gap-y-1 text-xs'>
            <dt className='text-muted-foreground'>키</dt>
            <dd className='font-mono'>
                {KEY_MASK}
                {credential.key_hint}
            </dd>
            <dt className='text-muted-foreground'>등록</dt>
            <dd>{formatDateTime(credential.created_at)}</dd>
            <dt className='text-muted-foreground'>마지막 검증</dt>
            <dd className='flex flex-wrap items-center gap-2'>
                {errorKind && <Badge variant={isWarning ? 'outline' : 'destructive'}>{AI_FAILURE_LABEL[errorKind]}</Badge>}
                {hasValidation && credential.last_validated_at !== null && formatDateTime(credential.last_validated_at)}
                {errorKind === null && credential.last_validated_at === null && '검증 전'}
            </dd>
        </dl>
    )
}
