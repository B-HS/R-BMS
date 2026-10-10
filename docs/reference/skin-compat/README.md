# 스킨 호환 조사 — 색인

> 최종 갱신 2026-10-11 · 대응 단계: 웨이브 7A(비트맵 폰트, 디코더 조사) 완료 · 기준 커밋 `9ce92bb` · 레퍼런스 체크아웃 HEAD `8320241d`
> 작업 체크리스트는 `docs/PROCESS.md` "레퍼런스 형식 Lua 스킨 완전 호환과 기본 스킨 교체" 절이다.

이 폴더는 외부 풀 Lua 스킨(ModernChic)을 R-BMS 가 수정 없이 읽어 레퍼런스 구현(beatoraja)과 같게 그리기 위한 조사 결과 전문이다. 조사자 14개 + 종합 1개 + 비판 1개(Workflow `wf_43ad1f8d-42b`, 에이전트 16개, 약 840만 토큰)의 산출물이며, **같은 탐색을 다시 하지 않기 위해** 저장한다.

## 사용 규칙

1. 스킨 관련 작업을 시작할 때는 원본(레퍼런스 소스, 외부 스킨, R-BMS 코드)을 통독하기 전에 이 색인에서 해당 보고서 절을 찾아 읽는다. 원본은 구현 직전에 필요한 줄만 확인한다.
2. 하위 에이전트 지시에는 원본 경로가 아니라 이 폴더의 보고서 절(`b3 §5` 형식)을 사양 근거로 넘긴다.
3. **살아 있는 문서다.** 구현 단위가 조사 대상 코드를 바꾸면 같은 단위 안에서 해당 절을 고친다. 조사 내용이 틀린 것으로 드러나도 즉시 고친다. 고친 문서는 맨 위 "최종 갱신" 줄과 아래 갱신 이력에 적는다.
4. 구현에 따라 바뀌는 문서는 `r1`·`r2`·`r3`(R-BMS 현황), `00-synthesis`(격차 매트릭스·작업 분해), `b5` §15~§17(R-BMS 격차 표)이다. `b1`~`b4`, `b5` §1~§14, `m1`~`m6` 은 원본이 고정이라 오류가 발견될 때만 고친다.
5. 수치의 신뢰도: 객체 수·좌표는 Lua 를 실행하지 않고 손으로 센 정적 추정이다. 스킨 덤프 도구가 생기면 덤프 결과를 믿고 여기 수치를 고친다.

## 경로 표기

보고서 안의 `R:` = 이 저장소 루트, `B:` = 레퍼런스 체크아웃의 `src/bms/player/beatoraja/`, `M:` = 외부 스킨 폴더 루트. 조사 시점의 절대 경로는 각각 `/Users/hyunseokbyun/development/R-BMS/`, `/Users/hyunseokbyun/development/beatoraja/`, `/Users/hyunseokbyun/Downloads/ModernChic/` 다.

## 먼저 읽을 것

| 문서 | 내용 | 분량 |
| --- | --- | --- |
| [00-synthesis.md](00-synthesis.md) | 결론 요약(§0), 화면별 요구 기능 매트릭스(§1: C1~C32 공통, P 플레이 SP, D 플레이 DP, S 선택, E 결정, T 결과, G 코스 결과, K 키 설정, N 스킨 선택), 목표 구조(§2: Lua 런타임·로드 결과 표현·호스트 계약·해상도·텍스트·동영상·입력·내장 화면·구 번들), 화면 흐름과 체감 변화(§3), 구현 웨이브 1~8(§4), 기본 스킨 단순화판(§5), 사용자 결정 13개(§6), 보고서 간 모순 판정·위험·미확인(§7) | 614줄 |
| [99-critique.md](99-critique.md) | 종합의 오류와 구멍. 틀린 주장 W1~W9(§2), 매트릭스에 빠진 기능 M1~M9(§3), 작업 분해 문제 P1~P8(§4: 소유 파일 누락, 빠진 선행, 과대 단위, 폴백 충돌, 스킨 선택 경로 부재, 빠진 단위, 검증 수단 부재), 스킨 단독 소유 시 사라지는 R-BMS 고유 기능 H1~H15(§5), 사실 확인된 행(§6·§7), 외부 스킨 grep 집계(§8) | 269줄 |

