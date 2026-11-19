# DOC 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- docs/acknowledge/2026-11-19-adr-output-latency-compensation.md (new)
- docs/acknowledge/2026-11-19-adr-exclusive-fullscreen.md (new)
- docs/acknowledge/2026-11-19-adr-chord-bindings-and-escape.md (new)
- docs/acknowledge/2026-11-19-adr-assist-grade-lamps-in-lists.md (new)
- docs/acknowledge/2026-11-19-adr-split-clear-lamp.md (new, plan only)
- docs/acknowledge/2026-11-19-adr-user-offsets-in-document-units.md (new)
- docs/acknowledge/2026-11-19-adr-play-screen-run-facts.md (new)
- docs/acknowledge/2026-11-19-adr-layout-pick-playback-and-capture.md (new)
- docs/acknowledge/2026-11-19-adr-select-screen-refresh.md (new)
- docs/acknowledge/README.md
- docs/acknowledge/reference-divergences.md
- docs/acknowledge/2026-10-22-adr-assist-lamp-grade.md
- docs/acknowledge/2026-10-22-adr-skin-colour-rows.md
- docs/acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md
- docs/acknowledge/2026-10-24-adr-select-iidx-layout.md
- docs/acknowledge/2026-10-29-adr-layout-pick.md
- docs/acknowledge/2026-10-29-adr-select-info-overflow.md
- docs/play-options.md
- docs/skin.md
- docs/crates.md
- docs/architecture.md
- docs/memory/client-versions-and-run-rules.md
- docs/reference/2026-10-24-i18n-glossary.md
- docs/reference/2026-11-19-feature-upgrade-12/README.md
- docs/reference/option-panels.md
- docs/reference/skin-color-customization.md
- docs/quality-assurance/2026-10-11-default-skin-manual-check.md
- docs/quality-assurance/2026-10-14-test-debt.md
- docs/plan/2026-10-11-skin-menu.md
- docs/plan/2026-10-14-settings-screen.md

## 요약

docs/ 만 고쳤고 코드는 건드리지 않았다. cargo·skin-dump 같은 코드 검사는 돌리지 않았다. 확인한 것은 git status(바뀐 파일이 전부 docs/ 아래), 표 열 수 점검 스크립트, 머리말 괄호 균형, 내가 새로 쓴 표의 일관성뿐이다. 새 ADR 9건(output-latency-compensation, exclusive-fullscreen, chord-bindings-and-escape, assist-grade-lamps-in-lists, split-clear-lamp, user-offsets-in-document-units, play-screen-run-facts, layout-pick-playback-and-capture, select-screen-refresh)을 기존 머리말 형식으로 썼다. split-clear-lamp 는 코드가 없어 결정과 계획서 U10 계약만 적고 구현 세부 칸은 모두 "구현 뒤 메인이 채움"으로 표시했다. 사용자가 정한 것(요구 원문 요지와 진행 방식)과 메인 결정 M1~M20 을 갈라 적었고 M 항목은 모두 사용자 확인 전으로 표시했다. 문서에 적은 줄 번호·상수·id 는 worktree(fe1f427 = origin/dev)의 코드에서 직접 확인했다. 확인하지 못한 것은 그렇게 썼다: 원본(beatoraja) 동작, 한국어·일본어 실제 화면, 실기 동작, CI 결과. 색인(README)에는 2026-11-19 절과 표를 넣고 미확인 39~47번을 더했으며 머리말을 갱신했다. 레퍼런스 차이표에는 기존 4열 형식으로 새 절을 더했다(오프셋 한 행만 코드 주석이 인용한 원본 위치를 옮겼고 나머지 행의 원본 칸은 "확인하지 않음"이나 "해당 없음"). 일부 대체된 기존 ADR 세 건(select-iidx-layout, select-info-overflow, layout-pick)에는 상태 줄에 대체 표시를, assist-lamp-grade 와 extension-id-spaces 에는 새 표시·배정을 덧붙였다. 사용자·개발 문서(play-options 19~24절 신설과 5.1·7.2·8·10.13·10.16·17절 정정, skin.md 의 id 표·lamp_graded·NPS Display 916/917·3.4.9절 신설·오프셋 단위 정정, 기본 팩 크기 재계산(80개 파일, 2,332,443 B), crates.md·architecture.md 영어 서술), 용어집 6.10절, 조사 README 의 각 절 "구현 결과(2026-11-19)", QA 3.29절(G1~G13)과 0절 표 열세 줄, 테스트 부채 3.163~3.182(3.182 는 U10 자리), option-panels·skin-color-customization·memory 의 낡은 서술을 고쳤다.

## 기준별 판정

