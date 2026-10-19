# 웹 AI 기능 — 조사 색인

> 최종 갱신 2026-10-19 · 대응 커밋 1eadbb5

웹(`web/`)에 AI 기능 10개를 넣기 위한 조사 보고서 전문과 기획 문서의 자리다. 다시 조사하지 않도록 보고서는 줄이지 않고 그대로 두었다. 코드가 바뀌면 해당 보고서의 절을 함께 고친다.

## 문서

| 문서 | 내용 | 먼저 볼 곳 |
|---|---|---|
| [../../plan/2026-10-19-web-ai-features.md](../../plan/2026-10-19-web-ai-features.md) | **기획 문서**: 구조, 데이터 계약(trace·차트 패턴), 표·마이그레이션, 분석 엔진 정의, 기능 10개 입출력, 공급자 어댑터, 키 암호화·보안, 웨이브 계획, 사용자 결정 질문 | 10절(결정 필요), 2절(구조), 3절(계약) |
| [server.md](server.md) | 웹 서버 조사: DB 스키마와 분석에 쓸 수 있는 열, 리플레이 형식·한도·공개 여부, 발광 표 스냅샷과 실측(1,035곡 전부 라이브러리에 있음, 노트 수 분포), 폴더·기록·라이벌 연결점, 인증·레이트리밋·환경변수·암호화 후보·마이그레이션 규칙·Vercel 한도·CSP·캐시, 의존성 후보 | 요약, 부록 A(기능별 데이터 격차), 부록 B(기존 서버 이상 징후) |
| [client.md](client.md) | 클라이언트 조사: 판 끝에 메모리에 있는 것(실제 배치, `NoteMark`, 게이지 9개 곡선, 속도 옵션), 새 엔드포인트 권고와 계약 교차 테스트 영향, `rbms-cli` 하위 명령으로 차트 패턴 추출, md5·sha256 일치, 크기 추정, 패턴 특징 후보와 RANDOM 계열의 영향 | 핵심 결론 5줄, 2.2(새 엔드포인트), 5.2(옵션 영향 표) |
| [providers.md](providers.md) | 공급자 3곳 공식 API: Anthropic Messages, OpenAI Responses·"codex"의 실체, Ollama Cloud(구조화 출력 미지원), 공통 스키마 부분집합, Vercel AI SDK 비교, BYOK 키 보관(AES-256-GCM 봉투, KEK, 약관 주의) | 4.1(공통 분모), 5(키 보관), 추천 구조 요약 |

## 주제별 찾기

| 찾는 것 | 위치 |
|---|---|
| 서버에 노트별 판정이 없는 이유, (a) 판별 업로드가 필요한 이유 | server.md 요약 1, 2.3 · 기획 1.2 |
| 노트별 판정 자료형(`NoteMark`)과 게이지 곡선(`GaugeLog`) | client.md 1.2·1.3 |
| trace 와이어 형식, 일관성 검사, 능력 플래그 | 기획 3.1·3.2 |
| 계약 교차 테스트에 새 요청 종류를 더하는 법 | client.md 2.3 · 기획 3.2 |
| 곡별 패턴 추출 경로(소리·BGA 를 읽지 않는 근거) | client.md 3.2·3.3 · 기획 3.3 |
| 발광 표 `data.json` 형식, md5 매칭 | server.md 3.2 · client.md 3.5 · 기획 3.4 |
| 패턴 분류 정의(트릴·계단·연타·동시치기·스크래치·LN·소프란) | 기획 4.2 · 후보는 client.md 5.1 |
| RANDOM·MIRROR·S-RANDOM 이 바꾸는 특징 | client.md 5.2 · 기획 4.5 |
| 녹색 숫자·JUDGE OFFSET 의 실제 의미 | client.md 1.5 · 기획 5.4 |
| 구조화 출력 스키마 제약(공급자별·공통) | providers.md 1·2·4.1 · 기획 6.3 |
| 공급자 오류·거절·잘림 신호와 사용자 문구 | providers.md 1·2·3 · 기획 6.5 |
| "codex, api key" 해석 근거 | providers.md 2("codex"의 실체, 제3자 웹 서비스) · 기획 6.4 |
| 키 암호화(봉투, AAD, KEK 회전) | providers.md 5 · server.md 5.4 · 기획 7.2 |
| Vercel 함수 한도(실행 시간, 본문 4.5 MB), CSP | server.md 5.6 |
| 마이그레이션 운영 순서 | server.md 5.5 · 기획 3.6 |
| 폴더 한도와 게임 MY FOLDERS 반영 | server.md 1.3·4.1 · 기획 5.6 |
| 의존성 후보와 최신 버전(2026-10-19) | server.md 6.2 · providers.md 4.2 · 기획 6.2 |

## 조사 사이 어긋남과 보탬

- 계약 fixture 수: server.md 2.3 은 "점수 12"로 적었으나 실제 `contract/client-requests/` 는 점수 10·코스 2·리플레이 2·설정 1, 합 15개다(2026-10-19 `ls` 확인). client.md 2.1 의 값이 맞다.
- 노트 수: client.md 4절의 평균 노트 수(1,500/3,000)는 가정이다. 발광 표 1,035곡의 실측 분포(최소 946, 중앙값 2,114, 최대 5,000, 합계 2,271,796)는 server.md 3.3 에 있다(기존 `target/release/rbms-cli` 바이너리, 현재 소스와 같은지는 확인하지 않음).
- 인증 HOF 이름: 전역 컨벤션 예시는 `withAuth` 이지만 웹 코드는 `withSession`·`withAdmin`(`web/src/server/lib/with-session.ts`)과 `withApiToken`·`withBearerOrSession`(`web/src/server/lib/with-api-token.ts`)이다.

## 비공개 데이터 주의

차트 노트 배치(곡별 패턴)와 사용자 trace 는 이 저장소(공개)·`web/data`·`docs` 어디에도 두지 않는다. 이 폴더의 문서는 형식과 근거만 담는다.
