# 현재 웹 AI 분석 기능 전수 조사와 격차 (훈련곡 생성 대시보드 조사 01)

> 최종 갱신 2026-11-04 · 대응 커밋 2026-11-04 AI 훈련곡 작업(커밋 예정). 본문은 조사 시점(HEAD `4bac699`)의 기록 그대로이고, 끝의 "구현 결과(2026-11-04)" 절이 무엇이 어디에 구현됐는지와 설계에서 달라진 점을 적는다. 본문의 줄 번호와 "없다" 는 구현 전 기준이므로 코드가 정본이다.

> 조사일 2026-11-04 · 브랜치 `dev` · HEAD `4bac699` · 작업 트리 깨끗함(조사 시작 시점). 이 문서는 조사 전용이며 코드·설정·Git 상태를 바꾸지 않았다.
> 경로는 `/Users/hyunseokbyun/development/R-BMS` 기준 상대 경로이고, `web/` 아래 파일은 `web/` 접두사를 붙였다. 줄 번호는 2026-11-04 HEAD 의 작업 트리 기준이다.
> `.env*`·비밀 파일·`scoredb.sqlite` 는 읽지 않았고 운영 사이트·운영 DB 에는 요청하지 않았다. 따라서 운영 데이터의 실제 양(트레이스 수, 패턴 수)은 "확인하지 못함"으로 적었다.

## 0. 조사 방법과 표기

- 먼저 읽은 저장 문서: `docs/reference/ai-features/{README,server,client,providers}.md`(2026-10-19~22 기준), `docs/web/architecture.md` 11~14절, `docs/HANDOFF.md` 5절·7절, `docs/plan/2026-10-19-web-ai-features.md` 제목 구조, `docs/acknowledge/2026-10-19-adr-web-ai-decisions.md`. 이 문서들은 2026-10-19~24 시점이므로 아래 사실은 모두 현재 코드로 다시 확인했다. 저장 문서만 근거인 항목은 "(저장 문서 근거)"로 표시했다.
- 코드 파일은 `web/src/server/service/domain/{analysis,trace,chart-pattern,ai}/*`, `web/src/server/{compose,dto,route}/ai*.ts`·`trace.ts`·`chart-pattern.ts`, `web/src/server/service/shared/ai-provider/*`, `web/src/server/db/schema.ts`, `web/src/entities/ai.*.ts`, `web/src/widgets/ai/*`, `web/src/app/(app)/ai/*` 를 직접 읽었다. 클라이언트(Rust)는 trace 형식·패턴 추출·옵션 셔플만 확인했다.
- 용어: "확인한 사실"은 코드 또는 문서에서 줄 번호로 확인한 것, "추정"은 코드 읽기에서 끌어낸 내 판단, "확인하지 못함"은 근거를 찾지 못한 것이다.
- 분석 대상 코드의 마지막 변경은 2026-10-20 이다(`git log -- analysis-constants.ts` 의 마지막 두 건 `57fdc08`, `ba86ffd`). 이후 AI 관련 변경은 `47a7f9c`(어시스트 등급 반영), `0405496`(출력 언어)뿐이며 분석 지표는 바뀌지 않았다.

## 1. 한 줄 결론

- 현재 분석은 "최근 100판(7키 SP 한정)의 판정 요약을 19개 패턴 태그와 EX 비율 잔차로 말해 주는 수준"이다. 서버는 노트별 판정(trace)과 곡별 노트 배치(chart_pattern)를 둘 다 갖고 있는데, 분석은 거기서 판당 요약(태그별 합계, 레인별, 1초 밀도 5단위 칸, 32구간)만 만들어 저장하고 그 요약만 쓴다.
- 사용자가 요구한 "못 치는 패턴, 못 버티는 밀도"에는 세 가지가 구조적으로 모자란다. (1) 밀도를 절대 노트/초의 슬라이딩 창이 아니라 "고정 1초 창, 5개 단위 칸, 25 이상 한 칸"으로만 본다. (2) 패턴 태그끼리 겹치고, 태그와 밀도·레인·손·속도를 교차한 지표가 없다. (3) 판 사이 집계는 매번 최근 100판 요약을 합쳐 계산하고, 사용자별 영속 프로필·추이·한계 추정이 없다.
- 원자료(trace)는 서버에 이미 있고 그 안에 새 지표에 필요한 노트별 정보가 전부 들어 있다. 다만 어떤 라우트도 trace 원본을 돌려주지 않고(테스트가 고정), 분석은 `analyzeTrace` 한 곳에서 요약으로만 나온다. 새 지표는 그 함수와 `PLAY_ANALYSIS_VERSION` 올림으로 넣어야 한다.
- "훈련곡 생성"에 재사용할 골격은 이미 있다: 결정론 digest, 구조화 출력 어댑터 3종, 사용량·동시성 한도, 폴더 서비스. 모자란 것은 (가) 생성할 곡의 파라미터로 쓸 수 있는 약점 지표, (나) 곡 파일을 만들어 사용자 클라이언트에 전달하는 길(현재 폴더는 md5 만 가리킨다)이다.

---

## 2. 데이터 원천

### 2.1 판별 trace (노트별 판정·타이밍·시각)

**형식** (확인한 사실)
- 정본은 Rust `crates/rbms-ir/src/dto/trace.rs`(`PlayTrace`)이고 서버 Zod 는 `web/src/server/dto/trace.ts:152-172` `traceUploadSchema` 이다. `format` 은 `"rbms-trace"`, `version` 은 1(`trace.ts:6,9`).
- 최상위 필드: `format`, `version`, `score_id`(필수, 36자 이하), `chart{md5, sha256}`, `mode`, `lntype`, `random`(옵션 이름), `auto_lanes[bool]`(SCRATCH AUTO 가 친 레인), `notes`, `empty_poors`, `gauge`, `gauge_bounds`, `failed_at_ms`, `play_time_ms`, `speed`, `tempo`.
- `notes`(병렬 열, 같은 길이, 시각 순 다음 레인 순, `trace.ts:108-110`): `time_us`(차트 시각 µs), `lane`(플레이어가 본 레인 = 옵션 적용 뒤, 0~63), `kind`(0 일반, 1 LN 머리, 2 일반 LN 끝(판정 안 됨), 3 CN·HCN 끝(판정됨), `trace.ts:34`), `state`(0 아직 판정 없음, 1 PGREAT … 5 POOR, `trace.ts:37`), `err_us`(부호 있는 타이밍 µs, 양수 = 빠름(FAST), state 0 이면 0).
- `empty_poors`: `time_us`, `lane`(빈 POOR 입력, `trace.ts:112`).
- `gauge`: `interval_ms`(클라이언트 로그에서 500 ms), `selected`(끝난 게이지 번호 0~8), `series[]`(게이지마다 f32 열, 최대 16개·열당 172,800 샘플, `trace.ts:28,31,114-122`). 9개 게이지 이름·순서는 `AI_GAUGE_NAMES`(`dto/ai-play.ts:6`): assist_easy, easy, normal, hard, ex_hard, hazard, class, ex_class, ex_hard_class. `gauge_bounds[{min,max,border}]`.
- `failed_at_ms`: 게이지가 비어 끝난 시각(없으면 null). 비어 끝난 판은 실패 시점 뒤를 0 으로 채운다(클라이언트 `GaugeLogRecorder::closed`, `play-metrics.ts:262-269` 주석).
- `speed{start, end, changes[]}`: 각각 `hispeed, constant, lane_cover, lift, hidden, offset_ms, auto_offset, green_ms, green`(`trace.ts:126-138`); `changes` 는 `{t_ms, field, value}` 최대 4,096개.
- `tempo[{t_us, bpm, stop_us, scroll}]` 최대 4,096개(`trace.ts:140-145`).
- 한도: 본문 2 MiB(`TRACE_MAX_BYTES`, `trace.ts:12`), 노트·빈POOR 열 각 200,000개(`TRACE_MAX_NOTES = CHART_PATTERN_MAX_NOTES`, `trace.ts:22`), 시각 상한 24시간(`dto/chart-pattern.ts:25`). 업로드 속도 30/분/사용자(`trace.ts:19`).

**누가 trace 를 갖는가** (확인한 사실 + 저장 문서 근거)
- 클라이언트는 손으로 친 판마다 trace 를 만들어 로컬에 보관한다(`apps/rbms-player/src/play_trace.rs:1-26`, `app_result.rs:448-460`). 실패한 판, Esc·종료로 나간 판도 포함한다. 오토플레이·리플레이 감상·연습 판은 저장·전송하지 않는다(ADR Q15, `docs/acknowledge/2026-10-19-adr-web-ai-decisions.md:31`).
- 코스의 스테이지는 trace 가 없다(`apps/rbms-player/src/trace_backfill.rs` 머리 문서).
- 서버가 점수를 받은 뒤 `POST /charts/{hash}/traces` 로 올린다. 조건: 서버 `capabilities.traces`, 설정 `AUTO UPLOAD TRACE`(기본 켬), 본문 2 MiB 이하(`docs/play-options.md:585,660`).
- 예전 판은 EVENT 시계 리플레이가 있는 판만 다시 판정해 소급 생성한다(FRAME 시계 리플레이는 제외, `trace_backfill.rs` 머리 문서).
- 서버는 trace 를 점수 행과 허용 오차 0 으로 맞춰 본다: 차트 sha256, 경로 해시, `mode`·`lntype`·`random`·SCRATCH AUTO 일치, 판정 5종 합계와 FAST/SLOW 분해(`epg…lbd`), 빈 POOR 수(`trace-check.ts:5-20,105-132`). 맞지 않으면 `mismatch` 로 거절(저장 안 함). 9키(POPN_9K)는 BAD 가 노트를 남기므로 bad·poor·ebd·lbd 비교를 건너뛴다(`trace-check.ts:33,128`).
- 점수 하나에 trace 하나(`play_trace_score_uniq`, `schema.ts:704`). 이미 있으면 본문과 무관하게 기존 id 를 돌려준다(`trace.service.ts:305-306`).
- 어떤 사용자가 몇 판의 trace 를 갖는지는 확인하지 못함(운영 수치 접근 금지). 운영 수치는 `GET /api/admin/traces/usage`(관리자 Bearer)가 `totals.users·traces·bytes` 를 돌려주지만 이번 조사에서는 호출하지 않았다.

**저장 위치·용량** (확인한 사실)
- 표 `play_trace`(`schema.ts:685-708`): `id`(`tr_…`), `score_id`(unique), `user_id`, `chart_sha256`, `chart_md5`, `played_at`, `format_version`, `note_count`, `byte_size`(압축 전), `stored_size`(압축 후), `data` mediumblob(gzip 한 정규화 JSON, `trace.service.ts:310-326`), `created_at`. 색인 `(user_id, played_at)`, `(chart_sha256, user_id)`.
- 한 판 크기: 6만 노트 trace 가 약 1.4 MB(저장 문서 근거, `docs/web/architecture.md` 13-1 "저장" 행). 노트 1개당 약 23 B(추정: 1.4 MB / 6만, 게이지 열 포함). 2 MiB 본문 상한은 약 95,000 노트(`trace.ts:21` 주석).
- 사용자당 보관 상한은 5,000판·512 MiB(압축 전 바이트 합)이지만 넘어도 거절하지 않고 `near`(90%)/`over` 표시만 한다(`trace.ts:46`, `trace.service.ts:150-164`).
- 읽기 경로 없음: trace 원본을 돌려주는 라우트는 없고(`docs/web/architecture.md` 13-1 "읽기 경로 없음", `tests/route/trace.test.ts` 가 고정), `play_trace` 를 읽는 compose 는 `compose/trace.ts` 하나뿐이다.

### 2.2 차트 패턴 (`chart_pattern`)

**형식** (확인한 사실)
- `rbms-cli chart-pattern` 이 쓰는 `<md5>.json`(`apps/rbms-cli/src/chart_pattern/document.rs`). 서버 Zod 는 `web/src/server/dto/chart-pattern.ts:121-140`.
- 필드: `format "rbms-chart-pattern"`, `version 1`, `parser_version`(현재 클라이언트 `PARSER_VERSION = 5`는 `docs/HANDOFF.md` 7절 '클라이언트 값'), `md5`, `sha256`, `mode`(리터럴 `BEAT_7K` 하나뿐), `declared_ln`(`ln|cn|hcn|null`), `has_undefined_ln`, `uses_control_flow`, `init_bpm`, `total`, `rank`, `play_level`(정수 아니면 null), `playable_notes`, `notes{time_us, lane, kind, section}`(병렬 열, 시각·레인 순), `tempo[{t_us, bpm, stop_us, scroll}]`.
- 노트: `kind` 0 일반, 1 LN 머리, 2 LN 끝, 3 지뢰(`dto/chart-pattern.ts:13`). 레인 0~6 = 건반 1~7, 7 = 스크래치. 옵션을 적용하기 전 "차트가 말한 그대로"의 배치다. 히든 노트·배경음·겹친 노트는 뺀다(`document.rs` `notes_of` 주석). `section`(마디 단위 위치)은 저장하지만 분석 코드는 쓰지 않는다(분석은 µs 와 BPM 으로 16분 길이를 계산).
- `#RANDOM`/`#SWITCH` 분기는 시드 0 으로 한 가지로 고정 해석된다(저장 문서 근거, client.md 3.6).
- 한도: 본문 4 MiB, 노트 200,000개, 시각 24시간(`dto/chart-pattern.ts:19-28`). 측정한 가장 큰 7K 차트는 약 64,000노트·1.6 MB JSON(같은 파일 주석).

**저장** (확인한 사실)
- 표 `chart_pattern`(`schema.ts:618-624`): PK `md5`, `sha256`(색인), `mode`, `format_version`, `parser_version`, `playable_notes`, `note_count`, `content_hash`, `byte_size`, `data` mediumblob(gzip JSON), `features` json, `features_version`, `uploaded_by`, `created_at`, `updated_at`.
- 노트 배치는 어떤 라우트로도 나가지 않는다. 읽는 곳은 서버 안 분석뿐이다(`chart-pattern.service.ts:181-182` 주석, `tests/route/chart-pattern-admin.test.ts` 가 고정).
- `features`(곡별 요약)는 `computeChartFeatures`(§3.3) 결과이고 `features_version` 이 현재 `ANALYSIS_VERSION`(1) 보다 낮으면 읽을 때 다시 계산한다(`chart-pattern.service.ts:138-143`).