**종합 §4(작업 분해)·§5(기본 스킨)·§6(결정)은 확정 사양 `docs/plan/2026-10-09-lua-skin-compat.md` 가 대체한다.** 사양은 비판 §4 의 수정 제안과 사용자 결정 D1~D7 을 반영했다. 종합에서 계속 유효한 것은 §1 매트릭스와 §2 구조 판단의 근거, §7 의 모순 판정·미확인 목록이다. 결정은 `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`.

## 레퍼런스 구현의 스킨 엔진 계약 (b1~b5)

| 문서 | 찾을 수 있는 것 |
| --- | --- |
| [b1-lua-env.md](b1-lua-env.md) | 로딩 파이프라인(§1), Lua 실행 환경: 표준 라이브러리·io 제한·luajava facade·require/dofile 경로 규칙(§2), `.luaskin` 2패스 실행과 `skin_config` 구조(§3), Lua 테이블 → 스킨 모델 변환 규칙과 최상위 필드 표(§4), 함수 값 속성(draw/value/timer/act)의 호출 규약과 오류 처리(§5), `main_state` 전체 API(§6), `timer_util`/`event_util`(§7), customTimers/customEvents 와 프레임 순서(§8), include·조건 분기·와일드카드 경로(§9), 헤더와 사용자 설정 병합·무작위 선택(§10), 해상도·스케일·좌표축(§11), LuaJ 5.2 와 Lua 5.4 의 차이(§12) |
| [b2-object-model.md](b2-object-model.md) | 좌표계와 스케일(§2), destination 모델(§3), 보간·acc·loop·오프셋 식(§4), 그리기 조건(§5), 공통 그리기: blend 별 GL 계수·filter 셰이더·stretch 11종·회전 중심(§6), 그리기 순서와 prepare/draw 2단계(§7), 소스 이미지 로드·캐시(§8), image/imageset·참조 이미지(§9), 숫자 시트 규칙(§10), 소수(§11), 텍스트: TTF·비트맵 폰트·overflow·그림자(§12), 슬라이더(§13), 그래프(§14), 마우스 클릭·드래그·호버(§15), **재현해야 할 원본 특이 동작 18개(§18)** |
| [b3-play-objects.md](b3-play-objects.md) | 플레이 헤더 필드 close/loadend/playstart/judgetimer/finishmargin(§2), note: 노트 y 누적식·LN/HCN 명명·마디선(§3), 레인 커버·리프트·히든과 오프셋 3/4/5(§4), gauge 36칸 선택식(§5), judge 와 콤보 위치(§6), bga 레이어(§7), judgegraph·bpmgraph·timingvisualizer·hiterrorvisualizer(§8), 플레이 상태기계(§10), **타이머 전체 표(§11)**, 키 모드별 레인 인덱스(§12), 구현 체크리스트(§14) |
| [b4-screens.md](b4-screens.md) | 화면 공통 수명: input/scene/fadeout·타이머 리셋·사운드 지점(§1), songlist(SkinBar) 스키마·스크롤 보간·클릭(§2), 선택 화면 상태·타이머·입력 키 표·패널·이벤트 표·검색·막대 종류·IR(§3), 결정(§4), 결과: 타이머·입력·gaugegraph·리플레이 슬롯(§5), 코스 결과(§6), 키 설정(§7), 스킨 설정(§8), 화면 전이(§9), 엣지 케이스 목록(§10) |
| [b5-properties.md](b5-properties.md) | **속성 id 전수 표**: 정수(§3), 불리언(§4), 비율·실수(§5), 문자열(§6), 타이머(§7), 이벤트(§8), 이미지 인덱스(§9), 오프셋(§10), 플레이어·키별 id 계산식(§11), 슬라이더·그래프 type(§12). 외부 스킨이 쓰는 id 와 R-BMS 격차(§15), R-BMS 의 정의 불일치와 근거 줄(§16), 구현 군집 A~M(§17) |

## 외부 스킨의 요구 기능 (m1~m6)