- [pass] 1. ADR 9건(output-latency-compensation, exclusive-fullscreen, chord-bindings-and-escape, assist-grade-lamps-in-lists, split-clear-lamp, user-offsets-in-document-units, play-screen-run-facts, layout-pick-playback-and-capture, select-screen-refresh): docs/acknowledge/2026-11-19-adr-*.md 9개 생성. 머리말은 최종 갱신·대응 커밋·상태·결정일·결정자·관계. split-clear-lamp 는 상태를 '계획만 있음, 구현 중'으로 하고 4절 구현 세부 칸에 '구현 뒤 메인이 채움' 표시. 기존 ADR 2026-10-22 assist-lamp-grade·2026-10-29 layout-pick·2026-10-24 select-iidx-layout·2026-10-29 select-info-overflow 와의 관계를 각 ADR 머리말과 기존 파일 상태 줄 양쪽에 적음.
- [pass] 2. acknowledge/README.md(2026-11-19 절·표·미확인 목록·머리말)와 reference-divergences.md: README 에 '2026-11-19 세션의 ADR' 절과 9행 표, 미확인 39~47번(지금 동작·선택지·ADR 링크), 머리말 갱신, 대체된 기존 행의 상태 문구 보강. reference-divergences.md 에 '기능 고도화 12건 (2026-11-19)' 절을 기존 항목|전|원본|지금 형식으로 추가하고 머리말 갱신. 원본 칸은 소스를 다시 열지 않아 오프셋 한 행 외에는 '확인하지 않음/해당 없음'.
- [pass] 3. 사용자·개발 문서(play-options, skin, crates(영어), architecture): play-options 19~24절 신설(OUTPUT LATENCY, FULLSCREEN, ESC·조합, 출발 폴더·옵션 줄·NPS, LAYOUT PICK 재생·capture·결과 화면 P, 오프셋 단위)과 5.1·7.2·8·10.13·10.16·17절 정정. skin.md 1.3 E6, 1.4 id 표(텍스트 21010~21014, 숫자 21085·21087~21102, lamp_graded, 21086 미사용), 3.4 옵션 NPS Display 916/917, 3.4.1 위치 행, 3.4.6 선곡 개편, 새 3.4.9, 기본 팩 크기 재계산. crates.md 영어로 latency 모듈·run_anchor_us·FULLSCREEN·chord.rs·captured_layouts·layout_pick/{clock,captured}·run_setup·Shortcut 34·ControlAction 21·SETTING_COUNT 101. architecture.md 화면 흐름 표·재생 루프·입력·오프셋 단위 문단.
- [pass] 4. 용어집: docs/reference/2026-10-24-i18n-glossary.md 6.10절 신설(OUTPUT LATENCY, 화면 모드, 주사율, 조합, CAPTURE LAYOUT, CAPTURED, STOPPED, KEY GUIDE, skin pixels 등을 카탈로그의 실제 문구로)과 6.1절 ALL KEYS AS THEY ARE SET·FREE PLAY 행 정정.
- [pass] 5. 조사 README 각 절 끝 '구현 결과(2026-11-19)'와 머리말: 1~12절과 공통 절 모두에 '구현 결과(2026-11-19)' 13개 추가(현재 줄 번호 포함, 조사 때와 달라진 점 명시). 머리말 최종 갱신·대응 커밋 갱신과 본문 미수정 안내 문단 추가.
- [pass] 6. QA 점검표와 테스트 부채: default-skin-manual-check.md 에 3.29절(G1~G13: AirPods, Game Mode, FULLSCREEN(Windows 포함), 패드 ESC, 조합 지정, 등급 램프 색, 플레이 화면 세 표시, LAYOUT PICK 재생·capture, JUDGE POSITION 해상도 무관, 선곡 세 언어; G13 은 U10 구현 뒤)와 0절 표 13행 추가. test-debt.md 에 3.163~3.181(재현 조건·생략 이유·남은 위험·실행 시점)과 3.182(U10 자리).
- [pass] 7. 틀려진 기존 서술 정정(skin-color-customization, option-panels 등): skin-color-customization.md 의 skin_screen.rs·skin_host/mod.rs 줄 번호와 units_per_pixel 서술, ADR 2026-10-22 skin-colour-rows 23행 줄 번호, option-panels.md 337행(WINDOW MODE 값)·378행(조작 21종), plan/2026-10-11-skin-menu.md 34행의 '단위는 대상 픽셀', plan/2026-10-14-settings-screen.md 의 OUTPUT 구획 행 목록, memory/client-versions-and-run-rules.md 의 SETTING_COUNT·단축키 수를 고침.

## 찾은 것