**수집 경로** (확인한 사실)
1. `rbms-cli chart-pattern <라이브러리 폴더> --out <폴더> [--table data.json] [--mode beat-7k]` 가 라이브러리를 재귀로 걷고(`walk.rs`, 심볼릭 링크 무시) 차트마다 JSON 을 쓴다. 기본 모드는 `Mode::BEAT_7K`(`args.rs:7-8`). 다른 모드 차트는 md5 만 기록해 표 커버리지 판정에만 쓴다(`document.rs` `Extracted::OtherMode`).
2. `bun scripts/upload-chart-patterns.ts <폴더> [--server] [--token-env]`(`web/scripts/upload-chart-patterns.ts:35`)가 `PUT /api/admin/chart-patterns/[md5]`(관리자 Bearer만, 세션 불가)로 올린다. 요청 간격 `UPLOAD_REQUEST_INTERVAL_MS`(서버 한도 1,200/분의 90%, `dto/chart-pattern.ts:22`). 토큰은 macOS 키체인(`rbms-admin`/`rbms-pattern-upload`)이 기본이다(`docs/PROCESS.md:132`).
3. `GET /api/admin/chart-patterns/coverage`(표별 `table_md5s`, `covered`, `missing[]`, `patterns_total`, `stale_features`)와 `POST /api/admin/chart-patterns/recompute`(배치 `limit` 1~200)가 운영 점검 수단이다(`chart-pattern.service.ts:164-179,221-239`).

**커버리지 현황** (확인한 사실 + 확인하지 못함)
- 2026-10-20 운영 업로드: 7K 7,903곡 실패 0, 발광 표 1,035/1,035 일치, stale 0(`docs/PROCESS.md:132`). 원본은 유지보수자 한 명의 라이브러리 `/Volumes/SSD/bms` 이고 추출물 333 MB 는 저장소 밖에 있다(같은 줄).
- 이후 표에서 곡 2개(SABOTHER, PopCoRian -EX-)를 빼고 운영 기록도 삭제했다. 표 사본은 지금 1,033곡·중복 없음이다(`web/data/tables/insane1/data.json` 직접 집계). `chart_pattern` 행 1개가 삭제됐다(`docs/PROCESS.md:37`).
- `ANALYSIS_VERSION` 은 2026-10-20 이후 바뀌지 않았다(`git log`). 따라서 stale 은 0 일 가능성이 높지만 운영 현재값은 확인하지 못함.
- 서버가 아는 표는 `insane1` 하나뿐이다(`table-snapshot.data.ts:30-32` `RAW_SNAPSHOTS`). 모든 점수 기반 AI 기능은 `loadTables().values().next().value`(첫 표)를 쓴다(`compose/ai-score-features.ts:238-243`). 즉 후보 곡은 발광 표 1,033곡이다.
- 패턴이 없는 경우: (1) 표 밖 곡 (2) 5K·14K·9K 곡(문서 스키마가 `BEAT_7K` 리터럴) (3) 새 곡·파서 세대가 바뀐 곡(재추출·재업로드 필요). 사용자가 직접 올리는 길은 없고 관리자 토큰으로 유지보수자가 올린다(ADR Q8, `docs/acknowledge/2026-10-19-adr-web-ai-decisions.md:24`). 플레이 중 자동 수집은 없다.
- 표 곡의 노트 수 분포: 1,035곡 최소 946·중앙값 2,114·최대 5,000·합계 2,271,796(저장 문서 근거, `docs/reference/ai-features/server.md` 3.3, 2026-10-19 기존 바이너리로 측정. 이후 2곡이 빠졌으니 현재 값은 확인하지 못함).
- 추정: 표 밖 곡이라도 trace 는 그 판의 노트 배치를 자기 안에 담고 있어(옵션 적용 뒤) 판 분석은 패턴 없이 된다(`trace-analysis.ts:154-166`: `computePlayMetrics` 는 trace 의 노트만 쓰고 패턴은 join 판정과 `pattern_md5` 에만 쓴다). 패턴이 막는 것은 "치지 않은 곡의 태그 비율"을 알아야 하는 추천·연습 폴더·목표 쪽이다.

### 2.3 옵션(RANDOM 등)이 패턴 해석에 미치는 영향

확인한 사실
- 곡별 패턴 특징은 항상 `layout: "original"`(옵션 OFF 배치)이고 `layout_dependent_tags`(레인을 읽는 태그)를 함께 저장한다(`chart-features.ts:104-106`). 판 분석은 `layout: "played"`(플레이어가 본 레인)이다(`play-metrics.ts:211`).
- 시각은 어떤 옵션도 바꾸지 않는다. 그래서 trace 의 객체 시각 다중집합이 패턴의 지뢰 제외 시각 다중집합과 같으면 RANDOM 판도 같은 차트로 본다(`joinPattern`, `trace-analysis.ts:90-102`). SCRATCH AUTO 가 친 레인은 trace 에서 빠지므로 패턴 쪽에서도 그 레인 노트를 뺀 뒤 비교한다.
- 예외: ALL-SCRATCH 는 건반 노트를 스크래치 레인으로 옮기는 유일한 옵션(`SCRATCH_MOVING_OPTIONS = {'AllScratch'}`, `trace-analysis.ts:32`)이라 SCRATCH AUTO 와 겹치면 원래 어느 노트였는지 알 수 없다. trace 시각이 패턴 시각의 부분집합이면 `unsupported`, 아니면 `mismatch` 이고 두 경우 모두 곡별 특징을 잇지 않는다(`trace-analysis.ts:95-97`).
- 옵션 이름은 서버 `RANDOM_OPTIONS` 9개(`dto/common.ts:19`: Off, Mirror, Random, RRandom, SRandom, Spiral, HRandom, AllScratch, Converge). 클라이언트 엔진의 `Rotate` 는 IR 의 `Spiral` 이다(`crates/rbms-ir/src/mapping.rs:88-96`). `Converge` 는 엔진 `NoteOption`(8개)에 없고(`crates/rbms-chart/src/shuffle.rs:7-26`), 빌린 배치 플레이는 이 옵션을 거절한다(`apps/rbms-player/src/borrowed_layout.rs:586-589`). 즉 Converge 판이 실제로 생기는지는 확인하지 못함.
- 옵션별 영향(저장 문서 근거, 셔플 상수 줄은 직접 확인: `shuffle.rs:72` H-RANDOM 125 ms, `:77` 스크래치 40 ms): MIRROR·RANDOM·R-RANDOM·ROTATE 는 판 전체에 레인 순열 하나(스크래치 고정)라 밀도·동시치기 크기·잭(같은 레인 관계)을 보존하고 인접성(계단·트릴)과 손 분배를 바꾼다. S-RANDOM 은 행마다 새 순열이라 잭·계단·트릴 구조를 파괴한다. H-RANDOM 은 125 ms 안 같은 레인을 피해 잭이 줄어든다. ALL-SCRATCH 는 스크래치 노트를 크게 늘린다(`docs/reference/ai-features/client.md` 5.2).
- 코드에서의 반영: `LAYOUT_FREE_TAGS`(chord_2·3·4plus, stream, ln_head, ln_end, soflan, dense, `pattern-tags.ts:57-66`)는 옵션이 바꾸지 않는다고 선언된 태그이고, 나머지 11개(`LAYOUT_DEPENDENT_TAGS`: jack, trill, stair, scratch_solo, scratch_chord, scratch_mixed, with_scratch, scratch_run, ln_hold_hit, ln_release_then_hit, mine_near)는 레인을 읽는다.
- 추정(코드 읽기, 확인 필요): `stream` 태그는 "잭·트릴·계단으로 이미 태그된 노트를 뺀 나머지"(`pattern-tags.ts:370-376`)라 레인 의존 태그에 간접 의존한다. 그런데 `LAYOUT_FREE_TAGS` 에 들어 있다. 옵션 아래에서 stream 태그 노트 집합이 달라질 수 있다.
- 약점 집계는 옵션을 가리지 않고 모든 판을 합친다(`aggregateTagEvidence(plays)` 는 옵션 구분 없음, `sample.ts:83-84`). 옵션별로 나누는 곳은 게이지·옵션 조언의 "옵션별 레인 의존 태그 잔차 상위 3"뿐이다(`gauge-advice.service.ts:116-142`, 옵션당 3판 이상).

### 2.4 AI 관련 DB 표 (확인한 사실)

| 표 | 위치 | 역할 |
|---|---|---|
| `ai_credential` | `schema.ts:522` | 사용자·공급자별 암호화 키(AES-256-GCM 봉투, `ciphertext/iv/auth_tag/wrapped_dek/dek_iv/dek_tag/kek_version`), `key_hint`, `model`, 검증 시각·오류 종류 |
| `ai_usage` | `schema.ts:546` | 사용자·UTC 일별 `calls`, `input/output/cached_input_tokens` |
| `ai_inflight` | `schema.ts:561` | 동시 실행 슬롯(`user_id, slot, run_id, expires_at`) |
| `ai_result` | `schema.ts:574` | 결과 캐시: unique `(user_id, feature, subject_key, input_hash)`, `analysis_version`, `prompt_version`, `provider`, `model`, `output` json, `usage` json |
| `ai_setting`, `ai_user_setting` | `schema.ts:598,607` | 운영자 한도, 사용자 월 토큰 상한·기본 공급자 |
| `chart_pattern`, `table_snapshot` | `schema.ts:618,640` | §2.2 / 표 사본 gzip |
| `ai_goal`, `ai_practice_folder` | `schema.ts:655,670` | 목표 3종 / 연습 폴더 미리보기 해시 → 폴더 연결 |
| `play_trace`, `play_analysis`, `play_share_setting` | `schema.ts:685,710,729` | §2.1 / 판 분석 요약(`trace_id` PK, `analysis_version`, `summary` json, `pattern_join`, `pattern_md5`, `computed_at`) / 라이벌 구간 공유 허용 |

점수 쪽 열(`score`)은 판정 6종·티어별 early/late(`epg…lbd, ems, lms`), `fast/slow/avgjudge`, 옵션(`gauge, random, random_p2, option, scratch_left, scratch_auto, assist, constant, lntype, judge_rate…`), `hispeed/green_number/lift/lane_cover/offset_ms`, `extra`(JSON, `mean_offset_us`·`rule_version`)이다(저장 문서 근거 server.md 1.2, `compose/ai-score-features.ts:163-205` 의 select 로 일부 재확인).

---

## 3. 현재 결정론 분석이 계산하는 것

### 3.1 상수 (전부 `web/src/server/service/domain/analysis/analysis-constants.ts`)

| 상수 | 값 | 줄 | 의미 |
|---|---|---|---|
| `ANALYSIS_VERSION` | 1 | 7 | 곡별 분류·특징 세대. 바꾸면 저장 특징이 낡은 것으로 취급되어 다시 계산 |
| `PLAY_ANALYSIS_VERSION` | 2 | 14 | 한 판 요약 세대(2: 구간 EX·FAST/SLOW·태그, 레인별 타이밍, 게이지는 실패 시점까지만) |
| `S16_US_PER_BPM` | 15,000,000 | 17 | 국소 16분음 길이 = 15,000,000 / bpm µs |
| `FALLBACK_BPM` | 120 | 20 | BPM 이 0 이하일 때 |
| `GAP_TOLERANCE_RATIO` | 0.01 | 23 | 간격 허용 +1% (µs 반올림으로 연속이 끊기지 않게) |
| `CHORD_MIN_KEYS`, `CHORD_TOP_KEYS` | 2, 4 | 25,27 | 4키 이상은 `chord_4plus` 하나 |
| `JACK_MAX_GAP_S16`, `JACK_MIN_RUN` | 2, 3 | 29-30 | 같은 레인 간격 2/16 이내, 3노트 이상 |
| `TRILL_MAX_GAP_S16`, `TRILL_MIN_RUN` | 1, 4 | 32-33 | 1/16 이내, 4행 이상 |
| `STAIR_MAX_GAP_S16`, `STAIR_MIN_RUN` | 1, 4 | 35-36 | 1/16 이내, 4행 이상 |
| `STREAM_MAX_GAP_S16`, `STREAM_MIN_RUN` | 1, 8 | 38-39 | 1/16 이내, 8행 이상 |
| `SCRATCH_LINK_S16` | 1 | 41 | 스크래치와 건반이 1/16 이내면 섞임 |
| `SCRATCH_RUN_GAP_S16`, `SCRATCH_RUN_MIN` | 2, 3 | 42-43 | 스크래치 연속 |
| `LN_FOLLOW_S16` | 1 | 45 | LN 을 뗀 뒤 1/16 이내 치기 |
| `SOFLAN_BPM_RATIO` | 1.25 | 47 | BPM 이 직전 대비 1.25배 이상 |
| `SOFLAN_STOP_MIN_MS`, `SOFLAN_TAIL_MS` | 100, 2000 | 48-49 | STOP 100 ms 이상, 구간 뒤 꼬리 2초 |
| `NEUTRAL_SCROLL` | 1 | 51 | scroll ≠ 1 이면 소프란 |
| `DENSE_PERCENTILE`, `DENSE_BASELINE_PERCENTILE` | 90, 50 | 53,55 | 고밀도 = 창 노트 수가 p90 이상이면서 중앙값보다 큼 |
| `DENSITY_WINDOW_US` | 1,000,000 | 57 | 밀도 창 = 고정 1초 |
| `MAX_DENSITY_WINDOWS` | floor(24h / 1초) + 1 = 86,401 | 59 | 창 개수 상한 |
| `MINE_NEAR_S16` | 1 | 61 | 지뢰 ±1/16 |
| `SP7_SCRATCH_LANE`, `SP7_LANE_NAMES` | 7, `['1'..'7','S']` | 64-65 | |
| `SP7_LANE_HANDS` | S·1·2·3 왼손, 4 공유, 5·6·7 오른손 | 67-70 | 실제 운지는 모름(주석) |
| `SEGMENT_COUNT`, `SEGMENT_TOP_TAGS` | 32, 3 | 72,74 | 판을 시간 기준 32등분, 구간당 많은 태그 3 |
| `DENSITY_BUCKET_WIDTH`, `DENSITY_BUCKET_TOP` | 5, 25 | 77,79 | 노트/초 5 단위 칸, 25 이상 한 칸 |
| `TIMING_HISTOGRAM_RANGE_MS` | 150 | 82 | 오차 히스토그램 ±150 ms, 1 ms 칸 |
| `GAUGE_DROP_WINDOW_MS`, `GAUGE_DROP_MIN_RATIO` | 4000, 0.15 | 84,86 | 4초 안에 게이지 범위의 15% 이상 하락 = 꺾임 |
| `GREEN_DISPLAY_FACTOR` | 0.6 | 89 | GREEN 표시 = 레인 통과 ms × 0.6 |
| `REPORT_RECENT_PLAYS` | 100 | 91 | 리포트가 읽는 최근 판 수 |
| `MIN_TAG_NOTES`, `MIN_TAG_PLAYS` | 300, 5 | 92-93 | 태그 약점·강점 판정의 최소 표본 |
| `MIN_TIMING_PLAYS`, `MIN_TIMING_NOTES` | 10, 2000 | 94-95 | 오프셋 제안 최소 표본 |
| `WILSON_Z_95` | 1.959964 | 97 | 95% 윌슨 구간 |
| `RATIO_DECIMALS` | 4 | 100 | 저장 비율 소수 자릿수 |

