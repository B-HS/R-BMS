import { clearLampToId, gaugeTypeFromId, NO_ASSIST_GRADE, type AssistGrade, type GaugeType } from '@server/dto/common'

export const LIGHT_ASSIST_LAMP_ID = clearLampToId('LightAssistEasy')

/**
 * The grade of the lamp a gauge gives on a clear without any demotion, with the full-combo overwrite ignored: the EASY lamp is 1,
 * NORMAL (the groove gauge) 2, HARD 3, and EX-HARD or above, HAZARD included, 4. The course gauges give the lamp of their
 * kind: CLASS the NORMAL one, EXCLASS the HARD one, EXHARDCLASS the EX-HARD one. The ASSIST EASY gauge gives LIGHT ASSIST itself,
 * so it has no grade.
 */
export const ASSIST_GRADE_OF_GAUGE = {
    AssistEasy: 0,
    Easy: 1,
    Normal: 2,
    Hard: 3,
    ExHard: 4,
    Hazard: 4,
    Class: 2,
    ExClass: 3,
    ExHardClass: 4,
} as const satisfies Record<GaugeType, AssistGrade>

/**
 * The assist grade of a run, from the lamp id it was stored with and the id of the gauge it ended on.
 *
 * A run has a grade when its lamp is LIGHT ASSIST on a gauge other than ASSIST EASY: the client only demotes a clear to that
 * lamp for SCRATCH AUTO, so such a run is the clear that gauge would have given without the demotion. Every other run is 0:
 * any other lamp (a widened judge or an own TOTAL gives ASSIST, which has no grade), a clear on the ASSIST EASY gauge, and a
 * gauge id this server does not know (a row from before the gauge was kept). A rule version is not an input: a run from before
 * the client demoted assisted clears stored the lamp of its gauge, so it never has LIGHT ASSIST on a stronger gauge.
 */
export const deriveAssistGrade = (run: { clear: number; gauge: number }): AssistGrade =>
    run.clear === LIGHT_ASSIST_LAMP_ID ? ASSIST_GRADE_OF_GAUGE[gaugeTypeFromId(run.gauge)] : NO_ASSIST_GRADE
