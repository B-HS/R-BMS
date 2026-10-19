import { describe, expect, test } from 'bun:test'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { z } from 'zod'

const WEB_DIR = join(import.meta.dirname, '..', '..')
const WORKFLOW_PATH = join(WEB_DIR, '..', '.github', 'workflows', 'web-db.yml')
const SCRIPTS_DIR = join(WEB_DIR, 'scripts')
const CHART_NOTES_SCRIPT = /^chart-notes\.(.+)\.sql$/
const RUN_SQL = /^bun scripts\/run-sql\.ts (\S+\.sql)$/

const workflowSchema = z.object({
    on: z.object({ workflow_dispatch: z.object({ inputs: z.object({ action: z.object({ options: z.array(z.string()) }) }) }) }),
    jobs: z.object({ run: z.object({ steps: z.array(z.object({ if: z.string().optional(), run: z.string().optional() })) }) }),
})

const workflow = workflowSchema.parse(Bun.YAML.parse(readFileSync(WORKFLOW_PATH, 'utf8')))
const actions = workflow.on.workflow_dispatch.inputs.action.options
const sqlSteps = workflow.jobs.run.steps.flatMap((step) => {
    const script = step.run?.trim().match(RUN_SQL)?.[1]
    return script ? [{ condition: step.if ?? '', script }] : []
})

const chartNotesScripts = readdirSync(SCRIPTS_DIR).flatMap((file) => {
    const kind = file.match(CHART_NOTES_SCRIPT)?.[1]
    return kind ? [{ file, action: `chart-notes-${kind}` }] : []
})

describe('운영 DB workflow 의 차트 노트 수 동작', () => {
    test('chart-notes SQL 파일마다 같은 이름의 동작이 있고, 그 동작만 그 파일을 돌린다', () => {
        expect(chartNotesScripts.map((script) => script.action).toSorted()).toEqual([
            'chart-notes-count',
            'chart-notes-drop-backup',
            'chart-notes-repair',
            'chart-notes-restore',
        ])
        for (const { file, action } of chartNotesScripts) {
            expect(actions).toContain(action)
            if (action === 'chart-notes-count') continue
            expect(sqlSteps).toContainEqual({ condition: `inputs.action == '${action}'`, script: file })
        }
    })

    test('workflow 가 돌리는 SQL 파일은 모두 scripts 에 있다', () => {
        const files = new Set(readdirSync(SCRIPTS_DIR))
        for (const step of sqlSteps) expect(files.has(step.script)).toBe(true)
    })

    test('관리자 지정 사전 확인 동작은 세기 SQL 만 돌린다', () => {
        expect(actions).toContain('admin-seed-count')
        expect(sqlSteps.filter((step) => step.script === 'admin-seed.count.sql')).toEqual([
            { condition: "inputs.action == 'admin-seed-count'", script: 'admin-seed.count.sql' },
        ])
        expect(sqlSteps.filter((step) => step.condition.includes('admin-seed'))).toHaveLength(1)
    })

    test('바꾸는 동작은 앞뒤로 세기를 돌린다', () => {
        const counts = sqlSteps.filter((step) => step.script === 'chart-notes.count.sql')
        expect(counts.map((step) => step.condition)).toEqual([
            "startsWith(inputs.action, 'chart-notes-')",
            "startsWith(inputs.action, 'chart-notes-') && inputs.action != 'chart-notes-count'",
        ])
    })
})