점수 기반 기능의 상수는 `web/src/server/service/domain/ai/score-feature.constants.ts`(전부 v1 초안)와 `play-feature.constants.ts`에 있다: `FRONTIER_CLEAR_RATE 0.5`, `FRONTIER_WINDOW_LEVELS 1`, `FRONTIER_DIGEST_LEVELS 2`, `RECOMMEND_CANDIDATES 40`, `RECOMMEND_PICKS 10`, `CANDIDATE_KIND_WEIGHT {failed 3, unplayed 2, lamp_up 1}`, `LEVEL_DISTANCE_WEIGHT 1.5`, `FAILED_EX_WEIGHT 1`, `CHART_TOP_TAGS 3`, `CHART_TAG_MIN_RATIO 0.05`, `CHART_TAGS_NOT_SHOWN ['ln_end','with_scratch']`, `FOLDER_CANDIDATES 60`, `FOCUS_TAG_LEVELS_BELOW 2`, `FOCUS_TAG_MIN_RATIO 0.1`, `GOAL_REMAINING_FOR_AI 20`, `GOAL_PACE_DAYS 28`, `WEEKLY_LIST_MAX 10`, `JUDGE_HABIT_RECENT_PLAYS 100`, `JUDGE_HABIT_MIN_FAST_SLOW 500`, `AVGJUDGE_MIN_HALF_PLAYS 5`, `REFERENCE_RULE_VERSION 4`, `OFFSET_SUGGEST_MIN_MS 1`, `GREEN_BUCKET_WIDTH 50`; `PLAY_REPORT_MIN_PLAYS 5`, `PLAY_REPORT_TOP_ITEMS 5`, `PLAY_REPORT_REPEAT_PLAYS 2`, `PLAY_REPORT_PREFERENCE_MIN_DIFF 0.01`, `PLAY_LANE_BIAS_TOP 3`, `COACH_PROBLEM_SEGMENTS 5`, `COMPARE_GAP_SEGMENTS 5`, `GAUGE_ADVICE_RECENT_PLAYS 100`, `GAUGE_ADVICE_OPTION_MIN_PLAYS 3`, `GAUGE_ADVICE_OPTION_TAGS 3`.

### 3.2 패턴 태그 분류 알고리즘 (`pattern-tags.ts` `classifyPattern`, 줄 301-453)

입력 `PatternInput{ notes[{timeUs, lane, kind}], tempo[], initBpm }`. 노트는 시각 순 다음 레인 순이어야 한다(주석 `:294-295`). 노트 종류는 `normal, ln_head, ln_end(판정 안 되는 LN 끝), charge_end(판정되는 CN·HCN 끝), mine`(`:78`). "치는 노트(onset)" = normal + ln_head(`:90`). "판정 노트" = normal + ln_head + charge_end(`:87-88`). 한 노트가 여러 태그를 가질 수 있다(태그는 서로 배타적이지 않다, `:304-306,452`).

공통 도구
- 국소 16분음: `createTempoLookup`(`:131-140`)이 `tempo` 점을 "다음 점까지 유지"로 읽어 `bpmAt(t)`, `scrollAt(t)`, `sixteenthUsAt(t) = 15,000,000 / bpm` 을 준다. `isWithinSixteenths(earlier, later, n)` = `later - earlier ≤ n × sixteenthUsAt(later) × 1.01`(`:144-145`). 간격은 "뒤 노트 시점의 BPM" 기준이다.
- 행(row): 같은 `timeUs` 의 onset 을 묶고 건반 노트(`keyNotes`)와 스크래치 노트(`scratchNotes`)로 나눈다(`:147-160`). 건반 행 = 건반 노트가 1개 이상인 행(`keyRows`, `:311`).

태그별
| 태그 | 정의 (줄) |
|---|---|
| `chord_2/3/4plus` | 건반 행의 건반 노트 수가 2 / 3 / 4 이상. 그 행의 건반 노트에만 붙는다(`:313-316`). 같은 행의 스크래치는 별도로 `scratch_chord` |
| `trill` | 건반 노트가 정확히 1개인 행이 이어질 때(스크래치만 있는 행은 `keyRows` 에 없어 끊지 않음). 연속 두 행의 레인이 다르고 간격 1/16 이내(`trillPairs`)이며, 3번째부터는 두 행 전 레인과 같아야 한다(ABAB). 4행 이상(`:321-336`). 인접 레인일 필요는 없다(1-7-1-7 도 트릴) |
| `stair` | 건반 1개 행이 이어지고 연속 두 행의 레인 차가 정확히 ±1, 간격 1/16 이내이며 방향(차의 부호)이 첫 두 행과 같다. 4행 이상(`:338-354`). 끝에서 되돌아오면(1-2-3-4-3-2-1) 계단 둘로 나뉜다(`docs/web/architecture.md` 11절 4항) |
| `jack` | 건반 레인마다, 그 레인의 연속 onset 두 개 사이에 다른 건반 행이 없고(`keyRowOrder` 가 정확히 +1) 간격 2/16 이내인 것이 3노트 이상 이어질 때(`:356-368`). 두 노트짜리 반복은 잭이 아니다. 동시치기 행이 같은 레인을 이어받는 코드 연타는 잡힌다 |
| `stream` | 건반 행이 간격 1/16 이내로 8행 이상 이어질 때, 그 노트 중 `jack`·`trill`·`stair` 가 붙지 않은 것만(`:370-376`). 동시치기 행도 `stream` 후보 행에 포함된다 |
| `scratch_chord` | 스크래치 onset 과 같은 시각에 건반 행이 있을 때(`:383-385`) |
| `scratch_mixed` | 같은 시각은 아니지만 ±1/16(×1.01) 안에 건반 onset 이 있을 때. 그 근처 건반 onset 들에는 `with_scratch` 가 붙는다(`:387-395`) |
| `scratch_solo` | 위 둘이 아닌 스크래치 onset(`:390-393`) |
| `scratch_run` | 스크래치 onset 이 간격 2/16 이내로 3개 이상(`:397-402`) |
| `ln_head` | LN 머리 노트(`:407-410`) |
| `ln_end` | 판정되는 끝(CN·HCN, `charge_end`)에만(`:413`). 선언 없는 LN 은 기본 LN 으로 보아 끝을 판정 노트로 세지 않는다(`chart-features.ts:37`) |
| `ln_hold_hit` | 어떤 LN 이 눌려 있는 동안(시작 < t < 끝) 다른 레인에서 치는 onset(`:424-431`) |
| `ln_release_then_hit` | LN 끝 직후 1/16(×1.01) 이내에 같은 손(공유 레인 4 는 양쪽과 같음)의 레인에서 치는 onset(`:432-434`, `isSameHand :267-271`) |
| `soflan` | 판정 노트가 소프란 구간 안에 있을 때: BPM 이 직전 대비 1.25배 이상 바뀐 점부터 2초, STOP 100 ms 이상인 점부터 STOP 길이 + 2초, 또는 그 시점의 scroll ≠ 1(`:214-232,437,444`). BPM 이 0 이하로 바뀌면 값이 다르기만 해도 소프란(`:214-217`) |
| `dense` | 판정 노트가 속한 1초 창(`floor(t / 1초)`, 절대 시각 고정 창)의 판정 노트 수가 모든 창 분포의 nearest-rank p90 이상이고 중앙값(p50)보다 클 때(`:273-291,438-446`). 창 배열은 0초부터 마지막 노트 창까지이며 빈 창도 0 으로 포함한다 |
| `mine_near` | 판정 노트로부터 ±1/16(×1.01) 안에 어느 레인이든 지뢰가 있을 때(`:441,447-449`) |

표본·정의상 주의 (확인한 사실)
- `dense` 는 곡 안의 상대 기준이다. 노트/초의 절대값이 아니라 "그 곡의 p90 이상"이므로 평균 6 노트/초 곡의 p90 구간과 평균 15 노트/초 곡의 p90 구간이 같은 태그다.
- 밀도 단위는 판정 노트 수/초이며 동시치기 노트를 각각 센다. 행(rows)/초와 구분하지 않는다.
- DP·5K·9K 는 분류하지 않는다. 판 분석은 `trace.mode === 'BEAT_7K'` 일 때만 지표를 만들고(`trace-analysis.ts:35,156-165`) 곡별 패턴 문서도 `BEAT_7K` 만 받는다. 다른 모드 trace 는 게이지·속도만 요약된다.

### 3.3 곡 단위 특징 (`computeChartFeatures`, `chart-features.ts:90-122`)

입력은 패턴 문서이고 `layout: 'original'`. 출력(`chart_pattern.features`):
- `analysis_version`, `layout`, `layout_dependent_tags`, `judged_notes`, `ln_end_judged`, `length_ms`(마지막 노트 시각).
- `tags[태그] = {notes, ratio}`: 19개 태그 각각의 판정 노트 수와 `ratio = notes / judged_notes`(4자리 반올림). 노트당 태그가 여러 개라 비율 합이 1 이 아니다.
- `density{peak, average, late_average}`(`densityOf :57-75`): 1초 창 판정 노트 수 열에서 `peak` = 최댓값, `average` = 평균의 1/4(floor) 이상인 창들의 평균, `late_average` = 뒤 1/3 창의 평균.
- `soflan{tempo_jumps, long_stops, bpm{min, max, max_to_min}}`, `ln_ratio`(onset 중 LN 머리 비율), `scratch_ratio`(onset 중 스크래치 비율), `mines`(지뢰 수).
- 곡별로 저장되지 않는 것: 노트/초 분포 히스토그램, 슬라이딩 창 최대, 고밀도 구간의 길이·위치, 태그 인스턴스(예: 트릴 런의 길이 분포), 동시치기 행 수, 최소 간격, 행/초. 소비자는 태그 비율 상위 3개(`topTagsOf`, `table-progress.ts:133-138`)와 `tags` 맵 전체뿐이다.

### 3.4 판 단위 지표 (`computePlayMetrics`, `play-metrics.ts:141-245`)

입력: trace 의 노트(옵션 적용 뒤 레인, SCRATCH AUTO 레인 제외), 빈 POOR 시각, 판 길이 `play_time_ms × 1000`(0 이면 마지막 노트 + 1), tempo. 7K 만.

판정 집계 (`emptyTally/addNote/finishTally :57-80`)
- 판정 등급: `state` 1~5 → pgreat, great, good, bad, poor(`:41-47`). 판정 노트 중 등급이 있는 것만 센다(도달하지 못한 state 0 노트는 세지 않는다. 구간의 `chart_notes` 에만 들어간다).
- 각 집계는 `{pgreat, great, good, bad, poor, fast{pgreat,great,good,bad}, slow{…}, notes, ex_score, ex_ratio}`. POOR 는 FAST/SLOW 를 세지 않는다(`:70`). `err_us >= 0` 이 FAST(`:71`).
- `ex_score = 2·PG + GR`, `ex_ratio = ex_score / (2·notes)`(4자리), 노트가 없으면 `null`(0 이 아니라 데이터 없음, `:75-80`).

출력 항목
| 항목 | 정의 | 줄 |
|---|---|---|
| `overall` | 판 전체 집계 | 151-152, 212 |
| `by_tag[태그]` | 그 태그가 붙은 판정 노트의 집계(노트가 여러 태그면 중복 집계). 오차 크기는 쓰이지 않고 FAST/SLOW 개수만 센다 | 205-207, 123-131 |
| `by_lane[레인 이름]` | 레인('1'~'7','S')별 집계 | 214-216 |
| `by_density[칸]` | 노트가 속한 절대 1초 창의 판정 노트 수를 5 단위 칸으로(`0-4, 5-9, 10-14, 15-19, 20-24, 25+`, `densityBucketOf :115-119`) 나눈 집계 | 217-223 |
| `segments[32]` | 판 시간(`lengthUs`)을 32등분한 구간마다 `id('s00'~'s31'), from_ms, to_ms, chart_notes(판정 객체 수, 도달 여부 무관), notes(판정된 수), ex_score, bp, combo_breaks, fast, slow, top_tags(많은 태그 3)`. `bp` = BAD + POOR + 빈 POOR. `combo_breaks` = 콤보가 0 보다 큰 상태에서 BAD/POOR 가 나온 횟수. 콤보는 BAD/POOR 에서 0 으로, 그 밖의 등급에서 +1 | 149-192, 224-230 |
| `empty_poor` | 빈 POOR 수 | 231 |
| `timing` | PGREAT·GREAT 오차만: `notes, mean_us, median_us, stdev_us(모집단 표준편차), iqr_us`, 1 ms 칸 히스토그램(±150 ms, 0 방향 내림) | 95-113, 232 |
| `timing_by_lane` | PG·GR 만, 레인별 `{notes, mean_us}` | 233-238 |
| `fatigue` | 앞 1/3(t×3 < 길이)과 뒤 1/3(t×3 ≥ 길이×2)의 EX 비율과 차 `delta = last - first`, 한쪽이라도 null 이면 null | 194-201, 239-243 |

