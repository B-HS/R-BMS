# ADR: 클라이언트 UI 다국어(영어·한국어·일본어)의 구조와 번역 범위

> 최종 갱신 2026-10-24 · 대응 커밋 66b9e63
> 상태: 유효(구조와 범위는 사용자 확인 2026-10-24 "18번까지 전부 추천안대로". 한국어는 사용자 검수 전, 일본어는 원어민 감수 전의 모델 초안이다) · 결정일 2026-10-24 · 결정자 사용자(다국어 지원 요청과 결정 묶음 1~18)와 메인(세부) · 커밋 `b7efac1`(기반·LANGUAGE 행·용어집), `d75b819`(스킨 언어 불린과 `t(key)`), `7c9bb0d`(설정·렌더·메뉴·알림 번역), `5014bb6`(선곡 번역), `eadde84`(기본 스킨 문구), `f82e6a9`(남은 플레이어 문구)
> 관계: 웹 다국어는 [웹 다국어 ADR](2026-10-24-adr-web-i18n.md). 스킨이 UI 언어를 읽는 id 는 [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md).

## 배경

- 사용자 요청(2026-10-24): "다국어 지원 (한글, 영어, 일본어)". 조사 보고서 [2026-10-24-i18n-survey.md](../reference/2026-10-24-i18n-survey.md) 는 사용자에게 보이는 문자열이 앱·렌더 크레이트·기본 스킨 Lua 에 약 8,000~10,500줄 걸쳐 영어로 하드코딩돼 있고, 설정 화면이 문자열 값을 비교해 상태를 판정하는 곳(`settings_ui.rs`, `stage/settings/info.rs`)이 있다고 밝혔다. 번역 라이브러리(fluent·gettext 계열)는 새 의존성이라 쓰지 않았다.
- 한국어·일본어 글꼴은 번들 Inter 에 없고 시스템 글꼴 폴백에 맡긴다(기존 결정, [font-cjk-support.md](../font-cjk-support.md)). 캡처 하니스는 번들 글꼴만 써서 CJK 모양을 볼 수 없다.

## 결정

사용자가 결정 묶음 1~18 을 추천안대로 확인했고, 보고서 질문 번호별 채택은 아래와 같다.

| 질문 | 채택 | 구현 |
| --- | --- | --- |
| Q1 범위 | 클라이언트(앱과 기본 스킨)와 웹(next-intl, 같은 체크리스트의 마지막 웨이브) | 이 ADR 과 [웹 ADR](2026-10-24-adr-web-i18n.md) |
| Q2 기본 언어 | `AUTO`: 설정 → 환경변수 `RBMS_LANGUAGE`(`en`·`ko`·`ja`) → OS 로캘 → 영어. 지원 밖 로캘(`zh-TW`, `fr`, 빈 문자열, `C`)은 영어. 언어 부분만 읽어 `en-KR` 은 영어 | `crates/rbms-model/src/language.rs`, `apps/rbms-player/src/app_language.rs` |
| Q3 알림 | 사용자가 한 일의 결과는 번역하고 OS·서버·라이브러리 오류 원문은 문장 끝에 그대로, 프로그램 내부 진단은 영어. 처음 약 60개 번역으로 시작했고 I8 이 사용자 행동 결과 약 50개를 더했다 | `notify_text`/`notify_with`, `text/notices.rs` |
| Q4 게임 용어 | 원어 유지, 설명문만 번역 | 용어집 1절 |
| Q5 chart | 한국어 "차트", 일본어 "譜面", 일본어 play 는 "プレー" | 용어집 2절 |
| Q6 감수 | 모델 초안 + 용어집 + 되번역. 한국어는 사용자 검수, 일본어는 원어민 감수를 릴리스 노트로 요청(게시는 초안 승인 뒤) | 용어집 4절 |
| Q7 글꼴 | 시스템 글꼴 폴백 유지. 실창 점검 뒤 필요하면 서브셋 글꼴 번들(후속) | 없음 |
| Q8 서버 오류 문구 | 클라이언트는 서버의 한국어 오류 문구를 그대로. 웹 화면은 오류 코드로 번역(웹 ADR) | 없음 |
| Q9 Discord | 영어 유지 | 없음 |
| Q10 구조 | 새 크레이트 없이 `rbms-model` 에 `Language` 와 `translations!` | 아래 1~2 |
| Q11 스킨 헤더 | 헤더 옵션 이름(스킨 메뉴의 약 160개)은 영어 유지. 본문은 확장 불린과 `t(key)` | 아래 4 |
| Q12 순서 | 기반 먼저, 이후 새 기능의 문자열은 처음부터 카탈로그 | 5.0 공통 규칙으로 W2~W5 가 지켰다 |