- [low, 안 고침] 설정 행 이름: 계획·조사·U5 보고는 JUDGE TIMING, 설정 화면·PLAY 메뉴 행과 알림 문구는 JUDGE OFFSET: crates/rbms-config 행 라벨 apps/rbms-player/src/text/settings/labels.rs:33 은 JUDGE OFFSET/판정 오프셋/判定オフセット. 같은 값을 스킨 선곡 옵션 패널 assets/skins/rbms-default/lua/i18n/select_other.lua:46 의 sheet_judge_timing 이 JUDGE TIMING 으로 부른다. 계획서 M1·M4, 조사 1.1, play-options.md 901·903행은 JUDGE TIMING. 같은 개념의 이름이 둘이라 문서에는 행 이름을 쓰고 용어집 6.10과 ADR 출력 지연에 두 이름을 적음. (시험: )
- [low, 안 고침] 조사 README 10절의 lane_offsets(stage/play/mod.rs:1227) 위치가 지금 코드와 다름: 함수는 crates/rbms-render/src/skin_render/notes.rs:412(lane_offsets), 429(first_lane_rect)에 있고 apps/rbms-player/src/stage/play/skin_preview.rs:53·773 이 가져다 쓴다. 조사 README 구현 결과 절에 현재 위치를 적었다(본문은 조사 시점 서술이라 그대로). (시험: )
- [low, 안 고침] U3 보고의 남은 위험 '설정 화면·옵션 오버레이의 키 안내가 조합을 표시하지 못함'은 메인 병합 수정으로 이미 해소됨: apps/rbms-player/src/stage/settings.rs:966 과 app_options.rs:298 이 keyconfig.shortcut_bind(...).map_or_else(..., KeyBind::label) 로 조합을 표기한다(git diff cac3795 origin/dev 에서 두 파일 변경 확인, 계획서 7절 '메인 수정'과 일치). ADR 조합과 ESC 와 play-options 21절은 코드 기준으로 '표기한다'고 적었다. (시험: )
- [low, 안 고침] 계획서 7절 U5 행의 '0.5 문서 화소 안'과 U5 보고의 허용치(max(0.5 문서 화소, 반 장치 화소), 720p 는 0.75)가 다름: U5 보고: LIFT 를 켠 720p 에서 실측 0.54 라 계획의 0.5 문서 화소는 물리적으로 지킬 수 없다고 적음. 오프셋 단위 ADR 과 play-options 24절에는 '0.5 문서 화소 안(720p 는 반 장치 화소 0.75)'으로 적었다. (시험: )
- [low, 안 고침] U5 와 U9 의 JUDGE POSITION 측정 기준 판정 글자 중심이 다름(U5 오프셋 0 기준 647.49, U9 646.19): U5 표는 '0 일 때 647.49', U9 표는 1920x1080 에서 646.19 로 같은 크기의 기준값이 다르다(측정 대상 부위나 방법 차이로 보이나 확인하지 못함). 이동량(30)은 둘 다 같아 결론은 같다. ADR 오프셋 단위 측정표에 두 기준값을 모두 적고 고친 뒤 값은 U9 것을 썼다. (시험: )
- [low, 안 고침] HANDOFF.md 7절의 클라이언트 값이 낡음(SETTING_COUNT 99, 단축키 32) 및 5절 의존성 예외 목록에 coreaudio-rs 없음: docs/HANDOFF.md:114 는 SETTING_COUNT 99, 단축키 32 로 적혀 있다. 코드는 SETTING_COUNT 101(settings.rs:283), Shortcut::ALL 34(keyconfig.rs:789), ControlAction::ALL 21(397). 이 문서는 수정 금지라 고치지 않았고 docs_to_update 로 넘긴다. memory·crates 의 같은 낡은 숫자는 고쳤다. (시험: )
- [low, 안 고침] 기본 스킨 팩 크기 서술(skin.md)이 낡아 있었음: skin.md 머리말의 2,324,022 B(80개 파일)는 2026-10-29 값. 이 worktree 에서 tools/ 를 뺀 80개 파일을 stat 으로 다시 세어 2,332,443 B(+8,421 B)를 확인하고 머리말에 적었다. tools/ 를 넣으면 88개(skin-dump 의 '88 files'와 같음). (시험: )
- [low, 안 고침] 내장 skin-dump 시나리오(apps/rbms-cli/scenarios/play.json·select.json)에 새 확장 id 값(21010~21014, 21085, 21087~21102)이 없음: grep 결과 두 시나리오 파일에 21085·21010·21087·21093 이 0건. U4·U6 보고가 스크래치 시나리오로만 검사했다고 보고했고 이 상태를 skin.md 1.4절과 테스트 부채 3.171 에 적었다. (시험: )

## 남은 위험