### 3.5 게이지 지표 (`play-metrics.ts:249-331`, `trace-analysis.ts:104-137`)

- 서버는 게이지를 계산하지 않고 클라이언트가 올린 9개 곡선만 읽는다(`:271-275,312` 주석).
- `run_end`: `failed_at_ms` 가 있으면 `gauge_empty`, 아니면 "state 0 인데 일반 LN 끝(kind 2)이 아닌 객체"가 있으면 `left`(Esc·종료), 아니면 `finished`(`trace-analysis.ts:104-108`).
- `findGaugeDrops`(`:276-295`): 친 게이지 곡선에서 `span = round(4000 / interval_ms)` 샘플 창 안 최고점 대비 `threshold = 0.15 × (max - min)` 이상 떨어진 지점을 꺾임으로 보고, 이웃한 꺾임은 하나로 합쳐 `{from_ms(창 안 마지막 최고점), to_ms(가장 깊은 지점), drop}` 으로 낸다. 비어 끝난 판은 실패 시각까지의 샘플만 쓴다(`liveGaugeValues :266-269`).
- `summarizeGaugeCurves`(`:319-329`): 게이지마다 `{end, cleared(true|false|null), reached_min, outcome(cleared|failed|unknown)}`. 끝까지 친 판: 생존 게이지(`min 0 && border 0`: HARD·EXHARD·HAZARD·CLASS 류)는 끝값 > 0 이면 통과, 나머지는 끝값 ≥ `border`. `left` 판은 모든 게이지 실패. `gauge_empty` 판: 친 게이지는 실패, 이미 바닥에 닿은 생존 게이지도 실패, 그 밖은 `unknown`(`cleared: null`).
- `segment_values`: 구간 끝 시각의 친 게이지 값. 비어 끝난 판은 실패 이후, `left` 판은 마지막 샘플 이후 구간이 null(`trace-analysis.ts:121-137`).

### 3.6 속도·녹색 숫자

- 점수 행의 `green_number` 는 레인 통과 시간(ms)의 정수 반올림이고 스킨 GREEN 은 그 값 × 0.6 이다. 그래서 점수 행만으로 만든 GREEN 은 `is_estimated: true`(`describeGreenNumber`, `play-metrics.ts:338-342`). trace 의 `speed.end.green/green_ms` 는 클라이언트가 스킨과 같은 식으로 계산한 값이라 추정이 아니다(`readTraceGreenNumber :345-349`).
- 판 요약 `speed`: `start`, `end`(각각 `{green, lane_ms, is_estimated:false}`), `changes`(변경 횟수), `changed_fields`(바뀐 필드 이름). 변경 시각·값 자체는 요약에 남기지 않는다(`trace-analysis.ts:191-196`).

### 3.7 판 요약(`play_analysis.summary`)과 패턴 join

`analyzeTrace`(`trace-analysis.ts:147-200`)가 한 판당 한 번 만든다.
- 필드: `analysis_version`(PLAY, 2), `pattern_analysis_version`(1), `format_version`, `mode`, `random`, `lntype`, `scratch_auto`, `run_end`, `metrics`(7K 만, §3.4), `gauge{selected, interval_ms, failed_at_ms, drops, curves[9], segment_values}`, `speed`, `pattern{join, md5}`.
- join 값: `matched`, `mismatch`, `unsupported`, `absent`(`:24`). `matched` 일 때만 `pattern_md5` 를 남긴다.
- 업로드 직후 같은 요청에서 계산해 `play_analysis` 에 저장한다(`trace.service.ts:190-212,332`). 실패하면 trace 는 남고 `pending`.
- 읽을 때 저장 요약의 두 버전(`analysis_version`, `pattern_analysis_version`)이 현재 값과 다르면 저장 trace 를 풀어 다시 계산한다. 한 번의 읽기에서 다시 계산하는 판은 최대 `PLAY_ANALYSIS_READ_RECOMPUTE_MAX = 10`개이고 나머지는 `stale` 로 세어 뺀다(`trace.service.ts:169,262-287`). 다시 계산이 예외로 끝난 판은 `failed` 표시를 저장해 버전이 오를 때까지 다시 시도하지 않는다(`:222-233,239-255`).
- 관리자 일괄 재계산: `POST /api/admin/traces/recompute`(`limit` 1~200, 기본 50, `dto/trace.ts:178-189`), 곡별 특징은 `POST /api/admin/chart-patterns/recompute`.

### 3.8 표본이 적을 때의 처리 (`sample.ts`)

- 등급 `EVIDENCE = strong | uncertain | insufficient`(`sample.ts:7`). AI 에는 등급만 주고 구간 값은 주지 않는다(주석).
- 태그 약점·강점(`tagEvidenceOf :45-72`): 표본 = 그 태그 노트가 있고 판 전체 노트도 있는 판들(SCRATCH AUTO 판은 스크래치 계열 태그에서 제외하고 `auto_plays` 로 센다). `exRatio = Σ태그EX / (2 × Σ태그노트)`, `baseline = Σ(태그노트 × 그 판 전체 EX 비율) / Σ태그노트`(판 난이도를 상쇄하려는 노트 가중), `residual = exRatio − baseline`. `notes ≥ 300 && 표본 판 ≥ 5` 이어야 판정하고 아니면 `insufficient`. 판정할 때 `successes = ΣEX점수, trials = 2 × 노트 수` 의 95% 윌슨 구간이 `baseline` 을 벗어나면 `strong`, 아니면 `uncertain`. `strong` 일 때만 `residual < 0` 이면 약점, 아니면 강점(`leaningOf :40-43`).
- 오프셋 제안(`timingEvidence :87-88`): 판 ≥ 10 && PG+GR 노트 ≥ 2,000 이면 strong.
- 판정 습관의 FAST 비율: 판 ≥ 10 && FAST+SLOW ≥ 500 이어야 판정하고 윌슨 구간이 0.5 를 벗어나면 strong(`judge-habit.service.ts:211-220`).
- 판 수 하한: 리포트·게이지 조언은 분석된 판 5 미만이면 AI 호출 없음(`PLAY_REPORT_MIN_PLAYS`), 판정 습관은 최근 판 10 미만이면 없음(`MIN_TIMING_PLAYS`).
- 등급이 없는 지표(확인한 사실): 레인별 EX·FAST 비율, 밀도 칸별 EX, 피로 평균, 타이밍 평균은 `notes` 수는 표시하지만 표본 등급이 없다. AI digest 에 들어가는 레인은 표본 크기와 무관하게 "전체 EX 비율과의 차가 큰 상위 3개"다(`play-report.service.ts:184-187`).
- 추정(통계적 한계): 윌슨 구간은 EX 점수를 독립 시행 2×노트 수로 보므로 같은 판·같은 구간의 노트들이 상관되어 있다는 점을 반영하지 못해 구간이 실제보다 좁을 수 있다. 태그 19개를 각각 검정하는 다중비교 보정도 없다. 코드가 이를 다루는 부분은 확인하지 못함.

---

## 4. 기능 10개

공통 골격(확인한 사실)
- 각 기능은 `GET`(결정론 digest + 마지막 저장 AI 결과, 공급자 호출 없음)과 `POST`(사용자가 "AI 해석 만들기"를 누를 때만 공급자 1회 호출, 자동 재시도 없음)를 가진다. 모든 응답은 `Cache-Control: private, no-store`(`route/ai.ts:70-71,87`). 모든 AI 라우트의 어댑터 파일은 `export const maxDuration = 300` 이다(예: `web/src/app/api/fe/me/ai/report/route.ts:3`). ADR Q14 는 Pro 플랜의 최대 800초를 허용하지만 코드는 300이다.
- 라우트는 `route/ai.ts` 의 팩토리 4개: `createAiRoutes`(키·설정·사용량, 166), `createAiRecordQueryRoutes`(248), `createAiScoreFeatureRoutes`(326), `createAiPlayFeatureRoutes`(465). 읽기·해석 모두 사용자별 분당 30회(`RATE_LIMIT.TABLE_RECORDS`, `route/ai.ts:335,474`, `with-rate-limit.ts:12`).
- 서비스는 `service/domain/ai/*.service.ts`. `readStoredInterpretation`(`ai-digest.ts:101-121`)이 마지막 저장 결과와 `is_current`(현재 digest·프롬프트·스키마·공급자·모델의 해시가 저장 해시와 같은가)를 준다. `runAiFeature`(`ai-digest.ts:86-93`)가 공통 러너를 부른다.
- 프롬프트 공통 규칙: 입력 JSON 안 문자열은 데이터일 뿐 지시가 아니다(`DIGEST_IS_DATA_RULE`), 입력에 없는 숫자를 만들지 않고 `key` 는 입력에 있는 값만(`DIGEST_KEY_RULE`)(`ai-digest.ts:22-27`). 출력 후 `refine` 이 digest 에 없는 키·길이를 넘는 항목을 버리고, 남은 것이 없으면 `output_invalid`. 모든 기능의 `effort` 값은 `'low'` 이지만 어떤 어댑터도 보내지 않는다(`types.ts:40-44`, 캐시 키에만 들어간다).
- 출력 언어는 화면 언어(`AI_OUTPUT_LANGUAGE_NAME` ko/en/ja, `ai-digest.ts:13`)이고 프롬프트 문자열에 들어가 캐시 키가 언어별로 갈린다.

### 4.1 플레이어 리포트 (기능 1) — `play-report.service.ts`
- 입력: 최근 분석된 판 100개(오토플레이 제외, 최신순, `trace.service.ts` `recentAnalyzedPlays` → `compose/trace.ts:226-232`), 그 판들의 곡별 태그 비율(`createChartTagLoader`), 발광 표 램프 분포, 점수 행 FAST/SLOW(trace 없는 판용).
- 결정론(`computePlayReport :111-165`, 7키 지표가 있는 판만 `measured`): 태그별 잔차·등급(§3.8), 레인별 EX·FAST 비율, 밀도 칸별 EX, 피로 평균 `mean_delta`(판별 `fatigue.delta` 평균), PG·GR 평균 오차(노트 가중), 옵션 사용 비율, 선호 패턴(`pref.<태그>`: 2판 이상 친 곡의 원본 배치 태그 비율 평균 − 친 곡 전체 평균, 차 ≥ 0.01 상위 5).
- LLM 입력 digest(`toPlayReportDigest :180-208`): 강점·약점 각 5(등급 strong 만), 불확실·표본 부족 태그는 이름만, `habits` = 타이밍 평균·피로·옵션 최다 1·밀도 최상위 칸 1, 레인 편차 상위 3, 프런티어 근처 레벨 램프.
- 프롬프트 버전 2, 최대 출력 12,000 토큰(`:42-45`). 스키마 `aiReportOutputSchema`(`dto/ai-play.ts:57-64`): `headline, strengths[{key,title,body,tone}], weaknesses[], habits[], practice_focus[{key, action}], caveats[]`.
- 검증(`refinePlayReportOutput :228-254`): 강점·약점에 strong 목록 밖 태그 키가 하나라도 있으면 전체 거절. 불확실·표본 부족 태그가 있는데 `caveats` 가 비면 거절.
- AI 를 부르지 않는 조건: 7키 지표 판 5 미만(`need: traced_plays`). 이때도 결정론 화면은 보인다.
- 캐시 `subject_key` = `recent`. UI: `/ai?tab=report` `AiReportPanel`(`widgets/ai/ai-report-panel.tsx`), 탭 배선 `ai-hub-tabs.tsx:56-58`.

### 4.2 리플레이 코치 (기능 2) — `replay-coach.service.ts`
- 입력: 내 점수 하나(`score_id`) + 그 판 분석.
- 결정론(`computeCoachAnalysis :107-145`): 32구간 각각의 EX 비율·BP·콤보 끊김·FAST/SLOW·상위 태그·구간 끝 게이지, 꺾임, 9게이지 결과, 문제 구간 = BP 큰 순(같으면 앞) 뒤에 BP 0 이지만 꺾임이 끝나는 구간, 최대 5개(`:126-128`).
- LLM 입력: 문제 구간별 `{key, segment_id, from_ms, to_ms, bp, combo_breaks, fast, slow, ex_ratio, top_tags, gauge_before, gauge_drop}`, 다른 게이지 결과, 곡 제목(데이터).
- 프롬프트 v1, 8,000 토큰. 스키마 `{summary, segments[{segment_id, diagnosis, advice}], next_try}`(`dto/ai-play.ts:105-109`). 검증: 문제 구간 밖 `segment_id` 제거, 문제가 있었는데 남은 구간이 없으면 거절(`:195-212`).
- 호출 안 함: trace 없음(`play_trace`), 7키 SP 아님(`play_metrics`). 키 `score:<id>`. UI: `/ai/plays/[scoreId]` `AiCoachPanel`(`widgets/ai/ai-play-detail-content.tsx:19`). 진입 링크는 플레이어 페이지 기록 행의 "판 코치"(본인 기록에만, `widgets/player/player-recent-panel.tsx:72`).

### 4.3 베스트·라이벌 비교 (기능 3) — `play-compare.service.ts`
- 입력: 내 점수 하나 + 비교 대상(`against=best`: 같은 차트·같은 SCRATCH AUTO 의 내 EX 최고 trace 판, 없으면 공개 최고 기록; `against=rival&rival=<login>`).
- 결정론: 두 판의 32구간 `chart_notes` 가 모두 같을 때만 구간 비교(`compareSegments :125-143`, 다르면 `note_layout_differs` 이유로 합계 표만). 구간 `ex_diff`, `bp_diff`(이 판 − 상대), 누적 EX 차. 한쪽이 도달하지 못한 구간(판정 객체가 모두 판정되지 않은 첫 구간부터)은 null 로 빼고(`reachedSegmentCountOf :114-117`), 차이 큰 구간 5개를 고른다(`pickGapSegments :148-157`).
- 라이벌: 서로 상대를 라이벌로 두고 상대가 `play_share_setting.rival_segments` 를 켰을 때만 구간 합계(`id, chart_notes, notes, ex_score, bp`와 판 결과)를 받는다(`trace.service.ts:417-453`). 노트·오차·레인·태그는 나가지 않는다. 그렇지 않으면 공개 최고 기록의 판정 합계 표.
- LLM digest: `base`, `target`(라이벌이면 `who` 만), `overall{ex_diff, compared_segments}`, `gaps[{key, segment_id, from_ms, to_ms, ex_diff, bp_diff, cumulative_ex_diff, top_tags}]`(`:86-109`).
- 프롬프트 v2, 6,000 토큰. 스키마 `{summary, gaps[{segment_id, explanation}], takeaway}`. 호출 안 함: 비교 가능한 구간 차이가 없음(`comparable_play`). 키 `cmp:<scoreId>:<against>:<targetScoreId>`. UI: 같은 `/ai/plays/[scoreId]` 페이지의 `AiComparePanel`.

