# ADR: 어시스트 없는 클리어 램프와 더 높은 어시스트 등급이 함께 있으면 목록 막대 램프를 위아래 반으로 쪼개 둘 다 보인다

> 최종 갱신 2026-11-19 · 대응 커밋 `4874930`(구현), `fe1f427`(계획)
> 상태: 유효(구현됨). 사용자가 정한 것은 추가 요청 한 줄뿐이고 M20(쪼개는 조건, 클라이언트 범위)은 메인의 결정이며 사용자 확인 전 · 결정일 2026-11-19 · 결정자 사용자(추가 요청)와 메인(M20), 구현 단위 U10(4절) · 커밋 `4874930`
> 관계: [등급 램프 목록 ADR](2026-11-19-adr-assist-grade-lamps-in-lists.md)의 `lamp_graded` 그림과 `SongBar.assist_grade` 를 쓴다. 최고 램프 하나가 `(램프 id, 등급)` 의 최댓값이라는 [어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md) 4절의 비교 규칙이 이 요구의 출발점이다. 계획 [plan/2026-11-19-feature-upgrade-12.md](../plan/2026-11-19-feature-upgrade-12.md) 1절 M20·2절 U10.

## 배경

- 사용자 추가 요청(2026-11-19, 원문): "추가기능으로 램프에있어서, Assist와 assist가 없는 클리어램프가 2개 있는경우 클리어램프를 반으로 쪼개서 두 개 다 표시가능하도록해".
- 지금 한 차트의 최고 램프는 `(램프 id, 등급)` 의 최댓값 하나다. 어시스트 없는 EASY CLEAR(id 4)가 생기면 LIGHT ASSIST(id 3) 안의 등급(예: ASSIST HARD)은 최고 기록 행에서 사라진다. `crates/rbms-store/src/scoredb.rs` 의 `merge_best`(램프 쌍이 오를 때만 행을 바꿈)와 `chart_best_many`(차트별 최고를 접음)가 그 자리다. 그러면 "EASY 로는 어시스트 없이 깼지만 HARD 게이지로는 SCRATCH AUTO 로만 깼다"는 정보가 목록에서 보이지 않는다.
- 판 기록(`scorelog`)에는 등급이 판마다 남아 있으므로 소급해 채울 수 있다.

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | 어시스트 램프와 어시스트 없는 클리어 램프가 둘 다 있는 차트는 클리어 램프를 반으로 쪼개 둘 다 보인다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M20 | 한 차트에 어시스트 없는 클리어 램프(EASY 이상)와 어시스트 램프(SCRATCH AUTO 로 깬 등급 1~4)가 함께 있고 **어시스트 쪽이 더 높은 게이지**일 때, 선곡 목록 막대의 램프를 위아래 반으로 나눠 위에 어시스트 없는 램프, 아래에 어시스트 등급 색을 그린다. 어시스트 없는 램프가 같거나 높으면 지금처럼 그 램프만 그린다(A-단위 결정과 같은 규칙: 정식이 있으면 정식이 이긴다). 범위는 클라이언트다 |

### 3) 계약(계획서 U10 절에서 옮김. 구현 단위가 이 계약을 지켜야 한다)