### 1) 언어와 설정

- `rbms_model::Language { English, Korean, Japanese }`(`ALL`, `id()` 0·1·2, `code()`, `from_code`, `from_locale`, 기본 영어).
- 설정 `display.language`: `AUTO`·`ENGLISH`·`KOREAN`·`JAPANESE` 토큰(`#[serde(default)]` = AUTO, 스키마 6 유지). `rbms-config` 는 `rbms-model` 에 의존하지 않고 독립 열거형 `LanguageChoice` 를 둔다(의존을 더하면 `Cargo.lock` 이 바뀌어 `--locked` 가 깨진다). `Language` 로 바꾸는 일은 플레이어 `app_language.rs` 가 한다.
- 설정 행 LANGUAGE: DISPLAY 탭 맨 위, 선택지 `AUTO`/`ENGLISH`/`한국어`/`日本語`(언어 이름은 그 언어로). `SETTING_COUNT` 97 → 98(이후 RANKING KEYS 로 99). DISPLAY 탭이 18칸을 넘어 스크롤이 생기므로 "NETWORK 만 스크롤" 단언 두 시험을 DISPLAY 도 스크롤한다고 고쳤다.
- 이 값은 `ir_sync.rs` 의 `keep_local`(기기 고유 값)에 넣지 않았다. 계정 동기화로 올라가고 내려온다.
- 렌더 언어는 전역 원자값이 아니라 스레드 로컬이다(`rbms_render::text::{set_language, language}`, `RenderCtx.language`). `app_language.follow` 가 매 프레임 `stage.update` 와 토스트 처리 앞에서 부르므로 언어를 바꾼 같은 프레임에 모든 화면과 시작 알림이 따른다.

### 2) 카탈로그

- `rbms_model::translations!` 매크로: 항목마다 영어·한국어·일본어 세 문자열이 필수다(`Name => ("en", "ko", "ja")`). 번역하지 않는 원어 용어는 한 문자열 `Name => ("EX SCORE")` 로 쓴다. 쓰는 쪽은 `Catalog` trait 의 `get(language)`, `fill(language, &[..])`.
- 각 카탈로그의 시험이 `catalog_problems` 를 부른다: 빈 문자열, 세 언어의 자리표(`{0}` 등) 집합 불일치, 앞뒤 공백 불일치, 원어 유지 표시가 없는데 한국어·일본어가 ASCII 뿐인 항목(영어 복사)을 잡는다. 쓰이지 않는 카탈로그 항목은 컴파일러가 잡지 못한다(조사 보고서는 `dead_code` 경고로 잡힌다고 적었지만, 다른 크레이트의 매크로가 만든 열거형에는 그 경고가 나지 않는 것을 일반 열거형과 견줘 확인했고 코드 문서(`translations.rs`)에 고쳐 적었다). 쓰이지 않는 항목은 사람이 확인한다.
- 위치: 플레이어 `apps/rbms-player/src/text/{settings,select,menus,notices,ranking,layout_play}` 와 하위 모듈, 렌더 크레이트 `crates/rbms-render/src/text/{select,result,hud}`.
- 영어 불변: 모든 번역 단위는 영어 문구가 이전과 글자 하나 다르지 않음을 기존 골든 시험과 영어 대조 시험으로 확인했다(설정 화면은 99행 전체를 전후 덤프해 바이트 동일).
- 조건 분기는 번역문을 비교하지 않는다. 설정 화면의 판별은 영어 정본 값(`value`)으로, 그리는 것은 번역된 `shown`/`choices` 로 갈랐다. IR 줄 색 판단도 `ir_outcome::leads_with` 가 세 언어의 머리말을 찾는다.
- 언어에 따라 달라지는 값을 캐시하는 화면(선곡의 장면 캐시와 폴더 라벨, 설정 목록)은 언어를 캐시 키에 넣는다.