### 4.4 판정 습관 진단 (기능 4) — `judge-habit.service.ts`
- 입력: 최근 100판의 점수 행(오토플레이 제외) + 각 판의 trace 분석(있으면).
- 결정론(`computeJudgeHabit :211-316`): FAST 비율과 윌슨 95% 구간·등급, 등급별(great·good·bad) 빠름/늦음 합계, 규칙 세대 4 이상 판의 `extra.mean_offset_us` 가중 평균(AUTO ADJUST 판 제외)과 오프셋 변경 제안(부호 반전 `-round(bias)`, `|shift| ≥ 1 ms`; trace 가 strong 이면 trace 기준, 아니면 점수 기준(10판·PG+GR 2,000·FAST 비율 strong·같은 쪽일 때만)), trace 타이밍(AUTO ADJUST 제외 PG·GR 노트 가중 평균, 분산 합성, 95% 구간, 등급, 편차 큰 레인 3), `avgjudge` 앞·뒤 절반 평균과 차(규칙 세대 4 이상, 절반이 5판 이상이면 strong), 녹색 숫자 최근값·중앙값·50 단위 칸별 `fast_share`, `ex_rate`.
- 프롬프트 v2, 8,000 토큰. 스키마 `{summary, offset_direction, offset_reasoning, green_direction, green_reasoning, notes[{key,title,body,tone}]}`(`dto/ai.ts:525-532`). 검증: `offset_direction` 을 서버 제안으로 덮어쓴다(AI 는 설명만, `refineJudgeHabitOutput :338-358`). JUDGE OFFSET 은 "노트가 보이는 위치를 옮긴다"로만 설명하게 문구를 고정(`:47-52`).
- 호출 안 함: 최근 판 10 미만(`recent_plays`). 키 `recent`. UI: `/ai?tab=habit` `AiJudgeHabitPanel`.

### 4.5 다음 도전 곡 추천 (기능 5) — `recommend.service.ts` + `table-progress.ts`
- 입력: 발광 표(첫 표) + 내 표 기록(`tableSnapshotService.getRecords`: 곡별 최고 램프·EX·BP·시각) + 후보 곡의 태그 비율(패턴이 있으면).
- 결정론: 레벨별 곡 수·플레이 수·클리어 수·HARD 이상 수·클리어 비율(`summarizeLevels :95-111`), 프런티어 = 클리어 비율 ≥ 0.5 인 가장 높은 레벨의 다음(없으면 첫 레벨, 전부 넘으면 마지막, `findFrontierIndex :118-123`). 후보 = 프런티어 ±1 레벨의 미플레이·FAILED·HARD 아래 클리어 곡. 점수 = 종류 가중(실패 3·미플레이 2·램프 갱신 1) − 1.5 × 레벨 거리 + 실패 곡의 EX 비율 × 1, 상위 40곡(`rankChallengeCandidates :162-182`). 각 후보에 태그 비율 ≥ 0.05 인 상위 3 태그(`ln_end`, `with_scratch` 제외).
- LLM: 후보 중 최대 10곡을 순서대로 고르고 곡마다 한 줄 이유와 `focus_key`(스키마 `{summary, picks[{md5, reason, focus_key}]}`, `dto/ai.ts:329-332`). 프롬프트 v2, 8,000 토큰. 검증: 후보 밖 md5·모르는 `focus_key`·한 줄 초과 제거, 중복 md5 제거, 남은 것 없으면 거절.
- 호출 안 함: 표 기록 없음(`table_records`)·후보 없음(`candidates`). 키 `table:<id>`. UI: `/ai?tab=recommend` `AiRecommendPanel`.
- 확인한 한계: 후보 선정과 점수에 플레이어의 태그 약점(리포트의 잔차)이 쓰이지 않는다. 태그는 후보를 보여 주는 정보일 뿐이다.

### 4.6 AI 연습 폴더 (기능 6) — `practice-folder.service.ts`
- 출처 3종: `recommend`(기능 5 후보), `tag`(사용자가 UI 에서 고른 태그), `goal`(목표의 남은 곡) (`dto/ai.ts:349`, `practice-folder.service.ts:163-190`).
- `tag` 출처(`fromTag :142-161`): 프런티어 −2 ~ +1 레벨의 곡 중 램프가 FULL COMBO 미만이고 그 태그 비율 ≥ 0.1 인 곡을 비율 큰 순(같으면 레벨, md5)으로 상위 60곡. 태그 비율은 곡별 패턴이 있는 곡만 안다(패턴이 하나도 없으면 `chart_patterns` 고지).
- LLM: 후보에서 최대 30곡(`AI_PRACTICE_LIMIT.ITEMS_MAX`)을 순서대로 고르고 곡마다 한 줄 메모(≤100자), 폴더 이름 접미사(≤40자), 설명(≤255자). 스키마 `{name_suffix, description, items[{md5, memo}]}`. 프롬프트 v2, 8,000 토큰.
- 만들기(`create :275-336`): 미리보기를 사용자가 확인하면 폴더 `AI 연습 · <이름>`(`AI_PRACTICE_LIMIT.NAME_PREFIX`)을 `folders.create` 로 만들고 `replaceItems` 로 곡을 넣는다. 항목은 `{md5, sha256:'', title, artist, note}`(`:285-292`). 같은 미리보기 해시를 다시 만들면 `ai_practice_folder` 연결로 같은 폴더의 곡을 바꾼다. 이름이 겹치면 오늘 UTC 날짜를 붙인다. 폴더가 50개면 미리보기도 하지 않는다.
- UI: `/ai?tab=recommend` 의 `AiPracticeFolderPanel`(`widgets/ai/ai-practice-folder-panel.tsx`). 출처가 `tag` 일 때 태그는 사용자가 드롭다운으로 직접 고른다(`:26-30,65-72`). 플레이어 리포트의 약점이 자동으로 넘어오지 않는다.

### 4.7 목표 계획 (기능 7) — `goal-plan.service.ts`
- 목표 3종만(`lamp_count_at_level`, `clear_all_level`, `ex_rate_on_chart`, `dto/ai.ts:406`), 활성 10개, 자유 문장 없음.
- 결정론: 목표 범위 곡 중 달성 수·목표 수·남은 수·최근 28일 안에 갱신된 곡 수(`computeGoalProgress :115-146`), 남은 곡은 램프 높은 순(가까운 순, `compareRemaining :94-100`).
- LLM: 남은 곡 20곡(`GOAL_REMAINING_FOR_AI`)을 주고 주별 계획 최대 8주. 스키마 `{summary, steps[{week_index, focus, chart_md5s[], check_key}]}`. 프롬프트 v2, 8,000 토큰. 검증: 주 범위·`check_key` 존재·한 줄·남은 곡 안의 md5 만, 주마다 첫 항목만(`refineGoalPlanOutput :36-53`).
- 호출 안 함: 표 없음, 목표 달성, 남은 곡 없음. 키 `goal:<id>`. UI: `/ai?tab=goals` `AiGoalPanel`·`AiGoalPlanSection`. 목표에 태그·밀도 약점 종류는 없다(문서에 따르면 `tag_residual` 종류는 보류, `docs/web/architecture.md` 12-2).

### 4.8 게이지·옵션 조언 (기능 8) — `gauge-advice.service.ts`
- 입력: 최근 100판의 분석(게이지 곡선 9개)과 점수 행의 게이지·램프, 레벨 라벨.
- 결정론(`computeGaugeAdvice :94-149`): 친 게이지별 판 수·통과 수, 레벨·친 게이지별 실패 판과 그중 다른 게이지가 `cleared` 였던 판 수(`rescued_by`; `unknown` 은 세지 않고 비어 끝난 판은 `unknown_runs`), 옵션별 판 수·EX 비율·레인 의존 태그 중 잔차 절댓값 상위 3(옵션당 3판 이상일 때).
- LLM: 스키마 `{summary, gauge_advice[{gauge, reasoning}], option_advice[{option, reasoning}]}`, 항목당 최대 5개. 프롬프트 v1, 6,000 토큰. 검증: digest 에 나온 게이지·옵션만.
- 호출 안 함: 분석된 판 5 미만. 키 `recent`. UI: `/ai?tab=gauge` `AiGaugeAdvicePanel`.

### 4.9 자연어 기록 질문 (기능 9) — `record-query.service.ts`, `record-query.compiler.ts`
- 사용자의 자연어 질문(200자 이하)을 LLM 이 필터 스키마로 바꾸고, 서버가 필터를 검증·컴파일해 사용자 자신의 기록에 실행한다. 사용자가 필터를 고쳐 다시 실행하는 `run` 은 AI 호출이 없다(`route/ai.ts:248-269`).
- 필드 10개(`level, lamp, ex_rate, bp, played_at, random, gauge, scratch_auto, title, table`), 연산 6개, 정렬 필드 5개, 최대 필터 8개, 최대 행 100(`dto/ai.ts:145-167`). 필드별 허용 연산표 `AI_RECORD_QUERY_ALLOWED_OPS`(`:170-181`). 패턴·밀도·판정 분포 같은 필드는 없다.
- 스키마 `{filters[{field, op, value:string}], sort_field, sort_dir, limit, explanation}`(값은 문자열, 서버가 필드별로 해석). 프롬프트 v2, 16,000 토큰(사고 여유). 키 `q:<해시(질문)>`. 같은 질문은 같은 UTC 날짜 안에서 캐시에서 나온다.
- UI: `/ai?tab=query`(기본 탭) `RecordQueryPanel`, 키 게이트 `AiReadyGate` 안에서만 보인다(`ai-hub-tabs.tsx:40-45`). 나머지 탭은 키 없이도 결정론 숫자를 보인다.
- 이 기능은 "LLM 이 구조화 파라미터만 내고 서버가 결정론으로 실행"하는 구조의 선례다(§6.1).

### 4.10 주간 성장 요약 (기능 10) — `weekly-summary.service.ts`
- 입력: 요청 주(UTC 월요일 00:00~다음 월요일, 기본 이번 주)와 앞 주의 점수 행, 그 주 이전의 곡별 최고 기록.
- 결정론(`summarizeWeek :27-109`): 판 수, 곡 수, 활동 일수, 처음 친 곡, 램프 갱신, 새 클리어, EX 갱신, FAST/SLOW 합과 비율, 규칙 세대 4 판의 치우침, 램프 변화·EX 변화 상위 10, 레벨별 판 수·갱신 수. SCRATCH AUTO 유무는 따로 최고를 센다.
- LLM: 스키마 `{headline, highlights[{key,title,body,tone}], next_week}`, 하이라이트 최대 6개. 프롬프트 v2, 8,000 토큰. 주간 키 `week:<월요일>`. 호출 안 함: 그 주 판 0.
- UI: `/ai?tab=weekly` `AiWeeklyPanel`. 자동 생성 없음(열 때 버튼으로만).
- 확인한 한계: 점수 행만 읽고 trace(패턴·밀도)는 쓰지 않는다.

### 4.11 UI 탭 구성 (확인한 사실)
- `AI_HUB_TABS = ['query', 'recommend', 'goals', 'weekly', 'report', 'habit', 'gauge']`(`entities/ai.type.ts:206`), 기본 탭 `query`(`ai-hub-tabs.tsx:20`). `recommend` 탭에 기능 5와 6이 함께 있다. 코치·비교는 허브가 아니라 `/ai/plays/[scoreId]` 페이지에 있다. 키 등록·모델 선택·한도는 설정 화면의 AI 탭(`AI_SETTINGS_TAB`, `shared/constants/navigation.ts:21`)과 `/api/fe/me/ai/credentials*`·`settings`·`usage`.

---

## 5. 격차

### 5.1 (a) "못 치는 패턴, 못 버티는 밀도"를 특정하려는데 없는 지표

표의 "원자료" 열은 현재 서버가 이미 저장한 trace 로 계산 가능한지를 뜻한다(확인한 사실: trace 에 노트별 `time_us, lane, kind, state, err_us` 와 게이지 열이 있다, §2.1).

