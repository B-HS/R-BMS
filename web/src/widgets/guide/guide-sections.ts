import { IR_SERVER_BASE_URL } from '@shared/constants/server-info'

export type GuideRow = {
    label: string
    description: string
}

export type GuideSection = {
    id: string
    title: string
    body: string
    steps?: string[]
    rows?: GuideRow[]
    code?: string
}

export const GUIDE_INTRO =
    'rbms 플레이어는 게임 안에서 IR 연동을 전부 끝낼 수 있습니다. 곡 선택 화면에서 Tab 을 눌러 SETTINGS 를 열고 NETWORK 탭으로 이동하면 서버 주소, 계정, 동기화, 라이벌을 모두 설정할 수 있습니다.'

export const GUIDE_SECTIONS: GuideSection[] = [
    {
        id: 'connect',
        title: '1. 서버 연결 (Connect)',
        body: '곡 선택 화면에서 Tab 을 누르면 SETTINGS 가 열립니다. 상단 탭 중 NETWORK 를 고르고 SERVER URL 행에서 Enter 또는 오른쪽 방향키를 누르면 그 자리에서 주소를 입력할 수 있습니다. Enter 로 확정, Esc 로 취소합니다. 주소를 비우면 오프라인으로 동작합니다.',
        steps: [
            '곡 선택 화면에서 Tab → SETTINGS',
            'Tab 으로 NETWORK 탭 선택 (PLAY · GAUGE · JUDGE · DISPLAY · INPUT · NETWORK)',
            `SERVER URL 행에서 Enter → ${IR_SERVER_BASE_URL} 입력 → Enter`,
            'PLAYER ID 행에 사용할 계정 아이디 입력 (기본값 guest)',
            'Esc 로 SETTINGS 를 닫으면 저장됩니다',
        ],
        code: `SERVER URL   ${IR_SERVER_BASE_URL}\nPLAYER ID    <계정 아이디>`,
    },
    {
        id: 'account',
        title: '2. 계정 (Account)',
        body: 'NETWORK 탭의 EMAIL · PASSWORD 행을 채운 뒤 REGISTER 로 가입하거나 LOGIN 으로 로그인합니다. PASSWORD 는 입력 중에도 * 로만 표시되고 설정 파일에 저장되지 않으며, 로그인이나 가입이 사용하는 즉시 메모리에서 지워집니다. 성공하면 토큰과 로그인 아이디가 로컬 설정에 저장되고 PLAYER ID 가 서버가 돌려준 로그인 아이디로 맞춰집니다.',
        rows: [
            {
                label: 'PLAYER ID',
                description: '서버 계정의 로그인 아이디이자 랭킹에 표시되는 식별자입니다. LOGIN · REGISTER 는 이 값을 아이디로 사용합니다.',
            },
            { label: 'EMAIL', description: 'REGISTER 에서만 사용합니다. 로그인 인증에는 쓰이지 않습니다.' },
            { label: 'PASSWORD', description: '마스킹 입력이며 저장되지 않습니다. 값이 없으면 (not set) 으로 표시됩니다.' },
            {
                label: 'LOGIN',
                description: 'PLAYER ID + PASSWORD 로 백그라운드 로그인. 성공하면 토큰 저장 · 스코어 서버 재구성 · 라이벌 목록 갱신까지 이어집니다.',
            },
            { label: 'REGISTER', description: 'PLAYER ID · EMAIL · PASSWORD 로 가입합니다. 성공 후 동작은 LOGIN 과 같습니다.' },
            {
                label: 'LOGOUT',
                description: '토큰과 로그인 아이디, 보관 중인 비밀번호를 지웁니다. PLAYER ID 는 남아 있어 재로그인이 한 번의 입력으로 끝납니다.',
            },
            {
                label: 'ACCOUNT',
                description:
                    '현재 상태를 보여주는 읽기 전용 행입니다. guest / logging in... / logged in as <id> / login failed: <오류> / session expired: <오류> 등이 표시됩니다.',
            },
        ],
    },
    {
        id: 'ranked',
        title: '3. 랭킹에 반영되는 플레이 (Playing ranked)',
        body: '플레이가 끝나면 결과 화면 왼쪽에 IR 결과가 표시됩니다. 서버가 받아들이면 IR: RANK #<순위> 또는 IR: SENT 가, 개인 베스트를 갱신하면 NEW BEST 가 함께 나옵니다. 아래 조건에 해당하는 플레이는 제출 자체를 건너뛰거나 UNRANKED 로 기록되어 랭킹에서 빠집니다.',
        rows: [
            {
                label: 'autoplay · 리플레이 재생',
                description:
                    '기록 대상이 아니므로 제출하지 않습니다. 결과 화면에 IR: SKIPPED (autoplay) 또는 IR: SKIPPED (replay playback) 이 표시됩니다.',
            },
            {
                label: '판정폭 확대',
                description: 'JUDGE WIDTH 나 LN 마진을 100 보다 넓게 설정한 플레이는 제출하지 않습니다 (IR: SKIPPED (judge window widened)).',
            },
            { label: '스크래치 어시스트', description: 'AUTO SCRATCH 는 어시스트로 취급되어 제출에서 제외됩니다 (IR: SKIPPED (scratch assist)).' },
            {
                label: '어시스트 옵션 · guest 제출',
                description:
                    '서버가 받아들이더라도 랭킹에서 제외하는 경우가 있습니다. 이때는 UNRANKED: assist options, guest submission 처럼 서버가 알려준 사유가 그대로 표시됩니다.',
            },
        ],
        code: 'IR: RANK #12\nNEW BEST\nREPLAY UPLOADED',
    },
    {
        id: 'ranking-panel',
        title: '4. 곡 선택 랭킹 패널 (IR RANKING)',
        body: '곡 선택 화면에서 I 를 누르면 오른쪽에 IR RANKING 패널이 열리고 포커스가 옮겨갑니다. 위/아래로 행을 이동하고 Enter 로 리플레이를 재생하며, I · Esc · 왼쪽 방향키로 닫습니다. 포커스한 차트가 잠깐 머무르면 랭킹 상위 10명, 내 베스트, 라이벌 기록, 리플레이 목록을 한 번에 불러와 차트별로 캐시합니다.',
        rows: [
            {
                label: '행 구성',
                description:
                    '순위 · 플레이어 이름 · EX 점수 · 클리어 램프 순으로 표시되고, 서버가 판정 상세를 주면 F<fast>/S<slow> 가, 내려받을 수 있는 리플레이가 있으면 REP 가 오른쪽에 붙습니다.',
            },
            {
                label: 'YOU · RIVAL 행',
                description:
                    '랭킹 아래에 내 베스트를 담은 YOU 행이, 그 아래에 RIVAL <플레이어 ID> 행이 이어집니다. 기록이 없으면 순위와 점수가 - 로 표시됩니다.',
            },
            {
                label: 'REP 재생',
                description:
                    'REP 가 붙은 행에서 Enter 를 누르면 리플레이를 내려받아 곧바로 재생합니다. 리플레이가 없는 행에서는 no replay for that row 가 표시됩니다.',
            },
            {
                label: '상태 메시지',
                description:
                    'IR OFF - SET SERVER URL IN SETTINGS · SELECT A CHART · LOADING... · NO SCORES YET · ERROR: <오류> 중 하나가 한 줄로 표시됩니다.',
            },
        ],
    },
    {
        id: 'replay',
        title: '5. 리플레이 업로드 (Replays)',
        body: 'NETWORK 탭의 AUTO UPLOAD REPLAY 를 ON 으로 두면 (기본값 ON) 스코어가 정상적으로 접수되고 랭킹 제외 사유가 없을 때 리플레이가 함께 올라갑니다. 결과 화면에 REPLAY UPLOADED 또는 REPLAY ERROR: <오류> 가 표시됩니다. 업로드된 리플레이는 랭킹 패널의 REP 행에서 누구나 내려받아 재생할 수 있습니다.',
        rows: [
            { label: 'AUTO UPLOAD REPLAY', description: '좌우 방향키 · Enter · 클릭으로 ON / OFF 를 전환하고 즉시 저장합니다.' },
            {
                label: '업로드 조건',
                description: '로그인 상태이고, 그 플레이가 실제로 리플레이를 저장했으며, 스코어가 접수되고 랭킹 제외 플래그가 없어야 합니다.',
            },
        ],
    },
    {
        id: 'sync',
        title: '6. 설정 동기화 (Settings sync)',
        body: 'SYNC SETTINGS 를 ON 으로 두면 (기본값 OFF) 플레이 설정과 키 배치를 서버에 보관할 수 있습니다. UPLOAD SETTINGS NOW 로 현재 설정을 올리고, DOWNLOAD SETTINGS NOW 로 내려받아 적용합니다. 다른 기기에서 먼저 올린 내용이 있으면 conflict: server newer, download first 가 표시되므로 먼저 내려받은 뒤 다시 올리면 됩니다.',
        rows: [
            { label: '올라가는 것', description: '플레이 설정과 키 배치입니다. 토큰 · 로그인 아이디 · 이메일 · 비밀번호는 절대 올라가지 않습니다.' },
            {
                label: '내려받을 때',
                description: '플레이 설정과 키 배치만 덮어씁니다. SERVER URL · PLAYER ID · 곡 폴더 · 폰트 경로 같은 기기별 값은 그대로 유지됩니다.',
            },
            {
                label: '필요 조건',
                description:
                    'SERVER URL 이 설정되어 있고 로그인되어 있어야 합니다. 아니면 set SERVER URL first · log in first 안내가 상태 줄에 표시됩니다.',
            },
        ],
    },
    {
        id: 'rivals',
        title: '7. 라이벌 (Rivals)',
        body: 'NETWORK 탭 맨 아래 RIVALS 행에서 Enter 를 누르면 라이벌 목록이 열립니다. + ADD RIVAL (PLAYER ID) 행에서 Enter 를 눌러 상대의 PLAYER ID 를 입력하고, D 로 선택한 라이벌을 지웁니다. Esc 로 닫으면 설정 파일에 저장되고, 로그인 상태라면 서버에도 목록 전체가 반영됩니다.',
        rows: [
            { label: '조작', description: 'UP DOWN 이동 · ENTER 추가 · D 삭제 · ESC 저장 후 닫기. 빈 아이디와 중복 아이디는 무시됩니다.' },
            { label: '동기화', description: '로그인 · 가입 직후와 저장된 토큰이 있는 상태의 시작 시점에 서버 목록으로 갱신됩니다.' },
            { label: '표시', description: '등록한 라이벌은 곡 선택 랭킹 패널에 RIVAL <플레이어 ID> 행으로 함께 표시됩니다.' },
        ],
    },
]