### 3) 알림

- `notify_text(Level, 항목, &[값])`: stderr 에는 영어를 즉시 찍고 큐에는 항목의 세 문자열과 값을 둔다. 프레임 스레드가 `drain` 할 때 현재 언어로 문장을 만든다(작업자 스레드의 알림도 화면 언어로 나온다). 조립 문장은 `notify_with`.
- 번역 기준(코드 `text/notices.rs` 머리 주석): 사용자가 한 일의 결과는 번역하고 오류 원문은 끝에 그대로, 프로그램 내부 상태·진단은 영어.
- 번역하지 않은 것(의도): 스킨 경고·로딩 실패, 동영상, 장치 오류, 내부 상태, 트레이스 저장, 창 열기 실패, CLI 사용법, 디버그 오버레이, 표본 데이터, Discord 문구, 곡 제목·표 이름 같은 데이터, 서버가 주는 문구.

### 4) 기본 스킨

- 스킨은 확장 불린 21150~21152(`21150 + Language::id()`, 하나만 켜짐)로 언어를 읽는다. `lua/i18n.lua` 의 `i18n.t(key)` 가 함수를 돌려주고(`text = i18n.t(key)` 로 그대로 쓴다) 호출될 때마다 불린을 읽으므로 언어를 바꾸면 다음 프레임에 바뀐다. 정적 `constantText` 는 문서를 다시 읽을 때만 바뀌므로 함수 필드로 쓴다.
- 영역 표 `lua/i18n/{common,select_bar,select_info,select_other,play,result,decide}.lua`: `{ 키 = { en, ko, ja } }`, 키는 접두사(`common_`, `bar_`, `info_`, `side_`, `sheet_`, `words_`, `result_`, `decide_`, `play_`)로 일곱 영역에서 유일하다. 지금 키는 234개(세 언어)이다. 숫자를 끼우는 문장은 `i18n.template(key)`(`{0}` 부터), 줄 목록은 `i18n.join(parts)`(일본어는 공백 없음).
- 시험(`crates/rbms-skin/tests/default_pack.rs`): 키 유일, 세 언어 모두 비어 있지 않음, 같은 항목의 자리표 집합 일치, 언어 변경을 따름, 없는 키는 문서 빌드 때 `unknown i18n key` 로 실패.
- 영어 픽셀: 결정·결과·코스 결과·플레이 캡처 106장이 번역 전 팩과 바이트 단위로 같다.
- 외부 스킨은 확장 불린을 읽지 않으므로 영향이 없다. 스킨 헤더 옵션 이름(스킨 메뉴)은 헤더 읽기 단계에서 `main_state` 가 비어 영어다(Q11).

### 5) 용어집과 새 용어

정본은 [2026-10-24-i18n-glossary.md](../reference/2026-10-24-i18n-glossary.md). 번역 단위가 쓰기 전에 용어를 먼저 더하는 규칙이었으나 W4~W5 에서 단위마다 새 용어가 생겨 이 DOC 단위가 모아 추가했고, 보고서끼리 번역이 어긋난 곳은 코드에 실제로 쓰인 것으로 하나로 정하고 어긋난 곳의 목록을 용어집 6절에 남겼다.

## 이유