| 필요한 지표 | 현재 가장 가까운 것 | 모자란 점 | 원자료 |
|---|---|---|---|
| 패턴 유형별 판정 분포 | `by_tag` 의 등급별 개수와 FAST/SLOW 개수(`play-metrics.ts:205-207`), 판 사이 집계는 EX 잔차 하나로 줄어든다(`sample.ts`) | 태그별 PG/GR/GD/BD/PR 분포를 사용자 단위로 합친 값이 화면·digest 에 없다. 태그별 오차의 평균·산포·히스토그램이 없다(`addNote` 가 오차 크기를 버리고 FAST/SLOW 개수만 센다, `:68-73`). 태그 인스턴스 단위(예: 트릴 런 하나가 모두 성공했는가, 런 길이별 성공률)가 없다 | trace |
| 태그 겹침 분리 | 한 노트에 여러 태그(`:304-306`). 잔차는 태그마다 독립 계산 | `dense` 와 `chord_*`, `stream` 처럼 자주 겹치는 태그의 효과를 분리하지 못한다(다변량 없음). 태그 단독(다른 태그가 없는 노트)과 조합의 구분이 없다 | trace |
| 밀도 구간별 성공 한계 | `by_density`: 고정 1초 창의 노트 수 5 단위 칸, 25+ 한 칸(`:115-119,217-223`) | 칸 안에서 EX 비율만 AI 에 가고(밀도 최상위 칸 하나, `play-report.service.ts:203`), 한계(예: EX 비율이 일정 값 아래로 떨어지는 노트/초, BP 비율이 임계를 넘는 노트/초)를 추정하지 않는다. 25+ 가 한 칸이라 고밀도 곡의 상한 구분이 안 된다. 칸별 표본 등급이 없다 | trace |
| 지속 밀도 vs 순간 밀도 | 곡별 `density{peak, average, late_average}`(`chart-features.ts:57-75`), 창은 고정 1초 | 슬라이딩 창(0.5/1/2/4/8초) 최대·지속 시간이 없다. 창 경계에 걸친 버스트가 둘로 나뉜다. "N 노트/초를 몇 초 이어 치는가" 같은 지속 지표가 없다. 판 쪽 분석은 지속 구간을 따로 만들지 않는다 | trace(판), 패턴(곡) |
| 행 밀도 vs 노트 밀도 | 노트/초만 | 동시치기 노트를 각각 센다. 같은 12 노트/초라도 4키 계단 12행/초와 6 동시치기 2행/초가 같은 칸이다 | trace |
| 곡 쪽 밀도 분포 | `peak/average/late_average` 세 숫자 | 노트/초 분포 히스토그램, 밀도 상위 구간의 길이·위치가 저장되지 않는다. 훈련곡 파라미터와 직접 비교할 수 있는 형식의 곡 특징이 없다 | 패턴(재계산해 `features` 확장 가능) |
| 손/레인 편향 | `by_lane` EX·FAST 비율, `timing_by_lane` 평균 오차, 레인 편차 상위 3(`play-report.service.ts:184`), 손 배정 상수(`SP7_LANE_HANDS`) | 손 단위 집계가 없다(손 상수는 `ln_release_then_hit` 판정에만 쓰인다, `:267-271`). 레인 전환(예: 1에서 7로 건너뜀), 레인별 BP 비율, 레인별 오차 산포, 스크래치 방향이 없다. 레인 표본 등급이 없다(§3.8) | trace |
| 연속 BAD/POOR 구간 | 구간(32등분)별 `bp`, `combo_breaks`(`:176-192`) | 연속 실패 길이 분포, 최장 연속, 실패 뒤 회복 시간(복구까지 걸린 노트 수), 콤보 끊김이 생긴 패턴 직전 문맥이 없다. 구간이 시간 기준 32등분이라 곡마다 구간 길이가 달라(130초 곡이면 약 4초) 곡 사이 비교가 안 된다 | trace |
| 체력 곡선과 패턴 | 꺾임 `{from_ms, to_ms, drop}`(`:276-295`), 구간 끝 게이지 값, 코치 digest 의 꺾임 구간 + 그 구간 상위 태그(`replay-coach.service.ts:164-185`) | 판 하나 안에서 코치가 보여 주는 정도에 그친다. 꺾임이 어떤 패턴·밀도에서 나는지의 사용자 단위 통계가 없다. 꺾임 임계(15%/4초)가 고정이다 | trace |
| 속도(녹색 숫자)·BPM 대비 패턴 | 판정 습관의 녹색 50 단위 칸별 `fast_share`, `ex_rate`(판 단위, `judge-habit.service.ts:247-314`) | 패턴·밀도와 속도를 교차한 지표가 없다. 판 요약은 속도 변경 횟수와 바뀐 필드 이름만 남기고 시각·값은 버린다(`trace-analysis.ts:191-196`) | trace |
| GOOD/BAD 타이밍 | 오차는 PG·GR 만(`:203`, `timing`) | GOOD·BAD 의 오차(너무 이른/늦은 정도)는 요약에 없다. 등급별 빠름/늦음 개수만 점수 열에서 온다(`egd, lgd, ebd, lbd`) | trace |
| LN 품질 | `ln_head`, `ln_end`(CN·HCN 만), `ln_hold_hit`, `ln_release_then_hit` 태그의 EX 잔차 | 일반 LN(끝 판정 없음)의 놓는 타이밍은 측정 불가(kind 2 는 state 0 이라, `dto/trace.ts:34`) | 해당 없음(데이터가 없음) |
| 피로·지구력 | 앞/뒤 1/3 EX 차 하나 | 곡 길이·밀도에 따른 감쇠 곡선, 구간 사이 추세가 없다 | trace |
| 모드 | 7키 SP 만 | 5K·14K·9K 는 `metrics` null, 곡별 패턴도 7K 뿐 | trace(다른 모드 노트 레인 규칙은 미구현) |

추가로 확인한 한계
- 판 안 구간이 "시간 32등분"이라 노트가 없는 인트로·아웃트로 구간이 구간 수를 차지하고, 곡의 음악적 단락(마디)과 맞지 않는다. 곡별 패턴의 `section`(마디 위치)은 저장되지만 판 분석은 쓰지 않는다.
- 판 분석 노트는 `trace.notes` 중 `kind` 가 LN 끝(2)·CN 끝(3)인 객체까지 포함해 분류하므로(`playNotesOf :48-60`) LN 곡에서 "치는 노트"의 정의가 곡별 특징과 같은 규칙(`toPatternInput`)을 따르는지는 trace 의 `lntype`·`declared_ln` 처리 차이에 달렸다. 곡별은 `declared_ln` 으로 CN/HCN 여부를 정하고(`chart-features.ts:37`) 판은 trace 의 `kind` 3 이 CN/HCN 끝이다. 두 규칙이 같은 결과를 내는지 이번 조사에서 대조하지 않았다(확인하지 못함).
- 약점의 방향성: "약점"은 EX 비율 잔차(음수)만이다. BP(미스) 위주의 약점, 시간대별 약점, 조합 약점은 정의하지 않는다.

### 5.2 (b) 한 곡 분석에 머물고 사용자 전체 프로필로 집계되지 않는 부분

확인한 사실
- 사용자별 영속 프로필이 없다. 리포트·게이지 조언은 요청 때마다 최근 100판(`REPORT_RECENT_PLAYS`, `GAUGE_ADVICE_RECENT_PLAYS`)의 `play_analysis` 요약을 읽어 합치고, 판정 습관은 최근 100판(`JUDGE_HABIT_RECENT_PLAYS`)의 점수 행에 있으면 그 판의 trace 분석을 붙인다(`compose/ai-score-features.ts:206-219`). 읽는 곳: `compose/trace.ts:226-232`. 장기 추이(주 단위 잔차 변화, 약점의 개선 속도)는 어디에도 저장·계산되지 않는다. 비슷한 것은 주간 요약의 "앞 주 대비" 합계와 `avgjudge` 앞·뒤 절반뿐이다.
- 최근 100판 중 7키 지표가 있는 판만 표본이다(`measured`). 한 곡을 여러 번 친 경우 그 곡의 패턴이 표본에서 크게 차지하는 편향을 보정하지 않는다(곡 단위 가중 없음).
- 약점이 다른 기능으로 이어지지 않는다. 리포트의 `practice_focus` 는 자유 문장이고(`dto/ai-play.ts:62`), 연습 폴더의 태그는 사용자가 수동 선택(§4.6), 추천(§4.5)·목표(§4.7)는 램프·레벨 기준이다. 즉 "약점 → 훈련 대상 선택"을 기계가 이을 구조적 값(예: 태그·밀도 범위 목록)이 출력에 없다.
- 코치·비교는 판 하나 단위, 비교는 같은 곡의 두 판 단위다. 서로 다른 곡 사이의 같은 패턴 비교는 리포트의 태그 잔차뿐이다.
- 라이벌 데이터는 구간 합계만 공유된다(옵트인, 서로 라이벌, `trace.service.ts:417-453`). 사용자 집단의 분포(같은 레벨 평균 등)는 없다.
- 점수 기반 기능(5, 6, 7, 10)은 trace 를 쓰지 않는다(`docs/web/architecture.md` 12절 머리 문장). 판 분석 기능(1, 2, 3, 8)과 점수 행·표 기록 기능이 서로 독립이다. 판정 습관(4)만 점수 행과 trace 를 함께 읽는다.

### 5.3 (c) 표본이 적을 때

- 태그: 300노트 && 5판 미만이면 `insufficient`(판정·AI 지목 금지), 이후 윌슨 밖이면 `strong`. AI 는 `strong` 만 강점·약점으로 말하고 나머지는 이름만 받는다(`play-report.service.ts:181-202`). 이 구조는 잘 막혀 있다.
- 구멍(확인한 사실): 레인(§3.8), 밀도 칸, 피로, 타이밍 평균, 옵션 비율에는 등급이 없고 digest 에 그대로 들어간다. 판정 습관·게이지 조언에는 표본 하한(5판/10판)만 있고 개별 항목 등급은 일부(FAST 비율, 오프셋, trace 타이밍, avgjudge)에만 있다.
- 표본 수는 "판 수·노트 수"로만 센다. 서로 다른 곡 수, 서로 다른 날짜 수, 레벨 분포(쉬운 곡만 친 표본인지)를 보지 않는다. 같은 곡 반복이 `MIN_TAG_PLAYS 5` 를 채울 수 있다.
- 새 지표를 만들 때의 기존 관례: 등급 3단계 + AI 에는 등급만 + 부족하면 AI 호출 자체를 막고 `need` 코드를 돌려준다(`notEnoughData`, `ai-digest.ts:95`). 새 `need` 는 `AI_DATA_NEEDS`(`entities/ai.type.ts:220-233`)와 화면 문구, 계약 테스트(`tests/components/ai-score-features-contract.test.ts:33-` "표본 부족 이유마다 문구가 있다")를 같이 고쳐야 한다.

### 5.4 (d) 표 곡 전체의 패턴 커버리지

- 수집 경로와 현황은 §2.2. 요약: 7K 전체 7,903곡이 유지보수자 라이브러리에서 2026-10-20 에 한 번 올라갔고 발광 표 곡은 모두 있었다. 이후 자동 수집은 없다.
- 격차: (1) 표는 `insane1` 하나뿐이다. (2) 업로더가 관리자 한 명이라 다른 사용자가 가진 곡·새로 나오는 곡은 패턴이 없다. (3) 5K·14K·9K 는 패턴 자체가 없다. (4) 파서 세대(`PARSER_VERSION`)가 바뀌면 재추출·재업로드가 필요하다(`chart-pattern.service.ts:58-59` 의 동일성 판정이 `parser_version` + 내용 해시). (5) 커버리지 점검 API 는 표 곡 기준이라 표 밖 곡이 얼마나 비는지는 서버가 보고하지 않는다.
- 확인하지 못함: 운영 `chart_pattern` 현재 행 수·`stale_features`, 표 밖 곡 중 사용자가 실제로 많이 치는 곡의 패턴 보유율, 사용자별 trace 보유 판 수.

---

## 6. "훈련곡 생성"에 재사용할 수 있는 것

### 6.1 결정론 결과를 입력으로 하는 구조

확인한 사실
- 공통 골격: 결정론 계산 → `key` 달린 digest → LLM(구조화 출력) → `refine`(digest 에 없는 키·범위를 벗어난 값 제거) → `ai_result` 캐시. 구현은 `ai-digest.ts`(`runAiFeature`, `readStoredInterpretation`, `collectDigestKeys`, `fitsLine`, `cutText`)와 `feature-runner.service.ts`(`run :93-165`).
- 가장 가까운 선례는 기능 9다. LLM 이 작은 구조화 계획(필터 목록)만 내고, 서버가 그 계획을 검증·컴파일해 결정론으로 실행한다(`record-query.service.ts:97-110,132-157`, `record-query.compiler.ts`). 훈련곡 생성도 LLM 은 파라미터(패턴 종류, 노트/초 목표, 길이, 스크래치·LN 비율 등)만 내고 서버가 차트를 합성하는 구조가 같은 모양이다. 이것은 설계 제안이 아니라 기존 구조와의 대응을 적은 것이다.
- 입력으로 쓸 수 있는 결정론 산출물: `PlayReport`(태그 잔차·등급, 레인, 밀도, 피로, 타이밍)·`play_analysis.summary`(32구간, 태그별 집계, 꺾임)·곡별 `features`(태그 비율, 밀도 3값, 소프란, LN·스크래치 비율). 훈련곡 쪽에서 더 필요한 형식(노트/초 분포, 태그 인스턴스 길이)은 §5.1 에 적었다.
- 새 판 지표를 추가하는 길: `computePlayMetrics`(또는 `analyzeTrace`)에 필드 추가 → `PLAY_ANALYSIS_VERSION` 올림 → 저장 요약은 읽을 때 최대 10판씩 다시 계산, 나머지는 `POST /api/admin/traces/recompute` 로 따라잡는다(`trace.service.ts:169,262-287,473-489`). trace 원본은 서버 안(`recomputeStored`, `trace.service.ts:239-255`)에서만 풀린다. 곡별 특징은 `computeChartFeatures` 수정 + `ANALYSIS_VERSION` 올림 + `POST /api/admin/chart-patterns/recompute`.
- 같은 digest·프롬프트·스키마·공급자·모델이면 캐시된 결과를 쓰고 공급자 호출·사용량 가산이 없다(`feature-runner.service.ts:100-114`). 캐시 키는 `hashAiRunInput`(digest, system, schemaName, analysisVersion, promptVersion, maxOutputTokens, effort, provider, model, `:47-58`).

### 6.2 LLM 호출 어댑터 (BYOK, 공급자 3곳, 구조화 출력)

