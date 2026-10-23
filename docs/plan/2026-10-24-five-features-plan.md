# 2026-10-24 다섯 기능 실행 계획 — 다국어·리플레이 배치 플레이·랭킹 보기·NOTES RADAR·선곡 UI

> 최종 갱신 2026-10-24 · 대응 커밋 `bae5d86`(작업 트리에 조사 보고 5개가 미커밋으로 있다) · 상태: 계획만. 이 문서 외에 저장소 파일을 바꾸지 않았고 빌드·테스트·앱 실행은 하지 않았다.
> 입력: [i18n](../reference/2026-10-24-i18n-survey.md), [리플레이 배치](../reference/2026-10-24-replay-pattern-play-survey.md), [랭킹](../reference/2026-10-24-select-ranking-survey.md), [NOTES RADAR](../reference/2026-10-24-notes-radar-survey.md), [선곡 UI](../reference/2026-10-24-select-ui-iidx-survey.md). 사용자 원문은 `docs/PROCESS.md` 첫 체크리스트(N0~N8).
> 작업 규칙의 정본은 `docs/HANDOFF.md` 5절. 이 계획은 그 규칙(Workflow 도구만, Fable·Haiku 금지, Opus high·Sonnet xhigh 상한, 에이전트 커밋·푸시 금지, worktree 병렬, `git apply`(`--3way` 금지)로 병합, 단위별 `git apply --cached` 뒤 staged 목록을 별도 명령으로 확인)을 전제로 한다.
> 근거 표기: `파일:줄` 은 `bae5d86` 기준으로 이 계획을 쓰며 직접 열어 본 것이다. 보고서 인용만 한 것은 "(보고서)" 를 붙였다.

## 0. 한눈에

| 웨이브 | 단위(병렬) | 핵심 | 선행 |
| --- | --- | --- | --- |
| W1 기반 | K0 확장 id 계약, I0 다국어 기반, R1 레이더 계산, RK-D 랭킹 데이터, RK-W 웹 순위 단위 배지, L1 배치 빌리기 코어 | 계약(id·언어·레이더 API·순위 행·배치 API)을 먼저 굳힌다 | 없음 |
| W2 기능 엔진 | R2 곡 DB 레이더(+CLI), R4 레이더 칠하기, RK-M 랭킹 모드 입력, RK-V 랭킹 오버레이, RK-S 순위 슬롯 스킨 값, RP-P 재생 중 배치 플레이·고스트·배치 골든 | 엔진·앱 기능 | W1 |
| W3 선곡 값 | A3 곡 분석 연결, BAR 막대 등급·썸네일, S3 선곡 보조 값, L2 리플레이 선곡 진입, I1 스킨 언어·`t(key)`, D0 기본 스킨 색·배치 토큰 | 스킨이 읽을 값과 진입 길 | W2 |
| W4 화면 | D1 선곡 목록·머리·바닥·시트, D2 정보 패널·NOTES RADAR·기록, I2 설정 번역, I3 렌더 번역, I5 메뉴·표·코스 번역, I6 알림 번역 | 기본 스킨 IIDX 식 화면, 번역 일괄 1차 | W3 |
| W5 번역 마무리·웹 기반 | I4 선곡 스테이지 번역, I7 기본 스킨 문자열 번역, WI0 웹 다국어 기반 | 선곡 쪽 파일이 모두 끝난 뒤 번역 | W4 |
| W6 마지막 | WI1~WI4 웹 문구 추출, DOC 문서·ADR·QA·테스트 부채 | 웹 다국어(사용자가 빼라면 제외)와 문서 | W5 |

단위 32개. 웨이브마다 병렬 6 이하. 웨이브가 끝나면 메인이 병합·통합 게이트(6절)·커밋(7절)·푸시·CI 확인을 한 뒤 다음 웨이브의 worktree 를 그 커밋에서 연다.

## 1. 결정 표 (A)

모든 행은 **사용자 미확인**이다(메인 채택안 또는 보고서 추천안. 답이 오면 8절의 "바뀌는 단위"만 고친다).

### 1.1 메인 채택안

| # | 결정 | 상태 |
| --- | --- | --- |
| M1 | 다국어는 클라이언트(앱+기본 스킨)를 이번에 한다. 웹(next-intl, 한·영·일)은 같은 체크리스트의 마지막 웨이브(W5 WI0, W6 WI1~WI4). 문자열 일괄 번역(I2·I3·I5·I6, I4·I7)은 기능 웨이브 뒤(W4·W5). 기반 I0 은 W1 | 사용자 미확인 |
| M2 | 리플레이 배치 플레이는 보고서 질문 1~7 추천안 + 고스트(G1, 로컬 리플레이)를 이번 범위에 넣는다. 서버 리플레이 판정 설정(S1)과 마이그레이션은 하지 않는다 | 사용자 미확인 |
| M3 | 랭킹 보기는 질문 1~8 추천안(파란 건반 300 ms 이상 = 랭킹 모드, 짧은 누름은 뗄 때 지금 동작, 9키 버튼 2·8, 키보드 S·D, 오버레이 확장, `extra.dans`, 캐시 틈 수리, RANKING KEYS 설정 행) | 사용자 미확인 |
| M4 | NOTES RADAR 는 질문 1~6 추천안. 화면 제목 "NOTES RADAR", 축 NOTES·PEAK·SCRATCH·SOF-LAN·CHARGE·CHORD(사용자 표현), 기본 스킨 선곡에 6각형, 다각형은 노란 반투명 | 사용자 미확인 |
| M5 | 선곡 UI 는 질문 2~11 추천안. 질문 1(색)은 "구조는 IIDX 식, 바탕은 모노톤 유지, 등급 배지·램프 띠·SCORE CLASS 메달·레이더 칠하기에 강조색"(2026-10-12 모노톤 결정의 부분 변경, ADR 필요). 질문 9 문구는 일반어, NOTES RADAR 만 사용자 표현 유지. RIVAL 자리는 내 기록 6줄(BEST+최근 5판) | 사용자 미확인 |
| M6 | 레이더 정의는 NOTES RADAR 보고서(0~200, 공개 구간표, `rbms-chart` 계산, `song_radar` 저장)를 따른다. 선곡 UI 보고서 4.3절 3번의 자체 0~100 레이더와 U2 의 레이더 계산은 버린다 | 메인 통합 결정, 사용자 미확인 |
| M7 | 확장 id 는 3절 단일 배정표가 정본이다. 보고서들이 제안한 번호(레이더 float·숫자 21150~21155·참조 이미지 21160, 리플레이 이벤트 21002~21005, 레이더 rate 21020~21025)는 쓰지 않는다 | 메인 통합 결정 |
| M8 | 랭킹 모드가 열려 있는 동안(불린 21105) 기본 스킨의 RIVAL 자리(MY RECORDS 6줄)를 IR 순위 6줄(원본 순위 슬롯 120~129·380~399)로 바꿔 보여 준다. Lua 만 바뀌고(D2) 새 엔진 값은 없다. 이유: 오버레이가 왼쪽 정보창 자리에 뜨므로 목록 오른쪽에서 곡을 넘기며 상위 순위를 같이 보는 데 쓸 수 있다 | 제안, 사용자 미확인 |
| M9 | `blue_keys` 는 옮기지 않는다. 이미 `pub(crate)` 라(`apps/rbms-player/src/stage/result.rs:161`) 선곡에서 가져다 쓴다(랭킹 보고서 7절의 "위치 이동"은 하지 않음) | 메인 통합 결정 |
| M10 | 질문 9 의 일반어 이름: MUSIC DATA → CHART DATA, SCORE DATA → MY SCORE, SCORE CLASS → SCORE TARGETS, CLEAR VISUALIZATION → (표지 없이 스크롤 막대 램프 누적), RIVAL SCORE DATA → MY RECORDS(랭킹 모드에서는 IR RANKING), CLEAR RATE 그대로, NOTES RADAR 그대로 | 사용자 미확인 |
| M11 | 웹 다국어: next-intl 을 쿠키 방식(경로 접두사 없음)으로, 메시지는 `web/messages/<locale>/<namespace>.json`, 기본 한국어. **새 웹 의존성(next-intl)은 사용자 승인 전에는 WI0 를 시작하지 않는다.** AI 결과 문구 언어는 UI 언어를 따른다(WI4) | 사용자 미확인 |

### 1.2 보고서별 추천안 채택(번호는 각 보고서 5절)

| 보고서 | 질문 | 채택 | 반영 단위 |
| --- | --- | --- | --- |
| 다국어 | Q1 범위 | 클라이언트 먼저 + 웹은 마지막 웨이브(M1, 보고서 추천과 달리 이번 체크리스트에 포함) | I0~I7, WI0~WI4 |
| 다국어 | Q2 기본 언어 | AUTO(OS 첫 선호 언어, 지원 밖은 영어) | I0 |
| 다국어 | Q3 알림 | 행동 알림 약 60개만 번역, 진단성 약 90개 영어 | I6 |
| 다국어 | Q4 게임 용어 | 원어 유지 + 설명문만 번역 | 용어집(I0), 번역 단위 전부 |
| 다국어 | Q5 chart | 한국어 "차트", 일본어 "譜面", プレー 표기 | 용어집(I0) |
| 다국어 | Q6 일본어 감수 | 모델 초안 + 용어집 + 되번역, 원어민 감수는 릴리스 노트로 요청(게시는 승인 뒤) | I0, DOC |
| 다국어 | Q7 글꼴 | 시스템 글꼴 폴백 유지, 실창 점검 뒤 필요하면 서브셋 번들 | (후속) |
| 다국어 | Q8 서버 오류 문구 | 클라이언트는 서버 한국어 그대로(1단계). 웹 다국어 단위(WI4)는 웹 화면에서 오류 코드로 번역 | I6, WI4 |
| 다국어 | Q9 Discord | 영어 유지 | 없음 |
| 다국어 | Q10 구조 | 새 크레이트 없이 `rbms-model` 에 `Language`·매크로 | I0 |
| 다국어 | Q11 스킨 헤더 | 헤더 옵션 이름 영어 유지, 확장 불린으로 본문만 | I1, I7 |
| 다국어 | Q12 순서 | 기반 먼저, 새 기능 문자열은 처음부터 카탈로그 | 5.0 공통 규칙 |
| 리플레이 | Q1 | 배치 빌리기 + 고스트(M2: 보고서 추천 A 보다 넓힘) | L1, RP-P |
| 리플레이 | Q2 | 기록 모달 버튼 + IR 패널 키 + 보는 중 단축키(파란 건반 쥐고 REPLAY 는 안 함) | L2, RP-P |
| 리플레이 | Q3 | 옵션+시드만 | L1 |
| 리플레이 | Q4 | 같은 배치 재시도에서만 유지 | L1, RP-P |
| 리플레이 | Q5 | 일반 판과 같이 저장·전송, 표지 없음 | L1 |
| 리플레이 | Q6 | 서버 리플레이 고스트·점수 행 배치 안 함 | (없음) |
| 리플레이 | Q7 | 시드 0·Converge 는 거절과 이유 표시 | L1, L2 |
| 랭킹 | Q1~Q8 | 모두 A(M3) | RK-D, RK-W, RK-M, RK-V, RK-S |
| NOTES RADAR | Q1 | 커뮤니티 가중치 가설 + 이름 있는 임시 스케일 상수 | R1 |
| NOTES RADAR | Q2 | 스캔 때 계산·`song_radar` 저장 + 없으면 커서가 머물 때 채움 | R2, A3 |
| NOTES RADAR | Q3 | 5·7키 SP 구간, 10·14키 DP 구간, 9·24키 없음 | R1 |
| NOTES RADAR | Q4 | 표시값 200 으로 자름, 원시 값은 DB | R1, R2 |
| NOTES RADAR | Q5 | 외부 스킨 노출 + `rbms-skin` 문 수리. 번호는 3절(M7) | K0, R4, A3 |
| NOTES RADAR | Q6 | 내장(비스킨) 선곡 화면 육각형 안 함 | 없음 |
| 선곡 UI | Q1 | M5 | D0, D1, D2, ADR |
| 선곡 UI | Q2 | 지금 레이아웃을 IIDX 식으로 교체 | D1, D2 |
| 선곡 UI | Q3 | `songlist` 확장 키 `rank`·`thumb`, 썸네일 중앙 자르기 정사각 | BAR, D1 |
| 선곡 UI | Q4 | 정수·실수·문자열도 21000~21199 숫자 범위 | K0 |
| 선곡 UI | Q5 | 내 기록 6줄 | S3, D2 |
| 선곡 UI | Q6a | IR 상위 6명은 랭킹 모드일 때만(M8) | D2 |
| 선곡 UI | Q6b | SCORE TARGETS = AAA·AA·A 경계(8/9·7/9·6/9) 금·은·동, 달성 표시 | D2 |
| 선곡 UI | Q7 | CLEAR RATE = CLEAR COUNT / PLAY COUNT | D2 |
| 선곡 UI | Q8 | 레이더 구현(정의는 M6) | R1~R4, A3, D2 |
| 선곡 UI | Q9 | 일반어(M10), NOTES RADAR 만 그대로 | D1, D2, I7 |
| 선곡 UI | Q10 | (a) 내장 선곡 렌더러 그대로 (b) 시각 칩 (c) 카테고리 아이콘 탭 생략·위치 경로만 | D1 |
| 선곡 UI | Q11 | 곡 그림은 256 정사각 상자, 확대 팝업 없음 | D2 |