| 문서 | 찾을 수 있는 것 |
| --- | --- |
| [m1-root-config.md](m1-root-config.md) | 조립 구조: `.luaskin` → 본체 → `Root/define` → 파트, 전역 변수와 헬퍼(§2), 10개 스킨 헤더 요약(§3), **최소 Lua 런타임 요구 체크리스트(§4)**, 전역 API 사용 집계(§5), id 이름표(§6), `custom*.lua` 줄 단위 설명(§7), `config.lua`(§8), 파일 읽기·쓰기 지도(§9), 구성 시점에 읽는 엔진 상태(§10), 원본 결함 목록(§12), 실제 참조 id 부록(§13·§14) |
| [m2-play-sp.md](m2-play-sp.md) | 5키·7키 플레이. 헤더 옵션·파일·오프셋(§1), 테이블 구성(§2), 객체 기능(§3), 함수 값 자리(§4), 참조 id(§5), **1920x1080 레이아웃과 BASE 좌표식(§7)**, 자산(§8), 단순화 후보(§9), note/gauge/judge/bga/커버/그래프 필드 원문(§10) |
| [m3-play-dp.md](m3-play-dp.md) | 10키·14키 플레이. 같은 구성 + 10키와 14키 차이(§10), 싱글과의 공유·차이(§11), 좌우 필드 객체 필드 값(§12) |
| [m4-select.md](m4-select.md) | 선곡. 헤더(§1), 테이블 구성(§2), songlist 전체 필드(§5), 참조 id(§6), **레이아웃(§8)**, UI 요소별 이벤트·타이머·마우스 영역(§9), 목록 배치 3종·언어 3종(§10), 자산(§11), 단순화 후보(§12), 구현 체크리스트(§13) |
| [m5-decide-result.md](m5-decide-result.md) | 결정·결과·코스 결과. 실행 모델(§2), 헤더(§3), 레이아웃(§9), 결과 전용 객체 값 원문(§10), 파트별 설명: mainmenu·irmenu·impression·history(§12), 배경·캐릭터 선택 규칙(§13), 결과와 코스 결과 차이(§14), 단순화 후보(§16), **화면별 상태 요구 체크리스트(부록 A·B)** |
| [m6-misc-assets.md](m6-misc-assets.md) | 키 설정·스킨 선택 스킨(§1~§7), **자산 전수: 폴더별 용량·mp4·ttf·fnt(§8)**, 단순화 절감 추정(§9), 사운드와 시스템 사운드 대응(§10), readme 의 사용자 옵션(§11), **라이선스와 제3자 자산(§12)** |

## R-BMS 현황과 격차 (r1~r3) — 구현에 따라 갱신

| 문서 | 찾을 수 있는 것 |
| --- | --- |
| [r1-rbms-skin.md](r1-rbms-skin.md) | `crates/rbms-skin` 공개 API 와 데이터 흐름(§2), 모델과 레퍼런스 모델 전수 대조(§3), 로더가 풀 Lua 스킨과 충돌하는 지점 L1~(§4), 샌드박스 구조와 필요한 변경 S1~(§5), 보간·타이머·속성 의미론 차이 D1~(§6), rbms 전용 확장의 위치와 제거 영향(§7), 테스트 자산 평가(§8), 변경 단위 권고(§9) |
| [r2-rbms-render.md](r2-rbms-render.md) | `Renderer` 프리미티브와 CPU·wgpu 구현 여부(§1), GPU 백엔드(§2), 텍스처 수명(§3), 1280x720 전제 위치(§4), `skin_render` 객체별 상태(§5), 텍스트(§6), 상태 공급 구조(§7), 내장 화면과 스킨 렌더 결합 지점(§8), 성능(§9), 변경 단위 권고(§10) |
| [r3-rbms-app.md](r3-rbms-app.md) | Stage 목록과 전이(§1), 스킨 선택·로드·캐시(§2), 기본 번들 설치와 바이너리 영향(§3), SKIN 탭 저장 구조(§4), 스킨 폴더 하나로 전 화면을 쓰기 위한 변경(§5), 화면별 상태 공급과 타이머(§6), 입력(§7), **혼합 합성이 걸린 코드 전부(§8)**, 창·해상도(§9), 사운드·BGA·폰트(§10), 변경 단위 권고(§12), 체감 변화(§13) |

## 실측 기록 — 구현에 따라 갱신

| 문서 | 찾을 수 있는 것 |
| --- | --- |
| [v2-video-decoder.md](v2-video-decoder.md) | mp4 디코더 선정 조사: 외부 스킨 mp4 8개의 코덱·프로파일(전부 H.264 High, CABAC, B 프레임), 후보별 평가(OpenH264 소스 빌드, FFmpeg 정적 링크, 사이드카, OS 디코더, GStreamer, 순수 Rust), 근거 URL, 권고와 구현 구조 제안, 미결 질문 |
| [v1-first-render.md](v1-first-render.md) | ModernChic 결정·결과·선곡·플레이 7키를 실제로 그린 첫 결과. 화면별로 나온 것·빠진 것·잘못 나온 것과 공통 원인(음수 id 참조 이미지 미구현, `dst` 없는 `songlist`·`note` destination 이 그려지지 않음, 음수 폭 객체가 버려짐). 웨이브 3~6 의 작업 근거 |