확인한 사실
- 계약 `AiProviderAdapter`(`service/shared/ai-provider/types.ts:55-61`): `validateKey`, `listModels`, `generateStructured({apiKey, model, system, input, schemaName, schema(Zod), maxOutputTokens, effort?, signal})` → `{ok:true, data, usage{inputTokens, outputTokens, cachedInputTokens}, requestId}` 또는 실패(`kind`, `requestId`, `httpStatus`, `providerErrorType`, `retryAfterSeconds`). 공급자 3개는 `service/shared/ai-provider/index.ts:7-13`.
- 직접 `fetch` 구현이고 새 의존성이 없다(ADR Q2). 공급자 호출은 DB 연결을 잡지 않은 상태에서 한다(`feature-runner.service.ts:117-137`). 호출 제한 시간 240초(`AI_CALL_TIMEOUT_MS`, `:11`), 키 확인·모델 목록 15초(`credential.service.ts:6`).
- Anthropic(`anthropic.ts:82-165`): `POST https://api.anthropic.com/v1/messages`, `output_config.format = {type:'json_schema', schema}`(`:126`), `stop_reason` 이 `refusal`→`refused`, `max_tokens`·`model_context_window_exceeded`→`truncated`(`:147-149`). `strictness: native`. 모델 목록은 `/v1/models` 의 `structured_outputs.supported === false` 인 것을 거른다(`:109`).
- OpenAI(`openai.ts:88-173`): `POST /v1/responses`, `text.format = {type:'json_schema', strict:true, name, schema}`, `store:false`(`:115-130`), `incomplete`→`truncated`(내용필터면 `refused`). `strictness: native`. 모델 목록은 구조화 지원 여부를 알 수 없어 `structured: null`.
- Ollama Cloud(`ollama.ts:85-152`): `POST https://ollama.com/api/chat`, `stream:false`, 스키마는 시스템 프롬프트에 JSON 으로 붙인다(`buildSystemPrompt :41-42`), `options.num_predict = maxOutputTokens`. 응답은 `<think>` 제거 → 통째 파싱 → 코드 펜스 → 최상위 `{…}` 후보 순으로 느슨하게 파싱(`parseLenientJson :74-83`). `strictness: best_effort`. 문서상 Ollama Cloud 는 구조화 출력을 지원하지 않는다(저장 문서 근거, `providers.md` 3절).
- 모든 응답은 원래 Zod 스키마로 다시 검증한다(`result.ts:22-34` `finalizeStructuredOutput`). 공급자 스키마에서 빠진 길이·범위 제약이 여기서 걸린다.
- 구조화 출력 한계(`json-schema.ts`): 허용 키워드 `type, properties, required, additionalProperties, items, anyOf, enum, description, format(날짜 등), minItems(0·1)`(`:22-33`). 제거되는 키워드: `minimum/maximum/multipleOf/minLength/maxLength/maxItems/uniqueItems/pattern/default` 등(`:34-49`). 금지: 널 허용 유니온(`type` 배열에 null, anyOf 의 null 가지, `:125-131`), 객체의 `additionalProperties` ≠ false, 모든 속성이 required 아닌 객체, 루트가 객체가 아니거나 anyOf, enum 의 비원시 값. 상한: 깊이 10, anyOf 16개, 속성 5,000개, enum 값 1,000개(`:7-12`). 즉 스키마에서 `.nullable()`/`.optional()` 을 쓸 수 없고 배열 길이·수치 범위는 Zod 후처리로만 강제된다.
- 어댑터는 `effort` 를 보내지 않는다(`types.ts:40-44` 주석: Anthropic 일부 모델이 `output_config.effort` 에 400 을 내는데 모델별 지원 플래그가 저장돼 있지 않음). 스트리밍 없음. 자동 재시도 없음(ADR Q10, `feature-runner.service.ts` 주석).
- 키 보관: AES-256-GCM 봉투(DEK 를 KEK 로 감싸고 AAD 에 사용자·공급자·KEK 버전을 묶음, `secret-box.ts:6-12,28-30`), KEK 는 환경변수 `AI_CREDENTIAL_KEKS`·`AI_CREDENTIAL_KEK_CURRENT`(`lib/env.ts:25-26`). 평문 키는 `withApiKey` 콜백 안에서만 쓰이고(`credential.service.ts` `withApiKey`) 로그·응답에 나가지 않는다.
- 출력 토큰 예산 선례: 6,000(비교·게이지 조언), 8,000(코치·추천·연습·목표·주간·판정 습관), 12,000(리포트), 16,000(기록 질문). 입력은 `JSON.stringify(digest)` 한 덩어리이고(`feature-runner.service.ts:125`) 크기 상한 검사는 없다. digest 의 크기는 각 기능이 개수 상한(`*_TOP_ITEMS`, 후보 40/60곡 등)으로 제한한다.

### 6.3 사용량·레이트리밋

확인한 사실
- 한도(`dto/ai.ts:39-54`): 하루 호출 기본 30(설정 상한 1,000), 동시 실행 기본 1(상한 5), 월 토큰 상한 기본 없음(사용자가 설정하거나 운영자 기본). 값은 운영자(관리자 역할)가 화면에서 바꾼다(`PUT /api/admin/ai/limits`, `route/ai.ts:238-241`).
- 확보 순서(`usage.service.ts:90-106`): 월 토큰 상한(읽기만) → 동시 실행 슬롯(`ai_inflight`, 조건부 쓰기, 보유 만료 300초 `AI_RUN_HOLD_MS`) → 그날의 호출 수(`ai_usage`, 조건부 쓰기). 하나라도 실패하면 앞 단계가 가져간 것을 돌려준다. 호출이 끝나면 토큰(입력·출력·캐시)을 그날 행에 더한다(`feature-runner.service.ts:132`).
- 캐시 적중은 호출 수·토큰을 세지 않는다. 공급자를 바꾸거나 모델을 바꾸면 해시가 달라 다시 호출한다.
- HTTP 레이트리밋: 인스턴스 메모리 고정 창 카운터(`with-rate-limit.ts:16-33`). AI 기능 GET·POST 모두 사용자별 `scope:userId` 키로 분당 30회(`RATE_LIMIT.TABLE_RECORDS`), 키 등록·검증·모델 목록은 분당 5회(`RATE_LIMIT.AUTH`, `route/ai.ts:171`), 설정 쓰기 30회. 다중 인스턴스에서는 best-effort 이고 콜드스타트마다 초기화된다(저장 문서 근거, `docs/web/architecture.md` 알려진 문제). 일별·월별 한도는 DB 카운터라 정확하다.
- 의미: 생성형 호출 하나가 "AI 해석" 1회와 같은 취급이라 기본 하루 30회 안에서 쓴다. 훈련곡 생성이 호출을 여러 번 쓰는 설계면 한도 안에 들어가는지 따로 따져야 한다.

### 6.4 폴더 연동 (훈련곡 → 내 폴더 → 구동기)

확인한 사실
- AI 연습 폴더가 쓰는 서비스는 `folderService`(`create`, `replaceItems`, `get`, `list`, `remove`)이고 HTTP 를 모른다(`practice-folder.service.ts:96`). 항목은 `{md5, sha256:'', title, artist, note}`이며 곡 식별자는 md5(소문자 32자 16진)이다(`:285-292`, 한도: 폴더 50개, 폴더당 2,000곡, 요청당 500곡, 이름 64자, 메모 100자 — 저장 문서 근거 `server.md` 1.3, `dto/folder.ts:6-19`).
- 클라이언트는 서버 폴더를 받아 MY FOLDERS 에 반영하고 가진 곡만 줄로 보이며 `(가진 곡/전체)` 로 표시한다(저장 문서 근거, `server.md` 4.1). AI 가 만든 폴더는 다음 새로 받기에서 자동으로 보인다.
- 서버가 곡(차트) 파일을 보관하거나 사용자에게 내려 주는 경로는 없다. 곡 미러(R2 업로드, 서버 내려받기)는 결정 17 로 보류되어 설계 문서만 있다(`docs/HANDOFF.md` 4절 '곡 미러' 행, `docs/plan/2026-10-23-song-mirror-prep.md` 머리 문서: "이 문서는 업로드를 하지 않는다"). 클라이언트에는 "표에는 있는데 라이브러리에 없는 곡"을 표 항목의 직접 zip 링크로 받아 DOWNLOAD FOLDER 에 풀고 스캔하는 기능이 있다(`GET CHART`, `docs/play-options.md` 13절, `apps/rbms-player/src/download.rs`·`download/`). 대상은 표 항목이다. 추정: 사용자 폴더 항목만 있는 생성 차트는 표 항목이 아니므로 이 받기 경로를 타지 못한다. 폴더 항목에서 같은 받기를 하는지는 코드로 확인하지 않았다(확인하지 못함). 그래서 생성된 차트가 폴더 항목으로 보여도 파일이 사용자 기기에 없으면 플레이할 수 없다.

### 6.5 기능 하나를 더할 때 건드릴 파일 체크리스트

기존 기능 구현에서 추린 것이다(예: 기능 8 게이지 조언이 가장 작은 판 분석 기능). 새 표가 필요하면 7번을 추가한다.

서버
1. 출력·뷰 스키마: `web/src/server/dto/ai.ts` 또는 `dto/ai-play.ts`에 LLM 출력 Zod(§6.2 의 공급자 부분집합 준수)와 화면용 뷰 스키마, 한도 상수, 필요한 `need` 코드.
2. 서비스: `web/src/server/service/domain/ai/<기능>.service.ts` — `AI_<기능>_FEATURE`(≤32자, `ai_result.feature` 가 `varchar(32)`, `schema.ts:581`), `PROMPT_VERSION`, `MAX_OUTPUT_TOKENS`, `EFFORT`, 시스템 프롬프트 빌더(언어 인자 `Locale`, `DIGEST_IS_DATA_RULE`·`DIGEST_KEY_RULE` 포함), `AiFeatureSpec`, digest 빌더, `refine*Output`, `view`(공급자 호출 없음)·`interpret`(`notEnoughData` 또는 `runAiFeature`). 캐시 `subject_key`(≤128자, `schema.ts:582`) 규칙 정하기.
3. 상수: 지표 임계값이면 `ai/play-feature.constants.ts` 또는 `ai/score-feature.constants.ts`(값을 바꾸면 digest 가 바뀌어 캐시 키가 바뀐다). 분류 정의 자체는 `analysis/analysis-constants.ts` + 버전 올림.
4. 새 판 지표라면: `analysis/play-metrics.ts` 또는 `trace/trace-analysis.ts` 수정 + `PLAY_ANALYSIS_VERSION` 올림 + `tests/service/analysis/{play-metrics,trace-analysis}.test.ts`. 곡별 특징이면 `analysis/chart-features.ts` + `ANALYSIS_VERSION` 올림 + 관리자 재계산 실행.
5. compose: `web/src/server/compose/ai-play-features.ts`(판 분석 기능, `composeAiPlayFeatures :142-159`) 또는 `compose/ai-score-features.ts`(점수·표 기능, `composeAiScoreFeatures :267-288`)에 서비스 생성과 DB 어댑터 추가. 새로 노출하는 서비스는 `compose/index.ts:46-65` 스프레드 반환에 이미 포함되는지 확인. 공통 러너(`aiFeatureRunner`)는 `compose/ai.ts:297-335`.
6. 라우트: `web/src/server/route/ai.ts` 의 해당 팩토리(`createAiScoreFeatureRoutes :326` 또는 `createAiPlayFeatureRoutes :465`)에 `<기능>View`/`<기능>Interpret` 핸들러(`withLimited(scope, …)`, `unwrapAiResult`, `readLocale`), `AiScoreFeatureRouteDeps`/`AiPlayFeatureRouteDeps` 타입에 서비스 `Pick` 추가. 어댑터 파일 `web/src/app/api/fe/me/ai/<경로>/route.ts` 신설(`export const maxDuration = 300`, GET/POST 연결, 예: `report/route.ts`).
7. (새 표가 있을 때) `web/src/server/db/schema.ts` + `bun run db:generate` 로 `drizzle/00NN_*.sql` 과 `drizzle/meta/` 커밋. `drizzle-kit push` 금지. 운영 순서: 마이그레이션 파일만 먼저 푸시 → `web-db.yml status` → `migrate` → `status` → 코드 푸시(`docs/HANDOFF.md` 5절, `docs/web/architecture.md` 3-2). 운영 `migrate` 는 사용자 `!` 실행이 필요할 수 있다(`docs/HANDOFF.md` 5절 "운영 반영").
8. 새 오류 코드가 필요하면 세 파일: `lib/error-code.ts`, `lib/error-message.ts`, `lib/error.ts` 의 `STATUS_MAP`. 기존 `AI_NOT_ENOUGH_DATA`(422) 등을 재사용하면 불필요.

클라이언트(웹)
9. 타입·상수: `web/src/entities/ai.type.ts`(응답 타입, 라벨, `AI_HUB_TABS`·`AI_HUB_TAB_LABEL` 탭 추가, `AI_DATA_NEEDS`), `web/src/entities/ai.query.ts`(`queryOptions` 팩토리, `AI_DIGEST`·`AI_FEATURE_PATH` 항목, `useInterpret…` 변이는 공통 팩토리가 AI 사용량과 그 기능 digest 를 무효화). 쿼리 키는 `shared/constants/query-key.ts` 의 `QUERY_KEY.AI.DIGEST(feature, params)` 를 재사용.
10. 화면: `web/src/widgets/ai/ai-<기능>-panel.tsx`(비즈니스 로직은 widgets 레이어), 순수 조각은 `web/src/features/ai/*`, 탭 연결은 `widgets/ai/ai-hub-tabs.tsx`, 판별 페이지는 `app/(app)/ai/plays/[scoreId]` 에 패널 추가(`ai-play-detail-content.tsx`). 새 페이지는 `proxy.ts` 의 `/ai/:path*` matcher 안이면 세션 보호가 이미 걸려 있다.
11. 문구: `web/messages/{ko,en,ja}/ai.json`(및 필요 시 `pages.json`, `server.json`) 세 언어 모두. HANDOFF 5절 규칙: 새 사용자 표시 문구는 처음부터 세 언어 카탈로그로. 일본어는 모델 초안 + 감수 요청 상태.
12. 테스트: `tests/service/ai/*`, `tests/service/analysis/*`, `tests/route/ai-*.test.ts`, 화면 `tests/components/ai-*.test.tsx`, 서버·화면 상수 일치 `tests/components/ai-score-features-contract.test.ts`·`ai-contract.test.ts`(화면 쪽 `AI_PATTERN_TAG_LABEL`, 게이지·옵션 이름, 탭 목록이 서버 상수와 일치해야 통과). 웹 게이트: `cd web && bun run verify && bun run format:check && bun run build`(`docs/HANDOFF.md` 7절).
13. 문서: `docs/web/architecture.md`(12~14절 형식), `docs/backend/api-spec.md`(경로·코드), 결정은 `docs/acknowledge/` ADR, 검증 항목은 `docs/quality-assurance/`.
14. 규칙: 화살표 함수만, `any`·`enum`·코드 주석 금지, FSD 단방향, barrel 금지, `useCallback`/`useMemo` 금지, 새 의존성 금지(HANDOFF 5절 웹 규칙과 `web/AGENTS.md`: Next 16 문서 확인).

---

## 7. 확인하지 못한 것

