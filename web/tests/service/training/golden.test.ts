import { describe, expect, test } from 'bun:test'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { z } from 'zod'
import { chartPatternDocumentSchema } from '@server/dto/chart-pattern'
import { writeTrainingBms } from '@server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '@server/service/domain/training/pattern-document'
import { trainingSpecSchema } from '@server/service/domain/training/spec'
import { synthesizeTraining } from '@server/service/domain/training/synthesize'
import { createTrainingZip } from '@server/service/domain/training/zip'

/**
 * The golden training charts of `contract/training-chart/`: a hand-written spec, the chart bytes this generator writes from it, and
 * the pattern document the client's own parser reads from those bytes (`training_contract_tests.rs` in rbms-cli writes and checks
 * it). With `RBMS_UPDATE_CONTRACT=1` the spec's filled fields, the `.bms` files and the stored zip are written again instead of
 * compared; the `.pattern.json` files then come from `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-cli --locked training_contract`.
 */
const GOLDEN_DIR = join(import.meta.dirname, '..', '..', '..', '..', 'contract', 'training-chart')
const GOLDEN_NAMES = ['basic-160', 'soflan-ln', 'scratch-mix'] as const
const ZIP_GOLDEN = 'basic-160'
const UPDATING = process.env.RBMS_UPDATE_CONTRACT === '1'

const goldenSpecSchema = z.object({ id: z.string(), spec: trainingSpecSchema })

const pathOf = (name: string, extension: string) => join(GOLDEN_DIR, `${name}.${extension}`)

const readGolden = (name: string) => {
    const golden = goldenSpecSchema.parse(JSON.parse(readFileSync(pathOf(name, 'spec.json'), 'utf8')))
    const { spec, notes } = synthesizeTraining(golden.spec)
    const written = writeTrainingBms(spec, notes)
    return { golden, spec, written }
}

describe('훈련곡 골든 픽스처 (contract/training-chart)', () => {
    for (const name of GOLDEN_NAMES) {
        test(`${name}: 사양을 합성·작성한 바이트가 .bms 와 같고 채운 필드가 사양과 같다`, () => {
            const { golden, spec, written } = readGolden(name)
            if (UPDATING) {
                writeFileSync(pathOf(name, 'spec.json'), `${JSON.stringify({ id: golden.id, spec }, null, 4)}\n`)
                writeFileSync(pathOf(name, 'bms'), written.bytes)
                return
            }
            expect(spec).toEqual(golden.spec)
            expect(Buffer.from(written.bytes).equals(readFileSync(pathOf(name, 'bms')))).toBe(true)
        })

        test(`${name}: TS 가 계산한 패턴 문서가 클라이언트 파서가 읽은 .pattern.json 과 같다`, () => {
            const { spec, written } = readGolden(name)
            const document = buildTrainingPatternDocument({ spec, lines: written.lines, md5: written.md5, sha256: written.sha256 })
            const parsed = chartPatternDocumentSchema.parse(JSON.parse(readFileSync(pathOf(name, 'pattern.json'), 'utf8')))
            expect(document).toEqual(parsed)
            expect(document.md5).toBe(written.md5)
            expect(document.mode).toBe('BEAT_7K')
        })
    }

    test(`${ZIP_GOLDEN}.zip: 저장 방식 zip 바이트가 골든과 같다`, () => {
        const { golden, written } = readGolden(ZIP_GOLDEN)
        const zip = createTrainingZip({ id: golden.id, bms: written.bytes, bmsMethod: 'store' })
        if (UPDATING) {
            writeFileSync(pathOf(ZIP_GOLDEN, 'zip'), zip)
            return
        }
        expect(Buffer.from(zip).equals(readFileSync(pathOf(ZIP_GOLDEN, 'zip')))).toBe(true)
    })
})
