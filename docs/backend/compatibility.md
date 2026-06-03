# rbms 백엔드 — 호환성 매핑 & 출처

> LR2IR(원조) · beatoraja IR(현대) ↔ rbms 슈퍼셋 필드 대응. 모든 매핑은 **원본 소스/실프로토콜 대조**(아래 출처).

## 1. 차트 키
| | 키 | 길이 |
|---|---|---|
| LR2IR | `bmsmd5` (MD5) | 32 hex |
| beatoraja IR | `sha256` | 64 hex |
| **rbms** | **md5 + sha256 동시 보유** | `{hash}` 경로는 길이로 판별 |
- LR2 코스/차트 구분: `len(hash)==32 → bms, else course`(lr2irscraper `bmsmd5.py`). 코스 해시 = 구성 차트 해시 결합.
- rbms `course_hash`: 코스 차트 **sha256를 순서대로 concat → SHA-256**(beatoraja 관행 준용). LR2 어댑터는 md5 결합 변형도 허용.

## 2. 클리어 램프
| id | beatoraja `ClearType` | LR2IR `clear`(1-5) | rbms `ClearLamp` |
|---|---|---|---|
| 0 | NoPlay | (없음/0) | NoPlay |
| 1 | Failed | 1 FAILED | Failed |
| 2 | AssistEasy | (→2 EASY) | AssistEasy |
| 3 | LightAssistEasy | (→2 EASY) | LightAssistEasy |
| 4 | Easy | 2 EASY | Easy |
| 5 | Normal | 3 CLEAR | Normal |
| 6 | Hard | 4 HARD | Hard |
| 7 | ExHard | (→4 HARD) | ExHard |
| 8 | FullCombo | 5 FULLCOMBO | FullCombo |
| 9 | Perfect | (→5) | Perfect |
| 10 | Max | (→5) | Max |
- **LR2 다운매핑**(슈퍼셋→1-5): `{NoPlay→0, Failed→1, Assist/LightAssist/Easy→2, Normal→3, Hard/ExHard→4, FC/Perfect/Max→5}`.
- 출처: beatoraja `ClearType.java`(id 0-10), lr2irscraper `ranking.py`("clear は 1-5 … FAILED, EASY, CLEAR, HARD, FULLCOMBO").

## 3. 판정 / 스코어
| 항목 | LR2IR XML | beatoraja IRScoreData | rbms |
|---|---|---|---|
| PGREAT | `pg` (합) | `epg`+`lpg` | epg/lpg(분리) |
| GREAT | `gr` (합) | `egr`+`lgr` | egr/lgr |
| GOOD | — | `egd`+`lgd` | egd/lgd |
| BAD | — | `ebd`+`lbd` | ebd/lbd |
| POOR | — | `epr`+`lpr` | epr/lpr |
| MISS | — | `ems`+`lms` | ems/lms |
| 空POOR | — | — | `empty_poor`(rbms 고유) |
| EX | `pg*2+gr` | `(epg+lpg)*2+egr+lgr` | 동일(유도) |
| BP/minbp | `minbp` | `minbp`(BAD+POOR+MISS 최소) | `minbp` |
| notes/combo | `notes`,`combo` | `notes`,`maxcombo`,`passnotes` | 동일 |
| avg판정 | — | `avgjudge`(µs) | `avgjudge` |
| FAST/SLOW | — | early/late 합으로 유도 | fast/slow(유도·미러) |
- 출처: beatoraja `IRScoreData.java`(early/late 12필드 + `getExscore()=(epg+lpg)*2+egr+lgr`), lr2irscraper `ranking.py`(XML 컬럼 `name,id,clear,notes,combo,pg,gr,minbp`).

## 4. 플레이 옵션
| 항목 | beatoraja | rbms |
|---|---|---|
| 노트옵션 비트필드 | `option`(int) | `option` 보존 + `random`/`random_p2` 문자열 병행 |
| 시드 | `seed`(long) | `seed` |
| 게이지(플레이) | `gauge`(int) | `gauge`(GaugeType) |
| 어시스트 | `assist`(int) | `assist` |
| 입력기기 | `deviceType` | `input_device` |
| 판정알고리즘 | `judgeAlgorithm` | `judge_algorithm`(Combo/Duration/Lowest/Score) |
| 룰 | `rule`(BMSPlayerRule) | `rule` |
| 스킨 | `skin` | `skin` |
| LN모드 | `lntype`(0:LN,1:CN,2:HCN) | `lntype` |
| **rbms 확장** | — | `judge_rate`·`offset_ms`·`constant`(하이스피드 고정) |
- 출처: beatoraja `IRScoreData.java`·`JudgeAlgorithm.java`(Combo/Duration/Lowest/Score).

## 5. 인증
| | 방식 |
|---|---|
| LR2IR | per-user 제출(클라가 user 세션 보유, `gateway.cgi` 난독) |
| beatoraja | `IRAccount{id,password,name}` → `register`/`login` → `IRPlayerData{id,name,rank}` |
| **rbms** | register/login(IRAccount 슈퍼셋, email 추가) → **Bearer 토큰**, guest 허용 |
- 출처: beatoraja `IRConnection.java`(register/login), `IRAccount.java`, `IRPlayerData.java`.

