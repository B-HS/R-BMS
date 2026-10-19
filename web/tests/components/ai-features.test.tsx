import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { changeValue, click, keyDown, mount, page, submit, unmountAll } = await import('../support/dom-harness')
const { formatDateTime } = await import('../../src/shared/lib/format')
const { AiCredentialStatus } = await import('../../src/features/ai/ai-credential-status')
const { AiKeyForm } = await import('../../src/features/ai/ai-key-form')
const { AiLimitNumberField } = await import('../../src/features/ai/ai-limit-number-field')
const { AiModelSelect } = await import('../../src/features/ai/ai-model-select')
const { AiProviderNotice } = await import('../../src/features/ai/ai-provider-notice')
const { AiSetupNotice } = await import('../../src/features/ai/ai-setup-notice')
const { AiTokenUsageLine } = await import('../../src/features/ai/ai-token-usage-line')
const { AiUsageMeter } = await import('../../src/features/ai/ai-usage-meter')

type Credential = import('../../src/entities/ai.type').AiCredential

const NOW = 1_780_000_000_000
const FAKE_KEY = 'sk-test-0123456789-ABCDwxyz'
const VALIDATED_AFTER_CREATION_MS = 18_000_000

const credentialRow = (overrides: Partial<Credential> = {}): Credential => ({
    provider: 'anthropic',
    key_hint: 'f00d',
    model: null,
    created_at: NOW,
    last_validated_at: null,
    last_error_kind: null,
    ...overrides,
})

afterEach(() => unmountAll())

describe('AiKeyForm', () => {
    const renderForm = async (props: Partial<Parameters<typeof AiKeyForm>[0]> = {}) => {
        const submitted: string[] = []
        await mount(
            <AiKeyForm
                inputId='key'
                label='API 키'
                submitLabel='등록'
                isPending={false}
                onSubmitKey={(apiKey) => submitted.push(apiKey)}
                {...props}
            />,
        )
        return { submitted, input: page.getByLabelText('API 키') as HTMLInputElement }
    }

    test('비밀번호 입력란이고 자동완성과 맞춤법 검사를 끈다', async () => {
        const { input } = await renderForm()

        expect([input.type, input.autocomplete, input.getAttribute('spellcheck'), input.required]).toEqual(['password', 'off', 'false', true])
        expect([input.minLength, input.maxLength]).toEqual([8, 512])
    })

    test('제출하면 앞뒤 공백을 뺀 키를 한 번 넘기고 입력란을 바로 비운다', async () => {
        const { submitted, input } = await renderForm()

        input.value = `  ${FAKE_KEY}\n`
        await submit(input.closest('form') as HTMLFormElement)

        expect(submitted).toEqual([FAKE_KEY])
        expect(input.value).toBe('')
    })

    test('키를 React 상태에 담지 않으므로 입력해도 다시 그리지 않고 DOM 값만 쓴다', async () => {
        const { input } = await renderForm()

        input.value = FAKE_KEY
        await changeValue(input, FAKE_KEY)

        expect(document.body.innerHTML).not.toContain(FAKE_KEY)
    })

    test('너무 짧거나 비어 있으면 넘기지 않고 입력값을 그대로 둔다', async () => {
        const { submitted, input } = await renderForm()

        input.value = 'short'
        await submit(input.closest('form') as HTMLFormElement)
        input.value = '   '
        await submit(input.closest('form') as HTMLFormElement)

        expect(submitted).toEqual([])
    })

    test('요청 중이면 입력란과 제출 버튼이 꺼지고, 취소 버튼은 있을 때만 나온다', async () => {
        const { input } = await renderForm({ isPending: true })

        expect(input.disabled).toBe(true)
        expect((page.getByRole('button', { name: '등록' }) as HTMLButtonElement).disabled).toBe(true)
        expect(page.queryByRole('button', { name: '취소' })).toBeNull()
    })

    test('취소 버튼은 onCancel 을 부른다', async () => {
        let cancelled = 0
        await renderForm({ onCancel: () => (cancelled += 1) })

        await click(page.getByRole('button', { name: '취소' }))

        expect(cancelled).toBe(1)
    })
})

describe('AiCredentialStatus', () => {
    test('키는 끝 4자만 마스크 뒤에 보인다', async () => {
        await mount(<AiCredentialStatus credential={credentialRow()} />)

        expect(page.getByText('••••f00d')).toBeTruthy()
        expect(page.getByText('검증 전')).toBeTruthy()
    })

    test('검증 성공 시각이 있으면 시각을 보이고 실패가 기록되어 있으면 실패 종류만 보인다', async () => {
        await mount(<AiCredentialStatus credential={credentialRow({ last_validated_at: NOW + VALIDATED_AFTER_CREATION_MS })} />)
        expect(page.queryByText('검증 전')).toBeNull()
        expect(page.getByText(formatDateTime(NOW + VALIDATED_AFTER_CREATION_MS))).toBeTruthy()
        await unmountAll()

        await mount(<AiCredentialStatus credential={credentialRow({ last_error_kind: 'quota' })} />)
        expect(page.getByText('잔액 또는 지출 한도 도달')).toBeTruthy()
        expect(page.queryByText('검증 전')).toBeNull()
    })
})

