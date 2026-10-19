'use client'

import type { FC } from 'react'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Switch } from '@shared/ui/switch'
import { useAiShareSetting, useSaveAiShareSetting } from '@entities/ai.query'

const SWITCH_ID = 'rival-segment-sharing'

/**
 * Whether players who are rivals of mine and keep me as a rival too may compare their play with mine segment by segment (ADR Q4, W5
 * decision: a one-sided registration gives nothing). Off until turned on: they send only my plays' segment totals (EX, BP and object
 * counts per segment, never notes, timing or lanes) to their own AI.
 */
export const RivalSharingPanel: FC = () => {
    const setting = useAiShareSetting()
    const save = useSaveAiShareSetting()

    const isOn = setting.data?.rival_segments ?? false

    const handleChange = (next: boolean) =>
        save.mutate(next, { onSuccess: () => toast.success(next ? '라이벌에게 구간 요약을 허용했습니다.' : '구간 요약 공유를 껐습니다.') })

    return (
        <PanelCard title='라이벌에게 구간 요약 허용'>
            {setting.isPending && <PanelSkeleton label='공유 설정 불러오는 중' />}
            {setting.isError && (
                <LoadErrorState title='공유 설정을 불러오지 못했습니다' isRetrying={setting.isFetching} onRetry={() => void setting.refetch()} />
            )}
            {setting.data && (
                <Field orientation='horizontal'>
                    <Switch id={SWITCH_ID} checked={isOn} disabled={save.isPending} onCheckedChange={handleChange} />
                    <div className='flex flex-col gap-1'>
                        <FieldLabel htmlFor={SWITCH_ID}>서로 라이벌인 플레이어에게만 내 판과 구간별로 비교하도록 허용</FieldLabel>
                        <FieldDescription className='break-keep'>
                            켜면 내가 라이벌로 두고 그 플레이어도 나를 라이벌로 둔 경우에만, 같은 곡의 내 판 하나에서 구간마다의 EX·BP·노트 수를 받아
                            자기 AI 키로 해석할 수 있습니다. 한쪽만 등록한 사이에는 보내지 않습니다. 노트별 판정·타이밍·레인은 보내지 않습니다. 끄면
                            공개된 판정 합계만 비교됩니다. 기본은 꺼짐입니다.
                        </FieldDescription>
                    </div>
                </Field>
            )}
        </PanelCard>
    )
}