## 2. 보고서 주장 대조 결과

계획에 쓰기 전에 직접 연 것. 어긋난 것은 계획에서 고쳐 쓴다.

| 보고서 주장 | 확인 위치 | 결과 |
| --- | --- | --- |
| 확장 이벤트 21000·21001, 불린 21100~21104·21110~21114·21120~21124·21130~21134·21140~21148 사용 | `apps/rbms-player/src/skin_host/extension.rs:33-85`(`KEY_INDEX_COUNT` = 9, `keyconfig.rs:1055`), `assets/skins/rbms-default/lua/props.lua:167-177` | 맞음(이벤트 2, 불린 29) |
| 정수·실수·문자열 확장 id 는 아직 없다 | 같은 파일, `skin_host/mod.rs:53-57`(쓰는 상수 import) | 맞음 |
| Lua `number`·`float_number` 가 원본 미구현 id 에 0 을 답한다 | `crates/rbms-skin/src/lua/main_state.rs:855-870`, `crates/rbms-skin/src/property/names.rs:363` | 맞음. `text` 는 문이 없다(`main_state.rs:874-879`) |
| 그 밖의 문 | `crates/rbms-render/src/skin_render/draw.rs:312`, `skin_render/text.rs:191`, `skin_render/songlist/build.rs:168` | 맞음. 그래프·슬라이더의 `ValueSource::Id` 는 문 없이 `state.float` 를 읽는다(`skin_render/object.rs:590`) |
| float·rate 는 공간이 따로다 | 호스트 `float()` 가 Float 에서 못 찾으면 Rate 로 다시 묻는다(`skin_host/mod.rs:975-977`) | **주의**: Lua `float_number(id)` 는 두 공간을 함께 본다. 그래서 3절은 Float 와 Rate 에 같은 번호를 쓰지 않는다(선곡 UI 보고서 4.6절의 rate 21020~21025 와 레이더 float 를 겹치지 않게 고침) |
| 이벤트 21002 를 랭킹·리플레이가 둘 다 제안 | 랭킹 보고서 4.1절, 리플레이 보고서 4.5절 | 충돌. 21002 = 랭킹, 리플레이 슬롯은 21003~21006 예약(이번 미구현) |
| 레이더 정의가 두 보고서에서 다름 | NOTES RADAR 4.1·4.2절(0~200, 구간표) 대 선곡 UI 4.3절 3번(0~100 자체 상한) | M6 로 하나로 |
| 참조 이미지는 음수 destination id | `crates/rbms-render/src/skin_render/refs.rs:46-73`(`ReferenceImage::of_destination`) | 맞음. 확장 참조 이미지를 더할 자리 |
| `blue_keys` 위치 | `stage/result.rs:161` `pub(crate) fn blue_keys` | 옮길 필요 없음(M9) |
| 9키 표 | `stage/select/keys.rs:132-142`(`POPN_TABLE`) | 인덱스 1(버튼 2)의 첫 칸은 `Option1Up`, 인덱스 7(버튼 8)은 `TargetUp`. "목록 역할 없음"은 RK-M 이 구현 전 다시 확인 |
| 순위 받기는 패널이 열렸을 때만 | `stage/select/mod.rs:882-897`(`update_ranking` 의 `!self.ranking_open` 반환) | 맞음 |
| 캐시·제출 | `ir_ranking.rs:17`(`RANKING_PANEL_LIMIT` 10), `:21`(용량 64), `app_ranking.rs:75-96`(`start_ranking_fetch`), `:263`(`poll_submit`) | 맞음 |
| 배치 로드 분기 | `app_play.rs:447-480`(`load`, `play_as_recorded` 분기, `put_back_own_play_options`, `retry_seed`), `:824`(`leave_play`) | 맞음 |
| 재시도 | `stage/result.rs:1019-1031`(같은 배치 재시도), `stage/play/mod.rs:1449-1456`(실패 직후, `app_result::retry` 를 부름) | 맞음. 실패 직후 길도 `retry` 를 거친다 |
| 단축키 표 | `keyconfig.rs:759` `Shortcut::ALL: [Shortcut; 31]` | 맞음(32 로 늘어남) |
| 설정 행 수 | `crates/rbms-config/src/settings.rs:275` `SETTING_COUNT = 97` | 맞음. I0 가 98, RK-M 이 99 |
| 렌더 컨텍스트 | `crates/rbms-render/src/ctx.rs:15-18`(`RenderCtx { theme, text }`), `theme.rs:368`(`set_theme`) | 맞음 |
| 골든 테스트 이름 | `apps/rbms-player/src/app_result/record_tests/contract_requests.rs:945` `fn a_settings_upload` | 맞음 |
| 캡처 게이트 | `apps/rbms-player/src/stage/capture.rs:81` `mod gate;` → `stage/capture/gate.rs` | 있음 |
| skin-dump 시나리오 | `apps/rbms-cli/src/skin_dump/scenario.rs:53-76`(`integers`·`rates`·`floats`·`texts` 지원), `apps/rbms-cli/scenarios/select.json`(불린 1·타이머 1뿐) | 새 id 값을 시나리오에 넣을 수 있다 |
| 웹 순위 서비스 | `web/src/server/service/domain/score/score.service.ts:496`(`getRanking`), 단위 배치 조회 `dan.service.ts:147`(`listDisplayed`), 캐시 `read-cache.ts:40-45` | 맞음 |
| 기본 스킨 회전 | `assets/skins/rbms-default/lua/ui.lua:109`(`angle`) | 레이더 격자를 회전 사각형으로 그릴 수 있다(새 시트 조각 불필요) |
| 문서 낡은 서술 | `docs/skin.md:574`("글자는 영어"), `:698`(선곡 밀도 빈칸이라 적었으나 360~365 는 채워짐) | DOC 에서 고침 |

## 3. 확장 스킨 id 단일 배정표 (B)

### 3.1 공간과 규칙

1. 모든 새 id 는 21000~21199 안이다. 이벤트는 21000~21099, 불린은 21100~21199(ADR `2026-10-22-adr-rbms-extension-ids`). 이번에 새로 여는 공간(정수, Float, Rate, 텍스트, 참조 이미지)도 같은 21000~21199 숫자를 쓴다(선곡 UI Q4). 범위 밖 배정 금지.
2. 공간끼리는 번호가 겹쳐도 된다. 단 **Float 와 Rate 는 한 공간처럼 다룬다**(Lua `float_number` 와 호스트 `float()` 가 둘을 함께 본다, `skin_host/mod.rs:975-977`).
3. 새 id 의 상수는 K0 가 한 번에 만든다. 다른 단위는 상수를 읽기만 한다. 필요한 id 가 없으면 만들지 말고 멈춰 보고한다.
4. 값을 아직 답하지 않는 id 는 부재 값(`INTEGER_ABSENT`, `FLOAT_ABSENT`, `TEXT_ABSENT`, 불린 거짓, 이벤트 무시)을 낸다. 기본 스킨은 부재 값이면 그리지 않는다.

### 3.2 현재 사용(코드에서 다시 셈)

| 공간 | 번호 | 뜻 | 근거 |
| --- | --- | --- | --- |
| 이벤트 | 21000, 21001 | SCRATCH AUTO, KEEP GREEN NUMBER 전환 | `extension.rs:38,42` |
| 불린 | 21100~21104 | SCRATCH AUTO·KEEP GREEN·LANE COVER·LIFT·HIDDEN 켜짐 | `extension.rs:49-58` |
| 불린 | 21110~21114 | 커서 곡 최고 램프의 어시스트 등급 0~4 | `extension.rs:68` |
| 불린 | 21120~21124 | 결과 화면 판의 등급 | `extension.rs:71` |
| 불린 | 21130~21134 | 결과 화면 이전 최고의 등급 | `extension.rs:75` |
| 불린 | 21140~21148 | 키 인덱스 0~8 에 닿는 키 없음 | `extension.rs:82-85` |
| 정수·Float·Rate·텍스트·참조 이미지 | 없음 | - | - |

### 3.3 새 배정

| 공간 | 번호 | 뜻(값) | 답하는 클러스터 | 값을 채우는 단위 | 읽는 단위 |
| --- | --- | --- | --- | --- | --- |
| 이벤트 | 21002 | 랭킹 모드 열기·닫기 | Select | RK-M | D1(RANKING 단추) |
| 이벤트 | 21003~21006 | 예약: 리플레이 슬롯 1~4 의 배치로 플레이(이번에는 처리하지 않음, 상수만) | Select | (없음) | (없음) |
| 이벤트 | 21010~21014 | 같은 곡 형제 차트 슬롯 1~5 로 커서 이동 | Select | S3 | D2(난이도 탭) |
| 불린 | 21105 | 랭킹 모드가 열려 있음 | Select | RK-M | D1, D2(M8) |
| 불린 | 21115~21119 | 형제 차트 슬롯 1~5 에 차트가 있음 | Select | S3 | D2 |
| 불린 | 21150~21152 | 해석된 UI 언어 = 영어·한국어·일본어(`21150 + Language::id()`, 하나만 켜짐) | System(모든 화면) | I1 | I7, D1, D2 |
| 불린 | 21153~21159 | 예약: 다음 언어 | System | (없음) | (없음) |
| 정수 | 21000~21015 | 커서 곡의 레인별 노트 수(모드의 레인 순서, 14키까지, 없는 레인·9키·24키는 부재) | Chart | A3 | D2 |
| 정수 | 21020~21025 | NOTES RADAR 표시값 0~200 의 정수(버림). 순서 NOTES, PEAK, SCRATCH, SOF-LAN, CHARGE, CHORD(화면 위에서 시계 방향) | Chart | A3 | D2 |
| 정수 | 21030~21034 | 형제 차트 슬롯 1~5 의 레벨 | Select | S3 | D2 |
| 정수 | 21040~21050 | 열린 폴더의 램프 id 0~10 별 차트 수 | Select | S3 | D1(스크롤 막대) |
| 정수 | 21051 | 열린 폴더의 차트 수 합 | Select | S3 | D1 |
| 정수 | 21060~21065 | MY RECORDS 줄 r(0 = BEST, 1~5 = 최근 판)의 EX | Select | S3 | D2 |
| 정수 | 21066~21071 | 줄 r 의 램프 id(0~10) | Select | S3 | D2 |
| 정수 | 21072~21077 | 줄 r 의 MISS COUNT | Select | S3 | D2 |
| 정수 | 21078~21083 | 줄 r 의 DJ LEVEL 인덱스(0 = F … 7 = AAA, `crates/rbms-render/src/result.rs:127-139`) | Select | S3 | D2 |
| Float | 21020~21025 | NOTES RADAR 표시값 0~200(정수판과 같은 순서) | Chart | A3 | D2 |
| Rate | 21000~21015 | 레인 막대 비율(레인 노트 수 / 가장 많은 레인) | Chart | A3 | D2 |
| Rate | 21040~21050 | 열린 폴더 램프 비율(램프 수 / 21051) | Select | S3 | D1 |
| 텍스트 | 21000~21005 | MY RECORDS 줄 r 의 날짜(저장 UTC, 표시는 기존 날짜 형식으로 로컬) | Select | S3 | D2 |
| 참조 이미지 | 21020 (destination id `"-21020"`) | NOTES RADAR 다각형(흰 불투명, 256x256, 스킨의 색·알파가 노란 반투명을 입힘) | 렌더러(`refs.rs`) | R4(칠하기), A3(값 공급) | D2 |

