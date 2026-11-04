'use client'

import type { FC } from 'react'
import { HomeBranchSkeleton } from '@widgets/home/home-branch-skeleton'
import { MemberAiStatusPanel } from '@widgets/home/member-ai-status-panel'
import { MemberCalendarPanel } from '@widgets/home/member-calendar-panel'
import { MemberCommunityPanel } from '@widgets/home/member-community-panel'
import { MemberDanCoursePanel } from '@widgets/home/member-dan-course-panel'
import { MemberFolderReplayPanel } from '@widgets/home/member-folder-replay-panel'
import { MemberGoalPanel } from '@widgets/home/member-goal-panel'
import { MemberLampPanel } from '@widgets/home/member-lamp-panel'
import { MemberProfilePanel } from '@widgets/home/member-profile-panel'
import { MemberRecentPanel } from '@widgets/home/member-recent-panel'
import { MemberRivalPanel } from '@widgets/home/member-rival-panel'
import { MemberSummaryPanel } from '@widgets/home/member-summary-panel'
import { MemberTableProgressPanel } from '@widgets/home/member-table-progress-panel'
import { MemberTrainingPanel } from '@widgets/home/member-training-panel'
import { MemberWeaknessPanel } from '@widgets/home/member-weakness-panel'
import { MemberWeekPanel } from '@widgets/home/member-week-panel'
import { useAiTrainingList } from '@entities/ai.query'
import { useAlignedViewer } from '@entities/dashboard.query'
import { DASHBOARD_MEMBER_GRID_CLASS_NAME } from '@shared/constants/dashboard'
import { useInView } from '@shared/hooks/use-in-view'

type MemberDashboardGridProps = {
    userId: string
    loginId: string
}

/**
 * The panels of a signed-in home in the order they are read. The grid packs them densely, so the widths the panels ask for
 * (one cell, wide, fill) have to add up without leaving a gap in the 3-column and the 4-column layout. The training chart list is
 * read once the grid is near the screen (the training panel shares it) so the recent plays can mark the plays of a training chart.
 */
export const MemberDashboardGrid: FC<MemberDashboardGridProps> = ({ userId, loginId }) => {
    const isAligned = useAlignedViewer(userId)
    const { ref, isInView } = useInView()
    const training = useAiTrainingList(isAligned && isInView)
    const trainingSha256s = new Set(training.data?.charts.map((chart) => chart.sha256))

    return (
        <div ref={ref} className={DASHBOARD_MEMBER_GRID_CLASS_NAME}>
            {isAligned ? (
                <>
                    <MemberProfilePanel loginId={loginId} />
                    <MemberSummaryPanel />
                    <MemberLampPanel loginId={loginId} />
                    <MemberTableProgressPanel />
                    <MemberCalendarPanel loginId={loginId} />
                    <MemberRecentPanel loginId={loginId} trainingSha256s={trainingSha256s} />
                    <MemberDanCoursePanel />
                    <MemberWeaknessPanel />
                    <MemberTrainingPanel />
                    <MemberWeekPanel />
                    <MemberGoalPanel />
                    <MemberRivalPanel />
                    <MemberFolderReplayPanel />
                    <MemberAiStatusPanel />
                    <MemberCommunityPanel />
                </>
            ) : (
                <HomeBranchSkeleton />
            )}
        </div>
    )
}