프레임 비용 실측(웨이브 2B, `rbms-cli skin-dump <팩> --frames 1000`, 조건을 보지 않고 함수 값 전부를 프레임마다 한 번씩 부른 상한): 선곡 1,801호출 평균 0.278ms·p99 0.339ms·최대 0.653ms, 플레이 5/7키 75호출 평균 0.03ms, 10/14키 123호출 평균 0.05ms, 결과 118호출 0.025ms, 코스 결과 119호출 0.040ms. 로드 한 패스의 명령 수는 8천~32만, Lua 메모리 최대 6.1 MiB. 예산 기본값: 로드 1억 명령·10초, 프레임 50ms·호출당 100만 명령, 메모리 256 MiB.

## 조사로 확정된 핵심 사실

- 외부 스킨은 데이터가 아니라 실행되는 Lua 프로그램이다(Lua 126개 약 25,500줄, `require` 145·`dofile` 109·`pcall` 110·`io.open` 45). 화면에 들어갈 때마다 진입 파일을 헤더 패스와 본체 패스로 두 번 실행하고, 본체가 만든 함수 값(draw 약 330, value 약 130, act 35, timer 17 + `timer_observe_boolean` 148)을 매 프레임 호출한다.
- 레퍼런스의 Lua 는 LuaJ 3.0.2(Lua 5.2 계열, 수가 double 하나)다. R-BMS 가 잠근 mlua 0.12.2 에 `lua52` 피처가 있어 새 의존성 없이 전환할 수 있다.
- 재사용 가능: `SkinDef` 모델(레퍼런스 최상위 46필드와 일치), `dst.rs` 보간, 속성·타이머 id 표 968개(차이 0), `Renderer` 프리미티브(블렌드·회전·클립 일치), `SkinViewport`.
- 새로 필요한 큰 덩어리: Lua 5.2 런타임과 2패스 로더, Lua 값 → 모델 변환과 함수 값 필드, id → 값 공급 계층(호스트 계약), 레퍼런스식 장면 수명과 플레이 상태기계, 객체 의미론 정합(텍스트·songlist·note·gauge·judge·그래프·클릭), 해상도 일반화와 텍스처 수명.
- START/SELECT 바인딩은 웨이브 1B(W1-10)에서 생겼다(`ControlAction::{Start, Select}`, 키보드 A/W, 질의 `AppShared::{start_pressed, select_pressed, key_index_pressed}`). 패널 1~3·결정 취소·차트 미리보기가 이 질의를 쓴다.
- 게이지 이력은 레퍼런스에서 전 종류를 500ms 간격으로 동시에 기록한다(`B:play/BMSPlayer.java:625-635`).
- **실측(웨이브 2A, 실제 로드 결과)**: ModernChic `.luaskin` 10개 중 9개가 로드된다. keyconfig 만 원본 결함(`Decide/lua/require/textproperty.lua:37`)으로 본체 패스가 실패한다. 화면별 type / destination / 함수 값 / Lua 메모리: play7_hw 0 / 286 / 76 / 1.4 MiB, play5_hw 1 / 277 / 76 / 1.4 MiB, play14_hw 2 / 372 / 124 / 1.7 MiB, play10_hw 3 / 354 / 124 / 1.7 MiB, musicselect 5 / 1914 / 1817 / 6.1 MiB, decide 6 / 69 / 2 / 0.7 MiB, result 7 / 195 / 135 / 1.0 MiB, course 15 / 175 / 121 / 0.8 MiB, skinselect 9 / 108 / 0 / 0.7 MiB. 9개 모두 pcall 실패 0, 경고 0. 조사 보고서의 수기 집계(play7 약 288, play14 약 375, play10 약 357, result 194)와 다른 곳은 이 실측이 맞다. 재현: `RBMS_SKIN_PACK=<스킨 폴더> cargo test -p rbms-skin --test skin_luaskin -- --nocapture`
- 외부 스킨 원본은 476.4 MiB(mp4 192.8, 비트맵 폰트 139.4, TTF 54.7)이고 단순화판은 35~45 MiB 로 추정된다.
- 외부 스킨 readme 는 스킨 자체의 2차 배포를 금지하고, 개변 스킨 공개 시 원작자명(KASAKO) 표기를 요구한다. 음성·캐릭터 일러스트·일부 랭크 그림 등 제3자 자산은 별도 조건이다(`m6` §12).