원본 id 를 새로 답하는 것(확장 id 아님, 표에 넣지 않음): 순위 슬롯 정수 380~399·이미지 인덱스 380~399·텍스트 120~129·Rate 8(RK-S), 선곡의 종류별 노트 수 350~353·주 BPM 92(A3), 선곡 `judgegraph` type 0·`bpmgraph`(A3).

### 3.4 남는 번호

| 공간 | 남는 번호 |
| --- | --- |
| 이벤트 | 21007~21009, 21015~21099 |
| 불린 | 21106~21109, 21149, 21160~21199 |
| 정수 | 21016~21019, 21026~21029, 21035~21039, 21052~21059, 21084~21199 |
| Float·Rate(합쳐서) | 21016~21019, 21026~21039, 21051~21199 |
| 텍스트 | 21006~21199 |
| 참조 이미지 | 21000~21019, 21021~21199 |

### 3.5 K0 가 여는 문

- `crates/rbms-skin/src/property/extension.rs`(새): 공간별 대역 상수, `RBMS_IMAGE_NOTES_RADAR = 21_020`, `pub fn implements(space, id) -> bool`(= `reference_implements(space, id)` 또는 그 공간의 확장 대역 안). `names.rs` 의 `reference_implements` 는 바꾸지 않는다.
- 그 함수로 바꾸는 호출부: `main_state.rs:858`, `:866`(정수·실수), `draw.rs:312`(정수 id 로 고르는 이미지 세트), `skin_render/text.rs:191`(`ref` 텍스트), `songlist/build.rs:168`.
- 앱 `skin_host/extension.rs` 는 3.3절 이름을 모두 갖고 `rbms-skin` 상수를 가리킨다. `skin_host/mod.rs` 의 `ROUTES`·`EVENT_ROUTES` 에 3.3절 라우트를 모두 넣는다(클러스터는 기본 `None` 이라 값이 아직 없다).

## 4. 공유 파일 소유권 (C)

"주"는 크게 고치는 단위, "최소"는 5.0절 4번의 최소 접촉 규칙만 허용. 칸이 비면 그 웨이브에서 아무도 고치지 않는다.

| 파일 | W1 | W2 | W3 | W4 | W5 | W6 |
| --- | --- | --- | --- | --- | --- | --- |
| `apps/rbms-player/src/stage/select/mod.rs` | - | RK-M 주 | L2 주, A3·BAR·S3 최소 | - | I4 주 | - |
| `apps/rbms-player/src/stage/select/events.rs` | - | RK-M(21002) | S3(21010~21014) | - | I4 | - |
| `apps/rbms-player/src/stage/select/list.rs` | - | - | A3(`chart_under_cursor` `:570-594` 만), BAR(`best_score` `:698` 만) | - | I4 | - |
| `apps/rbms-player/src/skin_host/extension.rs`, `skin_host/mod.rs` | K0 | - | - | - | - | - |
| `apps/rbms-player/src/skin_host/select.rs`, `select/shown.rs` | - | RK-M(21105 만) | S3 | - | - | - |
| `crates/rbms-skin/src/property/names.rs` | 읽기만 | - | - | - | - | - |
| `crates/rbms-skin/src/lua/main_state.rs`, `property/extension.rs`(새) | K0 | - | - | - | - | - |
| `crates/rbms-skin/src/model/objects.rs`, `loader/from_lua.rs` | - | - | BAR | - | - | - |
| `assets/skins/rbms-default/lua/props.lua` | K0(확장 표) | - | - | - | I7(문자열 있으면) | - |
| `assets/skins/rbms-default/lua/theme.lua` | - | - | D0 | 읽기만 | I7(난이도 이름 등 문자열) | - |
| `assets/skins/rbms-default/lua/select/{list,controls,main}.lua` | - | - | - | D1 | I7 | - |
| `assets/skins/rbms-default/lua/select/{info,radar(새),records(새)}.lua` | - | - | - | D2 | I7 | - |
| `assets/skins/rbms-default/lua/select/{words,sheet,panel,side,inform,kit}.lua` | - | - | - | - | I7 | - |
| `assets/skins/rbms-default/lua/i18n.lua`, `lua/i18n/*.lua`(새) | - | - | I1(뼈대) | D1(`select_bar.lua`), D2(`select_info.lua`) | I7(나머지) | - |
| 시트: `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` | - | - | - | D1 | - | - |
| `apps/rbms-cli/scenarios/select.json` | - | - | - | D2(D1 이 쓰는 값까지) | I7 | - |
| `apps/rbms-player/src/ir_ranking_view.rs` | - | RK-M(`PanelAction`·`panel_action` `:14-39` 만), RK-V(나머지) | L2(`PlayLayout`) | - | I4 | - |
| `apps/rbms-player/src/stage/select/overlay.rs` | - | RK-V | L2 최소(핫 영역 매핑이 필요할 때만) | - | I4 | - |
| `apps/rbms-player/src/ir_ranking.rs`, `app_ranking.rs` | RK-D | - | - | I6(`app_ranking.rs` 알림 줄만) | - | - |
| `apps/rbms-player/src/lib.rs` | I0 최소, L1 최소 | - | - | I6(알림 줄만) | - | - |
| `apps/rbms-player/src/app_play.rs` | L1 | - | - | I6(알림 줄만) | - | - |
| `apps/rbms-player/src/app_result.rs` | L1 | RP-P(`run_target` `:695` 만) | - | I6(알림 줄만) | - | - |
| `apps/rbms-player/src/stage/result.rs` | L1 | - | - | - | - | - |
| `apps/rbms-player/src/stage/play/mod.rs`, `keyconfig.rs`, `ghost.rs`, `target.rs` | - | RP-P | - | I5(`target.rs` 문자열) | - | - |
| `crates/rbms-config/src/{schema,settings}.rs` | I0(LANGUAGE, 97→98) | RK-M(RANKING KEYS, 98→99) | - | - | - | - |
| `apps/rbms-player/src/stage/settings/catalog.rs` | I0(새 행 설명) | RK-M(새 행 설명) | - | I2 | - | - |
| `contract/client-requests/settings-blob.json` | I0 | RK-M | - | - | - | - |
| `crates/rbms-render/src/skin_render/{refs,object}.rs`, `graphs/*` | - | R4 | - | - | - | - |
| `crates/rbms-render/src/skin_render/songlist*` | K0(`build.rs:168` 한 줄) | - | BAR | - | - | - |
| `crates/rbms-render/src/select.rs` | - | - | L2(모달 버튼) | I3 | - | - |
| `crates/rbms-render/src/{font,ctx,theme,lib}.rs`, `text/*`(새) | I0 | - | L2(`text/select.rs` 항목 추가) | I3 | - | - |
| `apps/rbms-player/src/text/*`(새) | I0(빈 모듈) | RK-V(`ranking.rs`), RP-P(`layout_play.rs`) | L2(`layout_play.rs`) | I2(`settings.rs`), I5(`menus.rs`), I6(`notices.rs`) | I4(`select.rs`) | - |
| `apps/rbms-player/src/stage/capture.rs` | - | RK-V(오버레이 캡처 항목만) | - | - | - | - |
| `crates/rbms-chart/src/lib.rs`, `radar*` | R1 | RP-P(`shuffle.rs` 시험 모듈만) | - | - | - | - |
| `crates/rbms-library/**` | - | R2 | - | - | - | - |
| `apps/rbms-player/src/library.rs`, `skin_host/{chart,overview}.rs` | - | - | A3 | - | - | - |
| `web/src/server/**` 순위 | RK-W | - | - | - | - | WI4(`server/lib`, AI) |
| `web/**` 다국어 | - | - | - | - | WI0 | WI1~WI4 |
| `docs/**` | I0(용어집 초안만), RK-W(`api-spec.md` 순위 절만) | - | - | - | - | DOC |

### 4.1 최소 접촉 규칙

1. 같은 웨이브에서 한 파일을 "주"로 고치는 단위는 하나다. 다른 단위는 "구조체 필드 1개 + 초기화 1줄 + 호출 1줄"(또는 표에 적힌 함수 하나)만 고친다. 로직은 자기 새 파일에 둔다.
2. 최소 접촉 단위는 줄을 옮기거나 재포맷하지 않는다(rustfmt 가 바꾸는 범위는 자기 줄 근처뿐이어야 한다).
3. 범위를 넘어야 하면 멈추고 보고한다. 메인이 다음 웨이브로 넘기거나 소유를 바꾼다.

### 4.2 병합 순서(웨이브 안)

| 웨이브 | 순서 | 이유 |
| --- | --- | --- |
| W1 | K0 → I0 → R1 → RK-D → L1 → RK-W | `lib.rs` 는 I0 다음 L1. K0 와 I0 는 `rbms-render` 의 서로 다른 파일 |
| W2 | R2 → R4 → RK-D 소비자 순: RK-M → RK-V → RK-S → RP-P | `ir_ranking_view.rs` 는 RK-M(키 매핑) 다음 RK-V(그리기). 설정 골든은 RK-M 하나 |
| W3 | D0 → I1 → L2 → A3 → BAR → S3 | `stage/select/mod.rs` 는 큰 L2 먼저, 최소 접촉 셋은 뒤에서 충돌을 메인이 손으로 푼다 |
| W4 | D1 → D2 → I3 → I2 → I5 → I6 | D2 는 D1 의 `main.lua` 를 안 고치지만 시나리오·캡처 확인은 D1 뒤가 맞다 |
| W5 | I4 → I7 → WI0 | 독립 |
| W6 | WI1 → WI2 → WI3 → WI4 → DOC | 메시지 네임스페이스 파일이 단위마다 따로라 순서 무관, 문서는 마지막 |

병합 충돌이 나면 메인이 그 파일만 손으로 합치고, 그 단위의 검증 명령 하나를 다시 돈다(같은 입력의 성공 결과는 재사용).

## 5. 웨이브 계획 (D)

### 5.0 모든 단위의 공통 계약

