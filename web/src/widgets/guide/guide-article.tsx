import type { FC } from 'react'
import Link from 'next/link'
import { IR_SERVER_BASE_URL } from '@shared/constants/server-info'

const SECTIONS = [
    {
        title: '1. 서버 주소 지정',
        body: 'rbms-player 를 실행할 때 --server 플래그로 이 IR 의 API 베이스를 지정합니다. 경로 끝의 /api 까지 포함해야 합니다.',
        code: `rbms-player --server ${IR_SERVER_BASE_URL} --player <플레이어_ID>`,
    },
    {
        title: '2. 계정 만들기',
        body: '가입 화면에서 계정을 만들면 아이디가 그대로 IR 플레이어 ID 가 됩니다. 이미 계정이 있다면 로그인만 하면 됩니다.',
        code: null,
    },
    {
        title: '3. API 토큰 발급',
        body: '설정 화면의 API 토큰 탭에서 토큰을 발급합니다. 평문 토큰은 발급 직후 한 번만 표시되므로 그 자리에서 저장하세요. 이전에 발급한 토큰은 계속 유효하므로 필요 없으면 목록에서 폐기하세요.',
        code: 'Authorization: Bearer rbms_<발급받은_토큰>',
    },
    {
        title: '4. 제출 확인',
        body: '플레이를 마치면 클라이언트가 스코어를 제출합니다. 같은 기록을 다시 보내도 중복 생성되지 않고 기존 score_id 가 돌아옵니다.',
        code: null,
    },
    {
        title: '5. guest 제출',
        body: '서버가 guest 제출을 허용한 경우 토큰 없이도 제출할 수 있습니다. guest 기록은 항상 unranked 이고 랭킹과 개인 베스트에 반영되지 않습니다.',
        code: null,
    },
    {
        title: '6. unranked 가 되는 조건',
        body: 'autoplay · 스크래치 자동 · 어시스트 사용 · 판정폭 변경 · TOTAL 오버라이드 · 미등록 클라이언트 빌드 · guest 제출 중 하나라도 해당하면 unranked 로 기록되며 랭킹에서 제외됩니다.',
        code: null,
    },
]

export const GuideArticle: FC = () => (
    <>
        <header className='bg-background/60 border-border sticky top-0 z-50 flex h-12 items-center border-b px-4 backdrop-blur-xs'>
            <span className='font-mono text-sm'>rbms IR · 연동 가이드</span>
        </header>
        <main className='mx-auto flex w-full max-w-3xl flex-col gap-6 px-4 pt-6 pb-20'>
            <h1 className='text-3xl font-extrabold tracking-tight'>클라이언트 연동 가이드</h1>
            <p className='text-muted-foreground text-sm leading-7'>
                rbms-player 를 이 IR 서버에 연결해 스코어를 제출하고 랭킹을 조회하는 방법입니다.
            </p>
            {SECTIONS.map((section) => (
                <section key={section.title} className='flex flex-col gap-2'>
                    <h2 className='text-xl font-semibold tracking-tight'>{section.title}</h2>
                    <p className='text-muted-foreground text-sm leading-7'>{section.body}</p>
                    {section.code && <pre className='bg-muted text-foreground overflow-x-auto p-3 font-mono text-2xs lg:text-sm'>{section.code}</pre>}
                </section>
            ))}
            <nav aria-label='다음 단계' className='flex flex-wrap gap-4 text-sm'>
                <Link href='/signup' className='underline'>
                    계정 만들기
                </Link>
                <Link href='/login' className='underline'>
                    로그인
                </Link>
                <Link href='/settings' className='underline'>
                    API 토큰 발급
                </Link>
            </nav>
        </main>
    </>
)
