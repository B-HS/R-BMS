# ADR: 선곡의 랭킹 모드(파란 건반 길게 누르기), 순위 캐시 수리, 단위 배지, RANKING KEYS 설정

> 최종 갱신 2026-10-24 · 대응 커밋 66b9e63
> 상태: 유효(결정 Q1~Q8 은 사용자 확인 2026-10-24 "18번까지 전부 추천안대로". 아래 "메인 결정"은 사용자 미확인) · 결정일 2026-10-24 · 결정자 사용자(요청과 결정 묶음)와 메인(세부) · 커밋 `d727923`(순위 받기·캐시), `d74794b`(웹 `extra.dans`), `423484b`(랭킹 모드 입력·오버레이·순위 슬롯·RANKING KEYS), `b5f15df`(순위 목록 스크롤), `90d57d1`(기본 스킨 IR RANKING 줄)
> 관계: 순위 응답의 표시 단위는 [단위 자동 인정 ADR](2026-10-23-adr-dan-auto-award.md)의 `user_dan`/`user_dan_display` 를 읽는다. 확장 id 는 [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md).

## 배경

- 사용자 요청(2026-10-24): "랭킹 보는 모드 (2, 4, 6 셋중 하나 누르면 해당 곡의 랭킹들을 볼수있어짐)". IIDX 선곡에서 건반 2·4·6 을 누르면 그 곡의 순위가 열리는 동작이다.
- 조사 보고서 [2026-10-24-select-ranking-survey.md](../reference/2026-10-24-select-ranking-survey.md): 선곡의 IR 랭킹 패널(`I` 키)은 있었지만 받는 줄이 10개(`RANKING_PANEL_LIMIT`)이고 캐시는 만료가 없었고(제출 뒤 낡은 값이 남음), 순위 받기는 패널이 열려 있을 때만 일어났고, 줄에 단위·DJ 등급·날짜가 없었다. 파란 건반(2·4·6)은 선곡에서 목록 역할(폴더 닫기, 정렬, 즐겨찾기)을 이미 가졌다.

## 결정

### 1) 들어가기와 나가기