1. 실행: 단위 = worktree 1개 = 에이전트 1개. 모델은 표에 적는다(Opus high 또는 Sonnet xhigh). 에이전트는 커밋·푸시·`git` 상태 변경을 하지 않는다. 결과는 worktree 의 작업 트리로 남긴다.
2. 먼저 읽기: `docs/HANDOFF.md` 5절, 이 계획의 1·3·4절과 자기 단위, 관련 보고서 절. 사용자 전역 컨벤션 중 Rust 에 해당하는 것만(`//` 주석 금지, `///`·`//!` 영어만, 매직넘버·새 `#[allow]`·`unsafe`·`todo!()` 금지, rustfmt 160, `--locked`), 웹은 `web/AGENTS.md`(Next 16.3.4 문서를 `node_modules/next/dist/docs/` 에서 먼저 읽음).
3. 금지(전 단위): 새 의존성(웹 WI0 의 next-intl 만 사용자 승인 뒤 예외), 판정·점수 의미 변경, 비밀 파일(`.env*`, `~/.config/rbms/settings.ron`) 열기, 운영 사이트 요청, `rbms-player` 실행(꼭 필요하면 scratch `HOME` 과 `--server http://127.0.0.1:9`), 다른 단위 소유 파일 수정, 요청 밖 리팩터링, 설정에 `skip_serializing_if` 새로 쓰기, 확장 id 새로 만들기(3.1절 3번).
4. 최소 접촉 파일은 4.1절대로만.
5. 문자열: W2 부터 새로 만드는 사용자 표시 문자열은 처음부터 `translations!` 카탈로그(영어·한국어·일본어 셋, 용어집 `docs/reference/2026-10-24-i18n-glossary.md` 따름)로 쓴다. 기존 문자열은 건드리지 않는다(번역 단위 몫). W1 단위(L1 등)는 카탈로그가 아직 없으므로 기존 `notify` 영어로 쓰고 I6 이 옮긴다. 기본 스킨의 새 문자열은 W4 부터 `t(key)`(I1)로.
6. 떠는 테스트 0: 시간·스레드에 기대는 시험은 가짜 시계·동기 채널로. 시험이 네트워크·실제 파일 시스템 밖 경로에 닿지 않는다.
7. 검증 약호(단위 표의 "검증"에 쓴다). 반복 중에는 좁은 필터로 돌려도 되지만 보고 전 마지막 한 번은 아래 그대로 돈다.
   - `V-R(c)`: `cargo test -p c --locked` + `cargo clippy -p c --all-targets --locked -- -D warnings` + `cargo fmt --all --check`
   - `V-SKIN`: `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`(문서 9개 load ok, warn 0, pcall 0, fn fail 0, over 0, 팩 unchanged)
   - `V-SET`: `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 다음 `cd web && bun test tests/dto/client-requests.test.ts`(통과, 골든은 `settings-blob.json` 만 바뀜)
   - `V-SHEET`: `uv run assets/skins/rbms-default/tools/generate-images.py --check`
   - `V-WEB`: `cd web && bun run verify && bun run format:check`
8. 보고 형식: 바뀐 파일 목록, 핵심 판단과 근거(`파일:줄`), 실행한 검증 명령과 실제 결과(통과 수·실패 수), 이 계획과 달라진 점, 남은 위험, 다음 단위가 알아야 할 계약 변경.
9. 멈춤 조건: 계약(이 절의 시그니처·id·소유)과 실제 코드가 맞지 않음, 같은 가정의 수정이 3번 실패, 판정·점수 의미가 바뀌어야 함, 새 의존성이 필요함.

### 5.1 W1 기반

#### K0 확장 id 계약 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 3.3절 모든 상수가 `crates/rbms-skin/src/property/extension.rs`(공간 대역·참조 이미지)와 `apps/rbms-player/src/skin_host/extension.rs`(이름)에 있고, 3.5절 문 다섯 곳이 `implements` 를 쓰며, 모든 새 id 가 라우트돼 부재 값을 답한다. `props.lua` 에 같은 이름의 표(`props.extension` 확장 + 새 `props.extension_number`, `extension_float`, `extension_rate`, `extension_text`, `extension_image`, `extension_event`)가 있다 |
| 소유 | `crates/rbms-skin/src/property/{extension.rs(새), mod.rs}`, `crates/rbms-skin/src/lua/main_state.rs`(`:855-870` 두 함수), `crates/rbms-render/src/skin_render/{draw.rs, text.rs, songlist/build.rs}`(3.5절 각 한 줄), `apps/rbms-player/src/skin_host/{extension.rs, mod.rs, tests.rs}`, `assets/skins/rbms-default/lua/props.lua` |
| 읽기 전용 | `crates/rbms-skin/src/property/names.rs`, `docs/acknowledge/2026-10-22-adr-rbms-extension-ids.md`, `stage/select/events.rs`·`events_tests.rs` |
| 금지 | 클러스터 응답 구현(값 채우기), 3.3절에 없는 번호, `reference_implements` 의 의미 변경 |
| 엣지 | (1) 새 id 는 그 공간에서 `reference_implements` 가 거짓이어야 한다(시험). (2) Float 와 Rate 번호가 겹치지 않는다(시험). (3) 불린 음수 id 는 부정(기존 규칙). (4) `events_tests.rs:493-496` "브라우저가 처리하는 모든 이벤트는 라우트된다" 가 계속 통과(라우트만 더하고 처리기는 없음). (5) `skin_host/tests.rs:451-456` 의 21099 무라우트 단언 유지. (6) `skin-dump --strict` 가 확장 대역 `ref` 텍스트에 경고를 내지 않는다 |
| 검증 | `V-R(rbms-skin)`, `V-R(rbms-render)`, `V-R(rbms-player)`, `V-SKIN` |

#### I0 다국어 기반 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | i18n 보고서 4.1절 1~5·8 과 용어집. (1) `rbms_model::Language { English, Korean, Japanese }`, `ALL`, `id()`, `Default = English`, `from_locale(&str)`. (2) `translations!` 매크로(항목마다 세 문자열 필수, `Text::get(lang)`, `fill(lang, &[..])`) + 자리표 집합·빈 문자열·앞뒤 공백·영어 복사 검사 도우미 시험. (3) `display.language`(`AUTO`·`ENGLISH`·`KOREAN`·`JAPANESE` 토큰, `#[serde(default)]` = AUTO, 스키마 6 유지), `SettingId::Language`(DISPLAY 탭 맨 위 LANGUAGE 구역, 선택지 AUTO/ENGLISH/한국어/日本語), `SETTING_COUNT` 98. (4) `TextContext` 가 OS 로캘 원문을 보관(`font.rs:332-336`)하고 `system_locale()`. (5) `RenderCtx.language` + 스레드 로컬(`theme.rs:368` 방식, 전역 원자값 금지). (6) 플레이어 `app_language.rs`(새): 설정 → `RBMS_LANGUAGE`(ko/ja/en) → OS 로캘 순으로 해석, 같은 프레임 적용, 캐시 문자열 다시 만들 신호. (7) 빈 카탈로그 모듈: `apps/rbms-player/src/text/{mod,settings,select,menus,notices,ranking,layout_play}.rs`, `crates/rbms-render/src/text/{mod,select,result,hud}.rs`. (8) 용어집 초안 `docs/reference/2026-10-24-i18n-glossary.md`(보고서 4.6절 + 1.2절 결정 Q4·Q5) |
| 소유 | `crates/rbms-model/src/{lib.rs, language.rs, translations.rs}`, `crates/rbms-config/src/{schema.rs, settings.rs, lib.rs, language.rs}`, `crates/rbms-render/src/{font.rs, ctx.rs, theme.rs, lib.rs, text/*}`, `apps/rbms-player/src/{app_language.rs, text/*}`, `apps/rbms-player/src/lib.rs`(최소: `mod` 줄, 필드 1, 초기화 1, 적용 호출 1), `apps/rbms-player/src/stage/settings/catalog.rs`(새 행 설명만), `contract/client-requests/settings-blob.json`, 용어집 문서 |
| 읽기 전용 | `apps/rbms-player/src/ir_sync.rs`(`keep_local` 에 넣지 않음, `:57-73`), `settings_ui.rs`, `stage/settings/info.rs` |
| 금지 | 기존 문자열 번역(I2~I7 몫), `rbms-config` 의 영어 정본 값 바꾸기(설정 화면이 문자열 비교를 한다, `settings_ui.rs:197-202`) |
| 엣지 | 옛 `settings.ron`(필드 없음) → AUTO, 잘못된 토큰은 `window_mode_token`(`schema.rs:759`) 선례대로, `en-KR` → 영어, `zh-TW`·`fr`·빈 문자열 → 영어, 시험은 영어를 명시(기존 골든 서명 그대로), 병렬 시험 간섭 없음 |
| 검증 | `V-R(rbms-model)`, `V-R(rbms-config)`, `V-R(rbms-render)`, `V-R(rbms-player)`, `V-SET` |