| 항목 | 내용 |
| --- | --- |
| 쪼개기 조건 | "최고 램프가 EASY·CLEAR·HARD(id 4~6)이고, 최고 어시스트 등급 + 3 이 그 id 보다 큼". 어시스트 등급 k(1~4)는 램프 id 3 안의 ASSIST EASY·CLEAR·HARD·EXHARD 이고 같은 게이지로 어시스트 없이 깼을 때의 램프 id 는 k + 3 이다(EASY 4, CLEAR 5, HARD 6, EXHARD 7). 그래서 `등급 + 3 > id` 는 "어시스트로는 더 높은 게이지로 깼다"와 같다. 최고 램프가 EXHARD(7) 이상이면 등급 4 도 넘지 못하므로 쪼개지 않는다. 순수 함수 하나로 두고 표로 시험한다 |
| 저장 | `score`(최고 기록) 행에 "지금까지의 최고 어시스트 등급"(0~4) 열을 더한다. 기존 열 추가 방식 `ADDED_COLUMNS` 를 쓰고 스키마 버전(`SCORE_DB_SCHEMA_VERSION`)·규칙 세대(`SCORE_RULE_VERSION`)는 그대로 둔다. 판을 저장할 때 갱신하고, DB 를 열 때 `scorelog` 에서 소급한다(멱등, 기존 `refold_assist_grades` 와 같은 방식). `scores.ron` 대체 저장소에도 같은 값을 둔다(`#[serde(default)]`) |
| 렌더 | `SongBar` 에 "쪼갤 어시스트 등급"(0 이면 쪼개지 않음)을 더한다. 값이 있고 `lamp_graded` 의 그 등급 그림이 있으면 램프 칸의 위 절반에 램프 그림의 위 절반, 아래 절반에 등급 그림의 아래 절반을 그린다. `lamp_graded` 가 없는 스킨은 한 화소도 달라지지 않는다 |
| 정보창 | MY SCORE 의 램프 이름 줄: 쪼개진 차트는 어시스트 등급 이름을 함께 보인다. 확장 불린 21160~21163 = 쪼개진 등급 1~4 |
| 내장 선곡 | 스킨이 없는 내장 선곡 막대의 LED 도 같은 규칙으로 반씩 칠한다 |
| 바꾸지 않는 것 | 서버 전송, 기록 동기화, IR 본문. 최고 기록의 램프 비교 순서(`(램프 id, 등급)`), 정렬 Clear, CLEARED 필터. 폴더 램프 분포와 스크롤 분포(11칸) |

### 4) 구현 세부(구현 단위 U10 이 정함, 메인이 캡처로 확인)

| 항목 | 내용 |
| --- | --- |
| 조건 함수 | `rbms_store::split_assist_grade(best: RecordLamp, best_assist_grade) -> u8`. 램프 id 4~6 이고 등급이 1~4 이며 `3 + 등급 > id` 일 때 그 등급, 아니면 0. 표 전체를 시험한다 |
| 저장 열 | `score.best_assist_grade INTEGER NOT NULL DEFAULT 0`(`schema.sql`, `ADDED_COLUMNS`). `merge_best` 가 판의 등급 최댓값을 유지하고, `refold_best_assist_grades` 가 DB 를 열 때와 코스 스테이지를 CLASS 로 옮길 때 `scorelog` 에서 소급한다(바뀔 행만 쓰므로 멱등, 규칙 세대 2 미만 행과 `assist = -1` 행은 등급 없음) |
| 조회 | `ScoreDb::best` 는 같은 차트·모드의 모든 LN 방식 행에서 최댓값을 접고 `chart_best_many` 는 `MAX` 로 접는다. 그래서 `best` 의 이 값은 차트 값이고 `best_in_ln_mode` 의 값은 행 값이다 |
| `scores.ron` | 필드를 더하지 않았다(계획과 다름). `ScoreRecord` 는 판 하나이고 등급은 판의 값에서 유도되므로, `ScoreBook::best_assist_grade_for_md5` 가 판들에서 접는다. 선곡 막대는 DB 경로에서도 같은 함수를 쓴다 |
| 렌더 | `SongBar.split_assist_grade`. 그림을 통째로 그리고 램프 칸의 가운데 선에서 자른다(`push_clip`): 위 절반은 램프 그림, 아래 절반은 `lamp_graded` 그림. 원본을 반으로 자르는 대신 클립이라 늘림·각도·깜빡이는 프레임이 그대로이고 홀수 높이에서도 빈틈이 없다(계획과 다름). `lamp_graded` 가 없는 스킨, 라이벌 모드, 범위를 넘는 등급은 통째로 그린다 |
| 플레이어 | `SelectBar.split_assist_grade`(`stage/select/list.rs` 의 `chart_bar`), `scene.rs` 가 막대와 내장 화면에 넘긴다. 내장 선곡은 `SelectRow.split_lamp: Option<Color>` 로 LED 를 위아래 반씩 칠한다 |
| 정보창 | 확장 불린 21160~21163(`RBMS_OPTION_SELECTED_BEST_SPLIT_GRADE_FIRST`·`LAST`, 등급 1~4). `select/info.lua` 의 MY SCORE 램프 줄이 쪼개진 차트에서 왼쪽 반칸에 램프 이름, 오른쪽 반칸에 등급 이름(등급 색)을 쓴다. 새 번역 문구는 없다(게임 용어) |
| 시험과 캡처 | 조건 표, 저장·소급, 렌더 화소(위·아래 색), 스킨 덤프 시나리오(`apps/rbms-cli/scenarios/select.json` 에 램프 3·등급·쪼개진 막대), 캡처 시험 `the_browser_splits_the_lamp_of_a_chart_cleared_without_assists_and_with_scratch_auto_on_a_stronger_gauge`. 메인이 캡처를 열어 확인: 막대 램프 위 노랑(CLEAR)·아래 라벤더(ASSIST HARD), MY SCORE `CLEAR  ASSIST HARD` |
| 확인하지 못한 것 | GPU 백엔드(실제 창)의 반쪽 자르기, 랭킹 모드의 등급 색 캡처, 세 언어 캡처 |