- 코드 세부(줄 번호, 시험 이름, 상수)는 fe1f427 시점의 것이라 다른 에이전트가 시험·수리 중인 코드가 바뀌면 줄 번호와 시험 이름이 어긋날 수 있다.
- 램프 반 쪼개기(U10)는 코드가 없다. split-clear-lamp ADR 4절, README 행과 미확인 43, QA G13, 테스트 부채 3.182, play-options 10.13·17.1절, skin.md 1.4절(21160~21163 계획 id)과 crates.md rbms-store 서술은 구현 뒤 메인이 채워야 한다.
- ADR 의 한국어·일본어 문구는 코드 카탈로그 인용이고 모델 초안이다. 레퍼런스(beatoraja) 소스는 이번에 다시 열지 않아 reference-divergences 의 원본 칸은 '확인하지 않음'이 많다. 오프셋 한 행만 U9 코드 주석이 인용한 위치(SkinObject.prepare, LaneRenderer.java:521-524)를 옮겼다.
- 3 OS CI 결과는 PROCESS 의 진행 기록(웨이브 1·2 통과)만 따랐고 run 을 다시 보지 않았다. Windows·Linux 동작과 FULLSCREEN·AirPods·컨트롤러·한국어·일본어 화면은 실기 확인 전이라고 모든 관련 문서에 적었다.
- 수정이 금지된 HANDOFF.md 7절(SETTING_COUNT 99, 단축키 32)과 5절 의존성 예외 목록, PROCESS.md K7, 계획서 U3 3항 오타·U4 소유 파일·U6 결과는 메인이 고쳐야 한다.
- docs/assets/screenshot-select.jpg(README 캡처)는 선곡 개편 전 모습이다(메인이 교체).
- 새 ADR 9건 모두 사용자 확인 전 결정(M1~M20 과 구현 단위 결정)을 포함한다. 기존 ADR 세 건에는 대체 표시만 하고 본문 문장은 고치지 않았다.

## 갱신이 필요한 문서

- docs/HANDOFF.md 7절 클라이언트 값: SETTING_COUNT 101, Shortcut 34, ControlAction 21, 확장 id 사용 상태(정수 21085·21087~21102, 텍스트 21010~21014), 5절 새 의존성 예외에 coreaudio-rs(macOS 한정) 추가
- docs/PROCESS.md K6·K7 체크리스트와 이번 문서 단계 완료 기록(ADR 9건 경로, 문서 목록, 테스트 부채 3.163~3.182, QA 3.29)
- docs/plan/2026-11-19-feature-upgrade-12.md: U3 3항 '누른다'는 '누르지 않는다' 정정, U4 소유 파일에 textures.rs·skin_host 추가, U6 결과(21086 미사용 등), U2 'BORDERLESS 로 열고 전환' 변경 기록(일반 창을 거침), 5절 U5 위험 '원인 미상'을 확정 원인(M19)으로
- 램프 반 쪼개기(U10) 구현 뒤: docs/acknowledge/2026-11-19-adr-split-clear-lamp.md 상태·대응 커밋·4절 구현 세부, docs/acknowledge/README.md 의 해당 행·미확인 43, play-options.md 10.13·17.1절, skin.md 1.4절(불린 21160~21163)·3.4.6절, crates.md rbms-store·rbms-render songlist, 테스트 부채 3.182, QA 3.29 G13, 용어집(새 문구가 생기면)
- docs/assets/screenshot-select.jpg 와 README 캡처 교체(선곡 개편 뒤 모습)
- apps/rbms-cli/scenarios/{play,select}.json 에 새 확장 id 값을 넣을지 결정하고 넣으면 skin.md 1.4절·테스트 부채 3.171 갱신

## 돌린 검사

- `git status --short (worktree)` → 바뀐 파일이 전부 docs/ 아래의 .md 이다(수정 21개, 새 ADR 9개). 코드·contract·PROCESS·HANDOFF·계획서(2026-11-19)·history·assets 는 건드리지 않음.
- `python3 /tmp/rbms-docs/check_tables.py (바뀐 .md 의 표 열 수 점검)` → 내가 쓴 표는 모두 일관. 불일치 3건은 기존 test-debt 3.13~3.15 행의 것(이번 변경 아님).
- `머리말 괄호 균형 점검(acknowledge/README, 용어집, test-debt, QA, skin, play-options, memory)` → 모두 여는 괄호와 닫는 괄호 수가 같음.
- `코드 대조(grep·sed 로 줄 번호·상수·id·시험 이름 확인)` → engine.rs, latency.rs, window_mode.rs, keyconfig.rs, stage/mod.rs, gamepad.rs, run_setup.rs, shown.rs, extension.rs, captured_layouts.rs, layout_pick*.rs, scene.rs, bars.rs, theme.lua, info.lua, field.lua 등에서 직접 확인. 없는 시험 이름을 적었던 것은 실제 이름으로 바로잡음.
- `cargo test / clippy / skin-dump / markdown lint` → 실행하지 않음. 문서 작업이고 다른 에이전트가 같은 저장소의 코드를 시험·수리 중이라 코드 검사는 범위 밖. 코드·시험·CI 결과를 이 문서가 새로 보증하지 않는다.
