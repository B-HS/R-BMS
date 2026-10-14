# ADR: 설정 화면에서 Enter 는 다음 값

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(추천안 A)

## 배경

웨이브 16 이 설정 화면(Tab)을 묶음 레일 + 행 목록 + 정보 패널로 재구성했다(`docs/plan/2026-10-14-settings-screen.md`). 그때까지 스위치·선택·SOUND FOLDER 행의 `Enter` 는 저장하고 화면을 닫았고, SKIN·PLAY 메뉴의 `Enter` 는 다음 값이었다. 리뷰가 두 문법의 차이를 찾아 사용자에게 물었다.

## 결정

- 스위치 행은 `Enter` 로 뒤집고, 선택 행은 다음 값(오른쪽 화살표와 같은 한 칸, `ROW_STEP_FORWARD`)으로 바꾼다. SOUND FOLDER 는 폴더를 고른다. SKIN·PLAY 메뉴와 같다.
- 화면을 닫는 키는 `Esc` 하나다. `Enter` 는 어느 행에서도 화면을 닫지 않는다.
- 숫자(직접 입력), KEY CONFIG, FONT, NETWORK 행의 `Enter` 는 전과 같다. LOGOUT 은 전처럼 확인 없이 바로 로그아웃한다.
- 바닥 안내는 지금 행이 실제로 듣는 키만 보인다(`info::row_keys`).

## 이유

세 메뉴(설정, SKIN, PLAY)의 조작 문법을 하나로 맞춘다. 사용자가 이전 설정 화면을 "딱딱하고 불편해 보인다"고 했고, 같은 키가 화면마다 다르게 동작하는 것이 그 원인 가운데 하나다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| B. 이전대로 `Enter` = 저장하고 닫기 | SKIN·PLAY 메뉴와 문법이 다르다. 사용자 A 선택 |

## 영향과 관련 파일

- `apps/rbms-player/src/stage/settings.rs`(`ROW_STEP_FORWARD`, `Enter` 가지가 `step(.., ROW_STEP_FORWARD, ..)`), `apps/rbms-player/src/stage/settings/info.rs`.
- 테스트 `the_line_of_keys_says_what_the_row_under_the_cursor_hears`(`apps/rbms-player/src/stage/settings/layout_tests.rs`).
- 설계 문서 `docs/plan/2026-10-14-settings-screen.md` 4절, 12절.
- 커밋: `8b11a17`. 웨이브 18.
- 남은 일(미착수): 설정 화면의 컨트롤러 조작(`handle_pad` 없음).

## 날짜와 사용자 원문

2026-10-14. 사용자 답 "(2) 설정 화면의 스위치·선택 행 Enter 는 다음 값(추천안)". 질문 원문은 보존되지 않았다(`docs/PROCESS.md` 웨이브 16 결과 아래 기록).