## 이유

- 한 램프 칸에 정보 둘을 보이려면 칸을 반으로 나누는 것이 새 칸을 만들지 않고 목록 막대 구조를 바꾸지 않는 방법이다.
- 어시스트 쪽이 더 높을 때만 쪼개면 "EASY 로 정식 클리어한 곡에 어시스트로 HARD 도 깼다"는 정보가 생긴다. 정식 램프가 같거나 높으면 어시스트 램프는 정보가 없다.
- 클라이언트 저장소에만 열을 더하면 서버 DB 마이그레이션이 필요 없고 점수 서버·IR 본문과 무관하다.
- `scorelog` 소급은 이미 등급 소급(`refold_assist_grades`)이 쓰는 방식이라 새 위험이 적다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 둘 다 있으면 항상 쪼개기 | EASY CLEAR 가 이미 ASSIST EASY 를 포함하는데 반으로 쪼개면 같은 게이지의 두 램프가 나란히 보여 정보가 없다. 어시스트가 더 높을 때만 정보가 있다 |
| 웹까지 같은 표시 | 서버 `chart_best` 에 열을 더하는 DB 마이그레이션(운영 적용은 사용자 실행)이 필요해 범위가 커진다. 요청의 대상은 곡 선택 램프라 클라이언트 범위로 했다 |
| 최고 램프 비교를 `(램프 id, 등급)` 에서 바꿔 두 값을 따로 최고로 두기 | 정렬·필터·분포·IR·웹이 모두 같은 비교를 쓴다 |
| 램프 칸을 위아래가 아니라 좌우로 쪼개기 | 램프 칸이 세로로 긴 띠(8px 폭)라 위아래가 읽기 쉽다. 이 판단은 계획서에 없고 문서를 쓴 쪽의 추정이다 |

## 영향과 관련 파일(계획 기준)

- 소유(계획서): `crates/rbms-store/**`, 플레이어의 최고 램프 조회와 막대 채우기(`apps/rbms-player/src/stage/select/{list.rs, scene.rs}`, 점수 캐시), `crates/rbms-render/src/skin_render/songlist*`, `assets/skins/rbms-default/lua/select/{list.lua, info.lua}`, 내장 선곡 막대의 LED.
- 완료 기준(계획서): 조건 표 시험, 저장·소급 시험(EASY 정식 + ASSIST HARD 판이 있는 DB), 렌더 화소 시험(위·아래 색), 스킨 덤프, 캡처.
- 구 빌드가 같은 DB 를 다시 쓰면 `best_assist_grade` 가 0 이 되고 다음 실행에서 소급된다(`assist_grade` 와 같은 성질).
- 알려진 위험(구현 전의 예상이고 확인한 것이 아니다): 소급이 `scorelog` 가 지워진 오래된 DB 에서는 등급을 복원하지 못한다. 쪼개진 막대의 위·아래 경계가 8px 폭에서 어떻게 보이는지는 캡처로 봐야 한다. 정렬·필터는 최고 램프 하나만 보므로 쪼개진 차트의 어시스트 등급은 정렬에 반영되지 않는다.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 추가 요청(원문): "추가기능으로 램프에있어서, Assist와 assist가 없는 클리어램프가 2개 있는경우 클리어램프를 반으로 쪼개서 두 개 다 표시가능하도록해"
- 어시스트 쪽이 더 높을 때만 쪼갠다는 조건, 클라이언트 범위, 저장 열 방식은 모두 메인의 결정이다.