1. 운영 데이터의 양: `play_trace`·`play_analysis` 행 수, trace 를 가진 사용자 수와 판 수, 사용자별 최근 100판 중 7키 판 비율, `chart_pattern` 현재 행 수와 `stale_features`. 모두 운영 접근이 필요하다.
2. 표 밖 곡(사용자가 실제로 많이 치는 곡)의 패턴 보유율과, 사용자가 가진 곡 중 7K 이외 모드의 비중.
3. 곡별 `declared_ln` 처리와 trace `kind` 3(CN·HCN 끝) 처리의 규칙 차이가 태그 결과를 다르게 만드는지(§5.1 추가 한계 둘째 항목).
4. `stream` 태그가 `LAYOUT_FREE_TAGS` 에 있는 것이 의도인지(§2.3 추정).
5. `Converge` 옵션 판이 실제로 생기는지(엔진 `NoteOption` 에 없음).
6. 윌슨 구간이 상관된 노트를 얼마나 좁게 보는지(통계적 영향의 크기)와 태그 19개 동시 검정의 위양성률.
7. 클라이언트가 사용자 폴더 항목(표 항목이 아닌 곡)에 대해 `GET CHART` 류 받기를 지원하는지(표 항목만 대상이라는 것은 `docs/play-options.md` 13절로 확인, 폴더 쪽 코드는 확인 안 함).
8. 생성 차트를 클라이언트가 읽으려면 필요한 BMS 파일의 음원(`#WAV`) 처리 방식. 곡별 패턴 문서는 키음 정보를 담지 않는다(`document.rs`).

## 8. 열린 질문 (사용자 결정이 필요한 것)

1. 훈련곡이 "기존 라이브러리 곡 선별"인지 "새 BMS 파일 합성"인지에 따라 필요한 지표 형식이 갈린다(이 문서는 둘 다 가능한 격차를 정리했다). 합성이라면 파일 전달 경로(곡 미러 재개 여부)가 먼저다.
2. 새 판 지표(밀도 한계, 태그 인스턴스, 손 단위)를 trace 에서 소급 계산해 모든 사용자의 기존 판에 적용하는 것을 허용할지(`PLAY_ANALYSIS_VERSION` 올림 + 관리자 일괄 재계산은 호출당 최대 200판, `dto/trace.ts:178`).
3. 곡별 특징을 확장해 훈련곡 파라미터와 같은 단위(노트/초 분포 등)로 만들 때 표 1,033곡 전체의 `chart_pattern` 재계산(저장된 노트로 서버에서 가능, 재업로드 불필요)을 해도 되는지.

---

## 부록 A. 원본 재조회 지도

| 알고 싶은 것 | 파일 |
|---|---|
| 태그 정의·분류 | `web/src/server/service/domain/analysis/pattern-tags.ts` |
| 모든 임계값 | `…/analysis/analysis-constants.ts` |
| 곡별 특징 | `…/analysis/chart-features.ts` |
| 판 지표·게이지 | `…/analysis/play-metrics.ts` |
| 표본 등급 | `…/analysis/sample.ts` |
| trace 분석·패턴 join | `web/src/server/service/domain/trace/trace-analysis.ts` |
| trace 일관성 검사 | `…/trace/trace-check.ts` |
| trace 저장·재계산·라이벌 구간 | `…/trace/trace.service.ts`, `web/src/server/compose/trace.ts` |
| trace 형식 | `web/src/server/dto/trace.ts`, Rust `crates/rbms-ir/src/dto/trace.rs` |
| 곡별 패턴 형식·저장·커버리지 | `web/src/server/dto/chart-pattern.ts`, `…/chart-pattern/chart-pattern.service.ts`, `web/src/server/compose/chart-pattern.ts`, `apps/rbms-cli/src/chart_pattern/*`, `web/scripts/upload-chart-patterns.ts` |
| 러너·캐시·한도 | `…/ai/feature-runner.service.ts`, `…/ai/usage.service.ts`, `…/ai/settings.service.ts`, `…/ai/result.service.ts`, `web/src/server/dto/ai.ts` |
| 공급자 어댑터 | `web/src/server/service/shared/ai-provider/{anthropic,openai,ollama,json-schema,result,http,types,index}.ts` |
| 기능 10개 | `…/ai/{play-report,replay-coach,play-compare,judge-habit,recommend,practice-folder,goal-plan,gauge-advice,record-query,weekly-summary}.service.ts` + `table-progress.ts`, `play-digest.ts`, `ai-digest.ts` |
| 라우트 | `web/src/server/route/ai.ts`, `web/src/app/api/fe/me/ai/**/route.ts`, `web/src/server/route/trace.ts` |
| 화면 | `web/src/app/(app)/ai/**`, `web/src/widgets/ai/*`, `web/src/features/ai/*`, `web/src/entities/ai.{type,query}.ts` |
| 표 | `web/src/server/db/schema.ts:522-735` |

---

## 구현 결과(2026-11-04)

설계 [docs/plan/2026-11-04-ai-training-dashboard.md](../../plan/2026-11-04-ai-training-dashboard.md) 2절(분석 고도화)과 결정 D1~D9·M1~M33 대로 구현했다. 아키텍처 정본은 [web 아키텍처 §17](../../web/architecture.md), 결정은 [약점 프로필 통계 ADR](../../acknowledge/2026-11-04-adr-weakness-profile-statistics.md)·[분석 v3와 재계산 ADR](../../acknowledge/2026-11-04-adr-analysis-v3-recompute.md). 위 5절의 격차가 어디서 닫혔는지:

| 이 문서의 격차·항목 | 구현된 곳(`web/src/server/service/domain/` 아래, 달리 적지 않으면) |
|---|---|
| 5.1 국소 밀도·밀도 한계(고정 1초 창 경계 분할) | `analysis/local-density.ts`(중심 2초 창 NPS, 문맥은 1초·8초 창의 비로 burst/sustained), `analysis/weakness-profile.ts` 의 `density`(NPS 2칸 등위회귀로 편안한 밀도·EX 한계·BP 한계), 화면 `AiDensityLimitChart` |
| 5.1 태그별 오차 분포·손 편향 | `analysis/play-metrics.ts` 의 `timing_by_tag`·`by_hand`·`timing_by_hand`·`lane_bp`, 판정 습관 `tag_timing`·`hand_timing` |
| 5.1 실패 구간·게이지 꺾임의 문맥 | `analysis/bp-runs.ts`(`bp_runs`), `play-metrics.ts` 의 `gauge_drop_contexts`·`gauge_drop_tag_totals`·`gauge_drop_notes` |
| 5.1 태그 조합 | `analysis/tag-combos.ts`(`by_combo`·`by_tag_solo`, 곡 특징의 `combos`) |
| 5.2 사용자 프로필 집계 | `analysis/weakness-profile.ts`·`profile-stats.ts`(곡 군집 t 검정, Benjamini–Hochberg), `ai/weakness-profile.service.ts`, 캐시 표 `play_profile`(마이그레이션 0014), `GET /api/fe/me/ai/profile` |
| 5.3 표본이 적을 때 | 셀마다 `sample_level`(`high`/`medium`/`low`/`insufficient`)과 근거 문구, AI 에는 `strong` 만 |
| 5.4 표 곡 전체의 커버리지 | 곡 특징 `ANALYSIS_VERSION` 2 와 `analysis-recompute-count`·`analysis-recompute`(운영 재계산, 사용자 `!`) |
| 6 훈련곡 생성에 재사용 | `ai/training-plan.service.ts`(기능 `training_plan`), `training/*`(합성기·BMS 작성기·zip), `route/training-charts.ts`. [02 문서의 구현 결과](02-training-chart-delivery.md) |

- **버전**: `ANALYSIS_VERSION` 1 → 2, `PLAY_ANALYSIS_VERSION` 2 → 3(3.7절), `PLAY_FEATURE_ANALYSIS_VERSION` 1 → 2, `PROFILE_VERSION` 1(`ai/play-feature.constants.ts`). 새 곡 특징은 `nps`(2초 창 중앙값·90분위·최대, 8초 창 최대, 지속 구간 초)·`combos`·`bpm_main_band`. 새 판 지표 필드는 [ai-features/server.md 부록 D](../ai-features/server.md).
- **기능 10개의 변경(4절)**: 모두 약점 프로필 키와 새 지표를 받도록 이었고 새 응답 필드는 전부 선택 필드라 옛 화면·시험과 호환된다. 프롬프트 버전: 리포트 v3·코치 v2·비교 v3·판정 습관 v3·게이지 조언 v2, 추천·연습 폴더·목표·기록 질문·주간 요약은 2 → 3.

| 기능 | 달라진 점(요약) |
|---|---|
| 1 리포트 | 입력에 프로필, 보기에 `profile_keys` |
| 2 코치 | 분석에 `bp_runs`·`profile_keys`·`drop_contexts`, 구간마다 `peak_nps` |
| 3 비교 | 구간마다 `peak_nps`·`mean_err_ms` |
| 4 판정 습관 | `tag_timing`·`hand_timing`(편차 ms 순) |
| 5 추천 | 약점이 일치하면 점수에 +1.0, 밀도 한계의 1.25배를 넘는 곡은 −1.0, 후보에 `focus_keys`(비율 0.1 이상), 보기에 `profile_keys`. 경계 ±1 레벨 전체의 곡 특징을 먼저 불러온 뒤 순위를 매긴다(예전에는 상위 40곡만) |
| 6 연습 폴더 | 출처 `weakness`(+`focus_key`), 폴더 이름 접미사 `약점 연습 / Weakness practice / 弱点練習` |
| 7 목표 | 종류 둘 추가(`weakness_residual`, `density_limit`), 입력과 저장 스키마를 나눴다(`aiGoalParamsSchema` / `aiGoalStoredParamsSchema`: 기준값 `baseline` 은 서버가 잡는다), 진행에 `current`·`ratio` |
| 8 게이지 조언 | `drop_contexts` |
| 9 기록 질문 | 필드 `tag`·`training` |
| 10 주간 요약 | `training`(그 주 만든 훈련곡·판·곡)·`focus`(프로필 약점 상위 3 의 주간 값) |

### 설계와 달라진 점

- **국소 NPS 창의 끝**: 설계 2.2.1 은 `[0, 판 길이]` 였다. 판 길이는 마지막 노트 + 5초(`PLAY_TIME_MARGIN_MS`)라 곡 끝 노트의 밀도가 최대 절반으로 낮게 읽히므로, 판과 곡 특징 모두 `[0, 마지막 판정 노트 + 1 µs]` 로 자른다(`localDensitiesOf`). 대신 곡 끝의 외딴 노트는 조금 높게 읽힌다(혼자 있는 마지막 노트는 1 NPS).
- **소프란 사건**: `scroll ≠ 1` 이면 점마다 사건이 겹쳐 세지므로 앞 점과 스크롤 값이 달라진 점(시작값 1)만 사건으로 센다(`soflanEventsOf`). 분류 규칙은 바꾸지 않았다.
- **요약 크기**: 6만 노트 판의 요약이 30,577 B 였고(시험이 40 KB 예산을 단언) `PLAY_COMBO_TOP` 자르기는 넣지 않았다. 크기를 늘리는 것은 꺾임 수에 비례하는 `gauge_drop_contexts`(꺾임 하나에 약 175 B, 상한 없음)다. 무작위 레인에서 꺾임이 250개 나오게 만든 합성 게이지로는 80.8 KB 였다.
- **코치·비교의 구간 값**: 2.6 이 요구한 구간별 `peak_nps` 와 평균 오차가 저장 요약에 없어 판 요약의 구간에 `peak_nps`·`mean_err_us` 를 더했다(버전 3 이 아직 나가지 않아 버전 변경 없음). 라이벌 공유는 `id`·`chart_notes`·`notes`·`ex_score`·`bp` 만 고르므로 새 필드가 노출되지 않는다.
- **약점 키 예시**: 가장 긴 키는 `combo:ln_release_then_hit+scratch_mixed` 가 아니라 앞선 태그가 먼저인 `combo:scratch_mixed+ln_release_then_hit`(39자)이고, `solo:` 는 `COMBO_TAGS` 만 받는다.
- **셀 해석**: 방향(약점/강점)은 `timing:` 이 항상 약점, 문맥 셀(`bp_context`·`gauge_context`)은 로그 lift 가 0 보다 크면 약점, 나머지는 부호; 심각도는 단위를 하나의 척도로 맞추는 `SEVERITY_UNIT_SCALE`(ms 0.01, 로그 lift 0.1, EX·비율 1)을 쓰고, 문맥 셀의 lift 는 각 군집 개수에 의사 개수 0.5 를 더해 로그가 무한대가 되지 않게 한다. 문맥 셀의 표본 조건은 노트 30 이상·군집 5곡 이상·판 5 이상이다. 이 값들은 휴리스틱이라 실제 데이터로 확인해야 한다(테스트 부채).
- **밀도 곡선**: 칸당 노트 150·곡 3 이상만 쓰고(`density.curve` 는 쓰인 칸만), 쓰인 칸이 3 미만이면 `insufficient`. 전체 평균 NPS(`overall_mean_nps`)는 저장된 `by_nps` 칸 중앙값으로 추정한다(40+ 는 40).
- **지문**: 설계 2.5.1 의 입력 외에 그 사용자의 훈련곡 개수를 넣었다(훈련곡을 지우면 낡은 판의 `isTraining` 이 바뀌는데 다른 입력이 알아채지 못한다). 지문은 읽은 뒤 다시 계산해 저장한다(낡은 판 재계산이 `computed_at` 을 바꾸기 때문).
- **digest 크기 상한**: 기존 상수가 없어 `PLAY_DIGEST_MAX_CHARS`(리포트 12,000, 그 밖 6,000)를 새로 두고 모든 항목을 채운 시험이 단언한다.
- **`play_trace`·`play_analysis` 불변식**: 프로필 저장·지문 질의(`createWeaknessProfileDb`)는 두 표만 읽을 수 있는 `compose/trace.ts` 에 있다.

### 남은 것

- 5.1 의 데이터가 없는 항목은 그대로다: 일반 LN 의 놓는 타이밍, 7K 밖 모드(5K·14K·9K)의 분석. 다변량 모형(태그·밀도 교락의 분리)은 테스트 부채다.
- 운영 재계산 전 기간의 동작(읽을 때 낡은 판 최대 10개, 프로필은 `stale` > 0 이면 저장하지 않음)은 [web 아키텍처 §17-5](../../web/architecture.md)에 적었다.