## 6. beatoraja IRConnection ↔ rbms 엔드포인트
| beatoraja 메서드 | rbms 엔드포인트 |
|---|---|
| `register(IRAccount)` | `POST /api/auth/register` |
| `login(IRAccount)` | `POST /api/auth/login` |
| `getRivals()` | `GET /api/players/{me}/rivals` |
| `getTableDatas()` | `GET /api/tables` |
| `getPlayData(player, chart)` | `GET /api/charts/{hash}/ranking`(player=null) · `/best?player=`(본인) · `GET /api/players/{id}/scores`(chart=null) |
| `getCoursePlayData(player, course)` | `GET /api/courses/{hash}/ranking` · `/best?player=` |
| `sendPlayData(chart, score)` | `POST /api/scores` |
| `sendCoursePlayData(course, score)` | `POST /api/courses/{hash}/scores` |
| `getSongURL(chart)` | `ChartMeta.url` / `extra.url` |
| `getCourseURL(course)` | `CourseMeta.extra.url` |
| `getPlayerURL(player)` | `PlayerProfile.extra.url` |
- 출처: beatoraja `IRConnection.java`(인터페이스 전체).

## 7. LR2IR 레거시 엔드포인트(어댑터 대상)
- 랭킹: `GET http://www.dream-pro.info/~lavalse/LR2IR/2/getrankingxml.cgi?id={playerid}&songmd5={md5}` → XML(`#` 접두·`<ranking>`+`<lastupdate>` 2 루트, well-formed 아님). `<score>{name,id,clear,notes,combo,pg,gr,minbp}`.
- 검색/정보: `GET .../search.cgi?mode=ranking&bmsmd5={md5}&page={n}`(HTML), `mode=editlogList&bmsid=`, `mode=downloadcourse&courseid=`.
- 제출: `gateway.cgi`(클라 내장, 난독 — **재현 비범위**).
- 출처: lr2irscraper `ranking.py`·`bms_info.py`(URL 템플릿·XML 파싱).

## 8. 시각 단위
- beatoraja `IRScoreData.date` = **unix초**. rbms/LR2 일부 = ms. → rbms 백엔드 정본은 **epoch ms**, beatoraja 호환 입출력 시 ×1000/÷1000 변환.

## 9. 클라이언트 갭 (rbms-ir 현행 → 슈퍼셋)
현행 `crates/rbms-ir/src/dto.rs`는 합계 judge(fast/slow)만 보유. 백엔드 슈퍼셋 수용을 위해 후속 확장 필요(→ [endpoint-tasks.md] §13):
- `JudgeBreakdown`: + `epg…lms`(early/late) · `avgjudge` · `empty_poor`.
- `ScoreSubmission`: + `lntype·seed·judge_algorithm·rule·skin·option` + **`client_build_sha256`·`client_platform`**(무결성).
- `PlayOptions`: + 전체옵션(`option·judge_rate·offset_ms·constant·hispeed·lift·lane_cover·total_override·autoplay·auto_offset·scratch_left·random_p2·green_number`).
- `ReplayData`: events를 **µs 구조**(`{t_us,lane,press}[]`)로(현행 `events:Vec<u8>` → µs 보존 직렬화).
- 신규: settings(get/put)·replay download·register/login/token·course_* 메서드.

## 9.5 rbms 전용(LR2IR·beatoraja에 없음)
| 기능 | LR2IR | beatoraja | rbms |
|---|---|---|---|
| 클라 빌드해시(변조탐지) | ✗ | ✗ | **`client_build_sha256` + allowlist** |
| ranked 적격 판정/flags | (서버측) | (서버측) | **명세화(autoplay·assist·judge폭·total·미상빌드)** |
| 전체 플레이옵션 보존 | 일부 | 다수 | **전수(공평 평가)** |
| µs 리플레이 | ✗ | (고스트 일부) | **µs 무손실(핵분석)** |
| 설정 동기화 | ✗ | ✗ | **named blob get/put** |
| 웹 FE 조회 | (HTML) | ✗ | **search/leaderboard/feed/뷰어 + envelope·CORS** |
> 서버는 누락 필드를 합계/유도로 수용(하위호환). 현행 rbms-ir 제출도 동작하되, `client_build_sha256` 없으면 `ranked=false`(UNKNOWN_BUILD) 또는 거부(env).

## 출처 (sources)
- beatoraja 원본: `/Users/hyunseokbyun/beatoraja/src/bms/player/beatoraja/ir/` — `IRConnection.java`·`IRScoreData.java`·`IRChartData.java`·`IRCourseData.java`·`IRTableData.java`·`IRPlayerData.java`·`IRAccount.java`·`IRResponse.java`; `ClearType.java`·`play/JudgeAlgorithm.java`.
- LR2IR 프로토콜: [naktazdim/lr2irscraper](https://github.com/naktazdim/lr2irscraper) (`ranking.py`·`bms_info.py`·`bmsmd5.py`), [new-lr2ir](https://sr.ht/~showy_fence/new-lr2ir/), [anqooqie/lr2ir-crawler](https://github.com/anqooqie/lr2ir-crawler), [BMS-Community/resources](https://bms-community.github.io/resources/).
- beatoraja IR 생태계: [exch-bms2/beatoraja Wiki — IR Tips](https://github.com/exch-bms2/beatoraja/wiki/IR-Tips), [Mocha-Repository](https://mocha-repository.info/), [GAFTALK beatoraja IR](https://www.gaftalk.com/blog/items/beatoraja-ir/).
- 기존 rbms 계약: `docs/reference/ir-api.md`, `crates/rbms-ir/`.