describe('AiModelSelect', () => {
    const models = [
        { id: 'plain', label: 'plain', structured: true },
        { id: 'unknown', label: 'unknown', structured: null },
        { id: 'blocked', label: 'blocked', structured: false },
    ]

    test('형식 지원 여부에 따라 이름 뒤에 표시를 붙이고 미지원 모델은 고를 수 없다', async () => {
        await mount(<AiModelSelect models={models} value='plain' onValueChange={() => undefined} />)
        await keyDown(page.getByRole('combobox', { name: '모델' }), 'ArrowDown')

        expect(await page.findByRole('option', { name: 'plain' })).toBeTruthy()
        expect(page.getByRole('option', { name: 'unknown · 형식 지원 확인 안 됨' })).toBeTruthy()
        expect(page.getByRole('option', { name: 'blocked · 형식 미지원' }).getAttribute('aria-disabled')).toBe('true')
    })

    test('저장된 모델이 목록에 없어도 선택지에 남겨 현재 값을 잃지 않는다', async () => {
        await mount(<AiModelSelect models={models} value='retired-model' onValueChange={() => undefined} />)
        await keyDown(page.getByRole('combobox', { name: '모델' }), 'ArrowDown')

        expect(await page.findByRole('option', { name: /retired-model/ })).toBeTruthy()
    })

    test('모델을 고르면 그 id 를 넘긴다', async () => {
        const picked: string[] = []
        await mount(<AiModelSelect models={models} value={null} onValueChange={(model) => picked.push(model)} />)
        await keyDown(page.getByRole('combobox', { name: '모델' }), 'ArrowDown')

        await click(await page.findByRole('option', { name: 'plain' }))

        expect(picked).toEqual(['plain'])
    })
})

describe('AiUsageMeter, AiTokenUsageLine', () => {
    test('한도가 없으면 사용량만 쓰고 제한 없음을 밝히며 사용률 막대는 그리지 않는다', async () => {
        await mount(<AiUsageMeter label='이번 달 토큰' used={1234} limit={null} unit='토큰' />)

        expect(page.getByText('1,234 토큰 · 제한 없음')).toBeTruthy()
        expect(page.queryByLabelText('이번 달 토큰 사용률')).toBeNull()
    })

    test('한도의 절반이면 막대 50% 이고 경고 색이 없다', async () => {
        await mount(<AiUsageMeter label='오늘 호출' used={15} limit={30} unit='회' />)

        expect(page.getByLabelText('오늘 호출 사용률').getAttribute('aria-valuenow')).toBe('50')
        expect(page.getByText('15 / 30 회').className).not.toContain('text-warning')
    })

    test('한도를 넘기면 막대는 100% 에서 멈추고 경고 색을 쓴다', async () => {
        await mount(<AiUsageMeter label='오늘 호출' used={45} limit={30} unit='회' />)

        expect(page.getByLabelText('오늘 호출 사용률').getAttribute('aria-valuenow')).toBe('100')
        expect(page.getByText('45 / 30 회').className).toContain('text-warning')
    })

    test('토큰 사용량 줄은 입력과 출력, 캐시를 천 단위로 쓴다', async () => {
        await mount(<AiTokenUsageLine inputTokens={12_345} outputTokens={678} cachedInputTokens={9_000} />)

        expect(page.getByText('입력 12,345 · 출력 678 토큰(캐시 9,000)')).toBeTruthy()
    })
})

describe('안내 컴포넌트', () => {
    test('고지 문구는 보관 방식, 취소 가능, 직접 요청 때만 호출, 지출 한도 권유를 모두 담는다', async () => {
        await mount(<AiProviderNotice />)

        const notice = page.getByRole('note')
        expect(notice.textContent).toContain('암호화해 보관하고 AI 호출에만 씁니다')
        expect(notice.textContent).toContain('언제든 취소할 수 있습니다')
        expect(notice.textContent).toContain('직접 요청할 때만')
        expect(notice.textContent).toContain('지출 한도를 걸어 두기를 권합니다')
    })

    test('설정 안내는 이유에 맞는 제목과 같은 설정 주소를 쓴다', async () => {
        await mount(<AiSetupNotice reason='no_key' />)
        expect(page.getByText('등록된 AI 키가 없습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: 'AI 키 등록하기' }).getAttribute('href')).toBe('/settings?tab=ai')
        await unmountAll()

        await mount(<AiSetupNotice reason='no_model' />)
        expect(page.getByText('사용할 모델을 고르지 않았습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: '모델 고르기' }).getAttribute('href')).toBe('/settings?tab=ai')
    })

    test('한도 입력란은 오류가 있을 때만 경고를 붙이고 설명과 연결한다', async () => {
        await mount(
            <AiLimitNumberField id='limit' label='한도' value='abc' onValueChange={() => undefined} description='설명 문장' error='범위 오류' />,
        )

        const input = page.getByLabelText('한도') as HTMLInputElement
        expect([input.getAttribute('aria-invalid'), input.getAttribute('aria-describedby'), input.inputMode]).toEqual([
            'true',
            'limit-description',
            'numeric',
        ])
        expect(page.getByText('범위 오류')).toBeTruthy()
        expect(page.getByText('설명 문장').id).toBe('limit-description')
        await unmountAll()

        await mount(<AiLimitNumberField id='limit' label='한도' value='5' onValueChange={() => undefined} description='설명 문장' />)
        expect(page.getByLabelText('한도').getAttribute('aria-invalid')).toBe('false')
        expect(page.queryByText('범위 오류')).toBeNull()
    })
})