## 갱신 이력

| 날짜 | 단계 | 고친 문서·절 | 내용 |
| --- | --- | --- | --- |
| 2026-10-09 | L1 | 전체 | 최초 저장 |
| 2026-10-09 | L2 | 색인 | 확정 사양·결정 문서 연결, 종합 §4~§6 이 사양으로 대체됨을 명시. 기본 스킨은 개변판이 아니라 자작으로 결정(D1) |
| 2026-10-09 | 웨이브 1A | `r1`·`r2`·`r3`·`00`·`99`·`b1` 상단 "웨이브 1A 반영 사항" | 혼합 합성·구 번들·모델 확장 필드 삭제, Lua 5.2 전환, 폰트 캐시 패밀리 분리. 리뷰 수정: 계정 설정 블롭 마이그레이션(`ir_sync.rs`), 문서 단독 그리기 테스트를 `stage/render_tests_document.rs` 로 복원, `*_on_background`·`object_ids`·`SelectListState.detail` 제거. 본문 줄 번호는 `9ce92bb` 기준이라 어긋날 수 있다 |
| 2026-10-10 | 웨이브 1B | `r1`, `r2`, `r3`, `00`, `99`, `b2`, `b4` 상단 "웨이브 1B 반영 사항" | 타이머 µs(OFF = `i64::MIN`)·`TimerRef`·오프셋 r 부호·acc 선언 순서·stretch 11종, 장면 시계와 전환 리셋(Open/Back 보존), 마우스 release·drag·휠, GPU 논리 크기 = 물리 뷰포트와 `ScaledRenderer`, 표면 비 sRGB(바이트 통과), 오프스크린 읽기, START/SELECT 와 키 인덱스 질의, 해상도·창 모드 설정, 캡처 하니스 `stage/capture.rs` |
| 2026-10-10 | 웨이브 2A | `r1`, `r2`, `r3`, `b5`, `b1`, `00`, `m1`, `b2`, `99` 상단 "웨이브 2A 반영 사항", 색인의 실측 수치 | Lua 5.2 런타임(환경, `require`/`dofile` 가두기, io 쓰기 오버레이, os, luajava facade, `main_state`, `timer_util`/`event_util`), `SkinHost`·`MapHost`, 값 참조 타입(`PropertyRef::Func`/`Name`, `TimerRef::Lua`, 이벤트·쓰기 참조), Lua 값 → `SkinDef` 변환, 2패스 로더와 헤더 병합, 구 샌드박스 삭제. 리뷰가 찾은 예산 우회 3건(`__gc` 종료자, 패턴 백트래킹, 이름 캐시)과 파일 읽기·핸들 상한을 수정. ModernChic 9/10 로드 실측 |
| 2026-10-10 | 웨이브 2B | `r1`, `r2`, `m5`, `m4`, `m2`, `00`, `r3`, `99`, `b1`, `m1` 상단 "웨이브 2B 반영 사항", 신규 `v1-first-render.md`, 색인의 프레임 비용 실측 | `rbms-cli skin-dump`(헤더·객체 수·진단·프레임 비용·예산 탐색), 앱의 스킨 팩 폴더(`skin.pack`, `RBMS_SKIN_PACK`, SKIN 탭 PACK FOLDER 행, 헤더 캐시, 오버레이 `<설정 폴더>/skin-data/<식별자>/`, 실패 시 내장 폴백), 오버레이 총 크기 상한(64 MiB·4,096 항목), 외부 스킨 정지 프레임 테스트 `crates/rbms-render/tests/skin_external.rs` |
| 2026-10-10 | 웨이브 3A | `r2`, `r3`, `v1`, `99`, `b5`, `b1`, `b2`, `m4`, `00`, `r1`, `m5`, `b3`, `m1` 상단 "웨이브 3A 반영 사항"(`v1` 에는 리뷰어의 화면 대조 포함) | prepare/draw 2단계와 `SkinHost` 직접 그리기, `FrameData`(능력별 프레임 데이터), 조건 의미론(내장·스킨 옵션·미지 op, 정적 1회 평가, 음수 타이머), 참조 이미지(-100~-111), 음수 크기 뒤집기, 이미지 인덱스 공간, 값 없음 센티널, 슬라이더·그래프 비클램프, TTF 텍스트(정렬·overflow·그림자·물리 픽셀 래스터·다국어 폴백), judgegraph·bpmgraph 재작성, 타이머 함수 프레임당 1회, 앱 `skin_host/`(군집 A·I·M) |
| 2026-10-10 | 웨이브 3B | `r3`, `b4`, `m5`, `99`, `00`, `v1`, `r2`, `b2`, `v1` 상단 "웨이브 3B 반영 사항" | 텍스처 관리자(`skin_render/textures.rs`: 참조 source 만 디코드, `SkinTexturePool`, 화면당 RGBA 1 GiB 예산, 화면 이탈 해제, 디버그 패널 `SKIN TEX` 줄), 문서가 오는 동안 장면 시계 정지, Decide Stage(`stage/decide.rs`, `stage/scene_life.rs`: STARTINPUT 1·FADEOUT 2, 건너뛰기·취소, 백그라운드 로드), `skin_host/overview.rs`(곡 메타·노트 분포·속도 변화). 실측: decide 3장 11 MB, result 12장 134 MB, musicselect 11장 170 MB, play7 22장 138 MB |
| 2026-10-10 | 웨이브 4 | `99`, `b4`, `b3`, `b5`, `r3`, `00`, `b2`, `b1`, `m5`, `r2`, `v1`, `m1` 상단 "웨이브 4 반영 사항" | 스킨 입력(`skin_render/input.rs`: z 역순 클릭, click 0~3, 슬라이더 드래그, `SkinAction`), 이벤트 분배(`skin_host` 의 `dispatch_calls`·`skin_requests`), libGDX 키 질의, gauge 칸 선택식과 결과 차오름, gaugegraph·timingdistributiongraph·judgegraph type 1·2, 엔진의 전 게이지 종류 이력(500ms)·판정별 early/late·타이밍 분포, Result·CourseResult Stage(타이머 150~152, 입력 잠금, FADEOUT, 게이지 종류 전환), 호스트 군집 B·G·H·E 일부(`ResultSnapshot`), 효과음 버스와 스킨 `audio_play` |
| 2026-10-10 | 웨이브 5 | `b4`, `99`, `r3`, `m6`, `r2`, `m4`, `v1`, `00`, `b2`, `b5` 상단 "웨이브 5 반영 사항" | songlist(`skin_render/songlist/`: `SongBars`, 원형 인덱스, 패스별 그리기, 스크롤 보간, `BarScroller`, 막대 클릭), 선곡 모델(막대 종류·램프·레벨·라벨·트로피·폴더 분포, 내장 뷰와 곡 바를 한 모델에서), 선곡 키 표(`stage/select/keys.rs`)와 패널(`panel.rs`)·옵션 이벤트(`events.rs`), 호스트 군집 F·H·E, 슬라이더 쓰기(`skin_host/writers.rs`)와 편집 텍스트·IME, 선택 BGM 루프와 미리듣기 전환, 시스템 오버레이(`overlay.rs`)와 단축키 안내 |
| 2026-10-11 | 웨이브 6 | `r3`, `b5`, `m2`, `b3`, `00`, `r2`, `v1`, `m3`, `99`, `m4`, `m5` 상단 "웨이브 6 반영 사항" | note(`skin_render/notes.rs`: `LaneNotes` 계약, y 누적식, LN/CN/HCN, 마디선류, 오프셋 3/4/5 식), judge(영역 콤보·shift), hiddenCover·liftCover, bga 레이어, timingvisualizer·hiterrorvisualizer, 플레이 상태기계(`stage/play/`: PRELOAD/READY/PLAY/FAILED/FINISHED, 미리보기 141, 즉시 재시작), 타이머 드라이버(`skin_host/play_timers.rs`), 호스트 군집 C·D(`PlayShown`), OFFSET_ALL, 구 어댑터·구 드라이버·폴백 삭제. 성능: 7키 512노트 프레임 prepare 19µs + draw 43µs |
| 2026-10-11 | 웨이브 7A | `b2`, `r2`, `m6`, `r3`, `00`, `v1` 상단 "웨이브 7A 반영 사항", 신규 `v2-video-decoder.md` | 비트맵 폰트(`crates/rbms-render/src/bitmap_font.rs`: BMFont 텍스트 형식, libGDX 1.9.9 바이트코드 기준 조판, 쓰이는 페이지만 로드), type 0 그리기와 distance field(`Renderer::draw_distance_field_quad`, CPU·GPU 동일 계산, 원본 셰이더 식 그대로), 디코더 조사 보고서 |