- 대상: 차트 막대 위에서, 서버 설정이 있고, 패널·모달·검색이 없을 때.
- 파란 건반(`blue_keys`, `stage/result.rs:161` 의 `pub(crate)` 함수를 선곡에서 가져다 쓴다. 옮기지 않았다)을 `SCRATCH_REPEAT_DELAY`(300 ms, 새 숫자 없음) 이상 누르면 열린다. 그보다 먼저 떼면 그때 원래 동작(폴더 닫기, 최상위에서는 정렬)을 한다. 목록 역할이 없는 키(7키 컨트롤러의 6번, 9키 컨트롤러의 2·8번)는 누르는 즉시 연다.
- 키 집합: 컨트롤러 7키 2·4·6, 5키·10키 2·4, DP 양쪽, 9키(pop'n) 버튼 2·8(`POPN_TABLE` 인덱스 1·7), 24키는 없음. 내장 선곡(스킨 없음)의 9키는 8번이 다른 역할을 가져 버튼 2 만 랭킹 키다. 키보드는 `S`·`D`(7·14키의 6번 `F` 는 즐겨찾기 그대로, 키보드 `F` 와 컨트롤러 6번을 함께 누르면 키보드 쪽으로 보아 열리지 않는다).
- 패널(START·SELECT)이 올라온 동안 시작된 누름은 닫힌 뒤 번지지 않는다(`SelectKeys::settled` 규칙). 기다리는 중 다른 랭킹 키를 눌러도 무시한다(원본의 "먼저 눌린 키가 답한다").
- 닫기: 파란 건반 짧게, `Esc`, `I`, 왼쪽. 열기 이벤트는 21002, 켜짐은 불린 21105.

### 2) 모드 안 조작

| 조작 | 동작 |
| --- | --- |
| 턴테이블·위아래·휠 | 줄 이동(`SCRATCH_REPEAT` 반복) |
| 흰 건반·Enter | 그 줄의 리플레이 재생(기존 `play_ranking_replay`) |
| `J`(키 설정의 ReplayPlayLayout) | 그 줄의 배치로 치기([배치 플레이 ADR](2026-10-24-adr-layout-play.md)) |
| SELECT | 내 줄로 점프(`your_line`, 내 줄이 쪽 밖이면 `page_to_find_you` 가 그 쪽을 받음) |
| START | 다음 쪽(`start_ranking_page`) |
| `Q`·`E` | 프로필 전환 |

- 곡에 5초(`RANKING_AUTO_FETCH_AFTER`) 머물면 패널이 닫혀 있어도 순위를 받는다(Songs 탭의 차트). 열려 있으면 기존 150 ms. 열면 즉시.
- 설정 RANKING KEYS(SELECT 탭의 SONG LIST 구획, 켬 기본, 토큰 `library.ranking_keys`, `SETTING_COUNT` 99). 끄면 **키로 여닫는 것만** 꺼진다. `I` 키, 이벤트 21002, 모드 안 조작은 그대로 작동한다(메인 결정). 옛 설정 파일은 켬으로 읽는다.

### 3) 순위 데이터(클라이언트)

- 쪽 받기: `chart_ranking_page(limit = 100, page)`. `chart_ranking_page` 를 지원하지 않는 서버는 `chart_ranking(100)` 으로 물러나고 이때 더 받을 쪽은 없다. `chart_replays` 는 더 부르지 않고 리플레이 id 는 줄의 `extra.replay_id` 에서 온다.
- `RankingRow` 에 `player_id`, `dans`, `total_notes`, `is_you`, `is_rival`, `scratch_auto` 가 늘었다. `RankingBoard` 에 `pages`, `more`, `your_page` 가 늘었다. `rows` 는 위에서 이어진 구간이고 내 순위가 그 밖이면 그 쪽을 `your_page` 에 따로 둔다. 첫 응답을 받은 직후 내 쪽 한 번 더 받는다.
- 캐시: 보드는 10분(`RANKING_READY_LIFETIME`), 실패는 30초(`RANKING_FAILED_RETRY_AFTER`) 뒤에 다시, 용량 64곡. 점수가 서버에 받아들여지면(`poll_submit`, `poll_profile_submits`) 그 곡만 무효화하고 진행 중이던 받기는 세대를 올려 버린다. 시계는 인자로 받아 시험이 가짜 시계로 돈다. `peek` 는 만료된 보드도 돌려주므로(IR BEST 목표와 고스트) 다시 받기는 `start_ranking_fetch` 가 불릴 때만 일어난다. 한 번에 하나만 받는다.
- 순위·정렬·`rank` 의미는 그대로다(EX 정렬 유지, 질문 8).

### 4) 웹 순위 응답의 단위 배지

`GET /api/charts/{hash}/ranking` 의 줄마다 `extra.dans: [{ system, name, label }]` 가 항상 있다(단위가 없으면 `[]`). `system` 은 `genoside2018_sp|genoside2018_dp|stella|satellite`, 체계 순서대로 체계마다 하나. 줄의 사용자 id 전체로 `danService.listDisplayed` 를 한 번만 부른다(쿼리 2개, 순위가 비면 부르지 않는다). `/best`, `/api/scores/{id}`, 코스 순위에는 붙지 않는다. 순위·정렬·`X-Total-Count`(넣지 않음)·캐시 태그는 그대로고 마이그레이션이 없다. 응답은 순위 캐시(`cacheLife('minutes')`)에 같이 들어가 단위를 얻거나 표시를 바꿔도 최대 그 시간만큼 이전 값이 보인다. 응답 타입 `RankingRecordOutput`(`dto/score.ts`), 필수 의존 `ScoreServiceDeps.listDisplayedDans`, 새 행 타입 `ScoreChartRankedRow` 는 웹 내부 변경이다. 자세한 계약은 [api-spec.md](../backend/api-spec.md) 4절.

### 5) 앱 오버레이(그리기)

줄 구성: 순위 | 이름 + 단위 배지(최대 2개, 이름이 좁아지면 배지 생략) | DJ LEVEL(`rbms_render::dj_rank` 로 `total_notes` 가 0 이면 비움) | EX | 램프 | 날짜 | REP. 내 줄은 밝은 레일과 옅은 바탕, 라이벌 줄은 어두운 레일. 머리에 `IR RANKING`, 오른쪽에 `YOU #r`, 둘째 줄에 곡 제목과 키 안내. `board.more` 면 "TOP n SHOWN, MORE ON THE NEXT PAGE", `your_page` 가 있으면 "YOUR PAGE, FROM RANK n" 구분줄. 이전 줄 끝의 FAST/SLOW 개수(F/S) 표시는 날짜 열이 그 자리를 써서 뺐다(`RankingRow.fast_slow` 는 남아 있다). 날짜는 `fmt_datetime`(UTC)의 날짜 부분이다. 내장 선곡(스킨 없음)의 머리에도 곡 제목이 들어간다.

### 6) 외부 스킨과 기본 스킨

- 외부 스킨의 순위 슬롯이 선곡에서 값을 낸다(원본과 같아짐, 지금 R-BMS 대비 바뀜): 정수 380~389(EX)·390~399(순위), 이미지 인덱스 380~389(종류: 0 없음, 1 나, 2 라이벌, 겹치면 나)·390~399(램프), 텍스트 120~129(이름), Rate 8 읽기(`offset / 받은 점수 수`, 최소 1)와 쓰기(`BrowserWrite::RankingPosition`, 원본 `MusicSelector.setRankingPosition` 과 같게 `(int)(max(1, 총원) * value)`, value 는 0 이상 1 미만). 오프셋이 창 끝에 닿으면 다음 쪽을 받는다. 결과 화면의 순위 값은 그대로다.
- **순위 오프셋은 곡이 바뀌면 0 으로 돌아간다**(메인 결정, 원본과 다름. 원본은 슬라이더로만 바꾼다). 총원은 받은 쪽 줄 수라 100줄 단위의 하한값이다. 결과 화면의 `ranking_total` 과 스킨의 총원·램프 분포가 최대 10 에서 100 으로 늘었다.
- 기본 스킨(M8, 사용자 확인 2026-10-24): 랭킹 모드가 열려 있는 동안(불린 21105) MY RECORDS 6줄 자리를 IR RANKING 6줄(텍스트 120~129, 정수 380~399, 이미지 인덱스 390~399)로 바꿔 보인다. Lua 만 바뀌었고 새 엔진 값은 없다.

## 메인 결정(사용자 미확인)

1. RANKING KEYS 를 끄면 키로 여닫기만 꺼진다(`I`, 이벤트 21002, 모드 안 조작은 유지).
2. 순위 오프셋은 곡이 바뀌면 0(원본과 다름).
3. 줄 끝의 F/S 표시를 뺐다.
4. 랭킹 모드에서 앱 오버레이가 정보창 자리를 덮어(x 972 까지) 기본 스킨의 IR RANKING 패널이 대부분 가려진다. M8 이 기대한 "목록 오른쪽에서 곡을 넘기며 같이 보기"와 다르다. 앱 쪽 판단이 필요해 테스트 부채 3.122 로 넘겼고 캡처 하니스에는 랭킹 모드가 열린 선곡 장면이 없다.
5. 길게 누름 300 ms 의 손맛과 "뗄 때" 동작은 실기로 보지 않았다(RANKING KEYS 끄기가 안전판, QA 3.23).

## 이유

- 기존 패널과 `play_ranking_replay`, 이벤트 처리를 그대로 두고 입력과 받는 시점만 바꿔 위험을 줄였다. 새 Stage 를 만들지 않는다(질문 4).
- 파란 건반을 길게 누르기로 가른 것은 목록 역할(폴더 닫기·정렬)을 지키기 위해서다. 설정으로 끌 수 있다.
- 서버에서 단위를 한 번의 쿼리로 붙이면 클라이언트가 줄마다 프로필을 부르지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 별도 랭킹 Stage | 입력·그리기·캐시를 모두 새로 만들어야 한다(질문 4) |
| 파란 건반을 짧게 눌러 열기 | 폴더 닫기·정렬과 충돌 |
| 줄마다 프로필 API 호출로 단위 | 요청 폭증. 서버가 한 번에 붙인다 |
| 순위를 EX 가 아닌 다른 기준으로 모드 안에서 재정렬 | 서버 `rank` 의미를 바꾼다(질문 8) |
| `blue_keys` 를 공유 위치로 이동 | 이미 `pub(crate)` 라 필요 없다(M9) |
| 자동 받기를 패널이 열렸을 때만 | 순위 슬롯 값(외부 스킨)이 패널을 닫은 채로는 비어 있다(질문 6) |

## 영향과 관련 파일

- 설정 묶음 계약 `contract/client-requests/settings-blob.json` 에 `ranking_keys: true` 한 줄(웹 DTO 영향 없음).
- 코드: `apps/rbms-player/src/{ir_ranking,app_ranking,ir_ranking_view}.rs`, `stage/select/{ranking_mode,ranking_scroll,mod,skinned,events,overlay}.rs`, `skin_host/{ir,select,select/bridge,writers}.rs`, `crates/rbms-ir/src/dto/score.rs`(`extra` 읽기 도우미 `replay_id()`·`is_scratch_auto()`·`dans()`), `crates/rbms-config/src/{schema,settings}.rs`, 웹 `web/src/server/{service/domain/score/score.service.ts,compose/index.ts,dto/score.ts}`.
- 테스트 부채: 3.120(응답 크기·지연), 3.122(오버레이가 IR RANKING 패널을 덮음), 3.123(손맛과 키 충돌). 남은 위험: 같은 순위 동점이 내 위에 있으면 내 행이 계산한 쪽의 다음 쪽에 있을 수 있어 `your_page` 에 내 행이 없을 수 있고 `you` 행은 따로 있다(내 줄로 점프가 `is_you` 행이 없을 때를 처리한다). 서버가 있으면 차트에 5초 머물 때마다 순위 요청이 간다. 쪽은 순차(`pages + 1`)로만 받아 `your_page` 가 있는 보드에서는 그 앞 구간이 빌 수 있다.
- 문서: [play-options.md](../play-options.md) 16절, [ir-api.md](../reference/ir-api.md).

## 날짜와 사용자 원문

2026-10-24.

- 사용자 요청: "3. 랭킹 보는 모드 (2, 4, 6 셋중 하나 누르면 해당 곡의 랭킹들을 볼수있어짐)"
- 사용자 결정: "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로"(랭킹 질문 1~8 의 추천안 A, 그리고 M8 의 결정 16 A).
