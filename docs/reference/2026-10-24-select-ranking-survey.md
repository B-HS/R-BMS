# 2026-10-24 조사: 요구 3 선곡 "랭킹 보는 모드" (2·4·6 건반)
> 최종 갱신 2026-10-24 · 대응 커밋 9fed3c0(작업 트리에 `docs/PROCESS.md`·`docs/HANDOFF.md` 변경이 있었으나 이 조사와 무관)
> 출처: workflow 하위 에이전트(Sonnet, 읽기 전용) 조사. 코드·문서는 읽기만 했고 빌드, 테스트, 플레이어 실행, 운영 사이트 요청은 하지 않았다. 파일:줄은 위 커밋 기준이다. 외부 사실은 출처 URL 을 붙였고 확인하지 못한 것은 "확인 못 함"이라 썼다.

## 0. 요구 원문과 참고 화면

요구 3(사용자 원문): "랭킹 보는 모드 (2, 4, 6 셋중 하나 누르면 해당 곡의 랭킹들을 볼수있어짐)". 참고 화면은 `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/115ab922-671c-4689-ab5b-8eb67a2561d2/images/9.png` 이고 Read 로 직접 보았다.

화면에서 관찰한 것(IIDX 선곡, SP FINAL STAGE):
- 가운데 아래의 `RIVAL SCORE DATA` 패널. 머리글 아래 6줄이고 각 줄은 `N位 이름 [DJ LEVEL 글자 AA/A/B] 점수 램프이름` 순이다(예: `1位 UKYO* AA 977 E-CLEAR`, `2位 823N AA 952 EXH-CLEAR`, `4位 KUSAHA A 863 H-CLEAR`, `5位 HTJI(SUB 표지) B 674 A-CLEAR`, `6位 AGEHA* 0 NO PLAY`).
- 줄의 순서는 점수 내림차순이다(977 EASY 가 952 EXH 보다 위). 램프 우선이 아니다.
- 패널 바로 아래에 별도 줄 `店内 (왕관) MI-OH? [AAA] 1067`. 매장 1위 한 줄이다.
- 이 패널은 키 조작 없이 곡에 커서가 있는 동안 그려져 있다. 화면 아래 안내줄은 건반 그림 두 줄에 `フォルダを閉じる`(위 줄, 검은 건반)와 `決定`(아래 줄, 흰 건반)이다. 즉 이 화면에서 검은 건반(2·4·6)의 일은 "폴더 닫기"로 적혀 있다.
- `SUB` 표지는 IIDX 32 에 들어온 サブライバル 일 것으로 보이나 확인 못 함(검색 결과가 기능 이름만 말했다: https://www.konami.com/arcadegames/products/am_bmiidx34/ 은 이 표지를 설명하지 않는다).

## 1. 요구 해석

1. 선곡에서 "랭킹 모드"라는 상태가 있고, 2·4·6번 건반(R-BMS 의 "파란 건반", 이미 `stage/result.rs:146-165` 의 `blue_keys` 로 7키·14키 2·4·6, 5키·10키 2·4, 9키 4·6 이 정의돼 있다) 가운데 하나를 누르면 그 모드로 들어가 커서가 있는 곡의 IR 순위 여러 줄을 볼 수 있다.
2. "2, 4, 6" 을 숫자 줄 키(기본 `2`=정렬, `4`=리플레이 슬롯, `6`=키 설정 단축키. `keyconfig.rs:925-929`)로 읽을 수도 있으나, 조사 지시문이 "검은 건반"이라 했으므로 건반으로 읽는다(질문 5번에서 한 번 더 확인).
3. 참고 화면이 말하는 표시 항목: 순위, 이름, 등급(DJ LEVEL), 점수, 램프. 지시문이 더한 항목: 단위 배지, 날짜. 모양은 IIDX 의 `RIVAL SCORE DATA` 줄과 같게 한다.
4. 다루는 범위: 키 충돌 해결, 순위 데이터, 표시, 내 순위·주변 순위·스크롤·리플레이 받기, 외부 스킨. 요구 2(리플레이 받기)·요구 5(IIDX 식 RIVAL 패널)와 파일을 같이 쓴다(7절).

## 2. 현재 R-BMS 상태(관련 코드 지도)

### 2.1 선곡 건반 배정: 2·4·6 이 지금 하는 일

선곡의 건반 표는 beatoraja `MusicSelectKeyProperty` 와 같은 아홉 칸이다. 건반 번호 1~7 은 인덱스 0~6, 턴테이블 두 방향은 7·8(`keyconfig.rs:1052-1058`). 2·4·6 은 인덱스 1·3·5 다. 어느 표를 읽는지는 스킨이 선곡 헤더에 `rbms_extensions` 를 선언했는지로 갈린다(`stage/select/keys.rs:225-262`, `KeyLayout::of_browser`). 목록 역할(START·SELECT 를 누르지 않은 때)은 두 표가 같다.

| 입력 | 2번(인덱스 1) | 4번(인덱스 3) | 6번(인덱스 5) | 근거 |
| --- | --- | --- | --- | --- |
| 7키·14키(키보드 기본 S·D·F, 14키 2P 는 K·L·;, 컨트롤러는 건반 2·4·6) | FOLDER_CLOSE | FOLDER_CLOSE | 목록 역할 없음(원본 NEXT_REPLAY 를 뺐다) | `keys.rs:117-127`(BEAT_TABLE), 같은 줄 구조의 원본 `MusicSelectKeyProperty.java:10-18` |
| 5키·10키 | FOLDER_CLOSE | FOLDER_CLOSE | 레인이 없다(인덱스 5·6 은 닿지 않는다) | `play-options.md` 12.6절, `keyconfig.rs:1071-1084` |
| 9키 pop'n(키보드 기본 Z S X D C F V G B, 스크래치 없음) | 목록 역할 없음(패널 전용) | DOWN(목록 이동) | UP(목록 이동) | `keys.rs:132-142`(POPN_TABLE) |
| 24키 | 인덱스 7·8 은 스크래치용이라 일부 레인은 인덱스가 없다 | | | `keyconfig.rs:1071-1084` |
| 턴테이블(인덱스 7·8) | UP·DOWN(목록 이동), 패널 1 에서 TARGET | | | `keys.rs:124-126` |

FOLDER_CLOSE 가 하는 일(`skinned.rs:318-321`, `close_bar` 355-364): 폴더 안이거나 검색 중이면 한 단계 위로(`select_back`), **최상위(폴더 밖)에서는 정렬 순서를 바꾼다**(`sort_event`). 원본도 같다(`BarManager.java:490-494`: `dir.size == 0` 이면 `EventType.sort`, 아니면 폴더 닫기. 입력 처리는 `MusicSelectInputProcessor.java:326-329`). 따라서 2·4번은 이미 두 가지 일을 맥락으로 갈라 하고 있다. 6번은 컨트롤러에서는 비어 있고, 키보드에서는 기본 키 `F` 가 즐겨찾기 단축키(`keyconfig.rs:912`, `Shortcut::Favourite`)와 같은 키다(`play-options.md` 12.2절 끝: "6번 키가 즐겨찾기만 하고").

START·SELECT(또는 상세 옵션 키)를 누르고 있는 동안 2·4·6 은 옵션 패널의 키가 된다(`play-options.md` 12.2절 표): 패널 1 은 RANDOM 이전·GAUGE 이전·HI-SPEED FIX 이전, 패널 2 는 CONSTANT·LEGACY NOTE·LANE COVER, 패널 3 은 GAUGE AUTO SHIFT·NOTE DISPLAY TIME 줄임·늘림(원본 표를 쓰는 스킨은 패널 1 의 4·6번이 DP OPTION·RANDOM 이전, 패널 2 의 6번은 죽은 칸). 그러니 랭킹 키는 패널이 올라온 동안 일하지 않아야 한다(패널이 누름을 가져간다, `skinned.rs:278-282`).

입력 경로의 차이:
- 스킨이 그리는 선곡(기본 스킨 포함)은 매 프레임 키 상태로 표를 읽는다(`skinned.rs:256-294`). 키보드의 레인 키도 `key_index_pressed` 로 건반 번호가 된다(`app_input.rs:348-358`).
- 스킨이 없는 내장 선곡은 컨트롤러 레인 이벤트만 건반으로 읽고(`mod.rs:1297-1338`, 목록 역할은 항상 `KeyLayout::Beat` = `mod.rs:116`) 키보드 레인 키는 단축키로만 해석한다(`mod.rs:1206-1278`).
- DP 는 두 번째 사이드의 레인이 같은 인덱스로 접힌다(`keyconfig.rs:1071-1084`). 양쪽 어느 2·4·6 이든 같은 일이다.
- 결과 화면에는 선례가 있다. 파란 건반(2·4·6)은 확인(OK)을 **누를 때가 아니라 뗄 때** 한다. START 를 옆에서 누를 시간을 주기 위해서다(`play-options.md` 10.7절 표 "파란 건반의 원래 역할", `stage/result.rs:470-478`).

### 2.2 이미 있는 IR 순위 패널

선곡에는 이미 "IR RANKING" 오버레이가 있다. 새로 만드는 일의 대부분은 이것을 건반으로 쓸 수 있게 하고 넓히는 일이다.

| 항목 | 현재 | 근거 |
| --- | --- | --- |
| 여는 길 | 키보드 `I`(단축키 `Shortcut::Ranking`, 키 설정에서 바꿈, 컨트롤러 버튼에도 줄 수 있음), 닫기는 `I`·Esc·왼쪽 화살표 | `keyconfig.rs:729,914,946-949`, `mod.rs:767-773,1035`, `ir_ranking_view.rs:28-37` |
| 패널 안 조작 | 위·아래 화살표, Enter·오른쪽 화살표=리플레이 받기, Q·E=IR 프로필 바꾸기, 마우스 클릭(선택된 줄을 다시 누르면 리플레이). **마우스 휠은 안 먹고, 건반(레인)과 턴테이블은 아무 일도 하지 않는다** | `mod.rs:805-835,837-851,1391-1396`, `skinned.rs:141-165`(패널이 열리면 목록이 "덮인" 상태라 건반을 읽지 않는다), `mod.rs:409-411` |
| 그리는 곳 | 스킨 위에 앱이 덧그림. 사각형은 내장 선곡의 목록 자리(논리 1280x720 의 x 32, y 60, 616x600), 스킨의 목록은 옆에 남는다. 한 화면 17줄 | `overlay.rs:97-100,223-229`, `theme.rs:110-120`, `ir_ranking_view.rs:170-173` |
| 줄의 내용 | 순위(`#n`), 이름, EX, 램프 이름·색(등급 포함), 꼬리(`F12/S8`, `REP`). **등급(DJ LEVEL)·단위·날짜는 없다** | `ir_ranking_view.rs:82-124` |
| 받는 것 | 곡에 멈춘 지 150 ms 뒤(`FOCUS_DETAIL_DEBOUNCE`, `lib.rs:192`) **패널이 열려 있을 때만** 한 번에 하나, 작업 스레드에서: ① `chart_ranking(limit=10)` ② 내 최고(`player_best`, 로그아웃이면 `guest`) ③ 라이벌(설정의 `network.rivals` 최대 8명, 한 명당 요청 1건) ④ `chart_replays(limit=10)`(플레이어당 첫 리플레이를 줄에 붙임). 순위 요청이 실패하면 나머지는 하지 않는다 | `mod.rs:882-897`, `app_ranking.rs:75-96`, `ir_ranking.rs:17,24,217-236` |
| 캐시 | md5 키, 최대 64곡 LRU. **만료 시간이 없다.** 지워지는 때는 서버·프로필 재구성과 프로필 전환뿐(`reset_ranking`) | `ir_ranking.rs:21,123-129`, `app_ranking.rs:118-136`, `app_network.rs:190` |
| 같은 캐시를 읽는 곳 | 선곡 스킨의 IR 타이머·옵션·숫자, 플레이의 IR BEST·RIVAL 목표(`ranking_targets`), IR BEST 고스트(`account_best_played_here`), 결과 화면의 순위 총원 | `skin_host/select/bridge.rs:90`, `app_result.rs:723-731`, `ghost.rs:146-160`, `stage/result.rs:932-941` |
| 서버 없음 | `has_primary_ir_server()` 가 거짓이면 한 줄 `IR OFF - SET SERVER URL IN SETTINGS`. 기본 서버 주소가 들어 있어 평소에는 참이다 | `mod.rs:788-791`, `ir_ranking_view.rs:75` |
| 로그아웃 | 순위 목록은 인증이 필요 없어 보인다. 내 줄(YOU)만 `guest` 로 물어 빈 줄이다 | `ir_session.rs:182-188`, 서버 2.4 절 |

### 2.3 서버(웹) 순위 API

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 요청 | `GET /api/charts/{md5 또는 sha256}/ranking?limit=&page=&lnmode=&rival_of=&include_scratch_auto=`. `limit` 1~500(기본 50), `page` 1 이상. 클라이언트는 `include_scratch_auto` 를 싣지 않아 일반 판 최고만 받는다 | `web/src/server/dto/score.ts:101-107`, `route/chart.ts:58-73`, `docs/backend/api-spec.md` 4절 |
| 인증 | 필요 없음. 전용 속도 제한도 없다(제한 표의 `PUBLIC_READ` 120/분/IP 는 `/api/fe/*` 용) | `with-rate-limit.ts:4-14`, `api-spec.md` 457행 |
| 캐시 | `cacheLife('minutes')` + 태그 `chartRanking(sha256)` 는 점수 제출 때 무효화 | `read-cache.ts:40-45,75-80` |
| 응답 | `ScoreRecord[]`: `player.id`, `player_name`, `clear`(램프 이름), `assist_grade`, `ex_score`, `max_combo`, `minbp`, `rank`(공동 순위, `RANK()`), `played_at`(ms), `lntype`, `option`, `total_notes`, `judge`(판정 합계와 fast/slow), `extra.replay_id`(리플레이가 연결된 행만), `extra.scratch_auto` | `score.ts:73-89`, `score.service.ts:213-228` |
| 정렬·순위 | 램프 → 어시스트 등급 → EX → BP(적은 쪽 위), 동점은 공동 순위. **원본(beatoraja)과 참고 화면은 EX 만으로 정렬한다.** 의도한 설계로 문서에 있다 | `best-scope.ts:133-148`, `api-spec.md` 187행 |
| 주변 순위 | "내 주변" 전용 경로는 없다. 내 순위는 `GET /charts/{hash}/best?player=` 의 `rank`, 주변은 `page`·`limit` 로 그 칸을 직접 받아야 한다 | `score.service.ts:510-524` |
| 총원 | 없다(배열만 돌려줌). 알려면 끝까지 넘겨 봐야 한다 | `route/chart.ts:72` |
| 단위 배지 | **순위 응답에는 없다.** 단위는 프로필(`extra.dans`)과 전체 리더보드(`/api/fe/...`)에만 붙는다. 배치로 한 번에 구하는 `danService.listDisplayed(userIds)` 가 있다(쿼리 2개) | `fe.service.ts:80-84,108-111`, `player.service.ts:35-37`, `dan.service.ts:147-163` |
| 오프라인 | 요청 하나당 시간 제한 5초(`REQUEST_TIMEOUT`). 실패는 `RankingState::Failed` 로 캐시되고 **같은 세션에서 다시 시도하지 않는다**(2.5절) | `ir-api.md` 4절, `ir_ranking.rs:123-129` |

클라이언트 IR 크레이트: `ScoreServer::chart_ranking(chart, limit)`(`rbms-ir/src/lib.rs:54`), 쪽·LN 방식·라이벌 거르기를 쓰는 `chart_ranking_page(&ChartRankingQuery)`(`lib.rs:103`, `http.rs:268-270`, `dto/query.rs:1-60`), `chart_replays`(`lib.rs:116`), `download_replay`(`lib.rs:66`). `ScoreRecord` 는 `extra: HashMap<String, Value>` 라 새 `extra` 키를 더해도 옛 클라이언트가 깨지지 않는다(`dto/score.rs:178-206`).

### 2.4 스킨이 순위를 그리는 길

- 원본이 선곡 스킨에 주는 순위 목록 id 는 R-BMS 가 이미 알고 있다: 수 380~389(순위 슬롯 n 의 EX), 390~399(슬롯 n 의 순위 숫자), 이미지 인덱스 380~389(플레이어 종류 0 없음·1 나·2 라이벌)·390~399(슬롯의 램프), 글 120~129(이름), 비율 8(목록 위치). 그러나 선곡에서는 **전부 "값 없음"을 답한다**(`skin_host/ir.rs:249,414,432`, `ir.rs:28-31`의 설명). 위치 쓰기(비율 8)는 결과 화면만 받는다(`skin_host/writers.rs:19`). `docs/skin.md` E8·6절도 "목록 값의 스킨 연결은 후속"이라 적었다.
- 순위 총원(`NUMBER_IR_TOTALPLAYER`)과 램프 분포는 지금 받은 줄 수(최대 10)로 센다(`ir.rs:186`, 설명 `ir.rs:29-31`). 원본은 서버가 돌려준 전체 수다.
- ModernChic 의 사이드메뉴 "랭킹" 탭은 단추(`act`)로 열고, 10줄을 380~399·120~129 로 그리며, 슬라이더 `type=8` 로 목록을 넘긴다(`/Users/hyunseokbyun/Downloads/ModernChic/Select/lua/sidemenu.lua:52-60,114-117,181-190,222`, 조사 문서 `skin-compat/m4-select.md` 476-486·750행). 값만 채우면 이 스킨은 새 확장 id 없이 그릴 수 있다.
- 기본 스킨은 선곡에서 순위 값을 읽지 않는다. 순위는 앱 오버레이가 그린다(`assets/skins/rbms-default/lua` 에서 `ranking`·`rival` 검색 결과 없음).
- 확장 id 현황: 이벤트 21000(SCRATCH AUTO)·21001(KEEP GREEN NUMBER) 사용 → 21002~21099 비어 있음(98개). 불린은 21100~21104, 21110~21114, 21120~21124, 21130~21134, 21140~21148 사용(29개) → 비어 있는 것은 21105~21109, 21115~21119, 21125~21129, 21135~21139, 21149~21199(71개). 근거 `skin_host/extension.rs:33-85`, `acknowledge/2026-10-22-adr-rbms-extension-ids.md`. 2026-10-23 설계서 둘이 새 id 를 쓰지 않는다고 적었다(`plan/2026-10-23-select-tree-download.md:232`, `client-small.md:270`).

### 2.5 코드를 읽고 찾은 틈(실행해 보지는 않았다)

1. **순위 캐시가 낡는다.** 만료가 없고 내 판을 제출해도 비우지 않는다(`app_ranking.rs` 에서 `ranking_cache.remove` 는 실패하거나 버려진 요청 정리에만 쓰인다). 곡을 치고 선곡으로 돌아와 패널을 열면 친 판 이전의 순위가 그대로 보인다. IR BEST 목표와 고스트도 같은 캐시를 읽는다.
2. **일시 실패가 세션 내내 남는다.** 실패(`Failed`)도 "끝난 답"으로 캐시돼(`has_settled_answer`, `ir_ranking.rs:123-129`) 오프라인에서 한 번 실패한 곡은 다시 받지 않는다.
3. **패널에 건반이 닿지 않는다.** 컨트롤러로는 단축키를 바인딩해 열 수는 있으나(`takes_a_pad` 참, `keyconfig.rs:946-949`) 안에서 움직일 길이 없다. 휠도 안 먹는다.
4. 리플레이 연결이 약하다. `chart_replays(limit=10)` 의 플레이어별 첫 항목을 붙이는데(`ir_ranking.rs:196-211`), 순위 응답의 `extra.replay_id`(그 최고 기록에 연결된 리플레이)를 읽지 않는다. 10개 밖의 사람 리플레이는 못 받고, 최고 기록이 아닌 판의 리플레이가 붙을 수 있다.
5. 순위 줄은 `total_notes`·`played_at` 을 갖고 있으나 패널은 그리지 않는다. 줄에 사용자 id 가 없어(이름이 비면 id 로 대체) 내 줄·라이벌 줄을 표지할 수 없다(`ir_ranking.rs:28-44,178-194`).
6. 라이브러리에 없는 곡·코스·폴더 막대에는 순위가 없다(`focused_md5` 는 `SelectItem::Song` 만, `app_input.rs:597-602`). 코스 순위(`course_ranking`)는 서버·클라이언트에 있으나 선곡 패널은 쓰지 않는다.

## 3. 원본(beatoraja)·IIDX·LR2 기준

### 3.1 beatoraja 선곡의 IR 순위

- **받는 시점**: 막대가 바뀌면(`selectedBarMoved`) `currentRankingDuration = rankingDuration(5000 ms) + 캐시가 있으면 max(10분 - 마지막 갱신 후 경과, 0)` 를 정한다. 그 시간이 지나면(`render`) 그 곡의 순위를 요청한다. 곡 막대(파일이 있는 것)와 코스 막대만 대상이고 IR 설정이 없으면 하지 않는다(`select/MusicSelector.java:76-79,223-246,618-633`).
- **캐시**: 곡 sha256(LN 정의가 없는 곡은 LN 방식 칸 공유)과 코스 해시별(`ir/RankingDataCache.java`). 갱신 간격 10분은 위 대기 시간에 들어 있다.
- **데이터**: `getPlayData(null, chart)` 로 **전체** 점수를 받아 EX 내림차순으로 정렬하고 같은 EX 는 같은 순위(`ir/RankingData.java` `updateScore`). 내 줄(PLAYER_YOU)·라이벌 줄(PLAYER_RIVAL)을 표지하고 램프 11칸 수·총원을 센다. 상태 NONE·ACCESS·FINISH·FAIL 이 스킨 타이머 172~174 를 켠다(`MusicSelector.java:248-250`).
- **스킨 속성**: 수 `ranking_exscore`(380+n)·`ranking_index`(390+n), 글 `ranking_name`(120+n), 이미지 인덱스 `playertype_ranking`(380+n)·`cleartype_ranking`(390+n), 슬라이더 비율 8, 불린 `ir_no_player`(603)·`ir_failed`(604)·`ir_busy`(608)·`ir_waiting`(606)(`skin/property/IntegerPropertyFactory.java:876-907,985-1003,1400-1430`, `StringPropertyFactory.java:84,350-362`, `BooleanPropertyFactory.java:665-689`). 슬롯 n 은 `rankingOffset + n` 번째 점수다.
- **화면 전환·스크롤**: 화면 전환은 없다(선곡 스킨 안의 패널). 선곡에서 `rankingOffset` 을 움직이는 것은 스킨의 슬라이더 쓰기(`setRankingPosition`)뿐이고 건반은 없다. 결과 화면만 마우스 휠로 움직인다(`result/AbstractResult.java:248-270`).
- **건반**: `MusicSelectKeyProperty.BEAT_7K` 에서 2·4번은 FOLDER_CLOSE, 6번은 NEXT_REPLAY(`select/MusicSelectKeyProperty.java:10-18`). 순위를 여는 건반은 없다. 키 명령 `OPEN_IR`(이벤트 210 `open_ir`)은 순위 목록이 아니라 IR 사이트의 그 곡 페이지를 브라우저로 연다(`skin/property/EventFactory.java:371-396`). R-BMS 선곡은 이 이벤트를 처리하지 않는다(`stage/select/events.rs` 에 없음).
- 순위 계산의 차이: 원본은 EX 만, R-BMS 서버는 램프→등급→EX→BP(2.3절).

### 3.2 IIDX(참고 화면과 공식 문서에서 확인한 것)

- 참고 화면: 0절. 점수순 6줄 + 매장 1위 한 줄. 줄에는 DJ LEVEL 글자와 램프 이름이 있고 단위·날짜는 없다.
- 건반: INFINITAS 공식 조작 설명은 선곡에서 검은 건반=폴더 닫기와 정렬 변경, 흰 건반=결정이라고 적었다(https://p.eagate.573.jp/game/infinitas/2/howto/play.html). 이는 위 beatoraja 표와 R-BMS 의 현재 동작(폴더 안은 닫기, 최상위는 정렬)과 같다.
- **IIDX 에 순위를 여는 건반이 있는지는 확인 못 함.** 공식 INFINITAS 설명·Lightning Model 안내(https://p.eagate.573.jp/game/2dx/31/howto/lightning_model/premium.html)에는 그런 조작이 없다. 2011년 Yahoo 지식인은 라이벌 승패 표시를 모바일 사이트에서 켜는 설정만 말한다(https://detail.chiebukuro.yahoo.co.jp/qa/question_detail/q1256976462). 요구의 "2·4·6 누르면 랭킹"은 IIDX 의 알려진 조작을 옮긴 것이 아니라 사용자의 새 조작으로 다룬다.

### 3.3 LR2

LR2 가 선곡에서 건반으로 순위 화면을 여는지는 **확인 못 함.** 검색 요약이 "ECHOES wiki 에서 IR 에 연결하고 점수를 보낸 적 있는 곡을 골라 4번 건반을 누르면 순위가 표시된다"고 인용했으나 원문은 403 으로 읽지 못했고 LR2 자체의 설명도 아니었다(https://w.atwiki.jp/echoes140/pages/220.html). LR2 입문 글 하나에는 선곡 건반 설명 자체가 없었고(https://lil.techblog.jp/archives/1638491.html), 다른 하나(https://w.atwiki.jp/bmssetuphelp/pages/18.html)는 403 으로 읽지 못했다. beatoraja 의 LR2 스킨 로더에는 순위 화면 처리가 없다(`skin/lr2/*.java` 에서 ranking 검색 결과 없음). 설계의 근거로 쓰지 않는다.

## 4. 설계안

### 4.1 추천안 A: 선곡 위 "랭킹 모드" 오버레이(기존 IR RANKING 패널 확장, 화면 전환 없음)

**들어가기·나가기(키 충돌 해결)**

| 상황 | 동작 |
| --- | --- |
| 차트 막대(`SelectItem::Song`) + 서버 설정됨 + 패널·모달·검색 입력 없음에서 파란 건반(2·4·6)을 `SCROLL_DURATION_LOW_MS`(300 ms, `songlist/bars.rs:106`, 새 상수를 만들지 않는다) 이상 누르고 있음 | 랭킹 모드가 열린다(누르고 있는 중에 열림) |
| 같은 건반을 그보다 짧게 눌렀다 뗌 | 지금 동작(폴더 닫기, 최상위는 정렬)을 **뗄 때** 한다. 결과 화면 파란 건반의 "뗄 때" 선례와 같다. 차트 막대가 아니거나 서버가 없으면 지금처럼 누를 때 즉시 한다 |
| 6번 | 목록 역할이 없는 컨트롤러에서는 짧게 눌러도 연다. 키보드 `F` 는 즐겨찾기 단축키와 겹쳐 키보드의 6번은 쓰지 않는다(질문 3) |
| START·SELECT 로 옵션 패널이 올라와 있는 동안 | 건반은 패널의 것이다. 랭킹 모드는 열리지 않는다 |
| 열려 있는 동안 닫기 | 파란 건반(짧게), Esc, `I`(단축키를 옮겼으면 그 키), 왼쪽 화살표 |

**모드 안 조작(건반이 곧 목록 조작이 되게)**

| 입력 | 동작 |
| --- | --- |
| 턴테이블·위아래 화살표·마우스 휠 | 줄 이동(눌러 반복은 기존 `SCRATCH_REPEAT_*` 재사용) |
| 흰 건반(1·3·5·7)·Enter | 선택한 줄의 리플레이를 받아 재생(기존 `play_ranking_replay` 경로, 리플레이가 없으면 `no replay for that row`). 요구 2 와 같은 경로 |
| SELECT | 내 줄(YOU)로 점프(내 순위가 현재 쪽 밖이면 그 쪽을 받는다) |
| START | 다음 쪽을 더 받는다 |
| Q·E | IR 프로필 전환(기존) |
| 마우스 | 줄 클릭(기존), 패널 밖 클릭은 스킨 몫(기존 `Hot::OverlayPanel`) |

곡을 바꾸려면 모드를 닫는다(기존 동작: 패널이 열리면 목록은 "덮인" 상태). IIDX 처럼 곡을 넘기는 동안 순위가 따라오는 상시 패널은 요구 5 의 몫이고, 같은 데이터·그리기 코드를 쓴다.

**표시(IIDX 줄 모양)**: 머리에 곡 이름과 `YOU #r`, 줄은 `순위 | 이름 + 단위 배지 | DJ LEVEL 글자 | EX | 램프 | 날짜(UTC 저장·표시는 기존 `fmt_datetime`) | REP`. DJ LEVEL 은 `rbms_render::result::dj_rank(ex, 2 * total_notes)`(`result.rs:133`, 색은 같은 파일의 `RANK_BANDS`)로 계산한다. 내 줄과 라이벌 줄은 강조·표지. 사각형은 기존과 같고 17줄이다. 외부 스킨 위에서도 같은 자리라 스킨의 목록이 옆에 남는다.

**데이터와 API**

1. 곡마다 요청: ① `chart_ranking_page(limit=100, page=1)`(최대 500, 응답 약 100줄 x 판정 내역 포함. 응답 크기는 실측하지 않았다) ② `player_best(나)` ③ 라이벌(기존 ≤8, 또는 서버의 `rival_of` 한 번, 질문 아님: 라이벌 동기화 여부가 확정돼야 하므로 기존 방식 유지) ④ `chart_replays` 는 없앤다(`extra.replay_id` 로 대체). 내 순위가 100 밖이면 그 `page` 를 한 번 더, "더 받기"는 `page+1`.
2. 캐시: 10분 만료(원본 `rankingReloadDuration`), 내 판 제출이 받아들여지면 그 곡만 무효화, `Failed` 는 짧게(예 30초) 뒤 다시 받기, 모드를 열 때마다 만료된 것은 다시 받기. 소비자(IR BEST 목표, 고스트, 스킨 id)는 `you`·`rivals`·`rows` 의 의미를 바꾸지 않는다.
3. 서버(`web/`): 순위 응답 줄의 `extra.dans`(`[{system,name,label}]`, 프로필과 같은 모양)를 더한다. 마이그레이션 없음, 추가 필드뿐이라 옛 클라이언트·웹 화면은 영향이 없다. 선택: `X-Total-Count` 헤더(총원, 본문은 그대로 배열). 순위 정렬·`rank` 의미는 바꾸지 않는다.
4. 설정: 선택적 새 필드 1개(질문 7). 있으면 `#[serde(default)]`, `SETTING_COUNT` 97→98, 설정 계약 골든 갱신(`RBMS_UPDATE_CONTRACT=1`).
5. DB: 클라이언트 scoredb 변경 없음, 웹 마이그레이션 없음.

**외부 Lua 스킨**

- 새 확장 id 없이 기존 id 에 실제 값을 답한다: 380~389(EX)·390~399(순위 숫자·램프 이미지 인덱스), 120~129(이름), 이미지 인덱스 380~389(내 줄 1·라이벌 2), 비율 8(위치, 읽기·쓰기), 순위 총원·램프 분포를 받은 줄 수 대신 실제 값으로. ModernChic 의 랭킹 탭이 그대로 그려진다.
- 스킨이 보기 전에 받아 둬야 하므로 앱 패널이 닫혀 있어도 곡에 5초(원본 `rankingDuration`) 머물면 자동으로 받는다(질문 6).
- 기본 스킨: 앱 오버레이가 그린다. 마우스용 RANKING 단추(푸터의 AUTOPLAY·PRACTICE·FAVORITE 옆)를 만들려면 확장 id 가 둘 필요하다: 이벤트 `21002`(랭킹 모드 열기·닫기), 불린 `21105`(랭킹 모드가 열려 있음). 남는 id 는 이벤트 21003~21099, 불린 21106~21109·21115~21119·21125~21129·21135~21139·21149~21199. 병렬 조사(요구 1·2·4·5)가 id 를 쓰면 메인이 한 곳에서 배정해야 한다(PROCESS.md 의 ADR 규칙).
- 기본 스킨 Lua 가 직접 목록을 그리게 하려면 단위 배지·날짜용 글 id 가 새로 필요한데, 현재 확장 대역에는 이벤트와 불린뿐이다. 대역을 넓히는 일은 사용자 결정이라 이 안에는 넣지 않았다.

**바뀌는 파일(예상)**

- 웹: `web/src/server/service/domain/score/score.service.ts`, `web/src/server/compose/index.ts`(순위 쿼리가 `chartBest.userId` 를 돌려주게), `web/src/server/dto/score.ts`, `docs/backend/api-spec.md`, 테스트 `web/tests/service/…`, `tests/route/api-ext.test.ts`, `tests/db/…`
- 클라이언트 데이터: `apps/rbms-player/src/ir_ranking.rs`, `app_ranking.rs`(제출 뒤 무효화는 `poll_submit` 에 연결), 필요하면 `crates/rbms-ir/src/dto/score.rs` 에 `extra` 읽기 도우미
- 입력·모드: `apps/rbms-player/src/stage/select/mod.rs`, `skinned.rs`, `keys.rs`, 새 `stage/select/ranking_mode.rs`, `stage/result.rs` 의 `blue_keys` 를 선곡과 나눠 쓰도록 위치 이동
- 그리기: `apps/rbms-player/src/ir_ranking_view.rs`, `stage/select/overlay.rs`
- 스킨 호스트: `apps/rbms-player/src/skin_host/ir.rs`, `skin_host/select/bridge.rs`, `skin_host/writers.rs`, `skin_host/extension.rs`, `assets/skins/rbms-default/lua/select/*`(단추)
- 문서: `docs/play-options.md`(12절 옆에 새 절), `docs/skin.md` E8·6절, `docs/reference/ir-api.md`, `docs/acknowledge/`(ADR), `docs/quality-assurance/`(실기 항목)

**테스트 방법**

- 건반 시간 논리: `SelectKeys`(`keys.rs`) 에 가짜 시간을 주는 단위 테스트(짧게 떼면 폴더 닫기·정렬이 뗄 때 한 번, 길게 누르면 열림, 패널이 올라온 동안은 열리지 않음, 9키·5키·DP 의 인덱스). 기존 `stage/select/skinned_tests.rs`·`panel_tests.rs` 의 `Browser` 도구를 쓴다.
- 데이터: `ir_ranking.rs` 의 `RankingDouble` 로 쪽 받기·내 순위 밖 쪽·만료·실패 재시도·제출 뒤 무효화.
- 그리기: `CpuCanvas` 머리 없는 그리기와 `stage/capture`(`OVERLAY_RANKING`)에 줄·배지·내 줄 강조 캡처 추가.
- 웹: `score.service` 에 `extra.dans` 단위 테스트와 DB 통합(`RBMS_DB_E2E=1 bun test tests/db`).
- 스킨 호스트: `skin_host/ir/tests.rs`·`select/tests.rs` 에 380~399·120~129·비율 8 값 테스트, `skin-dump` 로 ModernChic 랭킹 탭이 값을 읽는지.
- 실제 창: 컨트롤러 레인으로 열고 닫고 줄 이동·리플레이 받기, 로그아웃·오프라인·서버 없음 세 상태, 곡을 치고 돌아와 내 순위가 갱신되는지(QA 문서에 항목으로 추가).

### 4.2 대안

| 대안 | 내용 | 비추천 이유 |
| --- | --- | --- |
| B 별도 순위 화면 | `Stage` 에 열네 번째 화면을 더하고 선곡을 일시 정지 | 화면 13개의 상태기계·스킨 문서·캡처가 늘고, 곡 사이를 오가는 맥락이 끊긴다. 오버레이 확장으로 같은 일이 된다 |
| C 웹 열기 | 이벤트 210(`open_ir`)처럼 그 곡의 웹 페이지(`/charts/{hash}`, 이미 전체 순위가 있다)를 브라우저로 연다. `open_url.rs`·`rbms_download::browsable_url` 재사용 | 요구의 "2·4·6 로 본다"를 못 채운다. 보조 단추로는 값이 있다 |
| D 상시 컴팩트 패널 | IIDX 의 `RIVAL SCORE DATA` 처럼 곡을 넘기는 동안 나·라이벌·1위만 6줄로 늘 그린다 | 요구 5 의 범위. 같은 `RankingBoard`·줄 그리기를 쓰므로 A 와 함께 설계한다 |
| 키 대안(질문 1 의 B·C·D) | 맥락으로 가르기(폴더 막대에서만 폴더 닫기), 6번만 쓰기, 단축키만 쓰기 | 질문 1 참조 |

## 5. 사용자 결정이 필요한 질문

1. **트리거**: A(추천) 파란 건반을 300 ms 이상 누르면 열림, 짧은 누름은 지금 동작을 뗄 때 수행 / B 짧게 누르면 열림, 파란 건반의 폴더 닫기·최상위 정렬을 없앰(컨트롤러만으로 폴더를 나올 길이 사라지므로 대체 건반을 정해야 함) / C 6번 건반만 / D 단축키(`I`)와 컨트롤러 버튼 바인딩만. 추천 이유: 2·4번이 이미 폴더 닫기·정렬이라 컨트롤러만으로 폴더를 나오는 길을 지킨다.
2. **9키(pop'n)**: A(추천) 버튼 2·8(목록 역할이 없는 칸) / B 파란 건반 4·6 길게(이 둘이 목록 이동 키라 누르고 있으면 스크롤과 겹침) / C 9키는 지원 안 함. 추천 이유: 충돌이 없는 칸이다. 5키·10키는 2·4, DP 는 양쪽 2·4·6 로 정했다.
3. **키보드 6번(F)**: A(추천) 키보드는 2·4(S·D)만 쓰고 6번은 컨트롤러만 / B 즐겨찾기 기본 키를 옮김(출하 기본값 변경) / C F 길게=랭킹, 짧게=즐겨찾기(즐겨찾기가 누를 때 즉시 돌아 길게 누르면 별이 한 번 바뀜). 추천 이유: 출하 단축키와 겹치지 않는다.
4. **표시 방식**: A(추천) 선곡 위 오버레이 확장 / B 별도 순위 화면. 추천 이유: 화면 전환이 없고 요구 5 와 코드를 나눈다.
5. **"2·4·6" 의 뜻**: A(추천) 건반(검은 건반) / B 숫자 줄 키(`2` 정렬, `4` 리플레이 슬롯, `6` 키 설정 단축키와 겹침). 추천 이유: 조사 지시와 화면의 "검은 건반"이 건반이다.
6. **순위 받는 시점**: A(추천) 곡에 5초 머물면 자동(원본과 같음, 외부 스킨의 랭킹 탭을 위해) + 모드를 열면 즉시 / B 모드를 열었을 때만. 추천 이유: B 면 ModernChic 식 스킨의 랭킹 탭이 비어 있다.
7. **안전장치와 부가 결정 묶음**: (a) 설정에 `RANKING KEYS`(켬 기본, 끄면 파란 건반은 지금 그대로) 행을 더할지: A(추천) 더함(설정 필드 1개, `SETTING_COUNT` 98) / B 안 더함. (b) 순위 줄에 단위 배지(서버 응답에 `extra.dans` 추가, 마이그레이션 없음): A(추천) 추가 / B 배지 생략 / C 줄마다 프로필 요청(요청이 줄 수만큼 늘어남). (c) 순위·캐시 틈(2.5절 1·2)을 이 작업에서 고칠지: A(추천) 포함 / B 별건. 추천 이유: 각각 요청한 사용을 안전하게 하고, 단위는 이미 있는 배치 조회로 비용이 작으며, 모드가 "내 순위"를 보이므로 낡은 값이면 의미가 없다.
8. **순서 표시**: A(추천) 서버 순위 그대로(램프→등급→EX→BP, 기존 설계) / B 모드에서만 EX 로 다시 정렬(참고 화면·원본과 같은 순서이나 서버 `rank` 와 번호가 달라짐). 추천 이유: 정렬 의미를 바꾸는 일은 확인이 필요한 영역이라 기존을 유지한다.

## 6. 위험과 확인 못 한 것

- IIDX 와 LR2 에 순위를 여는 건반이 있는지 확인 못 함(3.2·3.3절). 요구는 사용자의 새 조작으로 다뤘다.
- 길게 누름(300 ms)과 "뗄 때" 동작은 실제 손맛을 측정하지 않았다. 짧은 누름의 폴더 닫기·정렬이 뗄 때로 밀려 느려지는 것이 대가다(차트 막대 + 서버 설정됨일 때만). 결과 화면 선례가 있으나 선곡의 목록 이동과 겹쳐 쓰는 손에는 다르게 느껴질 수 있다.
- 키보드 6번(F)과 9키의 충돌은 위 질문의 선택에 달렸다. 패널(START·SELECT)이 올라온 사이에 시작된 누름이 패널을 닫은 뒤 랭킹 모드로 번지지 않게 하는 처리가 필요하다(`SelectKeys::settled` 와 같은 "눌러 둔 것은 누름이 아님" 규칙).
- 2.5절의 캐시 틈과 `RankingBoard` 소비자(IR BEST 목표, 고스트, 결과 화면 총원)는 코드를 읽고 판단했으며 실행으로 재현하지 않았다. 순위 줄 수를 10에서 100으로 늘리면 결과 화면의 `ranking_total`(`stage/result.rs:932-941`)과 스킨의 총원·램프 분포가 더 정확해지지만 수치가 바뀐다(100 을 넘는 곡은 여전히 모자란 값).
- 응답 크기: 줄마다 판정 내역이 붙어 있어 100줄이 수십 KB 일 것으로 추정하나 실측하지 않았다(운영 사이트에 요청하지 않았다). 순위 읽기에는 속도 제한이 없다(2.3절). 클라이언트 쪽 한 번에 하나, 5초 시간 제한이 요청 수의 상한이다.
- `ScoreRecord.total_notes` 가 0 이거나 LN 계산이 다르면 DJ LEVEL 글자가 어긋날 수 있다(옛 행). 0 이면 글자를 비운다.
- 서버 `extra.dans` 는 순위 응답 캐시(`cacheLife('minutes')`)에 같이 들어가므로 단위를 바꿔도 최대 그 시간만큼 이전 값이 보인다.
- 외부 스킨의 랭킹 탭을 실제 ModernChic 으로 그려 본 것은 없다(`RBMS_SKIN_PACK` 확인은 사용자 몫, 기존 skin.md 6절과 같은 한계).
- 라이브러리에 없는 곡(표에만 있는 막대)·코스·폴더에는 이 모드가 열리지 않는다. 없는 곡은 표가 md5 를 알고 있어 서버 순위를 볼 수는 있으나 범위 밖으로 두었다.
- 아직 확인하지 않은 것: 라이벌 목록을 서버(`rival_of`)와 동기화하는 사용자 비율, 컨트롤러 실기.

## 7. 구현 단위 분해(worktree 병렬)

공유 형식을 먼저 굳히면 U3~U5 를 병렬로 돌릴 수 있다. `RankingRow`(필드 추가: `player_id`, `dan`, `total_notes`, `is_you`, `is_rival`, `scratch_auto`)·`PanelLine`(날짜·DJ LEVEL·단위·강조)의 형식은 U2 가 먼저 병합한다(U0 로 따로 떼도 되는 규모: 약 60줄).

| 단위 | 소유 파일 | 내용 | 의존 | 예상 규모 |
| --- | --- | --- | --- | --- |
| U1 서버 | `web/src/server/service/domain/score/score.service.ts`, `compose/index.ts`, `dto/score.ts`, `docs/backend/api-spec.md`, `web/tests/**` | 순위 응답 `extra.dans`(+선택 `X-Total-Count`), 테스트 | 없음 | 변경 약 80~120줄 + 테스트 약 150줄 |
| U2 클라이언트 데이터 | `apps/rbms-player/src/ir_ranking.rs`, `app_ranking.rs`, `crates/rbms-ir/src/dto/score.rs`(필요 시) | 쪽 받기·`extra.replay_id`·만료·제출 뒤 무효화·실패 재시도·줄 형식 확장 | 없음(U1 과 형식만 약속: `extra.dans` 모양) | 약 250~400줄 + 테스트 약 200줄 |
| U3 입력·모드 | `stage/select/mod.rs`, `skinned.rs`, `keys.rs`, 새 `ranking_mode.rs`, `stage/result.rs`(`blue_keys` 이동), 설정 행을 택하면 `crates/rbms-config/src/{schema,settings}.rs`·`stage/settings/catalog.rs`·계약 골든 | 길게 누름·뗄 때 동작, 모드 안 건반·휠, 9키·5키·DP, 설정 행 | U2 의 줄 형식(스텁으로 먼저 개발 가능) | 약 300~450줄 + 테스트 약 300줄 |
| U4 그리기 | `ir_ranking_view.rs`, `stage/select/overlay.rs`, `stage/capture/*`(캡처 추가) | 열·단위 배지·DJ LEVEL·날짜·내 줄 강조·쪽 안내 | U2 의 `PanelLine` 형식. 요구 5 의 컴팩트 패널과 같은 파일 | 약 250~350줄 + 캡처 |
| U5 스킨 호스트 | `skin_host/ir.rs`, `skin_host/select/bridge.rs`, `skin_host/writers.rs`, `skin_host/extension.rs`, `assets/skins/rbms-default/lua/select/*`, `docs/skin.md` | 380~399·120~129·이미지 인덱스·비율 8 의 실제 값, 자동 받기, 확장 id 21002·21105 와 기본 스킨 단추 | U2 | 약 200~300줄 + 테스트 약 150줄 |
| U6 문서·QA | `docs/play-options.md`, `docs/reference/ir-api.md`, `docs/acknowledge/**`, `docs/quality-assurance/**`, `docs/PROCESS.md` | ADR(확장 id 2개 포함), QA 항목, 갱신 | U1~U5 의 결과 | 문서 약 150줄 |

순서: U1 ∥ U2 → U3 ∥ U4 ∥ U5 → U6 → 메인이 병합·게이트·커밋. 병렬 불가 충돌: `stage/select/mod.rs`(U3 소유, U4 는 `ranking_lines` 호출 시그니처만 소비), `ir_ranking.rs`(U2 소유), `overlay.rs`·`ir_ranking_view.rs`(U4 소유). 다른 조사(요구 2 리플레이 받기, 요구 5 RIVAL 패널)가 같은 파일을 만지므로 메인이 단위 소유권을 하나로 정해 줘야 한다. 최소 검증은 단위마다 좁은 테스트 하나(예: `cargo test -p rbms-player --lib --locked <단위 필터>`, 웹은 해당 `bun test` 파일)이고, 메인이 통합 뒤 게이트를 한 번 돈다.