#### R1 레이더 계산 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | NOTES RADAR 보고서 4.1·4.2절. 공개 API(메인이 고정): `pub struct RadarRaw { notes, peak, scratch, soflan, charge, chord: f64 }`, `pub enum RadarAxis`(위 순서), `pub fn radar_raw(model: &Model, mode: &Mode) -> Option<RadarRaw>`(9키·24키 `None`), `pub fn radar_display(raw: &RadarRaw, mode: &Mode) -> [f32; 6]`(SP·DP 구간표, 0~200 자름), `pub const RADAR_VERSION: u32` |
| 소유 | `crates/rbms-chart/src/{radar.rs(새), radar/tests.rs(새), lib.rs(mod·재수출)}` |
| 읽기 전용 | `crates/rbms-model/src/lib.rs:69-192`(노트 종류, `counts_as_playable_note`), `mode.rs` |
| 금지 | `note_density` 변경, `PARSER_VERSION` 변경 |
| 실행 순서 | 1) 구간표 원문(https://raw.githubusercontent.com/UkiyoAmeto/iidx33-notesradar/main/docs/algorithm.md 2.1절)을 보고서 3.3절 표와 대조, 못 읽으면 보고서 값을 쓰고 "대조 못 함"을 보고. 2) 축별 원시 지표. 3) 정규화. 4) 시험 |
| 엣지 | 보고서 4.8절 표 전부(롱노트 `Undefined` 와 `Cn`·`Hcn` 같은 값, 노트 0·1개, 시간 0 → 하한 1초, DP 합산, 장난 BPM, NaN·무한대 없음, 구간 끝점 100·150·175·200) |
| 검증 | `V-R(rbms-chart)` |

#### RK-D 랭킹 클라이언트 데이터 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 랭킹 보고서 4.1절 "데이터와 API" 1·2 와 2.5절 틈 1·2·4·5. (1) `RankingRow` 에 `player_id`, `dans`, `total_notes`, `played_at`, `is_you`, `is_rival`, `scratch_auto`, `replay_id`. (2) 쪽 받기 `chart_ranking_page(limit = 100, page)`, 내 순위가 쪽 밖이면 그 쪽 한 번 더, 다음 쪽 API. (3) `chart_replays` 대신 `extra.replay_id`. (4) 10분 만료, 제출이 받아들여지면 그 곡만 무효화(`app_ranking.rs:263` `poll_submit`), `Failed` 는 30초 뒤 다시. (5) 소비자 의미 유지(`you`·`rivals`·`rows`). (6) `extra.dans` 모양 `[{system, name, label}]`(RK-W 와 같은 약속) |
| 소유 | `apps/rbms-player/src/{ir_ranking.rs, app_ranking.rs}`, `crates/rbms-ir/src/dto/score.rs`(`extra` 읽기 도우미만) |
| 읽기 전용 | 소비자 `app_result.rs:723-731`, `ghost.rs:146-160`, `stage/result.rs:932-941`, `skin_host/select/bridge.rs:90`, `stage/select/mod.rs:882-897` |
| 금지 | 선곡 입력·그리기 변경(RK-M·RK-V 몫), 순위 정렬 바꾸기(Q8) |
| 엣지 | `extra.dans` 가 없는 옛 서버, `total_notes` 0, 로그아웃(guest), 서버 없음, 받는 중 프로필 전환(`reset_ranking` `:131`), 한 번에 하나만 받기 규칙(`:70-75` 주석) |
| 검증 | `V-R(rbms-ir)`, `V-R(rbms-player)` |

#### RK-W 웹 순위 단위 배지 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | `GET /api/charts/{hash}/ranking` 의 줄마다 `extra.dans`(프로필과 같은 모양, 없으면 빈 배열)를 `danService.listDisplayed(userIds)`(`dan.service.ts:147`) 한 번으로 붙인다. 순위·정렬·`rank` 의미와 캐시 태그(`read-cache.ts:40-45`) 그대로. 마이그레이션 없음. `X-Total-Count` 는 넣지 않는다 |
| 소유 | `web/src/server/service/domain/score/score.service.ts`(`getRanking` `:496` 주변), `web/src/server/compose/index.ts`(순위 쿼리가 사용자 id 를 돌려주게, 단위 조회 주입), `web/src/server/dto/score.ts`, 새 시험 `web/tests/service/ranking-dans.test.ts`, `docs/backend/api-spec.md`(순위 절만) |
| 금지 | 순위 SQL 의 정렬 변경, 새 의존성, DB 변경 |
| 엣지 | 단위 없는 사용자, 삭제된 사용자, 라이벌 거르기(`rival_of`), `lnmode`, 1~500 `limit` |
| 검증 | `V-WEB`(메인은 W1 게이트에서 `RBMS_DB_E2E=1 bun test tests/db` 를 Docker 로 한 번) |

#### L1 배치 빌리기 코어 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 리플레이 보고서 4.1절 1단계 코어와 4.3절. 계약(메인이 고정): `pub(crate) struct LayoutRequest { md5: String, random_label: String, seed: u64, source: String }`, `pub(crate) struct BorrowedLayout { random: NoteOption, seed: u64, source: String }`, `pub(crate) enum LayoutRefusal { ChartMismatch { replay: String, chart: String }, UnknownLayout, UnsupportedOption(String), NotFreePlay }`, `impl AppShared { pub(crate) fn borrow_layout(&mut self, request: LayoutRequest, chart_index: usize) -> Result<Transition, LayoutRefusal>; pub(crate) fn borrowed_layout(&self) -> Option<&BorrowedLayout> }`. 옵션 라벨은 서버 변환(`ir_replay.rs:131-136` 의 Converge→OFF) **전** 문자열로 받는다. `load()`(`app_play.rs:470-480`)가 `play_as_recorded`·`retry_seed` 보다 먼저 빌린 값을 쓰고 플레이어 RANDOM 은 `own_play_options` 로 보관. 같은 배치 재시도(`stage/result.rs:1019-1031`, 실패 직후도 `app_result::retry` 를 거침 `stage/play/mod.rs:1449-1456`)는 유지, 선곡·다음 곡·다른 배치(`leave_play` `app_play.rs:824`, `next_song`)는 버림 |
| 소유 | `apps/rbms-player/src/borrowed_layout.rs`(새: 타입·검사·시험), `app_play.rs`, `app_result.rs`, `stage/result.rs`, `lib.rs`(최소: 필드 1, 초기화 1, `mod` 줄) |
| 읽기 전용 | `stage/play/mod.rs`(RP-P 몫), `stage/select/*`(L2 몫), `crates/rbms-chart/src/shuffle.rs` |
| 금지 | 저장·전송 본문 형식 변경(계약 골든 불변), 게이지·SCRATCH AUTO·LN MODE 복사 |
| 엣지 | md5 대소문자 무시, `seed == 0` 이고 옵션이 OFF 아님 → 거절, OFF 는 허용, Converge 거절, 코스·연습·오토플레이 거절, DP 허용, 정의 안 된 LN + 노트 기반 옵션 + LN MODE 다름 → 알림만, 판 도중 설정 저장 시 플레이어 RANDOM 이 남음(`settings_to_save` `:100`) |
| 검증 | `V-R(rbms-player)`(계약 시험 `contract_requests` 의 골든이 바뀌지 않아야 한다) |

### 5.2 W2 기능 엔진

#### R2 곡 DB 레이더 + CLI 출력 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | NOTES RADAR 보고서 4.3절 A. 새 표 `song_radar`(`path` 기본키, `song(path)` 참조 `ON DELETE CASCADE`, `version`, `applies`, 원시 6열), `CREATE TABLE IF NOT EXISTS`, `SCHEMA_VERSION` 1·`PARSER_VERSION` 5 유지(`songdb/schema.rs:10,31`). 스캔이 `DetailRow` 와 같은 배치로 쓴다. `ChartDetail` 에 레이더(원시+버전+적용 여부). 선택(같은 단위): `rbms-cli` 차트 요약에 원시·표시값 |
| 소유 | `crates/rbms-library/src/{lib.rs, scan.rs, songdb.rs, songdb/rows.rs, songdb/schema.sql, songdb/tests.rs, tests.rs}`, `apps/rbms-cli/src/` 의 차트 요약 모듈(구현 때 경로 확인) |
| 읽기 전용 | `crates/rbms-chart/src/radar.rs`(R1) |
| 엣지 | 9키·24키 `applies = 0`, 표가 없는 옛 DB, 외래 키 설정 확인, 버전이 다른 행, 곡 삭제 |
| 검증 | `V-R(rbms-library)`, CLI 를 고치면 `V-R(rbms-cli)` |

#### R4 레이더 칠하기 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | `ReferenceImage::NotesRadar`(destination `"-21020"`, K0 상수), `ReferenceImages.notes_radar: Option<[f32; 6]>`(표시값 0~200), 256x256 CPU `Pixmap` 에 흰 불투명 볼록 육각형(중심 128, R 124, 위에서 시계 방향 60도), 세로 4배 샘플 앤티앨리어싱, 값이 바뀔 때만 다시 칠함. 값이 없으면 그리지 않음 |
| 소유 | `crates/rbms-render/src/skin_render/{refs.rs, object.rs, graphs/mod.rs, graphs/radar.rs(새)}` + 시험 |
| 읽기 전용 | `graphs/pixmap.rs`, `hit_error.rs:166-183`(삼각형 칠하기 선례), `frame.rs`(바꾸지 않음: 값은 `ReferenceImages` 로) |
| 엣지 | 전부 0, NaN → 0, 200 초과 자름, 다른 음수 id 는 기존대로 그리지 않음 |
| 검증 | `V-R(rbms-render)`(CPU 골든 화소 몇 개) |

#### RK-M 랭킹 모드 입력 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 랭킹 보고서 4.1절 "들어가기·나가기", "모드 안 조작", 질문 1·2·3·5·6·7a. 차트 막대 + 서버 설정 + 패널·모달·검색 없음에서 파란 건반(`blue_keys`, `stage/result.rs:161`)을 `SCROLL_DURATION_LOW_MS`(300, `songlist/bars.rs:106`) 이상 누르면 열림, 짧은 누름은 뗄 때 지금 동작, 키보드는 S·D(6번 F 는 즐겨찾기 그대로), 9키는 버튼 2·8, 5·10키 2·4, DP 양쪽. 모드 안: 턴테이블·위아래·휠 이동, 흰 건반·Enter 리플레이(기존 `play_ranking_replay`), SELECT 내 줄, START 다음 쪽, Q·E 프로필, 닫기 파란 건반 짧게·Esc·`I`·왼쪽. 곡에 5초 머물면 자동 받기(패널 닫혀도), 열면 즉시. 이벤트 21002, 불린 21105. 설정 RANKING KEYS(켬 기본, 끄면 지금 동작 그대로, `SETTING_COUNT` 99) |
| 소유 | `apps/rbms-player/src/stage/select/{mod.rs(주), skinned.rs, keys.rs, events.rs(21002), ranking_mode.rs(새), ranking_mode_tests.rs(새)}`, `apps/rbms-player/src/skin_host/select.rs`(21105 만), `apps/rbms-player/src/ir_ranking_view.rs`(`PanelAction`·`panel_action` `:14-39` 만), `crates/rbms-config/src/{schema.rs, settings.rs}`, `stage/settings/catalog.rs`(새 행 설명만), `contract/client-requests/settings-blob.json` |
| 읽기 전용 | `stage/result.rs`, `ir_ranking.rs`·`app_ranking.rs`(RK-D API) |
| 금지 | 오버레이 그리기(RK-V), 순위 슬롯 값(RK-S) |
| 엣지 | 패널(START·SELECT)이 올라온 동안 시작된 누름은 닫힌 뒤 번지지 않음(`SelectKeys::settled` 규칙), 폴더·코스·없는 곡 막대는 즉시 기존 동작, 서버 없음, 9키 인덱스 확인(`keys.rs:132-142`), 키보드 레인 키는 `key_index_pressed`(`app_input.rs:348`), 내장 선곡(스킨 없음)은 컨트롤러 레인만(`mod.rs:1297-1338`) |
| 검증 | `V-R(rbms-config)`, `V-R(rbms-player)`, `V-SET` |

#### RK-V 랭킹 오버레이 그리기 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 줄: 순위 \| 이름 + 단위 배지 \| DJ LEVEL(`rbms_render::result::dj_rank`, `result.rs:131-139`) \| EX \| 램프(등급 포함) \| 날짜 \| REP. 머리: 곡 이름과 `YOU #r`. 내 줄·라이벌 줄 강조, 쪽 안내. 문자열은 `text/ranking.rs` 카탈로그. 캡처에 오버레이 항목 추가 |
| 소유 | `apps/rbms-player/src/ir_ranking_view.rs`(`:14-39` 제외 전부), `stage/select/overlay.rs`, `stage/select/overlay_tests.rs`, `stage/capture.rs`(오버레이 캡처 항목만), `apps/rbms-player/src/text/ranking.rs` |
| 엣지 | `total_notes` 0 → 등급 비움, 긴 이름·CJK 이름 자르기, 단위 없음, 17줄, 로딩·실패·서버 없음 줄 |
| 검증 | `V-R(rbms-player)` |

#### RK-S 순위 슬롯 스킨 값 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 선곡에서 원본 id 에 실제 값: 정수 380~389(EX)·390~399(순위), 이미지 인덱스 380~389(종류 1 나·2 라이벌)·390~399(램프), 텍스트 120~129(이름), Rate 8 읽기·쓰기(선곡도 받음, `skin_host/writers.rs:19`), 총원·램프 분포를 받은 쪽의 실제 값으로. 지금 부재로 답하는 곳 `skin_host/ir.rs:249,414,432,451` |
| 소유 | `apps/rbms-player/src/skin_host/{ir.rs, ir/tests.rs, select/bridge.rs, writers.rs}` + 시험 |
| 엣지 | 보드 없음 → 부재, 위치가 줄 수 넘음, 로그아웃, 결과 화면 기존 동작 불변 |
| 검증 | `V-R(rbms-player)`, `V-SKIN` |

#### RP-P 재생 중 배치 플레이·고스트·배치 골든 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | (1) `Shortcut::ReplayPlayLayout`(`ALL` 31→32, 기본 키는 `Control` 키와 겹치지 않게, 라벨·`config_key`·그룹 Replay), `analysis_shortcut`(`stage/play/mod.rs:765`)이 L1 `borrow_layout` 을 부름. (2) G1 고스트: `GhostSources`(`ghost.rs:120`)에 "이번 판이 빌린 로컬 리플레이" 칸, `attach_ghosts`(`stage/play/mod.rs:488`)가 이 판 TARGET 을 그 페이스로(이름 `REPLAY <출처>`, 카탈로그 `text/layout_play.rs`), 결과 화면 TARGET 도 같은 값(`app_result.rs:695` `run_target`), 검사 실패면 비례로 물러나고 콘솔에 이유. 서버 리플레이에서 빌린 판은 고스트 없음. (3) L4: 옵션 8개 x 7K·14K·9K 순열 값을 고정한 골든 시험 |
| 소유 | `apps/rbms-player/src/{keyconfig.rs, stage/play/mod.rs, ghost.rs, target.rs, skin_host/score/standing.rs, text/layout_play.rs}`, `app_result.rs`(`run_target` 만), `crates/rbms-chart/src/shuffle.rs`(`#[cfg(test)]` 모듈만) |
| 읽기 전용 | `borrowed_layout.rs`(L1) |
| 엣지 | 옛 키 파일(항목 없음 → 출하 키, 이미 쓰인 키면 비움, `keyconfig.rs:1369-1375`(보고서)), FRAME 시계 거절(`ghost.rs:259` `rejudge`), 노트 수·최종 EX 불일치, 설정 TARGET 행 불변(한 판 한정) |
| 검증 | `V-R(rbms-player)`, `V-R(rbms-chart)` |

### 5.3 W3 선곡 값

#### A3 곡 분석 연결 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | (1) `chart_detail`(`library.rs:112-155`)이 레이더 행이 없거나 버전이 다르면 다시 계산·저장. (2) `ChartMeta` 에 레이더 표시값·레인별 노트 수·종류별 노트 수(350~353)·주 BPM(92). (3) 정수·Float 21020~21025, 정수·Rate 21000~21015 를 Chart 클러스터가 답함. (4) `ReferenceImages.notes_radar` 채움. (5) 선곡 프레임 `FrameData.series` 의 `notes`·`bpm`(`overview.rs:157` `series()`)으로 `judgegraph` type 0·`bpmgraph` 가 선곡에서 그려짐. (6) 새 분석은 작업자 스레드(`stage/select/analysis.rs`), 프레임 스레드에서 파싱하지 않음, 낡은 요청은 버림. (7) `skin_preview.rs` 합성값 |
| 소유 | `apps/rbms-player/src/{library.rs, skin_host/chart.rs, skin_host/chart/tests.rs, skin_host/overview.rs, stage/select/skin_preview.rs, stage/select/analysis.rs(새)}`, `stage/select/list.rs`(`chart_under_cursor` `:570-594` 만), `stage/select/mod.rs`(최소: 필드 1, 폴링 호출 1, `:1442` `FrameData` 구성 줄의 `series`·`images` 인자) |
| 엣지 | 드라이브 빠짐 → 부재, 9키·24키 레이더 부재, 큰 곡, 커서 빠른 이동, DP 16레인, 프레임 비용(측정은 DOC 의 테스트 부채로) |
| 검증 | `V-R(rbms-player)`, `V-SKIN` |

#### BAR 막대 등급 배지·썸네일 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 선곡 UI 보고서 4.2·4.8절. `SongList` 에 `rank: Vec<Destination>`(8개, F~AAA = `dj_rank` 인덱스 0~7), `thumb: Option<Destination>`(`objects.rs:597`, 로더·serde 기본). 그리기 순서 램프 → 등급 → 레벨(원본 패스 순서 사이). `SongBar` 에 `rank: Option<u8>`, `thumb: Option<TextureId>`(`bars.rs:203`), 썸네일은 `SongBars`(`bars.rs:456`)로 실음(`frame.rs` 는 바꾸지 않음). 등급은 정보창 RANK 와 같은 기준(보조 판 제외 `best_ex_for_md5`, `crates/rbms-store/src/score.rs:350`)으로 `best_score`(`list.rs:698`)를 맞춤. 썸네일 작업자 `stage/select/thumbs.rs`: 보이는 막대 그림을 128x128 중앙 자르기, 최근 48장 LRU, 텍스처 풀 24, 지나간 요청 건너뜀, 디코드 원본 버림, 새 의존성 없음(`image` 0.25 의 `imageops`, 쓸 수 없으면 직접 박스 필터) |
| 소유 | `crates/rbms-skin/src/{model/objects.rs, loader/from_lua.rs}`, `crates/rbms-render/src/skin_render/{songlist.rs, songlist/bars.rs, songlist/build.rs, songlist/tests.rs}`, `apps/rbms-player/src/stage/select/{scene.rs(`wheel_bar` `:86-99`), thumbs.rs(새), thumbs_tests.rs(새)}`, `list.rs`(`best_score` 만), `mod.rs`(최소) |
| 엣지 | 기록 없는 막대·폴더·코스 막대 배지 없음, 그림 없음, 외부 스킨(키 없음) 영향 없음, JSON 스킨, 휠 빠르게 돌림 |
| 검증 | `V-R(rbms-skin)`, `V-R(rbms-render)`, `V-R(rbms-player)`, `V-SKIN` |

#### S3 선곡 보조 값 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 3.3절 Select 클러스터 id 전부. 형제 차트(같은 곡 폴더·같은 제목·같은 모드, 난이도 1~5 칸), 열린 폴더 램프 분포(정수·Rate), MY RECORDS 6줄(BEST 1 + 최근 5판, 정수 24개 + 날짜 텍스트 6개), 이벤트 21010~21014 커서 이동. 계산은 새 파일 `stage/select/browse_facts.rs` |
| 소유 | `apps/rbms-player/src/stage/select/{browse_facts.rs(새), browse_facts_tests.rs(새), events.rs(21010~21014)}`, `skin_host/{select.rs, select/shown.rs, select/tests.rs}`, `mod.rs`(최소) |
| 엣지 | 코스·폴더 막대 부재, 기록 없음·5판 미만, 묶음 팩에서 형제 판단이 틀릴 수 있음(보고서 6절 3), 슬롯 없는 이벤트 무시 |
| 검증 | `V-R(rbms-player)` |

#### L2 리플레이 선곡 진입 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 기록 모달 버튼 "PLAY THIS LAYOUT"(카탈로그) + 키(`mod.rs:974-983`, `crates/rbms-render/src/select.rs:690-696`, `SelectHot::ModalLayout`, `mod.rs:1366`·`:1475` 매핑), IR 패널 `PanelAction::PlayLayout` + 힌트(`ir_ranking_view.rs`), 내려받기 갈래 "배치만"(`poll_replay_download` `mod.rs:900-937`, 받은 본문의 `chart.md5/sha256` 대조) → L1 `borrow_layout`. 새 판의 차트 경로는 포커스된 라이브러리 항목 것 |
| 소유 | `apps/rbms-player/src/stage/select/mod.rs`(주: 위 구역), `stage/select/recorded_tests.rs` 또는 새 `layout_tests.rs`, `ir_ranking_view.rs`(`PlayLayout` 만), `crates/rbms-render/src/select.rs`(모달 버튼), `crates/rbms-render/src/text/select.rs`(항목 추가), `apps/rbms-player/src/text/layout_play.rs`, `overlay.rs`(최소, 필요할 때만) |
| 엣지 | 받기 실패, 리플레이 없는 행, 시드 0, Converge, 코스 막대, 라이브러리에 없는 곡, 랭킹 모드 안의 키 충돌(RK-M 의 흰 건반=재생과 겹치지 않는 키를 고르고 보고) |
| 검증 | `V-R(rbms-render)`, `V-R(rbms-player)` |

#### I1 스킨 언어·`t(key)` (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 불린 21150~21152 를 System 클러스터가 모든 화면에서 답함(`skin_host/system.rs`). `lua/i18n.lua`: `t(key)` 가 `main_state.option(21150..)` 로 언어를 읽는 함수 필드 도우미, 영역 표 `lua/i18n/{common, select_bar, select_info, select_other, play, result, decide}.lua`(빈 뼈대). 기본 팩 시험에 "모든 키가 en·ko·ja 셋을 가짐" 검사 |
| 소유 | `apps/rbms-player/src/skin_host/system.rs`(+ 시험), `assets/skins/rbms-default/lua/{i18n.lua(새), i18n/*.lua(새)}`, `crates/rbms-skin/tests/default_pack.rs`(검사 추가만) |
| 엣지 | 헤더 읽기 단계(`main_state` 빔) → 영어, `op` 에는 못 씀(함수 필드만), 언어 변경 시 다음 프레임 반영 |
| 검증 | `V-R(rbms-player)`, `cargo test -p rbms-skin --locked --test default_pack`, `V-SKIN` |

#### D0 기본 스킨 색·배치 토큰 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | `theme.lua` 에 이름 있는 토큰만 더함: 등급 F~AAA 색, 램프 띠 색(기존 램프 색 재사용 확인), 메달 금·은·동(코스 트로피 `TROPHY_COLORS` `lua/select/list.lua:19` 와 같은 값), 레이더 노랑 + 알파, 레이더 축 라벨 색 6, 랭킹 강조 색, 새 IIDX 식 배치 상수(선곡 UI 보고서 4.4절 표의 좌표를 이름 있는 값으로). 바탕·면·글자의 모노톤은 그대로(M5) |
| 소유 | `assets/skins/rbms-default/lua/theme.lua` |
| 금지 | 기존 토큰 값 변경(화면이 바뀜) |
| 검증 | `V-SKIN`(팩 unchanged 는 기대하지 않음, 경고 0) |

### 5.4 W4 화면과 번역 1차

#### D1 선곡 목록·머리·바닥·시트 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 막대 안: 등급 배지(0..56, `rank` 키), 램프 띠(56..64), 레벨 칸(64..128), 제목(144..), 썸네일 64x64(664..728, `thumb` 키), 라벨은 썸네일 왼쪽. 선택 막대 돌출·높이·테두리. 머리: PLAY STYLE 칩(SP/DP), FREE PLAY/COURSE 칩, 시각 칩(정수 24·25), 위치 경로(`STRING_DIRECTORY` 를 `>` 로), 정렬 라벨, 줄인 검색 상자. 스크롤 막대 트랙을 열린 폴더 램프 누적(Rate 21040~21050)으로. 바닥: 안내 2줄을 키 이름 상자 모양, AUTOPLAY·PRACTICE·FAVORITE 옆에 RANKING 단추(이벤트 21002, 불린 21105 로 켜짐). 시트에 등급 배지 8조각. 새 문자열은 `t(key)` 와 `lua/i18n/select_bar.lua` |
| 소유 | `assets/skins/rbms-default/lua/select/{list.lua, controls.lua, main.lua}`, `lua/i18n/select_bar.lua`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` |
| 읽기 전용 | `theme.lua`(D0), `props.lua`(K0), `info.lua`(D2) |
| 엣지 | 폴더·코스 막대(배지 없음), 긴 제목·CJK, `rbms_extensions` 헤더 유지(`main.lua:20`), 부재 값 |
| 검증 | `V-SKIN`, `V-SHEET` |

#### D2 정보 패널·NOTES RADAR·기록 (Opus high)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | 선곡 UI 보고서 4.4절 레이아웃. 곡 그림 256 정사각(64,352). CHART DATA(352,352): 난이도 탭 5칸(정수 21030~21034, 불린 21115~21119, 이벤트 21010~21014), TOTAL NOTES·LEVEL·BPM, 밀도 그래프(`judgegraph` type 0)와 BPM 그래프, 레인 막대(정수·Rate 21000~21015), LENGTH·JUDGE·TOTAL·KEYS 작은 글자. NOTES RADAR(64,640, 256x256): 제목 "NOTES RADAR", 격자(외곽선·세 축선)를 회전 사각형으로(`ui.lua:109` `angle`), 다각형은 참조 이미지 `-21020` 에 노랑 반투명, 축 라벨 6색, 값 정수 21020~21025, 최대 축 꼭짓점에 별(동률이면 화면 순서의 앞 축). MY SCORE(352,640): SCORE, MISS COUNT, 큰 등급, 램프 이름, 다음 등급까지(`NUMBER_DIFF_NEXTRANK`), F~AAA 진행 막대(`RATE_SCORE`), MAX COMBO·PLAY·CLEAR COUNT·RATE, 판정 5행. SCORE TARGETS(AAA·AA·A 필요 EX = 최대 EX x 8/9·7/9·6/9, 금·은·동, 달성 표시)와 CLEAR RATE(CLEAR COUNT / PLAY COUNT)를 기록 패널 아래 한 줄. MY RECORDS 6줄(784,640): 정수 21060~21083, 텍스트 21000~21005. 랭킹 모드(21105)에서는 같은 자리가 IR RANKING 6줄(텍스트 120~129, 정수 380~399, 이미지 인덱스 390~399)(M8). 폴더·코스 정보(`info.lua:80-98`)는 그대로. 새 시트 조각을 쓰지 않는다(사각형·글자·숫자·참조 이미지·그래프만). `apps/rbms-cli/scenarios/select.json` 에 D1·D2 가 읽는 새 id 값 |
| 소유 | `assets/skins/rbms-default/lua/select/{info.lua, radar.lua(새), records.lua(새)}`, `lua/i18n/select_info.lua`, `apps/rbms-cli/scenarios/select.json` |
| 읽기 전용 | `theme.lua`, `props.lua`, `main.lua`(info.build 호출이 이미 있다 `main.lua:51`) |
| 엣지 | 부재 값이면 그리지 않음, 9키 레이더 없음, 기록 0줄, DP 레인 16, 오버레이(앱 그림, `overlay.rs:97-100` 의 왼쪽 자리)가 정보창을 덮는 모양 |
| 검증 | `V-SKIN`, 메인 게이트의 선곡 캡처 |

#### I2 설정 화면 번역 (Sonnet xhigh)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | i18n 보고서 7절 I2. 줄 구조체에 화면용 `shown`, 로직 비교는 영어 정본(`settings_ui.rs:197-202`, `stage/settings/info.rs:139,156`), `SettingId` exhaustive `match`(LANGUAGE·RANKING KEYS 포함), 선택지는 인덱스로 번역, 조각 조립 문장을 문장 단위로(`stage/settings.rs:896,899`) |
| 소유 | `apps/rbms-player/src/{stage/settings/*, stage/settings.rs, settings_view.rs, settings_ui.rs, ir_panel.rs, app_options.rs, number_row.rs, text/settings.rs}` |
| 엣지 | 칸 폭(`settings_view.rs:155,164`), 언어 바꾼 프레임에 다시 만듦, `rbms-cli config` 출력 불변 |
| 검증 | `V-R(rbms-player)` |

#### I3 렌더 크레이트 번역 (Sonnet xhigh)

| 소유 | `crates/rbms-render/src/{select.rs, result.rs, hud.rs, toast.rs, result/graphs.rs, result/grade.rs, text/*}` |
| --- | --- |
| 목표·완료 | 고정 문구를 카탈로그로, 도메인 용어(1.2절 Q4)는 원어 유지, 영어에서 기존 골든 서명 불변 |
| 엣지 | Inter 에 없는 기호 3개(`select.rs:320,342,408`(보고서)) 그대로 |
| 검증 | `V-R(rbms-render)` |

#### I5 메뉴·표·폴더·코스 번역 (Sonnet xhigh)

| 소유 | `apps/rbms-player/src/{stage/tables.rs, stage/folders.rs, stage/loading.rs, stage/practice.rs, stage/practice/*, stage/play_menu.rs, stage/play_menu/*, stage/skin_menu.rs, stage/skin_menu/*, stage/preview_menu.rs, stage/keyconfig/table.rs, practice.rs, course_ui.rs, format.rs, target.rs, ir_outcome.rs, text/menus.rs}`, `crates/rbms-course/src/model.rs` |
| --- | --- |
| 목표·완료 | 메뉴·표·폴더·로딩·연습·코스·결과 줄의 번역 대상. 키 이름·램프·게이지 이름은 원어. 단수·복수 문장 둘(`skin_menu.rs:802-803`(보고서))을 한 문장 템플릿으로 |
| 검증 | `V-R(rbms-player)`, `V-R(rbms-course)` |

#### I6 알림 번역 (Sonnet xhigh)

| 소유 | `apps/rbms-player/src/{notify.rs, toast.rs, dialog.rs, app_network.rs, record_sync.rs, scoredb_store.rs, tablesrc.rs, user_folders.rs, text/notices.rs}`, `app_play.rs`·`app_result.rs`·`app_ranking.rs`·`lib.rs`(알림 줄만) |
| --- | --- |
| 목표·완료 | `notify_text(Level, Text, args)`(큐에는 `Text`+인자, 프레임 스레드에서 현재 언어로, stderr 는 영어 즉시), 행동 알림 약 60개 번역, 진단성 약 90개 영어 유지, rfd 대화상자 제목. `stage/**` 안 알림은 그 스테이지 단위 몫(I2·I4·I5) |
| 엣지 | 작업자 스레드 알림, 88자 자르기(`toast.rs:30`(보고서)), 서버 한국어 오류 문구 그대로(Q8) |
| 검증 | `V-R(rbms-player)` |

### 5.5 W5 번역 마무리와 웹 기반

#### I4 선곡 스테이지 번역 (Sonnet xhigh)

| 소유 | `apps/rbms-player/src/{stage/select/*, select_tree.rs, ir_ranking_view.rs, text/select.rs}`(`text/ranking.rs`·`text/layout_play.rs` 는 이어 쓰기만) |
| --- | --- |
| 목표·완료 | 가이드·장면·필터·오버레이·다운로드 모달·목록·트리·IR 패널의 남은 고정 문구. 키 이름 유지, 표본 데이터(`skin_preview.rs`) 제외, `scene.rs:259` 류 조립 문장 템플릿화 |
| 검증 | `V-R(rbms-player)` |

#### I7 기본 스킨 문자열 번역 (Sonnet xhigh)

| 소유 | `assets/skins/rbms-default/lua/**`(i18n 영역 표 포함, `props.lua` 는 문자열이 있을 때만), `apps/rbms-cli/scenarios/*.json` |
| --- | --- |
| 목표·완료 | 번역 대상 약 470개를 `t(key)` 로(`words.lua`, `sheet.lua`, `theme.lua` 난이도 이름, `info.lua`, 결과·결정·플레이 머리글). 스킨 메뉴 헤더 이름(`colors.lua`, `layout.lua` 약 160개)은 영어 유지(Q11) |
| 엣지 | 정적 `constantText` 는 문서를 다시 읽을 때만 바뀜 → 함수 필드로, 프레임 비용은 `skin-dump` 표로 확인, 언어별 skin-dump 는 시나리오 불린 21150~21152 를 바꿔 세 번 돌려 결과를 보고(시나리오는 원래 영어로 되돌림) |
| 검증 | `V-SKIN`(세 언어), `cargo test -p rbms-skin --locked --test default_pack` |

#### WI0 웹 다국어 기반 (Opus high, **사용자가 next-intl 추가를 승인한 뒤에만 시작**)

| 항목 | 내용 |
| --- | --- |
| 목표·완료 | next-intl(게시 14일 지난 최신판, Next 16.3.4 호환을 그 문서로 확인), 경로 접두사 없는 쿠키 방식(https://next-intl.dev/docs/getting-started/app-router/without-i18n-routing), 기본 한국어, `Accept-Language` 첫 방문 감지, 언어 선택 위젯, `<html lang>` 동적(`web/src/app/layout.tsx:24`), 메시지 `web/messages/{ko,en,ja}/{common,ai,settings,browse,pages,server}.json`(네임스페이스 고정, common 만 채우고 나머지 `{}`), 서버 컴포넌트가 쿠키를 읽어 생기는 캐시 영향(`cacheLife`·`cacheTag` 를 쓰는 `read-cache.ts`)을 Next 16 문서로 확인해 데이터 캐시는 언어와 무관하게 유지. 라이브러리 선택과 사유는 DOC 의 ADR 로 |
| 소유 | `web/package.json`, `web/bun.lock`, `web/next.config.*`, `web/src/shared/i18n/*`(새), `web/messages/**`(새), `web/src/app/layout.tsx`, `web/src/widgets/locale-switcher/*`(새), `web/src/proxy.ts`(필요할 때만) |
| 엣지 | 쿠키 없는 봇, 지원 밖 언어, 정적 페이지, 테스트 환경 |
| 검증 | `V-WEB`, `cd web && bun run build` |

### 5.6 W6 마지막

| 단위 | 모델 | 소유 | 목표·완료 | 검증 |
| --- | --- | --- | --- | --- |
| WI1 웹 문구 A | Sonnet xhigh | `web/src/widgets/{ai,settings}/**`, `web/messages/*/ai.json`·`settings.json` | 한글 문구를 메시지로(파일 29개), 세 언어 | `V-WEB` |
| WI2 웹 문구 B | Sonnet xhigh | `web/src/widgets/{admin,auth,chart,course,folder,guide,home,leaderboard,player,replay,search,shell,table}/**`, `web/messages/*/browse.json` | 같은 일(파일 39개) | `V-WEB` |
| WI3 웹 문구 C | Sonnet xhigh | `web/src/{features,entities,app,shared}/**`(WI0 소유 파일 제외), `web/messages/*/pages.json` | 같은 일, 메타데이터 제목 포함 | `V-WEB` |
| WI4 웹 서버 문구·AI 언어 | Opus high | `web/src/server/lib/error-message.ts`, `web/src/server/service/domain/ai/*.service.ts`(프롬프트 언어 인자), 오류 코드를 화면에서 번역하는 공용 도우미, `web/messages/*/server.json`, 관련 시험 | API 오류 응답 형식은 바꾸지 않고 웹 화면이 코드로 번역, AI 결과 언어 = UI 언어(M11) | `V-WEB` |
| DOC 문서·ADR·QA·부채 | Sonnet xhigh | `docs/**`(9절 목록), `README.md`, `docs/assets/screenshot-select.jpg` | 9절 | 문서 diff 검토, 링크 경로 확인(`rg` 로 깨진 상대 경로 없음) |

## 6. 메인 통합 게이트 (E)

웨이브마다 병합 뒤 한 번. 관련 입력이 바뀌지 않은 단위의 성공 결과는 재사용하고, 병합에서 손으로 고친 파일에 닿는 것만 다시 돈다.

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
3. `cargo test --workspace --no-fail-fast --locked`(실패 0, 떠는 테스트 0. 마지막 기록 6,644 통과)
4. 기본 스킨이 바뀐 웨이브(W1, W3, W4, W5): `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 시트가 바뀐 W4: `uv run assets/skins/rbms-default/tools/generate-images.py --check`
5. 설정 필드가 바뀐 웨이브(W1, W2): `cd web && bun test tests/dto/client-requests.test.ts`(골든은 단위가 이미 다시 썼다)
6. 웹 단위가 있는 웨이브(W1, W5, W6): `cd web && bun run verify && bun run format:check && bun run build`. W1 은 순위 쿼리가 바뀌므로 `RBMS_DB_E2E=1 bun test tests/db`(Docker `mysql:8.4.11`, 끝나면 컨테이너 삭제)
7. 화면 캡처(선곡·플레이에 닿는 W2·W3·W4·W5): `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<scratch 폴더> cargo test -p rbms-player --lib --locked the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through_its_states the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library` 를 메인이 돌려 사람 눈으로 본다(앱이 기본 표 주소를 GET 한다, HANDOFF 7절). 한국어·일본어 모양은 이 하니스로 볼 수 없다(번들 Inter 만, `font.rs:858-870`(보고서)) → 실기 QA 로 넘김
8. 커밋(7절) 뒤 푸시, `gh run list --branch dev` 로 ci 3 OS 와 web CI 확인. 진행 중인 ci 가 중요하면 로컬 커밋만 해 두고 끝난 뒤 푸시

## 7. 커밋 단위 계획 (F)

메인이 단위 패치마다 `git apply --cached` → `git diff --cached --name-only`(별도 명령)로 목록 확인 → 커밋. 제목은 Conventional Commits, 한국어 설명, AI 트레일러·세션 링크 없음.

| 웨이브 | 단위 | 제목 초안 |
| --- | --- | --- |
| W1 | K0 | `feat(skin): 확장 id 를 정수·실수·문자열·참조 이미지 공간으로 넓히고 다섯 기능의 번호 배정` |
| W1 | I0 | `feat(i18n): 언어 설정과 문자열 카탈로그 기반 추가` |
| W1 | R1 | `feat(chart): NOTES RADAR 6축 계산 추가` |
| W1 | RK-D | `fix(ranking): 순위 캐시 만료·제출 뒤 무효화·실패 재시도와 쪽 받기` |
| W1 | RK-W | `feat(web): 차트 순위 응답 줄에 단위 배지 추가` |
| W1 | L1 | `feat(replay): 리플레이의 배치로 새 판을 시작하는 코어` |
| W2 | R2 | `feat(library): 곡 DB 에 NOTES RADAR 원시값 저장` |
| W2 | R4 | `feat(render): NOTES RADAR 참조 이미지 칠하기` |
| W2 | RK-M | `feat(select): 파란 건반 길게 눌러 여는 랭킹 모드와 RANKING KEYS 설정` |
| W2 | RK-V | `feat(select): 랭킹 오버레이에 DJ LEVEL·단위·날짜·내 줄 강조` |
| W2 | RK-S | `feat(skin): 선곡 스킨의 순위 슬롯에 실제 값 연결` |
| W2 | RP-P | `feat(replay): 리플레이 보는 중 배치 플레이 단축키와 고스트 대결`, `test(chart): 배치 순열 골든 고정` |
| W3 | A3 | `feat(select): 커서 곡의 레이더·레인별 노트·밀도 그래프를 스킨에 연결` |
| W3 | BAR | `feat(skin): 곡 막대 등급 배지와 썸네일 확장 키` |
| W3 | S3 | `feat(select): 형제 난이도·폴더 램프 분포·내 기록 6줄 값` |
| W3 | L2 | `feat(select): 기록 모달과 IR 패널에서 배치 플레이 시작` |
| W3 | I1 | `feat(skin): 스킨에 UI 언어를 알리는 확장 불린과 번역 도우미` |
| W3 | D0 | `style(skin): 기본 스킨 강조색과 IIDX 식 배치 토큰 추가` |
| W4 | D1 | `feat(skin): 기본 스킨 선곡 목록·머리·바닥을 IIDX 식으로` |
| W4 | D2 | `feat(skin): 기본 스킨 선곡 정보창에 NOTES RADAR·MY SCORE·MY RECORDS` |
| W4 | I2·I3·I5·I6 | `feat(i18n): 설정 화면 번역`, `feat(i18n): 내장 화면 번역`, `feat(i18n): 메뉴·표·코스 번역`, `feat(i18n): 알림 번역` |
| W5 | I4·I7 | `feat(i18n): 선곡 화면 번역`, `feat(i18n): 기본 스킨 문자열 번역` |
| W5 | WI0 | `build(web): next-intl 추가와 다국어 기반` |
| W6 | WI1~WI4 | `feat(web): 웹 문구 다국어(AI·설정)`, `…(탐색 화면)`, `…(페이지·공용)`, `feat(web): 서버 오류 문구와 AI 결과 언어` |
| W6 | DOC | `docs: 다섯 기능 ADR·사용자 문서·QA·테스트 부채` |

## 8. 위험과 사용자 답에 따라 바뀌는 부분 (G)

### 8.1 위험

| # | 위험 | 대응 |
| --- | --- | --- |
| 1 | 한국어·일본어 글자의 실제 모양(폭·기준선·소형 가독성)을 하니스로 못 본다 | QA 실기 항목, 문제면 서브셋 글꼴(Q7) 후속 단위 |
| 2 | 레이더 3축(CHARGE·CHORD·SOF-LAN) 스케일 근거 없음, 구간표 한 출처 | 이름 있는 상수, ADR 에 "IIDX 와 다를 수 있음", 테스트 부채 |
| 3 | W3 `stage/select/mod.rs` 병합 충돌(L2 주 + 셋 최소) | 4.1절 규칙, 병합 순서 4.2절, 메인 손 병합 |
| 4 | 길게 누름 300 ms 와 "뗄 때" 동작의 손맛 | RANKING KEYS 끄기 설정, QA |
| 5 | 썸네일·분석 작업자 프레임 비용 미측정 | 작업자 스레드, `skin-dump` 프레임 표, 테스트 부채 |
| 6 | 앱 랭킹 오버레이가 새 레이아웃의 왼쪽 정보창을 덮음(`overlay.rs:97-100`) | M8 로 오른쪽 RIVAL 자리에 순위를 같이 보임, 캡처로 보고 필요하면 DOC 웨이브 전에 RK-V 위치 상수만 수리 |
| 7 | 모노톤 결정의 부분 변경 | ADR, 사용자에게 보고 |
| 8 | next-intl 새 의존성, 서버 컴포넌트의 쿠키 읽기로 생기는 캐시 영향 | 승인 전 WI0 시작 금지, WI0 가 Next 16 문서로 확인 |
| 9 | 순위 100줄 응답 크기 미측정 | 5초 시간 제한·한 번에 하나, 결과를 QA 에 |
| 10 | 형제 차트 묶음 기준이 팩에 따라 틀림 | 보고서 그대로 시작, 테스트 부채 |
| 11 | 번역량(약 8,000~10,500줄)과 일본어 감수 부재 | 용어집 먼저, 되번역, 원어민 감수 요청(게시 승인 뒤) |
| 12 | S-RANDOM 등 노트 기반 옵션 + 정의 안 된 LN + LN MODE 차이로 배치가 다를 수 있음 | 알림만, QA 실기 |
| 13 | 세션 한도로 끊긴 단위 | 같은 worktree 에서 이어받음(HANDOFF 5절) |

### 8.2 답이 오면 고칠 단위

| 질문 | 다른 답이면 | 고칠 단위 |
| --- | --- | --- |
| M1 웹 다국어 빼기 | W5 WI0, W6 WI1~WI4 삭제, DOC 의 웹 ADR 삭제 | 계획만 |
| i18n Q2 기본 언어 | 기본값 하나 | I0 |
| i18n Q3 알림 범위 | 번역 대상 수 | I6 |
| i18n Q4·Q5 용어 | 용어집 → 모든 번역 단위 | I0 용어집, I2~I7, WI1~WI4 |
| i18n Q7 글꼴 번들 | 새 단위(서브셋 글꼴·`font.rs`) W5 추가 | 새 단위 |
| i18n Q8·Q9 | 서버·Discord 문구 | I6, WI4 |
| i18n Q11 헤더 번역 | 로더가 언어를 Lua 에 전달하는 새 단위(`crates/rbms-skin/src/loader/**`) | 새 단위 S-i18n |
| 리플레이 Q1 고스트 빼기 | G1 부분 삭제 | RP-P |
| 리플레이 Q2 진입 길 | 모달·패널·단축키 중 일부 | L2, RP-P |
| 리플레이 Q3 복사 범위 | 게이지·SCRATCH AUTO 복사(점수 의미 확인 필요) | L1 |
| 리플레이 Q4 수명 | 유지 범위 | L1 |
| 리플레이 Q5 표지 | 저장 행에 열(로컬 DB, 서버면 마이그레이션) | L1 + 새 단위 |
| 리플레이 Q6 서버 고스트 | S1(리플레이 판정 설정, 마이그레이션 0013, 계약 골든) | 새 웨이브 |
| 랭킹 Q1~Q3·Q5 키 | 키 규칙 | RK-M |
| 랭킹 Q4 별도 화면 | 새 Stage | RK-M, RK-V |
| 랭킹 Q6 받는 시점 | 자동 받기 삭제 | RK-M, RK-S |
| 랭킹 Q7a 설정 행 빼기 | `SETTING_COUNT` 98 로, 골든 | RK-M |
| 랭킹 Q7b 단위 배지 빼기 | 웹·열 삭제 | RK-W, RK-D, RK-V |
| 랭킹 Q8 EX 정렬 | 모드 안 재정렬 | RK-D, RK-V |
| 레이더 Q1 정의 | 원시 지표·상수 | R1 |
| 레이더 Q2 저장 | 재스캔 또는 커서 계산 | R2, A3 |
| 레이더 Q3·Q4 | 모드·자름 | R1, A3 |
| 레이더 Q5 노출 | id | K0, R4, A3, D2 |
| 레이더 Q6 내장 화면 | 새 단위(`rbms-render` `select.rs`) | I3 와 같은 웨이브의 새 단위 |
| 선곡 Q1 색 | 토큰 값 | D0(+ D1·D2 확인) |
| 선곡 Q2 두 레이아웃 | 헤더 속성으로 둘 유지(Lua·시트 두 배) | D1, D2 |
| 선곡 Q3 막대 확장 빼기 | BAR 삭제, D1 의 배지·썸네일 삭제 | BAR, D1 |
| 선곡 Q5·Q6a RIVAL | IR 상위 상시(서버 요청 증가) 또는 목표 대비 | S3, D2 |
| 선곡 Q6b·Q7 | SCORE TARGETS·CLEAR RATE 산식 | D2 |
| 선곡 Q9 문구 | IIDX 문구 그대로 | D1, D2, I7 |
| 선곡 Q10·Q11 | 내장 렌더러·시각·탭·큰 그림 | D1, D2, I3 |
| M8 랭킹 모드 RIVAL 전환 | 빼기 | D2 |
| M11 웹 방식 | `[locale]` 경로(라우트 구조 변경, 캐시 태그 재검토) | WI0~WI3 |

## 9. 문서·ADR·QA·테스트 부채 (H, W6 DOC)

| 종류 | 파일 | 내용 |
| --- | --- | --- |
| ADR | `docs/acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md` | 2026-10-22 확장 id ADR 의 부분 변경: 정수·Float·Rate·텍스트·참조 이미지 공간 개방, Float·Rate 공유 규칙, 3절 배정표 |
| ADR | `…-adr-ui-language.md` | 다국어 구조(Q2~Q12), 용어집 위치, 번역 범위 |
| ADR | `…-adr-layout-play.md` | 배치 빌리기·고스트(Q1~Q7, M2) |
| ADR | `…-adr-ranking-mode.md` | 파란 건반 길게, 캐시 수리, `extra.dans`, RANKING KEYS |
| ADR | `…-adr-notes-radar.md` | 정의·구간표·저장·노출, IIDX 와 다를 수 있음 |
| ADR | `…-adr-select-iidx-layout.md` | IIDX 식 구조, 2026-10-12 모노톤 결정의 부분 변경(M5), 일반어 문구(M10), M8 |
| ADR | `…-adr-web-i18n.md` | next-intl·쿠키 방식·네임스페이스·AI 언어(M11), 컨벤션 `frontend.md` 8.6 의 선택 기록 |
| 색인 | `docs/acknowledge/README.md` | 2026-10-24 절, 미확인 목록에 1절 M1~M11·보고서 질문 |
| 사용자 문서 | `docs/play-options.md` | 언어 설정, 배치 플레이(진입·재시도·거절), 랭킹 모드(키·설정), 선곡 화면(배지·레이더·기록) |
| 스킨 문서 | `docs/skin.md` | 1.4절 id 표(3절), `:574` "글자는 영어" 수정, `:698` 낡은 한계(밀도 채워짐, 350~353·92 이제 답함), E8·6절 순위 슬롯 연결, `songlist` 확장 키 `rank`·`thumb` |
| 그 밖 | `docs/theme.md`, `docs/architecture.md`, `docs/crates.md`, `docs/development.md`(`RBMS_LANGUAGE`), `docs/font-cjk-support.md`, `docs/reference/ui-design.md:26,220`(레이더 불가 서술), `docs/reference/ir-api.md`(쪽 받기·`extra.dans`), `docs/reference/reference-divergences.md`(랭킹 키·배치 플레이·막대 확장), `docs/web/*`(다국어 구조), `docs/memory/client-versions-and-run-rules.md`(`SETTING_COUNT` 99, 확장 공간, `song_radar`, 단축키 32), `README.md` 기능 목록·선곡 캡처(`docs/assets/screenshot-select.jpg`) | 갱신 |
| QA | `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 3.23절(새) | 세 언어 x 설정·선곡·플레이 메뉴·결과·기본 스킨(macOS·Windows), 배치 플레이(S-RANDOM 리플레이로 같은 배열), 고스트, 랭킹 모드(컨트롤러·키보드·9키, 로그아웃·오프라인·서버 없음, 치고 돌아와 갱신), 레이더 5~10곡(스크래치·LN·BPM 변화·9키·DP), 막대 배지·썸네일·휠 속도, 웹 세 언어 |
| 테스트 부채 | `docs/quality-assurance/2026-10-14-test-debt.md` 3.112~ | CJK 하니스 골든 없음, 레이더 IIDX 대조 자료 없음, 3축 스케일, 썸네일·분석 프레임 비용, 형제 묶음 오판, S-RANDOM·LN MODE 배치 차이, 순위 응답 크기, 서버 리플레이 고스트(S1 보류), 일본어 감수 |
| 상태 | `docs/PROCESS.md` N2~N8 체크·진행 기록, `docs/HANDOFF.md` | 메인 |