export const GUIDE_ADVANCED_SECTIONS: GuideSection[] = [
    {
        id: 'api-token',
        title: '8. 다른 도구용 API 토큰 (Advanced)',
        body: '게임 클라이언트는 위의 LOGIN 만으로 충분합니다. 직접 만든 스크립트나 외부 도구에서 이 IR 을 호출하려면 웹 설정 화면의 API 토큰 탭에서 토큰을 발급해 Authorization 헤더에 실어 보내세요. 평문 토큰은 발급 직후 한 번만 표시되므로 그 자리에서 저장해야 하며, 필요 없어진 토큰은 목록에서 폐기합니다.',
        code: `curl -H 'Authorization: Bearer rbms_<발급받은_토큰>' ${IR_SERVER_BASE_URL}/auth/me`,
    },
    {
        id: 'cli-overrides',
        title: '9. 실행 인자 덮어쓰기 (Advanced)',
        body: '평소에는 필요하지 않습니다. 저장된 설정을 먼저 적용한 뒤 그 실행에서만 값을 바꾸고 싶을 때 --server 와 --player 로 서버 주소와 플레이어 아이디를 덮어쓸 수 있습니다.',
        code: `rbms-player --server ${IR_SERVER_BASE_URL} --player <플레이어_ID>`,
    },
]