- 새 의존성 없이 컴파일 시점에 세 언어 필수와 자리표 일치를 강제하고, 영어 화면은 이전과 같게 두는 것이 번역 단위 병렬 작업의 안전선이다.
- 스레드 로컬은 병렬 시험이 서로 언어를 오염시키지 않게 한다(시험은 영어를 명시하고, 필요하면 `InstalledLanguage` 가드가 종료 때 영어를 복원한다).
- 게임 용어는 IIDX·beatoraja 화면의 원어 표기에 사용자가 익숙하고, 설정 값을 영어 정본으로 비교하는 코드와도 맞는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 번역 라이브러리(fluent, gettext 계열) | 새 의존성. 자리표·세 언어 필수를 이미 매크로로 강제 |
| 번역 파일을 런타임에 읽기 | 누락을 컴파일·시험 시점에 잡지 못한다. 배포 파일이 늘어난다 |
| 전역 원자값 언어 | 병렬 시험 오염 |
| 게임 용어도 번역 | 사용자가 원어를 쓰고, 설정 화면의 문자열 비교 로직과 충돌 |
| 스킨 헤더 옵션 이름까지 번역(로더가 언어를 Lua 에 전달) | 로더 변경이 필요한 새 단위. 사용자 선택은 영어 유지(Q11) |
| 서버 오류 문구를 클라이언트가 번역 | 서버가 한국어 문구와 코드를 함께 준다. 웹 화면은 코드로 번역한다(웹 ADR) |

## 영향과 관련 파일

- 설정 묶음 계약 `contract/client-requests/settings-blob.json` 에 `language: "AUTO"` 한 줄(웹 DTO 는 알 수 없는 필드를 그대로 둔다).
- 알려진 한계(테스트 부채 3.112~3.114, QA 3.23): 한국어·일본어 글자 폭과 줄 높이를 실창에서 보지 못했고(일본어 PLAY MENU·SKIN MENU 단추는 키 이름이 `BACKSLASH` 일 때만 macOS 실측으로 1px 넘침), 일본어 원어민 감수가 없고, 캡처 하니스의 `capture/play_parts.rs` 는 영어 외 언어로 캡처할 때 라벨을 영어로 비교해 맞지 않는다.
- 영어로 남은 사용자 표시 문구: 스킨 메뉴 상단 탭 이름(SELECT, DECIDE, RESULT, COURSE, KEYS), `user_folders.rs` 의 `MY FOLDERS` 표 행 라벨, `tablesrc.rs` 의 `UNLOADED_TABLE`·`UNNAMED_TABLE`, `build_blob` 의 ron 오류 원문, 워커 중단 문구, `preview.rs` 의 `[preview]` 진단, 표본 데이터. 같은 화면에서 한국어와 영어가 섞여 보일 수 있다.
- 코드: `crates/rbms-model/src/{language,translations}.rs`, `crates/rbms-config/src/{language,settings,schema}.rs`, `crates/rbms-render/src/{font,ctx,text}`, `apps/rbms-player/src/{app_language,notify,text}`, `apps/rbms-player/src/stage/settings/`, `assets/skins/rbms-default/lua/{i18n,i18n/*}.lua`.
- 개발: 환경변수 `RBMS_LANGUAGE`(`ko`·`ja`·`en`)가 설정이 AUTO 일 때만 OS 로캘보다 앞선다(세 언어에 닿지 않는 값은 건너뛴다. [development.md](../development.md)).
- 문서: [play-options.md](../play-options.md) 15절, [skin.md](../skin.md) 3.4.7절, [글꼴 문서](../font-cjk-support.md), [용어집](../reference/2026-10-24-i18n-glossary.md).

## 날짜와 사용자 원문

2026-10-24.

- 사용자 요청: "1. 다국어 지원 (한글, 영어, 일본어)"
- 사용자 결정: "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로"(웹 다국어와 next-intl 추가를 포함한 결정 1 의 추천안 A).
