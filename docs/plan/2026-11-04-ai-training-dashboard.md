# AI 분석 고도화 · 훈련곡 생성 · 로그인 대시보드 통합 설계

> 작성 2026-11-04 · 브랜치 `dev` · 기준 HEAD `4bac699`. 이 문서는 설계서이며 코드를 바꾸지 않는다.
> 근거 조사: [01 현재 AI 분석](../reference/2026-11-04-ai-training-dashboard/01-current-ai-analysis.md), [02 훈련곡 전달](../reference/2026-11-04-ai-training-dashboard/02-training-chart-delivery.md), [03 대시보드](../reference/2026-11-04-ai-training-dashboard/03-dashboard.md), 기존 기획 [2026-10-19-web-ai-features.md](2026-10-19-web-ai-features.md).
> 경로는 저장소 기준이고 `web/` 안 파일은 `web/` 를 붙인다. "01 §3.4" 는 01 보고서 3.4절을 뜻한다. 구현 에이전트는 이 문서와 세 보고서만 보고 코드를 쓴다. 확인이 필요한 곳은 "구현 때 확인" 으로 적었고, 그곳에서만 원본을 읽는다.
> 적용 규칙: `docs/HANDOFF.md` 5절·7절(단일 출처), `web/AGENTS.md`(Next 16.3.4: `web/node_modules/next/dist/docs/` 를 읽고 쓴다), 전역 컨벤션(화살표 함수, `any`·`enum`·코드 주석 금지, JSDoc 영어만, FSD 단방향, barrel 금지, `useMemo`/`useCallback` 금지). Rust 는 `//` 주석 금지(`///`·`//!` 영어만), 매직넘버·새 `#[allow]`·`unsafe`·새 의존성 금지, rustfmt 160, `--locked`.

---

## 1. 목표·결정·비목표

### 1.1 사용자 요청(2026-11-04 원문 요지)

1. 사용자가 친 패턴을 분석해 못 치는 패턴, 못 버티는 밀도 수준 등을 체계적으로 찾아내고, 그것을 단련하는 1분짜리 곡을 생성한다.
2. 현재 AI 분석은 빈약하고 범위를 정확히 덮지 못한다. 최대한 고도화한다.
3. 로그인 사용자는 `/` 에서 대시보드를 본다. iidx-rank 처럼 보일 수 있는 내용을 최대한 보인다.

### 1.2 확정된 사용자 결정

| 번호 | 결정 |
| --- | --- |
| D1 | 생성 방식: 결정론 분석 결과(약점 프로필)를 입력으로 LLM 이 패턴 구성안을 만들고, 서버가 결정론 합성기로 BMS 로 바꾼다. LLM 은 기존 BYOK 어댑터 3곳(Anthropic, OpenAI, Ollama Cloud)을 쓰고, 키가 없으면 다른 AI 기능처럼 버튼을 막고 등록을 안내한다. |
| D2 | 산출물: 생성 → 사용자의 내 폴더 "훈련곡" → 구동기(`apps/rbms-player`)에서 바로 받아 플레이. 클라이언트 변경 포함(내 폴더에서 라이브러리에 없는 곡을 줄로 보이고 받기 모달을 연다). |
| D3 | 다운로드: 추측 불가한 무작위 id 경로의 공개 GET. 경로가 `.zip` 으로 끝난다(`DirectZip`, 02 §2.2). |
| D4 | 훈련곡 판: 랭킹·검색 포함 전부 반영, 제외 조건 없음. 모르는 md5 는 지금처럼 `chart` 행 자동 생성(02 §6.1). 훈련곡 판 결과는 분석에 다시 반영되고, 어느 훈련곡의 판인지 연결해 훈련 전후를 비교한다. |
| D5 | 소리: 노트 키음으로 짧은 클릭 wav 를 zip 에 넣는다. 서버 TypeScript 가 PCM 으로 만든다. 새 의존성 없음. |
| D6 | 분석 고도화 뒤 `PLAY_ANALYSIS_VERSION` 을 올리고 기존 판 전체와 발광 표 곡 특징값을 재계산한다. 운영 실행은 개수 확인 뒤 사용자 `!`. 읽기 count action 과 쓰기 action 을 분리한다. |
| D7 | 보관: 사용자당 훈련곡 20곡. 넘으면 생성 전에 직접 지우도록 안내(자동 삭제 없음). 지울 때 함께 사라지는 것과 남는 것의 개수를 보인다. |
| D8 | 클라이언트 변경 뒤 `release.yml` draft 까지 만든다(게시는 사용자). |
| D9 | 대시보드: 손님 홈은 그대로, 로그인 사용자에게만 `/` 를 대시보드로. 구성은 이 문서가 정한다(최대한). |

### 1.3 비목표

- 곡 미러(저작권 곡 재배포)는 계속 보류(결정 17). 훈련곡은 서버가 만든 합성 차트라 별개로 다룬다.
- 7키 SP(`BEAT_7K`) 밖 모드(5K·14K·9K)의 패턴 분석과 훈련곡 생성은 하지 않는다(01 §3.2 한계 유지).
- 일반 LN(끝 판정 없음)의 놓는 타이밍 측정은 데이터가 없어 하지 않는다(01 §5.1 LN 품질).
- 대시보드의 게시판·팔로우·e-amusement 대응은 없다(R-BMS 에 개념이 없다, 03 §4.4).
- 운영 DB·운영 사이트·`.env*` 를 에이전트가 읽거나 쓰지 않는다. 운영 반영은 10.3 순서대로 메인과 사용자가 한다.
- 차트 노트 배치와 사용자 trace 는 공개 저장소·docs 에 두지 않는다. 골든 픽스처(3.9)는 손으로 쓴 사양과 고정 시드로 만든 합성 차트뿐이며 어떤 사용자의 프로필·판에서도 만들지 않는다.

---

## 2. 분석 고도화 명세

### 2.1 원칙

1. 새 지표는 모두 trace 만으로 계산한다(01 §2.1: 노트별 `time_us, lane, kind, state, err_us` 와 게이지 열이 있다). 곡별 패턴은 join 판정과 곡 특징에만 쓴다.
2. 판 요약(`play_analysis.summary`)에는 **합칠 수 있는 형태**(개수·합·제곱합)만 저장한다. 평균·표준편차·검정은 사용자 프로필 집계에서 계산한다. 그래야 여러 판을 정확히 합칠 수 있다.
3. 밀도는 차트의 성질이므로 "판정 노트(도달 여부 무관)" 로 센다. 성능은 "판정을 받은 노트" 로 센다(현재 `computePlayMetrics` 의 구분, 01 §3.4 과 같다).
4. 표본 부족은 기존 관례(등급 3단계, AI 에는 등급만, 부족하면 AI 호출 안 함, 01 §5.3)를 따르고, 구멍(레인·밀도·피로 무등급)을 메운다.
5. 상수는 전부 `web/src/server/service/domain/analysis/analysis-constants.ts` 에 이름을 붙여 둔다(매직넘버 금지). 값을 바꾸면 버전을 올린다.

### 2.2 새 판 지표 (`computePlayMetrics` 확장)

입력 `PlayMetricsInput` 에 두 필드를 더한다.

```typescript
export type PlayMetricsInput = Omit<PatternInput, 'notes'> & {
    notes: readonly JudgedPlayNote[]
    emptyPoorTimesUs: readonly number[]
    lengthUs: number
    gaugeDropsMs: readonly { from_ms: number; to_ms: number }[]
    trainingSections?: readonly TrainingSectionSpan[] | null
}
export type TrainingSectionSpan = { index: number; kind: string; focus_key: string; from_us: number; to_us: number }
```

- `analyzeTrace` 의 인자도 `training?: readonly TrainingSectionSpan[] | null`(선택, 기본 `null`)로 더한다. 그래야 U2(W1) 가 시그니처를 바꿔도 지금 호출부 `trace.service.ts:192` 와 `tests/support/play-feature-fixture.ts:33` 가 그대로 컴파일된다. 훈련곡 구간을 실제로 넘기는 배선(`trace.service.ts`, `compose/trace.ts`)은 U7(W3) 이 한다(7.1).

공통 셀 타입(새 파일 `web/src/server/service/domain/analysis/play-cells.ts`):

```typescript
export type CompactCell = { notes: number; ex_score: number; bp: number; fast: number; slow: number }
export type TimingMoments = { n: number; sum_us: number; sumsq_ms2: number; fast_n: number; fast_sum_us: number; slow_n: number; slow_sum_us: number }
```

- `CompactCell`: `notes` 는 판정을 받은 노트 수(POOR 포함), `ex_score = 2·PG + GR`, `bp = BAD + POOR`, `fast`/`slow` 는 GREAT~BAD 중 `err_us >= 0` / `< 0`(지금 구간 집계 규칙과 같다, `play-metrics.ts:180-183`).
- `TimingMoments`: PGREAT~BAD 의 오차만(POOR 제외). `sumsq_ms2 = Σ (err_us / 1000)²`(크기를 줄이려 ms² 로). `fast_*` 는 `err_us >= 0`, `slow_*` 는 `< 0`. 저장 전 `sum_us` 는 정수 반올림, `sumsq_ms2` 는 `RATIO_DECIMALS` 자리 반올림.

#### 2.2.1 절대 밀도: 슬라이딩 창 NPS 와 판정 성능 (`local-density.ts` 신규)

- 판정 노트 시각 배열 `T`(정렬, `isJudgedNote` 인 것, 판정 여부 무관)를 두 포인터로 훑는다.
- 노트마다 **국소 NPS** 세 개: 중심 창 `[t − W/2, t + W/2)` 안 판정 노트 수 / 그 창을 `[0, lengthUs]` 로 자른 길이(초). W = `LOCAL_NPS_WINDOW_US = 2_000_000`(기본), `BURST_WINDOW_US = 1_000_000`, `SUSTAIN_WINDOW_US = 8_000_000`. 자른 길이로 나누므로 곡 처음·끝에서 과소평가하지 않는다.
- 문맥 분류(노트마다, 상수 `DENSITY_CONTEXT`):
  - `burst`: `nps1 >= BURST_RATIO(1.5) × nps8`
  - `sustained`: `nps2 >= SUSTAIN_MIN_NPS(6)` 이고 `nps8 >= SUSTAIN_RATIO(0.85) × nps2`
  - 그 밖 `mixed`
- 저장 `metrics.by_nps`: 키 `"0"`~`"39"` 와 `"40+"`(`NPS_BIN_WIDTH = 1`, `NPS_BIN_TOP = 40`), 값 `{ ...CompactCell, burst: CompactCell, sustained: CompactCell }`. 노트는 `floor(nps2)` 칸에 든다. 노트가 없는 칸은 저장하지 않는다.
- 저장 `metrics.peak_nps`: `{ "500": x, "1000": x, "2000": x, "4000": x, "8000": x }`(키는 창 ms, `PEAK_NPS_WINDOWS_MS`). 각 값 = 판정 노트 시각마다 `[t, t + w)` 안 개수의 최댓값 / (w 초), 소수 2자리.
- 저장 `metrics.peak_rps_2s`: 같은 방식으로 서로 다른 행(같은 `timeUs`) 수의 2초 창 최댓값 / 2. 행 밀도와 노트 밀도를 가른다(01 §5.1 "행 밀도").
- 기존 `by_density`(고정 1초 창 5단위 칸)는 호환을 위해 그대로 둔다.

#### 2.2.2 패턴 태그별 오차 분포

- 저장 `metrics.timing_by_tag[태그] = TimingMoments`(PATTERN_TAGS 순서, 노트가 있는 태그만).
- 기존 `by_tag`(등급별 Tally)는 그대로 둔다. 둘을 합치면 태그별 판정 분포 + 평균·표준편차·FAST/SLOW 크기가 나온다.
- 저장 `metrics.timing_by_lane[레인]` 은 기존 `{notes, mean_us}` 에 `moments: TimingMoments` 를 더한다(PG~BAD).

#### 2.2.3 손·레인 편향

- 손 상수 `SP7_LANE_HAND_KEYS = ['left','left','left','shared','right','right','right','scratch']`(레인 0~7). 기존 `SP7_LANE_HANDS` 는 `ln_release_then_hit` 판정에 쓰이므로 건드리지 않고 새 상수를 둔다.
- 저장 `metrics.by_hand[left|shared|right|scratch] = CompactCell`, `metrics.timing_by_hand[...] = TimingMoments`.
- 저장 `metrics.lane_bp[레인] = { notes, bp }`(레인별 BP 비율용, `by_lane` 에서 계산 가능하지만 프로필 집계를 단순하게 하려 둔다). 레인별 오차 산포는 `timing_by_lane.moments` 로 나온다.

#### 2.2.4 연속 BAD/POOR 구간과 그 직전 패턴 (`bp-runs.ts` 신규)

- 판정을 받은 노트를 시각·레인 순으로 훑어 BAD/POOR 가 이어진 구간(run)을 만든다. 빈 POOR 는 넣지 않는다.
- 저장 `metrics.bp_runs`:
  - `histogram`: `{ "1": n, "2": n, "3-4": n, "5-9": n, "10+": n }`(`BP_RUN_BUCKETS = [1, 2, 3, 5, 10]`).
  - `longest`: 가장 긴 run 의 노트 수(없으면 0).
  - `runs[]`: 길이 `BP_RUN_MIN_REPORT(3)` 이상, 길이 큰 순·같으면 앞선 순 `BP_RUN_TOP(8)` 개. 항목 `{ from_ms, to_ms, length, start_nps, recovery_notes, preceding_tags: [{tag, notes}] }`.
    - `start_nps`: 첫 실패 노트의 `nps2`(소수 1자리).
    - `recovery_notes`: run 끝 뒤 PG/GR 가 `RECOVERY_STREAK(10)` 번 연속될 때까지의 판정 노트 수. 끝까지 안 되면 `null`.
    - `preceding_tags`: run 시작 노트 앞 `PRECEDING_NOTES(8)` 개 판정 노트(시작 노트 포함 안 함)의 태그 빈도 상위 3(같으면 PATTERN_TAGS 순서).
  - `context_tags[태그] = 개수`, `context_notes = 개수`: 길이 2 이상 모든 run 의 직전 창(위 8노트)을 합친 태그 빈도와 노트 수. 프로필에서 "실패 직전에 이 태그가 평소보다 얼마나 많았나(lift)" 를 계산한다.

#### 2.2.5 게이지 급락과 패턴의 관계

- `analyzeTrace` 가 먼저 `findGaugeDrops` 로 꺾임을 구하고 그 `{from_ms, to_ms}` 를 `gaugeDropsMs` 로 넘긴다(지금은 metrics 뒤에 계산한다, 순서를 바꾼다).
- 저장 `metrics.gauge_drop_contexts[]`(꺾임마다, 꺾임 순서): 창 `[from_ms − GAUGE_DROP_LOOKBACK_MS(1000), to_ms]` 안 판정 노트로 `{ from_ms, to_ms, chart_notes, bp, mean_nps (nps2 평균, 소수 1자리), top_tags: [{tag, notes}] (상위 3) }`.
- 저장 `metrics.gauge_drop_tag_totals[태그] = 개수`, `metrics.gauge_drop_notes`: 모든 꺾임 창 합(lift 계산용). 겹치는 창은 노트를 한 번만 센다.

#### 2.2.6 태그 조합 (`tag-combos.ts` 신규)

- 조합 대상 `COMBO_TAGS`(15개): `chord_2, chord_3, chord_4plus, jack, trill, stair, stream, scratch_chord, scratch_mixed, with_scratch, scratch_run, ln_hold_hit, ln_release_then_hit, soflan, dense`.
- 노트마다 그 노트의 COMBO_TAGS 부분집합에서 모든 쌍(사전순 `a+b`, PATTERN_TAGS 순서로 a 가 앞)을 만들어 `metrics.by_combo["a+b"] = CompactCell` 에 더한다(최대 105 키, 노트가 있는 쌍만 저장).
- `metrics.by_tag_solo[태그] = CompactCell`: 그 노트의 COMBO_TAGS 가 정확히 그 태그 하나뿐인 노트. 단독 대 조합을 가른다(01 §5.1 "태그 겹침 분리"). `ln_head`, `ln_end`, `mine_near` 는 조합 대상이 아니다.

#### 2.2.7 BPM·소프란 대응

- `metrics.by_bpm_band[band] = CompactCell`: 노트 시각의 `bpmAt(t)`(`createTempoLookup`) 를 `BPM_BAND_EDGES = [120, 150, 180, 210]` 로 나눈 `"<120" | "120-149" | "150-179" | "180-209" | "210+"`.
- 소프란 사건: `tempo` 점 중 기존 `soflan` 정의(BPM 1.25배 이상 변화, STOP 100 ms 이상, scroll ≠ 1, `pattern-tags.ts:214-232`)에 맞는 점의 시각 `e`.
- `metrics.soflan_phase = { events, before: CompactCell, after: CompactCell, settled: CompactCell }`: `before` = `[e − SOFLAN_PHASE_MS(2000), e)`, `after` = `[e, e + stop + SOFLAN_PHASE_MS)`, `settled` = `[e + stop + 2000, e + stop + SOFLAN_SETTLED_MS(6000))` 의 판정 노트. 한 노트는 가장 가까운 사건 하나에만 든다.
- `summary.speed.soflan_adjust = { events, adjusted }`(`analyzeTrace`): 사건 뒤 `SOFLAN_ADJUST_WINDOW_MS(3000)` 안에 `trace.speed.changes` 가 하나라도 있으면 `adjusted` +1. 녹색 숫자를 사건마다 바꾸는지 본다.

#### 2.2.8 훈련곡 구간 결과

- `trainingSections` 가 있으면(훈련곡 판, 7절) `metrics.training_sections[] = { index, kind, focus_key, from_ms, to_ms, chart_notes, ...CompactCell, moments: TimingMoments }`. 없으면 `null`.
- 구간 경계는 훈련곡 사양의 시각(µs)이다. RANDOM 등 옵션은 시각을 바꾸지 않으므로(01 §2.3) 옵션 판도 그대로 나뉜다.

### 2.3 곡 특징 확장 (`computeChartFeatures`, `ANALYSIS_VERSION` 1 → 2)

`chart_pattern.features` 에 더한다(`layout: 'original'`).

- `nps: { p50_2s, p90_2s, max_2s, max_8s, sustained_s }`: 판정 노트마다의 `nps2` 분포의 nearest-rank p50·p90·최대, `nps8` 최대, `sustained` 문맥 노트가 이어진 시간의 합(초, 소수 1자리. 연속 노트 간격이 `SUSTAIN_GAP_MAX_US(500_000)` 이하일 때 이어진 것으로 본다).
- `combos: { "a+b": ratio }`: `by_combo` 와 같은 쌍 정의로 노트 비율 상위 `CHART_COMBO_TOP(10)`(비율 4자리).
- `bpm_main_band`: 판정 노트가 가장 많은 BPM 띠.
- 기존 필드는 그대로 둔다. `ANALYSIS_VERSION` 이 오르면 판 요약의 `pattern_analysis_version` 도 낡은 것이 되어 판 재계산이 한 번에 같이 돈다(01 §3.7).
- **새 필드는 읽을 때 없을 수 있다.** 버전이 오른 뒤 재계산이 끝나기 전에는 `readFeaturesMany` 가 옛 모양 특징을 `isStale: true` 로 그대로 돌려준다(`chart-pattern.service.ts:209`, `hasTagShares` 만 검사). 한 번의 읽기가 새로 계산하는 곡은 `CHART_PATTERN_READ_RECOMPUTE_MAX(10)` 개뿐이다. 그래서 `chart-features.ts` 에 접근자 `readChartFeatureExtras(features: unknown) → { nps: ChartNps; combos: Record<string, number>; bpmMainBand: BpmBand } | null` 을 export 한다. 안에서 `chartFeatureExtrasSchema`(Zod, `nps`·`combos`·`bpm_main_band` 세 필드만)로 `safeParse` 하고, 실패하면 `null` 을 돌려준다. 기능 5·6·7(2.6)은 `features.nps` 를 직접 읽지 않고 이 접근자만 쓴다. 결과가 `null` 이거나 읽은 값이 `isStale` 이면 밀도 초과 감점은 0, `density:*`·`combo:*`·`bpm:*` 의 `share` 는 0 으로 둔다(태그 비율 기반 `tag:*`·`solo:*` 일치는 지금처럼 쓴다).

### 2.4 판 요약 형식과 버전

- `PLAY_ANALYSIS_VERSION` 2 → **3**, `ANALYSIS_VERSION` 1 → **2**. JSDoc 에 "3: sliding-window NPS cells, timing moments by tag/lane/hand, BP runs, gauge-drop contexts, tag pairs, BPM bands, soflan phases, training sections (2026-11-04)" 를 적는다.
- 요약 최상위 모양은 그대로이고 `metrics` 안에 2.2 의 필드가, `gauge.drops[]` 는 그대로, `speed` 에 `soflan_adjust` 가 더해진다. 더해지는 `metrics` 필드 목록(정본):

| 필드 | 타입 | 절 |
| --- | --- | --- |
| `by_nps` | `Record<string, CompactCell & { burst: CompactCell; sustained: CompactCell }>` | 2.2.1 |
| `peak_nps` | `Record<'500'|'1000'|'2000'|'4000'|'8000', number>` | 2.2.1 |
| `peak_rps_2s` | `number` | 2.2.1 |
| `timing_by_tag` | `Record<PatternTag, TimingMoments>` | 2.2.2 |
| `timing_by_lane[*].moments` | `TimingMoments` | 2.2.2 |
| `by_hand`, `timing_by_hand` | `Record<HandKey, CompactCell>`, `Record<HandKey, TimingMoments>` | 2.2.3 |
| `lane_bp` | `Record<LaneName, { notes: number; bp: number }>` | 2.2.3 |
| `bp_runs` | `{ histogram, longest, runs[], context_tags, context_notes }` | 2.2.4 |
| `gauge_drop_contexts`, `gauge_drop_tag_totals`, `gauge_drop_notes` | 2.2.5 참고 | 2.2.5 |
| `by_combo`, `by_tag_solo` | `Record<string, CompactCell>` | 2.2.6 |
| `by_bpm_band`, `soflan_phase` | 2.2.7 참고 | 2.2.7 |
| `training_sections` | `array | null` | 2.2.8 |

- 크기 예상(추정): 판 하나 요약 15~30 KB(JSON). `by_combo` 가 가장 크다. `play_analysis.summary` 는 MySQL `json` 이라 상한 걱정은 없다. 측정은 U2 시험에서 6만 노트 합성 trace 로 바이트 수를 출력해 확인하고 30 KB 를 크게 넘으면 `by_combo` 를 노트 수 상위 `PLAY_COMBO_TOP(40)` 로 자른다.
- `TraceAnalysisSummary` 는 `ReturnType` 유도라 소비자 타입이 자동으로 따라온다. 기존 필드는 이름·뜻을 바꾸지 않는다(화면이 W4 전까지 옛 필드만 읽어도 컴파일되게).

### 2.5 사용자 약점 프로필

#### 2.5.1 어디서 계산·캐시하나

- 순수 집계: `web/src/server/service/domain/analysis/weakness-profile.ts`(`computeWeaknessProfile(plays, options)`), 통계 도구: `.../analysis/profile-stats.ts`(아래 2.5.4).
- 서비스: `web/src/server/service/domain/ai/weakness-profile.service.ts`(`createWeaknessProfileService(deps)` → `view({ userId })`, `measureFocus({ userId, keys, sinceMs })`, `digest({ userId })`). 공급자 호출 없음(결정론 화면).
- 입력: 새 trace 서비스 메서드 `analyzedPlaysInWindow({ userId, sinceMs, limit })` → `{ traceId, scoreId, chartSha256, chartMd5, playedAt, isTraining, summary }[]`(`isTraining` = 그 사용자의 `training_chart.sha256` 과 `chartSha256` 이 같은지, compose 의 left join. 7.1 의 군집 상한에 쓴다. 최신순, `measured`(7K 지표 있는) 판만, 낡은 요약은 기존 규칙대로 최대 10판 다시 계산하고 나머지는 `stale` 로 센다, 01 §3.7).
- 창: `PROFILE_WINDOW_DAYS = 90`, `PROFILE_MAX_PLAYS = 300`(최신 300판까지). 추세용 하위 창: 최근 `PROFILE_RECENT_DAYS = 30` 대 그 이전.
- 캐시: 새 표 `play_profile`(4절). 지문 `fingerprint = sha256(JSON.stringify([PROFILE_VERSION, CURRENT_ANALYSIS_VERSIONS, 창 시작 UTC 날짜, 창 안 분석 수, max(play_analysis.computed_at), max(play_trace.played_at)]))`. 읽을 때 지문만 싼 질의(색인 `play_trace_user_played_idx`)로 계산해 저장값과 같으면 `data` 를 그대로 돌려주고, 다르면 다시 계산해 upsert 한다. 업로드 쪽에 무효화 훅을 두지 않는다(지문이 대신한다). `stale` 이 0 이 아닐 때 계산한 프로필은 저장하지 않는다(다음 읽기에서 다시).
- `PROFILE_VERSION = 1`(`play-feature.constants.ts`). 집계 정의를 바꾸면 올린다.

#### 2.5.2 셀과 표본

"셀" 은 집계 단위다: 태그(19), 태그 쌍, 태그 단독, NPS 칸, 밀도 문맥(sustained 대 burst), 레인(8), 손(4), BPM 띠(5), 소프란 국면(3), 피로(앞/뒤 1/3). 각 셀은 판마다 `CompactCell` 과 그 판의 전체 EX 비율 `b_p` 를 갖는다.

- **곡 단위 군집**: 같은 차트(`chartSha256`)의 판을 하나의 군집 k 로 묶는다. 군집 k, 셀 c: `N_kc = Σ notes`, `E_kc = Σ ex_score`, `B_kc = Σ notes × b_p`. 잔차 `r_kc = E_kc / (2 N_kc) − B_kc / N_kc`(판 난이도를 상쇄하는 기존 baseline 정의를 곡 군집으로 옮긴 것, 01 §3.8).
- 셀 추정 `R_c = Σ w_k r_kc / Σ w_k`, `w_k = min(N_kc, CHART_WEIGHT_CAP_NOTES(500))`(한 곡 반복이 지배하지 못하게, 01 §5.2 편향).
- 표본 조건: `N_c ≥ MIN_CELL_NOTES(300)`, 군집 수 `K_c ≥ MIN_CELL_CHARTS(5)`, 판 수 `≥ MIN_CELL_PLAYS(5)`. 못 채우면 `insufficient`.
- **표본 수 등급** `sample_level`: `high`(K ≥ 20 && N ≥ 3000), `medium`(K ≥ 10 && N ≥ 1000), `low`(표본 조건만 충족), `insufficient`. 화면은 등급을 숫자와 함께 보이고, AI digest 는 `low` 이상만 받는다.
- **훈련곡 군집 상한**: 훈련곡은 곡마다 sha256 이 달라 20곡이면 한 셀의 K 를 합성 차트만으로 20 까지 채울 수 있다. 그래서 `isTraining` 판은 차트와 관계없이 모두 **하나의 군집 `training`** 으로 묶는다(N·E·B 를 합치고 `w = min(N, CHART_WEIGHT_CAP_NOTES)` 하나). 그 결과 훈련곡은 K 에 최대 1, 가중에 최대 한 군집분만 보탠다. D4(훈련곡 판도 프로필에 반영)는 유지하고, 훈련 효과는 일반 곡 판으로 확인된다는 한계를 화면 도움말(`pages.ai.profile.trainingClusterNote`)에 적는다. 훈련곡 판만의 변화는 7절 전후 비교와 구간 결과가 따로 보인다.
- **SCRATCH AUTO 판**(`summary.scratch_auto === true`, `trace-analysis.ts:174`)은 집계의 명시 입력이다. trace 에 자동 레인 노트가 없어(01 §2.3) 그 판의 NPS·행 밀도는 스크래치가 빠진 값이기 때문이다.
  - 밀도 셀(`density:limit` 곡선·`comfort_nps`·`limit_nps_ex`·`limit_nps_bp`·`max_seen_nps`·`peak_seen_2s`, `density:sustained`)은 `scratch_auto` 가 아닌 판만으로 계산한다. 그래야 스크래치를 포함한 곡 특징 `nps`(2.3)와 같은 단위로 비교된다(기능 5·6·7, 3.5 의 `bounds`).
  - 스크래치 셀(`lane:S`, `hand:scratch`, `SCRATCH_TAGS`(`pattern-tags.ts:73`) 의 `tag:`·`solo:`·`timing:`·`bp_context:`·`gauge_context:`, 한쪽이라도 스크래치 태그인 `combo:`)은 auto 판을 뺀다(현행 `sample.ts` 규칙과 같다). 각 셀 응답에 `auto_plays`(뺀 판 수)를 더한다.
  - 그 밖 셀(건반 태그·레인 1~7·좌우 손·BPM·소프란·피로)은 auto 판을 그대로 넣는다(현행 규칙).
  - 창 응답 `window` 에 `auto_plays` 를 더한다. 창 안 판이 모두 auto 면 밀도는 `insufficient` 이고 `notice` 는 그대로 둔다.

#### 2.5.3 검정과 다중비교 보정

- 군집 강건 표준오차: `SE_c = sqrt(K/(K−1)) × sqrt(Σ w_k² (r_kc − R_c)²) / Σ w_k`. `t = R_c / SE_c`, 자유도 `K − 1`, 양측 p = `2 × (1 − T_{K−1}(|t|))`. `SE_c = 0` 이면 p = 1(모든 곡이 같은 값이면 정보가 없다고 본다).
- Student t 누적분포는 `profile-stats.ts` 에 정규화 불완전 베타 함수(연분수 전개, 반복 상한 `BETA_CF_MAX_ITERATIONS = 200`, 수렴 `BETA_CF_EPSILON = 1e-12`)로 구현한다. 새 의존성 없음. 시험은 자유도 1·4·30 의 알려진 값(예: t=2.776, df=4 → 양측 0.05)과 대조한다.
- **Benjamini–Hochberg** 보정을 한 프로필 안의 두 가족에 따로 적용한다: `accuracy` 와 `timing`. `FDR_Q = 0.10`. p 를 오름차순 정렬해 `p_(i) ≤ i/m × q` 를 만족하는 최대 i 까지 `strong`. 표본 조건을 채운 셀만 m 에 넣는다. 셀 종류별 가족(정본, `PROFILE_TEST_FAMILY` 상수):

| 셀 종류 | 가족 | 검정하는 값 |
| --- | --- | --- |
| `tag:`, `solo:`, `combo:`, `lane:`, `hand:`, `bpm:` | `accuracy` | 군집별 EX 잔차 `r_kc` |
| `soflan:after`, `soflan:settled` | `accuracy` | 군집별 (국면 EX 비율 − `before` EX 비율) |
| `fatigue` | `accuracy` | 군집별 `fatigue.delta` 의 노트 가중 평균 |
| `density:sustained` | `accuracy` | 군집별 (같은 NPS 칸 안 `sustained` − `burst` EX 비율의 노트 가중 평균) |
| `bp_context:`, `gauge_context:` | `accuracy` | 군집별 log lift |
| `timing:` | `timing` | 군집별 (태그 평균 오차 − 판 전체 평균 오차) |
| `density:limit` | 검정 없음 | 곡선 추정(아래), 등급은 9단계 규칙 |

- 레인·손 셀의 타이밍 평균·표준편차(`timing_by_lane.moments`, `timing_by_hand`)는 설명값으로만 보이고 검정하지 않는다(가족에 넣지 않는다).
- 모든 검정은 위의 군집 강건 t 검정 하나를 쓴다. 군집 값이 다르기만 하다.
- 등급: `strong`(BH 통과), `uncertain`(표본 충족, BH 미통과), `insufficient`. `leaning`: `strong` 이고 `R_c < 0` 이면 `weakness`, `> 0` 이면 `strength`. 기존 윌슨 검정(`sample.ts`)은 남기되 리포트 digest 는 프로필 등급을 쓴다.

#### 2.5.4 셀별 계산

| 셀 | 값 | 비고 |
| --- | --- | --- |
| 태그 `tag:<t>` | `R_c`, EX 비율, BP 비율, 판·곡·노트 수 | `by_tag` 에서 |
| 단독 `solo:<t>` | 같음 | `by_tag_solo` |
| 조합 `combo:<a>+<b>` | 같음 + `interaction = R_combo − min(R_a, R_b)` | `interaction ≤ −COMBO_INTERACTION_MIN(0.02)` 이고 조합이 `strong weakness` 일 때만 "조합 약점" 으로 올린다 |
| 레인 `lane:<1..7,S>` | EX 잔차, BP 비율, 타이밍 평균·표준편차 | `by_lane`, `lane_bp`, `timing_by_lane.moments` |
| 손 `hand:<left|shared|right|scratch>` | 같음 | `by_hand`, `timing_by_hand` |
| 타이밍 `timing:<t>` | 태그 평균 오차 − 판 전체 평균 오차(ms), 태그 표준편차, FAST/SLOW 평균 크기, FAST 비율 | 군집별 평균 차이로 t 검정(가족 `timing`). 판 전체 평균은 `timing`(PG·GR) 이 아니라 `timing_by_hand` 네 손의 `TimingMoments` 를 더한 PG~BAD 평균이다. 한 노트는 레인 하나·손 하나에만 들어 중복 없는 분할이다. `timing_by_tag` 는 한 노트를 태그마다 중복 집계하므로(01 §3.4) 합치지 않는다 |
| BPM `bpm:<band>` | EX 잔차 | `by_bpm_band` |
| 소프란 `soflan:after`, `soflan:settled` | `after`·`settled` EX 비율 − `before` EX 비율 | 판마다 차이를 군집해 t 검정 |
| 피로 `fatigue` | 판 `fatigue.delta` 의 곡 군집 평균 | 기존 평균에 등급을 붙인다 |
| 지속 `density:sustained` | 같은 NPS 칸 안에서 `sustained` EX 비율 − `burst` EX 비율의 노트 가중 평균 | 두 문맥 노트가 모두 `MIN_BIN_CONTEXT_NOTES(50)` 이상인 칸만 |

밀도 한계(검정이 아니라 곡선 추정, `density:limit`):

1. 판의 `by_nps` 를 모두 합쳐 `PROFILE_NPS_BIN_WIDTH = 2` 칸(0-1, 2-3, …, 40+)으로 묶는다. 칸마다 EX 비율 `x_i`, BP 비율 `y_i`, 노트 `n_i`, 곡 수 `k_i`.
2. `n_i ≥ MIN_BIN_NOTES(150)` 이고 `k_i ≥ MIN_BIN_CHARTS(3)` 인 칸만 쓴다.
3. 가중 등위회귀(PAVA, 가중치 `n_i`)로 `x` 는 감소, `y` 는 증가 단조 곡선 `x̂`, `ŷ` 를 만든다.
4. `baseline` = 플레이어 노트의 NPS 중앙값 이하 칸들의 `x` 노트 가중 평균.
5. `comfort_nps` = `x̂ ≥ baseline − COMFORT_EX_DROP(0.03)` 인 가장 높은 칸의 위쪽 경계.
6. `limit_nps_ex` = `x̂ ≤ baseline − LIMIT_EX_DROP(0.10)` 인 가장 낮은 칸의 아래쪽 경계. 없으면 `null` 이고 `ceiling_reached_ex = false`.
7. `limit_nps_bp` = `ŷ ≥ LIMIT_BP_RATE(0.08)` 인 가장 낮은 칸의 아래쪽 경계. 없으면 `null`.
8. `max_seen_nps` = 쓴 칸 중 가장 높은 칸의 아래쪽 경계, `peak_seen_2s` = 판 `peak_nps["2000"]` 의 최댓값.
9. 등급: 한계 칸과 그 아래 칸이 각각 `MIN_BIN_CHARTS` 를 채우고 전체 곡 수가 `MIN_CELL_CHARTS` 이상이면 `strong`, 아니면 `uncertain`, 쓸 칸이 3개 미만이면 `insufficient`.

실패·체력 문맥(lift):

- `bp_context:<t>`: `lift = (context_tags[t] / context_notes) / (by_tag[t].notes / overall.notes)`(창 전체 합). `context_tags[t] ≥ MIN_CONTEXT_NOTES(30)` 이고 `lift ≥ CONTEXT_LIFT_MIN(1.5)` 인 것만 후보. 판 단위로 lift 의 군집 t 검정(log lift, 가족 `accuracy`)을 해 등급을 붙인다.
- `gauge_context:<t>`: 같은 식으로 `gauge_drop_tag_totals` / `gauge_drop_notes`. 추가로 `drop_mean_nps` 대 판 평균 `nps2`.
- `bp_runs` 요약: 판당 `longest` 의 곡 군집 평균, histogram 합, `recovery_notes` 중앙값.

#### 2.5.5 약점 목록과 키 (다른 기능과의 기계적 연결)

- `weaknesses[]`: `leaning = weakness` 이거나 `density:limit` 이 `strong` 인 셀. 정렬 = `strong` 먼저, 그 안에서 `severity` 내림차순. `severity = |R_c| × sqrt(min(N_c, SEVERITY_NOTES_CAP(3000)) / SEVERITY_NOTES_CAP)`. `density:limit` 은 `severity = (x̂(max_seen) 대 baseline 의 차)`. 상위 `PROFILE_WEAKNESS_TOP(8)`.
- 항목 `{ key, kind, label_tags, value, unit, evidence, sample_level, notes, charts, plays, severity, trend_delta, selectable_for_folder, training_kinds }`.
  - `key` 문법(정본, `WEAKNESS_KEY` 함수들이 만들고 읽는다): `tag:<tag>`, `solo:<tag>`, `combo:<a>+<b>`, `lane:<name>`, `hand:<hand>`, `timing:<tag>`, `bpm:<band>`, `soflan:<after|settled>`, `fatigue`, `density:limit`, `density:sustained`, `bp_context:<tag>`, `gauge_context:<tag>`.
  - 키 도구 파일 `web/src/server/service/domain/analysis/weakness-key.ts`(U6): `WEAKNESS_KEY_MAX_LENGTH = 64`, `parseWeaknessKey(value: string) → { kind; tags: PatternTag[]; arg: string | null } | null`(접두사 13종을 위 문법으로 나누고, 태그는 `PATTERN_TAGS`, `combo` 는 `COMBO_TAGS` 두 개이고 a 가 PATTERN_TAGS 순서상 앞, 레인은 `1`~`7`·`S`, 손은 4종, BPM 띠는 `BPM_BAND_EDGES` 에서 만든 5종인지 확인. 하나라도 어긋나면 `null`), `weaknessKeySchema = z.string().max(WEAKNESS_KEY_MAX_LENGTH).refine((value) => parseWeaknessKey(value) !== null)`. 길이 상한 64 는 가장 긴 키(`combo:ln_release_then_hit+scratch_mixed`, 39자)보다 넉넉하고 `subject_key`(128자, 3.2)에 들어간다.
  - 외부에서 들어오는 약점 키(훈련곡 생성 `focus_key`, 연습 폴더 `weakness` 출처의 `focus_key`, 목표 `weakness_residual.key`)는 모두 요청 DTO 에서 `weaknessKeySchema` 로 문법을 검사하고(어긋나면 400 `VALIDATION_ERROR`), 서비스가 공급자 호출 전에 **현재 프로필 `weaknesses` 에 그 키가 있는지** 확인한다(없으면 422 `AI_NOT_ENOUGH_DATA`, need `weakness_profile`). 훈련곡은 추가로 `training_kinds` 가 비지 않았는지, 연습 폴더는 `selectable_for_folder` 인지 본다(2.6, 3.1).
  - `selectable_for_folder`: 곡 특징으로 곡을 고를 수 있는 키(`tag`, `solo`, `combo`, `density:*`, `bpm`, `soflan`)만 true. 레인·손·타이밍은 옵션이 바꾸므로 false.
  - `training_kinds`: 3.3 의 `TRAINING_KINDS_OF_KEY` 로 정한 합성 종류 목록.
- `strengths[]`: `leaning = strength` 상위 `PROFILE_STRENGTH_TOP(5)`.
- `trend_delta`: 최근 30일 판만으로 계산한 `R_c` − 31~90일 판만으로 계산한 `R_c`(둘 다 표본 조건을 채울 때만, 아니면 `null`).
- 이 `key` 가 리포트의 `practice_focus[].key`, 연습 폴더 출처 `weakness`, 목표 종류 `weakness_residual`, 추천 후보의 `focus_keys`, 훈련곡 구간의 `focus_key` 로 그대로 쓰인다(2.6).

#### 2.5.6 프로필 응답 (`GET /api/fe/me/ai/profile`)

`dto/ai-play.ts` 에 `weaknessProfileViewSchema`:

```typescript
{
  profile_version: number,
  window: { from: number, to: number, days: number, plays: number, charts: number, notes: number, stale: number, auto_plays: number, training_plays: number },
  density: {
    curve: { from_nps: number, to_nps: number | null, notes: number, charts: number, ex_ratio: number, ex_fit: number, bp_rate: number, bp_fit: number }[],
    baseline_ex: number | null, comfort_nps: number | null, limit_nps_ex: number | null, limit_nps_bp: number | null,
    max_seen_nps: number | null, peak_seen_2s: number | null, ceiling_reached_ex: boolean, evidence: Evidence,
    sustained_minus_burst: { value: number | null, evidence: Evidence, sample_level: SampleLevel },
  },
  cells: { key: string, kind: string, value: number | null, unit: 'ex_residual' | 'ms' | 'ratio', ex_ratio: number | null, bp_rate: number | null,
           timing: { mean_ms: number, sd_ms: number, fast_mean_ms: number, slow_mean_ms: number, fast_share: number } | null,
           notes: number, charts: number, plays: number, auto_plays: number, p_value: number | null, q_threshold: number | null,
           evidence: Evidence, leaning: 'weakness' | 'strength' | null, sample_level: SampleLevel, trend_delta: number | null }[],
  weaknesses: WeaknessItem[], strengths: WeaknessItem[],
  bp_runs: { longest_mean: number | null, histogram: Record<string, number>, recovery_median: number | null },
  gauge: { drops_per_play: number | null, drop_mean_nps: number | null, overall_mean_nps: number | null },
  soflan: { events: number, adjusted_share: number | null },
  notice: { need: string } | null,
}
```

- `notice.need = 'traced_plays'`: 창 안 7K 분석 판이 `PROFILE_MIN_PLAYS(10)` 미만(기존 need 재사용).
- 라우트 `createAiPlayFeatureRoutes` 에 `profileView`(GET 만), 어댑터 `web/src/app/api/fe/me/ai/profile/route.ts`(`maxDuration` 불필요, 기본값), `private, no-store`, 분당 30회(`withLimited`).

### 2.6 기존 기능 10개의 변경

모든 변경은 응답 스키마에 필드를 **더하는** 것으로 한다(웹 화면은 W4 에서 따라온다). 프롬프트 버전을 올려 캐시가 새로 갈린다.

| 기능 | 파일 | 변경 | 프롬프트 |
| --- | --- | --- | --- |
| 1 리포트 | `ai/play-report.service.ts` | digest 의 강점·약점을 프로필 `weaknesses`/`strengths`(strong 만, `key` 포함)로 바꾸고 `density{comfort_nps, limit_nps_ex, limit_nps_bp, max_seen_nps, sustained_minus_burst}`, 조합 약점, `bp_context`·`gauge_context` 상위 3, 손 편향, 타이밍 태그 상위 3 을 더한다. `practice_focus[].key` 는 digest 의 weakness `key` 만 허용(`refine` 이 아니면 버림). 뷰에 `profile_keys` 를 더해 화면이 "이 약점으로 연습 폴더 / 훈련곡" 버튼을 단다. 7K 판 5 미만 조건은 유지 | v3 |
| 2 코치 | `ai/replay-coach.service.ts` | 문제 구간마다 `peak_nps`(그 구간 판정 노트 `nps2` 최대), 그 구간에 시작한 `bp_runs.runs` 의 `preceding_tags`, 그 구간의 `gauge_drop_contexts`, 구간 상위 태그 중 프로필 약점과 겹치는 `profile_keys` 를 더한다 | v2 |
| 3 비교 | `ai/play-compare.service.ts` | 차이 구간에 두 판 구간의 `nps2` 최대(차트가 같으므로 하나)와 이 판의 구간 타이밍 평균을 더한다. 라이벌 공유 범위는 그대로(구간 합계만) | v3 |
| 4 판정 습관 | `ai/judge-habit.service.ts` | 프로필 `timing:<t>` 중 strong 상위 5(`mean_ms` 크기 순)와 `hand:*` 타이밍을 digest `tag_timing`, `hand_timing` 으로 더한다. 오프셋 제안 규칙은 그대로 | v3 |
| 5 추천 | `ai/recommend.service.ts`, `ai/table-progress.ts` | 후보 점수에 `WEAKNESS_MATCH_WEIGHT(1.0) × match` 와 `− DENSITY_OVER_PENALTY(1.0) × [features.nps.p90_2s > limit_nps_ex × DENSITY_OVER_RATIO(1.25)]` 를 더한다. `match = Σ_{w ∈ strong 약점 중 selectable 상위 3} min(1, share(chart, w) / WEAKNESS_FULL_SHARE(0.3)) / 3`. `share` 는 `tag`/`solo` 면 태그 비율, `combo` 면 쌍 비율, `density:limit` 이면 `p90_2s` 가 `[limit × 0.9, limit × 1.15]` 안이면 1, `bpm` 이면 `bpm_main_band` 일치 1. `nps`·`combos`·`bpm_main_band` 는 `readChartFeatureExtras`(2.3)로만 읽고, `null` 이거나 `isStale` 이면 감점 0·해당 `share` 0. 후보마다 `focus_keys` 를 붙이고 LLM `focus_key` 는 후보 `focus_keys` 또는 기존 태그만 허용 | v3 |
| 6 연습 폴더 | `ai/practice-folder.service.ts`, `dto/ai.ts` | 출처 `weakness` 추가(`AI_PRACTICE_SOURCES = ['recommend','tag','goal','weakness']`, 파라미터 `focus_key` 는 `weaknessKeySchema`, 2.5.5). 후보 = 프런티어 −2~+1 레벨, 램프 FULL COMBO 미만, `share(chart, focus_key) ≥ FOCUS_TAG_MIN_RATIO(0.1)`(density 키는 위 구간 규칙, 특징은 `readChartFeatureExtras` 로), 비율 큰 순 60곡. 프로필 `weaknesses` 에 없는 키는 422 `AI_NOT_ENOUGH_DATA` + need `weakness_profile`, `selectable_for_folder` 가 false 인 키는 422 + need `candidates`(둘 다 공급자 호출 전) | v3 |
| 7 목표 | `ai/goal-plan.service.ts`, `dto/ai.ts` | 목표 종류 `weakness_residual`(`{ key, target_residual }`, `key` 는 `weaknessKeySchema` + 프로필 `weaknesses` 에 있어야 함, `target_residual` −0.2~0.05) 와 `density_limit`(`{ target_nps }`, 4~40) 추가. 진행은 프로필 현재 값(`R_c`, `limit_nps_ex`)과 목표 생성 때 스냅샷(`ai_goal.params.baseline`)의 비. 남은 곡 대신 `weakness_residual` 은 6 의 후보 상위 20, `density_limit` 은 `p90_2s` 가 목표 ±1.5 인 곡을 계획 입력으로 | v3 |
| 8 게이지 조언 | `ai/gauge-advice.service.ts` | digest 에 `drop_contexts`(프로필 `gauge_context:*` 상위 3, `drop_mean_nps` 대 평균)를 더한다 | v2 |
| 9 기록 질문 | `ai/record-query.service.ts`, `ai/record-query.compiler.ts`, `dto/ai.ts` | 필드 `tag`(연산 `eq`·`in`, 값은 PATTERN_TAGS. 곡 특징 태그 비율 ≥ `CHART_TAG_MIN_RATIO(0.05)` 인 md5) 와 `training`(연산 `eq`, 값 `true|false`, `score.chart_sha256` 이 그 사용자의 `training_chart.sha256` 에 있는지. `score.chart_md5` 는 NULL 허용·무색인이라 쓰지 않는다, 7.1)을 더한다. 컴파일은 drizzle `sql` 템플릿의 매개변수 바인딩만 쓴다(JSON 경로는 `JSON_EXTRACT(features, ?)` 의 매개변수, 태그는 enum 검증 뒤) | v3 |
| 10 주간 요약 | `ai/weekly-summary.service.ts` | `training{ created, plays, charts }`(그 주 생성 훈련곡 수, 훈련곡 판 수. 판은 `chart_sha256` 조인) 와 `focus[]`(프로필 약점 상위 3 의 그 주 판만으로 계산한 `R_c` 와 앞 주 값)를 더한다. trace 를 처음 쓰는 점수 기능이 된다 | v3 |

- 상수: 기능 임계값은 `ai/play-feature.constants.ts`(1~4, 8, 프로필) 와 `ai/score-feature.constants.ts`(5~7, 9, 10). `PLAY_FEATURE_ANALYSIS_VERSION` 1 → 2.
- 새 `need` 코드: `weakness_profile`(프로필이 없어 약점 기반 출처·목표를 만들 수 없음). 서버는 W2·W3 에서 내보내고, 화면 쪽 `AI_DATA_NEEDS`·문구·계약 시험 목록(`tests/components/ai-score-features-contract.test.ts`)은 W4 의 U10 이 더한다. 시험의 서버 목록이 손으로 적힌 목록이라 W2·W3 동안 시험은 깨지지 않는다.
- 목표 종류는 서버 `AI_GOAL_KINDS` 를 화면 시험이 직접 import 하므로(같은 시험 25-27행) 서버를 바꾸는 단위(U9)가 화면 쪽 `AI_GOAL_KINDS`(`entities/ai.type.ts`)와 `pages.ai.goalForm.goalKind` 세 언어 키를 같이 더한다.

### 2.7 새 UI (차트 라이브러리 없이 CSS/SVG)

AI 허브 탭에 `profile`(약점 프로필)과 `training`(훈련곡)을 더한다: `AI_HUB_TABS = ['query','profile','training','recommend','goals','weekly','report','habit','gauge']`. 기본 탭은 그대로 `query`.

`/ai?tab=profile` 의 `widgets/ai/ai-profile-panel.tsx` 가 아래 순수 조각(features)을 조립한다.

| 조각(`web/src/features/ai/`) | 보이는 것 | 그리는 법 |
| --- | --- | --- |
| `ai-density-limit-chart.tsx` | x = NPS 칸, y = EX 비율. 점(원 반지름은 노트 수의 제곱근 비례, 상한 있음), 등위회귀 선, `baseline` 가로 점선, `comfort_nps`·`limit_nps_ex`·`limit_nps_bp` 세로선과 라벨, BP 비율은 아래 작은 막대 | 인라인 SVG `viewBox="0 0 320 180"`, `vectorEffect="non-scaling-stroke"`, 색 `var(--chart-1..3)`, `role="img"` + `aria-label` 에 세 한계값 문장, 애니메이션 없음 |
| `ai-weakness-list.tsx` | 약점 8·강점 5: 이름, 값, 등급 배지, 표본 등급, 추세 화살표(lucide `TrendingUp/Down`), "연습 폴더"·"훈련곡" 버튼(콜백 props) | 목록 + 기존 `AiTagResidualList` 막대 재사용 |
| `ai-tag-timing-chart.tsx` | 태그별 평균 오차(가운데 0, 좌 SLOW·우 FAST), 표준편차 수염 | CSS 막대(`--rbms-timing-fast/slow`) |
| `ai-combo-grid.tsx` | 15×15 상삼각 격자, 칸 색 농도 = 잔차(음수만 강조), 표본 부족 칸은 빗금 없이 빈 칸 | CSS grid, `color-mix(in oklch, var(--chart-1) N%, transparent)` |
| `ai-hand-balance.tsx` | 손 4·레인 8 의 EX 잔차·BP 비율·타이밍 | CSS 막대 두 줄 |
| `ai-bp-context-list.tsx` | 실패 직전 태그 lift, 게이지 급락 태그 lift, 최장 연속 실패 평균, 회복 노트 중앙값 | 목록 + 개수 |
| `ai-sample-badge.tsx` | `sample_level` 과 `evidence` 를 함께 보이는 배지 | 텍스트 배지 |

- 화면 문구 키 접두사: `pages.ai.profile.*`(세 언어 `web/messages/{ko,en,ja}/pages.json`), 셀 이름은 `pages.ai.patternTag.*`(기존)과 새 `pages.ai.weaknessKind.*`.
- 판 코치 페이지(`/ai/plays/[scoreId]`)의 구간 차트(`ai-segment-chart.tsx`)에 실패 run 표지(작은 삼각형)를 더한다.

### 2.8 재계산 경로

- 재사용: `traceService.recomputeStale({ limit, after })`(01 §3.7, `trace.service.ts:473`), `chartPatternService.recomputeStale`(같은 서비스 `:221`).
- 새 스크립트 `web/scripts/analysis-recompute.ts`(형식은 `dan-recompute.ts` 와 같다). **새 서비스 메서드를 만들지 않는다.** 이미 공개된 것만 쓴다.
  - `count`: `chartPatternService.coverage()` 의 `stale_features`(`chart-pattern.service.ts:176-179`) 와 `traceService.adminUsage()` 의 `analysis.missing`·`analysis.stale`(`trace.service.ts:361-366`). `adminUsage` 의 나머지 필드(사용자 `login_id` 가 든 `top`)는 출력하지 않는다.
  - `apply`: `chartPatternService.recomputeStale({ limit, after })` 를 먼저, 다음 `traceService.recomputeStale({ limit, after })` 를 `ANALYSIS_RECOMPUTE_BATCH = 100` 씩, `next_after` 가 null 이 되거나 `examined` 가 0 이 될 때까지 돈다. 두 메서드가 이미 `remaining` 을 돌려준다.
  - 배선: 스크립트가 `createScriptDb()` 로 `db` 를 받아 `const patterns = composeChartPattern(db, { listCoverageTables: () => [] })`(표 목록은 count 에 필요 없다, `stale_features` 는 표와 무관), `composeTrace(db, { chartPatternService: patterns.chartPatternService })` 를 만든다(`compose/index.ts:377-386` 과 같은 순서). 테스트용으로 `recomputeAnalyses({ command, patterns: Pick<ChartPatternService, 'coverage' | 'recomputeStale'>, traces: Pick<TraceService, 'adminUsage' | 'recomputeStale'>, now, log })` 를 export 한다.
  - 숫자만 출력(Actions 로그는 공개): `plan patternsStale N, analysesMissing N, analysesStale N` / `applied patterns N, analyses N, skipped N, remaining N` / 시간 상한으로 멈추면 `stopped elapsedMs N, remaining N`.
  - 시간 상한 `ANALYSIS_RECOMPUTE_MAX_MS = 8 × 60 × 1000`. `web-db.yml` 의 작업 제한이 `timeout-minutes: 10`(25행)이라 그보다 2분 짧게 둔다. 배치마다 시작 전에 `now() − 시작 ≥ ANALYSIS_RECOMPUTE_MAX_MS` 이면 멈추고 남은 수를 출력하며 성공 코드로 끝난다(다시 실행하면 이어진다). 배치를 100 으로 줄인 것은 한 배치가 상한 뒤 남은 2분 안에 끝나게 하려는 것이다. Actions 가 작업을 강제 종료하는 경로는 이 상한으로 피하고, 강제 종료되면 출력은 앞서 찍힌 숫자 줄까지만 남는다(숫자만 출력 규칙은 깨지지 않는다).
- `.github/workflows/web-db.yml`: `options` 에 `analysis-recompute-count`, `analysis-recompute` 를 더하고 설명 문장에 추가, 단계 3개(`count` → `apply`(쓰기 action 일 때만) → `count`)를 `dan-recompute` 와 같은 모양으로. `timeout-minutes` 는 바꾸지 않는다(다른 action 과 같은 작업을 쓴다).
- 읽기 action(`analysis-recompute-count`)은 메인이 실행, 쓰기 action 은 사용자 `!`(HANDOFF 5절).

---

## 3. 훈련곡 생성 명세

### 3.1 흐름

1. 사용자가 `/ai?tab=training` 에서 초점(`auto` 또는 프로필 약점 키)과 강도(`build | push | over`)를 고르고 "만들기" 를 누른다. 키가 없으면 버튼은 막히고 `AiSetupNotice` 가 등록을 안내한다.
2. `POST /api/fe/me/ai/training-charts`(본문 `trainingChartCreateSchema`, `dto/training.ts`: `z.object({ focus_key: z.union([z.literal('auto'), weaknessKeySchema]), intensity: z.enum(TRAINING_INTENSITIES) }).strict()`, 본문 상한 `TRAINING_LIMIT.WRITE_MAX_BYTES = 1024`, 어긋나면 400 `VALIDATION_ERROR`): 인증 → 보관 수 확인(20곡 이상이면 409 `TRAINING_CHART_LIMIT`, 화면은 지우기 목록을 연다) → 폴더 확인(5.3, 409 `TRAINING_FOLDER_UNAVAILABLE`) → 프로필(2.5)이 `PROFILE_MIN_PLAYS` 미만이면 422 `AI_NOT_ENOUGH_DATA`(need `traced_plays`) → `focus_key` 가 `auto` 가 아니면 프로필 `weaknesses` 에 그 키가 있고 `training_kinds` 가 비지 않았는지(아니면 422 `AI_NOT_ENOUGH_DATA`, need `weakness_profile`), `auto` 면 `training_kinds` 가 있는 약점이 하나 이상인지(아니면 같은 422) → digest 만들기(3.2) → `runAiFeature`(feature `training_plan`) → `refineTrainingPlanOutput`(3.4) → 서버 보정(3.5) → 합성(3.6) → BMS 작성(3.7) → 해시 → 패턴 문서(3.10) → 저장(4절, 트랜잭션) → 폴더 항목(5.3) → 패턴 등록 → 응답(상세 뷰).
3. 같은 요청에서 공급자 호출은 1회. 자동 재시도 없음(ADR Q10). 라우트 어댑터 `maxDuration = 300`.

### 3.2 입력 digest (`trainingPlanDigestOf(profile, history, request)`)

`web/src/server/service/domain/ai/training-plan.service.ts`:

```typescript
{
  request: { focus_key: string, intensity: 'build' | 'push' | 'over' },
  window: { plays, charts, days },
  density: { comfort_nps, limit_nps_ex, limit_nps_bp, max_seen_nps, peak_seen_2s, sustained_minus_burst },
  weaknesses: { key, kind, value, evidence, sample_level, trend_delta, training_kinds }[],   // 상위 8, low 이상만
  strengths: { key, value }[],                                                               // 상위 3
  timing: { key, mean_ms, fast_share }[],                                                    // timing 가족 strong 상위 3
  context: { key, lift }[],                                                                  // bp_context·gauge_context 상위 3
  hands: { key, value }[],
  frontier_level: string | null,                                                             // 발광 표 프런티어(기존 recommend 결정론)
  previous: { created_at, focus_keys: string[], kinds: string[], result_delta: number | null }[],  // 최근 3곡
  bounds: { nps_min, nps_max, bpm_min, bpm_max, sections_min, sections_max, length_s },
  allowed: { kinds: TrainingPatternKind[], lane_hints: TrainingLaneHint[], keys: string[] },  // keys = weaknesses 키 + 'warmup' + 'cooldown'
}
```

- `focus_key` 가 `auto` 가 아니면 digest 의 `weaknesses` 를 그 키 하나와 관련 조합으로 줄이고 `allowed.keys` 도 그 키 + `warmup`/`cooldown` 만.
- `bounds` 는 3.5 의 NPS 범위를 미리 계산해 준다. LLM 이 범위를 지키지 않아도 서버가 클램프한다.
- 캐시 키 `subject_key = train:<focus_key>:<intensity>`. `focus_key` 가 3.1 에서 `auto` 또는 64자 이하 키로 검사되므로 최대 6 + 64 + 1 + 5 = 76자로 `ai_result.subject_key varchar(128)`(`schema.ts:582`)에 들어간다. 같은 digest 면 같은 구성안이 캐시에서 나오지만 시드가 생성 시각에 묶이므로(3.6) 곡은 매번 다르다. 캐시 적중은 호출 수를 세지 않는다.

### 3.3 LLM 출력 스키마 (`web/src/server/dto/training.ts`)

공급자 공통 제약(01 §6.2 `json-schema.ts`): 널 유니온 금지, 모든 속성 required, `additionalProperties: false`, 깊이 10 이하, 수치 범위·배열 최대 길이는 공급자 스키마에서 빠지므로 Zod 재검증과 서버 보정으로만 강제한다.

```typescript
export const TRAINING_PATTERN_KINDS = [
    'stream', 'chord_stream', 'trill', 'stair', 'jack', 'chord_jack',
    'scratch_solo', 'scratch_mixed', 'scratch_run', 'ln_hold', 'ln_release', 'burst', 'rest',
] as const
export const TRAINING_LANE_HINTS = ['all', 'left', 'right', 'center', 'wide', 'adjacent'] as const
export const TRAINING_INTENSITIES = ['build', 'push', 'over'] as const

export const aiTrainingPlanOutputSchema = z.object({
    description: z.string().trim().max(TRAINING_LIMIT.DESCRIPTION_MAX),
    base_bpm: z.number(),
    sections: z
        .array(
            z.object({
                focus_key: z.string(),
                kind: z.enum(TRAINING_PATTERN_KINDS),
                weight: z.number(),
                target_nps: z.number(),
                lane_hint: z.enum(TRAINING_LANE_HINTS),
                chord_size: z.number(),
                scratch_ratio: z.number(),
                ln_ratio: z.number(),
                bpm_ratio: z.number(),
                intent: z.string().trim().max(TRAINING_LIMIT.INTENT_MAX),
            }),
        )
        .min(1)
        .max(TRAINING_LIMIT.SECTIONS_MAX),
    advice: z.string().trim().max(TRAINING_LIMIT.ADVICE_MAX),
})
```

`TRAINING_LIMIT = { CHARTS_PER_USER: 20, DESCRIPTION_MAX: 255, INTENT_MAX: 100, ADVICE_MAX: 300, SECTIONS_MIN: 3, SECTIONS_MAX: 12, NOTE_MAX: 100, WRITE_MAX_BYTES: 1024 }`. 깊이는 루트 → sections 배열 → 항목 객체 → 필드로 4단이다.

- LLM 은 곡 제목을 쓰지 않는다. 공개되는 `#TITLE` 은 서버가 정한다(3.7). LLM 이 쓴 `description`·`intent`·`advice` 는 사용자의 약점 digest 를 바탕으로 한 문장이라 비공개 필드(`training_chart.request`, 소유자 전용 API 7.2, 폴더 항목 `note`)에만 둔다.

- 기능 상수(`training-plan.service.ts`): `AI_TRAINING_PLAN_FEATURE = 'training_plan'`, `TRAINING_PLAN_PROMPT_VERSION = 1`, `TRAINING_PLAN_MAX_OUTPUT_TOKENS = 8000`, `TRAINING_PLAN_EFFORT = 'low'`.
- 시스템 프롬프트(출력 언어 인자 `Locale`): `DIGEST_IS_DATA_RULE`, `DIGEST_KEY_RULE`, "구간의 `focus_key` 는 `allowed.keys` 중 하나", "`kind` 는 그 키의 `training_kinds` 중 하나(warmup·cooldown 은 `rest`·`stream`)", "합이 약 60초가 되도록 `weight` 로 비율만 정한다(서버가 길이를 맞춘다)", "`target_nps` 는 `bounds` 안", "앞 구간은 예열, 가운데에 초점 구간을 2회 이상, 마지막은 정리", "`intent` 는 사용자에게 보일 한 줄". `description`·`intent`·`advice` 는 화면 언어로.
- `TRAINING_KINDS_OF_KEY`(약점 키 → 허용 종류, `training-constants.ts`): `tag:jack→[jack, chord_jack]`, `tag:chord_2|3|4plus→[chord_stream, chord_jack]`(chord_size 2/3/4 권장), `tag:trill→[trill]`, `tag:stair→[stair]`, `tag:stream→[stream]`, `tag:scratch_solo→[scratch_solo]`, `tag:scratch_chord|scratch_mixed|with_scratch→[scratch_mixed]`, `tag:scratch_run→[scratch_run]`, `tag:ln_head|ln_end|ln_hold_hit→[ln_hold]`, `tag:ln_release_then_hit→[ln_release]`, `tag:dense→[burst, chord_stream]`, `tag:soflan`·`soflan:*`·`bpm:*`→`[stream, chord_stream]`(bpm_ratio 로 소프란), `tag:mine_near→[stream]`(지뢰는 합성하지 않는다, 3.6), `solo:<t>`·`combo:<a>+<b>`→ 각 태그 목록의 합집합, `density:limit→[stream, chord_stream]`, `density:sustained→[chord_stream, stream]`, `lane:*`·`hand:*`→`[stream, trill, chord_stream]`(lane_hint 로 편향), `timing:<t>`→ 그 태그 목록, `bp_context:<t>`·`gauge_context:<t>`→ 그 태그 목록, `fatigue→[chord_stream, burst]`, `warmup|cooldown→[rest, stream]`.

### 3.4 Zod 재검증과 `refine`

`refineTrainingPlanOutput(output, digest)`:

`refine` 결과는 `ai_result.output` 에 스키마 모양 그대로 저장되고, 캐시 적중 때는 `refine` 없이 저장값을 스키마로 다시 파싱만 한다(`feature-runner.service.ts:102-107, 148-161`). 그래서 `refine` 은 **거절 여부와 글자 다듬기만** 하고, 값을 바꾸는 보정(구간 버리기·종류 바꾸기)은 모두 `correctTrainingPlan`(3.5)이 매번 다시 계산해 `corrections` 에 남긴다. 캐시 적중이어도 보정 기록이 같게 나온다.

1. `finalizeStructuredOutput` 이 원 Zod 스키마로 다시 검증(길이·배열 최대).
2. `focus_key` 가 `allowed.keys` 안인 구간이 `SECTIONS_MIN(3)` 개 미만이면 `null`(→ `output_invalid`, 사용량은 이미 셌다, 기존 규칙).
3. `intent`·`description`·`advice` 를 `fitsLine`·`cutText`(01 §6.1) 로 한 줄·길이에 맞춘다. 구간은 버리거나 바꾸지 않는다.

### 3.5 서버 보정 (`web/src/server/service/domain/training/spec.ts`)

`correctTrainingPlan(plan, { allowed, bounds }) → { spec: TrainingSpec, corrections: TrainingCorrection[] }`(`allowed`·`bounds` 는 3.2 digest 의 같은 이름 필드). 바꾼 값은 모두 `corrections[]` 에 `{ section: number | null, field, from, to }` 로 남겨 화면에 보인다. `section` 은 LLM 출력의 원래 구간 번호(0부터)다.

- 구간 고르기(맨 먼저): `focus_key` 가 `allowed.keys` 밖인 구간을 버리고 `{ section, field: 'focus_key', from: <키>, to: null }` 을 남긴다. `kind` 가 `TRAINING_KINDS_OF_KEY[focus_key]` 밖이면 그 목록의 첫 종류로 바꾸고 `{ field: 'kind', from, to }` 를 남긴다. 남은 구간이 `SECTIONS_MIN` 미만인 경우는 `refine` 이 이미 거절했으므로 여기 오지 않는다(방어로 같은 검사를 두고 던진다).

- 길이: `TRAINING_LENGTH_US = 60_000_000` 고정. 첫 마디(마디 0)는 카운트인(노트 없음, 4분 클릭 4개). 노트는 마디 1 부터.
- BPM: `base_bpm` 을 `[TRAINING_BPM_MIN(100), TRAINING_BPM_MAX(220)]` 로 클램프하고 `TRAINING_BPM_STEP(4)` 의 배수로 반올림(배율 1 에서 60초 = `bpm/4` 마디가 정수가 된다, 02 §4.5). `bpm_ratio` 는 `TRAINING_BPM_RATIOS = [0.5, 0.75, 1, 1.25, 1.5, 2]` 중 가장 가까운 값, 유효 BPM 은 `[TRAINING_EFFECTIVE_BPM_MIN(60), TRAINING_EFFECTIVE_BPM_MAX(300)]` 로 다시 클램프(벗어나면 배율을 1 쪽으로 한 단계씩).
- 구간 길이(마디 수 분배, `distributeMeasures(sections, baseBpm)`): `weight` 를 `[1, 10]` 로 클램프한 `w_i`, 구간 유효 BPM `bpm_i`(위 BPM 보정 뒤). 마디 길이 `M(bpm) = 240_000_000 / bpm`(µs, 실수. 비교에만 쓰고 저장 시각은 3.10 의 `Math.trunc` 누적으로 다시 계산한다).

```
A = TRAINING_LENGTH_US − M(baseBpm) − TRAINING_TAIL_US            // 카운트인 1마디와 꼬리를 뺀 시간
d_i = A × w_i / Σw
m_i = max(TRAINING_SECTION_MIN_MEASURES(1), round(d_i / M(bpm_i)))
total(m) = Σ m_i × M(bpm_i)

// 1단계: 줄이기
while total(m) > A:
    j = m_j > 1 인 구간 중 m_j 가 가장 큰 것(동률이면 번호가 큰 것)
    m_j -= 1

// 2단계: 채우기
repeat:
    added = false
    for i in w 내림차순(동률이면 번호가 작은 것 먼저):
        if total(m) + M(bpm_i) <= A:
            m_i += 1; added = true; break
until not added
```

  - 불변식: (a) 1단계 시작 전 `Σ m_i ≥ n`, 1단계는 `Σ m_i` 를 1씩 줄이므로 유한하게 끝난다. 모든 `m_i = 1` 이 되어도 `total ≤ A` 다: 구간 12개 × 최장 마디 4초(유효 BPM 60) = 48초 < `A` 의 최솟값(60 − 2.4(BPM 100 카운트인) − 1 = 56.6초). 그래서 1단계는 `m_j > 1` 인 구간이 남아 있는 동안에만 돈다. (b) 2단계는 한 번 돌 때마다 `total` 이 늘고 `total ≤ A` 를 지키므로 유한하게 끝난다. (c) 끝나면 `A − min_i M(bpm_i) < total(m) ≤ A`. 즉 곡 길이 `M(baseBpm) + total(m)` 은 `60s − TRAINING_TAIL_US` 이하이고, 그보다 가장 짧은 구간 마디 하나 이내로 짧다. (d) 1단계는 큰 구간부터 줄이고 2단계는 무게가 큰 구간부터 채우므로 구간 사이 마디 비가 `w` 비에서 크게 벗어나지 않는다.
  - 바뀐 `m_i`(반올림 값과 다를 때)는 `{ section, field: 'measures', from, to }` 로 남긴다.
- NPS 범위(`bounds`, 3.2 에도 같은 값): `limit = limit_nps_ex ?? max_seen_nps ?? TRAINING_DEFAULT_LIMIT_NPS(12)`, `comfort = comfort_nps ?? limit × TRAINING_COMFORT_FALLBACK_RATIO(0.7)`. 강도별 `[min, max]`: `build = [comfort × 0.6, comfort]`, `push = [comfort × 0.8, limit]`, `over = [comfort, limit × TRAINING_OVER_RATIO(1.15)]`(계수는 `TRAINING_INTENSITY_RANGE` 상수). 전역 `[TRAINING_NPS_FLOOR(2), TRAINING_NPS_CEIL(40)]` 안으로 다시 자른다. `rest` 는 `[1, TRAINING_REST_NPS_MAX(3)]`, `warmup`/`cooldown` 키 구간은 `[min, comfort]`.
- `chord_size`: 정수 반올림 `[1, 4]`. `chord_stream`·`chord_jack` 은 최소 2, 그 밖은 1 로 고정.
- `scratch_ratio`·`ln_ratio`: `[0, TRAINING_MIX_RATIO_MAX(0.5)]`. `scratch_*` 종류는 `scratch_ratio` 를 무시(종류가 정한다), `ln_*` 는 `ln_ratio` 무시.
- 시드: `seed = parseInt(sha256(`${userId}:${createdAt}:${canonicalJson(spec 보정 결과)}`).slice(0, 8), 16)`. `GENERATOR_VERSION = 1`. 같은 `(spec, seed, GENERATOR_VERSION)` 은 같은 바이트를 낸다(시험으로 고정).

`TrainingSpec`(저장되는 정본, `training_chart.spec`):

```typescript
{
  generator_version: number, seed: number, base_bpm: number, title: string, subtitle: string, play_level: number,
  sections: { index, focus_key, kind, lane_hint, chord_size, scratch_ratio, ln_ratio, bpm, target_nps,
              measure_from, measure_count, from_us, to_us, grid, actual_nps, notes, intent }[],
  corrections: TrainingCorrection[],
}
```

`from_us`·`to_us`·`grid`·`actual_nps`·`notes` 는 합성 뒤 채운다.

### 3.6 결정론 합성기 (`training/synthesize.ts`)

`synthesizeTraining(spec) → TrainingNote[]`, `TrainingNote = { measure: number; num: number; den: number; lane: 0..7; kind: 'normal' | 'ln_head' | 'ln_end' }`. 위치는 유리수 `measure + num/den` 이다(부동소수 누적 없음).

공통

- PRNG: `mulberry32(seed)`(`training/prng.ts`), 구간마다 `mulberry32(seed ^ (index × TRAINING_SECTION_SEED_STRIDE(0x9e3779b1)))` 로 분기해 구간 순서가 바뀌어도 다른 구간이 흔들리지 않게 한다.
- 레인 점유: 레인마다 바쁜 구간 `[start, end]` 목록. 같은 레인·같은 위치 중복 금지(02 §4.1 주의: 나중 것이 덮는다), LN 몸통 안에 같은 레인 노트 금지.
- 레인 후보(`lane_hint`): `all` 0-6, `left` 0-3, `right` 3-6, `center` 1-5, `wide` 0-6 이고 직전 레인과 차 ≥ 3 을 우선, `adjacent` 0-6 이고 차 = 1 을 우선. 스크래치(7)는 후보에 넣지 않고 종류·`scratch_ratio` 로만 놓는다.
- 격자: `TRAINING_GRIDS = [4, 8, 12, 16, 24, 32]`(마디당 칸). 칸 속도 `G × bpm / 240` 칸/초.
- 행 속도 `rows_per_sec = target_nps / notes_per_row`(`notes_per_row` = 종류별 평균: chord 종류는 `chord_size`, `scratch_mixed` 는 1 + 스크래치 비율, 그 밖 1 + `scratch_ratio`).
- 비연속 종류(`chord_stream`, `scratch_solo`, `scratch_mixed`, `ln_hold`, `ln_release`, `burst`, `rest`): `rows_per_sec` 이상인 가장 작은 G 를 고르고 오차 확산(`acc += f; if (acc >= 1) { 칸 사용; acc -= 1 }`, `f = rows_per_sec / 칸속도`)으로 칸을 고른다. 32 로도 모자라면 32 와 최대 행 속도로 줄이고 `corrections` 에 `target_nps` 를 남긴다.
- 연속 종류(`stream`, `trill`, `stair`, `jack`, `chord_jack`, `scratch_run`): 런 안 행은 연이은 칸이어야 분류기가 태그를 붙인다(간격 1/16 이내, 잭은 2/16 이내, 01 §3.2). 런 칸 G = 16(필요 행 속도가 16분 속도를 넘으면 24, 32). 런 길이 `TRAINING_RUN_ROWS = { stream: 16, trill: 8, stair: 7, jack: 4, chord_jack: 3, scratch_run: 4 }`, 주기 `C = ceil(L × 칸속도 / rows_per_sec)` 칸, 주기마다 앞 L 칸에 행, 나머지 비움.

종류별 배치

| 종류 | 규칙 |
| --- | --- |
| `stream` | 행마다 레인 1개, 직전·두 행 전 레인 제외(잭·트릴이 우연히 생기지 않게) |
| `chord_stream` | 행마다 `chord_size` 개 서로 다른 레인, 직전 행 레인과 겹치지 않게(후보가 모자라면 겹침 허용) |
| `trill` | 런마다 레인 쌍 (a, b), a ≠ b(`adjacent` 면 차 1, `wide` 면 차 ≥ 3), ABAB… |
| `stair` | 런마다 시작 레인과 방향(+1/−1), 끝에 닿으면 방향을 바꾸지 않고 런을 끝낸다(분류기가 되돌림을 둘로 나누므로, 01 §3.2) |
| `jack` | 런마다 레인 하나를 4행 반복, 다음 런은 다른 레인 |
| `chord_jack` | 런마다 `chord_size` 레인 묶음을 3행 반복 |
| `scratch_solo` | 고른 칸에 스크래치만. 건반은 `TRAINING_SCRATCH_SOLO_KEY_SHARE(0.25)` 비율로 다른 칸에 단타 |
| `scratch_mixed` | 건반 단타 행 + 그 행의 1칸(16분) 뒤 또는 같은 칸에 스크래치(번갈아) |
| `scratch_run` | 연속 칸 스크래치 4개 런 + 런 사이 칸에 건반 단타 |
| `ln_hold` | 한 손(왼 0-2 / 오른 4-6 을 번갈아) 레인에 LN(길이 `TRAINING_LN_HOLD_BEATS(2)` 박), 그동안 반대 손 레인에 단타 행 |
| `ln_release` | LN(길이 1박) 끝 1칸(16분) 뒤 같은 손 다른 레인에 단타, 반복 |
| `burst` | 1박 밀집(`rows × TRAINING_BURST_HIGH(1.6)`) + 1박 성김(`× TRAINING_BURST_LOW(0.4)`) 번갈아, 평균이 목표 |
| `rest` | 4분 단타, 레인 순환 |

덧붙이기

- `scratch_ratio`(스크래치 종류 밖): 행마다 확률로 같은 칸에 스크래치를 더한다(스크래치 레인이 비었을 때).
- `ln_ratio`(LN 종류 밖): 단타를 확률로 LN 으로 바꾼다. 길이는 2칸~1박(PRNG), 같은 레인의 다음 노트 전에 끝나야 하고 아니면 바꾸지 않는다.
- 지뢰는 만들지 않는다(`mine_near` 약점은 일반 연타로만 훈련, 비목표에 가깝다고 화면에 안내).
- 7키 판정: 곡 전체에 레인 5 또는 6(채널 18·19) 노트가 하나도 없으면 마지막 구간 마지막 행에 레인 6 단타를 더한다(02 §4.2: 18·19 중 하나라도 있어야 7키).
- 마지막 노트는 `60s − TRAINING_TAIL_US` 이전이어야 한다(넘는 노트는 버림).

`play_level` = 구간 `actual_nps` 의 노트 가중 최댓값을 반올림한 정수, `[1, TRAINING_PLAY_LEVEL_MAX(25)]`(정직한 밀도 표기, 난이도표 레벨과 무관하다는 것을 상세 화면에 적는다).

### 3.7 BMS 작성기 (`training/bms-writer.ts`)

`writeTrainingBms(spec, notes) → { text: string; bytes: Uint8Array; lines: WrittenLine[] }`.

- 헤더(이 순서, 줄 끝 `\n`, UTF-8 BOM 없음, 마지막 줄도 `\n`):
  `#PLAYER 1`, `#GENRE R-BMS TRAINING`, `#TITLE <title>`, `#SUBTITLE [<id 앞 8자>]`, `#ARTIST R-BMS`, `#BPM <base_bpm>`, `#PLAYLEVEL <play_level>`, `#RANK 2`, `#TOTAL <total>`, `#LNMODE 1`, `#WAV01 hit.wav`, `#WAV02 count.wav`, 그리고 BPM 변화가 있으면 `#BPM01 <bpm>`… (값 오름차순으로 번호).
- `title = TRAINING_TITLE_PREFIX('R-BMS Training · ') + <만든 날 UTC YYYY-MM-DD> + ' · ' + TRAINING_INTENSITY_TITLE[intensity]`(`TRAINING_INTENSITY_TITLE = { build: 'BUILD', push: 'PUSH', over: 'OVER' }`, 예 `R-BMS Training · 2026-11-04 · PUSH`). 서버가 정하는 고정 형식이고 초점(약점) 이름도 LLM 문장도 넣지 않는다. 판이 랭킹·검색에 들어가 다른 사용자가 제목을 보므로(D4) 제목으로 약점이 드러나지 않게 한다. 같은 날 여러 곡은 `#SUBTITLE` 의 id 로 구별된다. 초점 라벨은 비공개인 폴더 항목 `note`(5.3)와 웹 상세에서 보인다. `#SUBTITLE` 에 id 를 넣어 md5 가 사용자·곡마다 다르다(`training_chart.md5` unique).
- `#TOTAL = max(TRAINING_TOTAL_MIN(160), round(7.605 × N / (0.01 × N + 6.5)))`(N = 플레이 노트 수, 계수는 상수 `TRAINING_TOTAL_COEFFICIENTS`).
- 채널 표(02 §4.2): 레인 0-6 → `11,12,13,14,15,18,19`, 스크래치 7 → `16`; LN 은 `51,52,53,54,55,58,59,56`(5x 쌍: 같은 레인 첫 토큰 머리, 다음 끝). 노트 토큰은 모두 `01`(hit.wav 키음), LN 끝 토큰도 `01`. 카운트인: `#00001:02020202`(BGM 채널 01, count.wav).
- BPM 변화: 구간 시작 위치에 채널 `08` 토큰(`#BPMxx` 참조, 16진 두 자리 번호).
- 줄: 마디 오름차순, 그 안 채널 문자열 오름차순. 한 줄의 해상도 N = 그 마디·채널 노트 위치 분모의 최소공배수(상한 `TRAINING_MAX_LINE_RESOLUTION = 192`, 격자가 `TRAINING_GRIDS` 라 최대 96). 토큰 i 의 위치는 i/N(02 §4.1).
- 마디 번호는 십진 3자리(최대 999, 1분 곡은 최대 약 76마디).
- `WrittenLine = { measure, channel, resolution, tokens }` 를 돌려 3.10 의 시각 계산이 **쓴 그대로의 N** 으로 위치를 계산하게 한다.
- 해시: `bytes` 를 그대로 `md5`, `sha256`(node:crypto). 재인코딩 금지(02 §5.3).

### 3.8 클릭 wav (`training/wav.ts`)

`createClickWav({ frequencyHz, durationMs, decayMs }) → Uint8Array`: RIFF/WAVE, PCM(`audioFormat 1`), 모노, 16비트, `TRAINING_WAV_SAMPLE_RATE = 44_100`. 샘플 `round(TRAINING_WAV_AMPLITUDE(0.6) × 32767 × sin(2π f i / sr) × exp(−i / (sr × decayMs / 1000)))`, 마지막 `TRAINING_WAV_FADE_SAMPLES(32)` 샘플은 선형으로 0 까지.

| 파일 | 상수 | 값 |
| --- | --- | --- |
| `hit.wav`(노트 키음) | `TRAINING_HIT_WAV` | 2,000 Hz, 30 ms, 감쇠 8 ms(약 2.7 KB) |
| `count.wav`(카운트인) | `TRAINING_COUNT_WAV` | 1,000 Hz, 60 ms, 감쇠 20 ms(약 5.3 KB) |

키음 파일은 `#WAV` 이름을 곡 폴더에서 찾고 못 찾으면 조용히 버린다(02 §4.4). 이름은 zip 안 이름과 같아야 한다.

### 3.9 zip 작성기 (`training/zip.ts`, `training/crc32.ts`)

02 부록 B 에서 Rust `extract_zip` 통과를 확인한 방식 그대로:

- 로컬 헤더(`0x04034b50`, 버전 20, 플래그 `0x0800`(UTF-8 이름), 방식 8(deflate, `node:zlib.deflateRawSync`) 또는 0(저장), CRC-32, 압축·원본 크기, 이름 길이, 확장 0) + 데이터 → 중앙 디렉터리(`0x02014b50`) → 끝 레코드(`0x06054b50`). 데이터 기술자·확장 필드·ZIP64 없음.
- 날짜·시간은 고정 DOS `1980-01-01 00:00`(날짜 `0x0021`, 시간 0). 같은 입력은 같은 zip(같은 런타임 안에서).
- CRC-32 는 256칸 표 직접 구현(`node:zlib.crc32` 는 타입·런타임 확인이 안 돼 쓰지 않는다, 02 §3.2).
- 항목(zip 뿌리, 폴더 없음): `rbms-training-<id 앞 8자>.bms`(deflate), `hit.wav`(저장), `count.wav`(저장). 클라이언트가 `{다운로드 폴더}/{safe_folder_name(제목, md5)}` 로 놓는다(02 §2.2).
- `createTrainingZip({ id, bms, bmsMethod = 'deflate' }) → Uint8Array`(`bmsMethod: 'deflate' | 'store'`). 운영 다운로드는 `deflate`, 골든 픽스처는 `store` 로 만든다(저장 방식은 런타임의 zlib 구현과 무관하게 바이트가 고정된다). wav 두 개는 모듈 안에서 한 번 만들어 재사용.
- **TS 가 쓴 zip → Rust `extract_zip` 경계 시험**(저장소 시험으로 지킨다):
  - 골든 `contract/training-chart/basic-160.zip`: `basic-160.bms` 를 `bmsMethod: 'store'` 로 담은 zip. TS 시험(`golden.test.ts`)이 `createTrainingZip` 결과 바이트가 이 파일과 같은지 비교하고 `RBMS_UPDATE_CONTRACT=1` 이면 다시 쓴다.
  - Rust 시험 `crates/rbms-download/src/tests/extract.rs` 에 `a_training_chart_zip_from_the_server_is_unpacked`: 이 파일을 `include_bytes!` 로 읽어 기존 `unpack(tag, archive)` 도우미로 풀고, 차트 1개·wav 2개가 나오며(보고서의 개수), 풀린 `.bms` 바이트가 `contract/training-chart/basic-160.bms` 와 같다(바이트가 같으면 md5 도 같다. 새 의존성 없음). `is_kept_extension` 이 `wav` 를 남기는지는 구현 때 확인(02 §2.2).
  - deflate 변형은 TS 시험이 `node:zlib.inflateRawSync` 로 풀어 내용·CRC-32 가 원본과 같은지만 본다(압축 바이트는 런타임에 따라 다를 수 있다, 부채 (6)).

### 3.10 패턴 문서와 검증 (골든 픽스처)

- `training/chart-timing.ts`: Rust 와 같은 연산 순서로 시각을 계산한다(02 §4.5, 부록 A). 타임라인 위치 집합 = 모든 마디 시작(0..마지막 마디) ∪ 쓴 줄의 모든 토큰 위치(`measure + i/N` 를 double 로) ∪ BPM 변화 위치. 정렬해 `time += Math.trunc(240_000_000 × (section − previous) / bpmOfPrevious)` 누적, STOP 은 쓰지 않는다.
- `training/pattern-document.ts`: `buildTrainingPatternDocument({ spec, lines, md5, sha256 }) → ChartPatternDocument`(`web/src/server/dto/chart-pattern.ts` 스키마로 검증). `format 'rbms-chart-pattern'`, `version 1`, `parser_version = TRAINING_PARSER_VERSION(5)`, `mode 'BEAT_7K'`, `declared_ln 'ln'`, `has_undefined_ln false`, `uses_control_flow false`, `init_bpm`, `total`, `rank 2`, `play_level`, `playable_notes`(일반 + LN 머리, LN 모드라 끝은 세지 않는다, 02 §4.3), `notes{ time_us, lane, kind(0 일반, 1 머리, 2 끝), section }`(시각 다음 레인 순), `tempo[{ t_us, bpm, stop_us: 0, scroll: 1 }]`(BPM 변화 점만, 없으면 빈 배열).
- 골든 픽스처 폴더 `contract/training-chart/`(공개 저장소, 손으로 쓴 사양만):
  - `basic-160.spec.json`(BPM 160, stream·trill·stair·jack·chord_stream·rest 6구간), `soflan-ln.spec.json`(BPM 150, 배율 1.5 구간, ln_hold·ln_release), `scratch-mix.spec.json`(BPM 176, scratch_solo·scratch_mixed·scratch_run·burst). 각 `.spec.json` 은 보정이 끝난 `TrainingSpec` 과 고정 `id`(`tc_` + 0 32개 같은 시험용 값)다.
  - 각각의 `.bms`(TS 합성·작성 결과 바이트)와 `.pattern.json`(Rust `extract` 결과), 그리고 `basic-160.zip`(3.9, 저장 방식).
- TS 시험 `web/tests/service/training/golden.test.ts`: 사양 → 합성 → 작성 바이트가 `.bms` 와 같고, `buildTrainingPatternDocument` 결과가 `.pattern.json` 과 깊은 비교로 같다. `RBMS_UPDATE_CONTRACT=1` 이면 `.bms` 를 다시 쓴다.
- Rust 시험 `apps/rbms-cli/src/chart_pattern/training_contract_tests.rs`(`chart_pattern.rs` 에 `#[cfg(test)] mod training_contract_tests;`): 각 `.bms` 를 `extract(bytes, path, Mode::BEAT_7K)` 해 `serde_json::to_value` 가 `.pattern.json` 과 같다. `RBMS_UPDATE_CONTRACT=1` 이면 `.pattern.json` 을 다시 쓴다(기존 `contract_requests.rs` 의 `UPDATE_ENV` 관례).
- 두 시험이 같은 파일을 읽어 "TS 가 계산한 시각 = 클라이언트 파서가 읽은 시각" 을 웹 CI(Rust 없음)와 Rust CI 양쪽에서 지킨다. `PARSER_VERSION` 이 오르면 Rust 쪽 갱신 뒤 TS 시험이 깨져 `TRAINING_PARSER_VERSION` 을 함께 올리게 된다.
- 생성 직후 서버가 `chartPatternService.upload({ document, uploadedBy: userId })` 로 `chart_pattern` 을 **직접 저장한다**(결정). 그래야 훈련곡 판의 trace 가 `matched` 로 이어지고 곡 특징(2.3)이 생겨 추천·통계와 같은 언어로 묶인다. 실패해도 생성은 성공으로 두고(판 분석은 trace 만으로 된다) 경고를 로그(코드만)에 남긴다.

---

## 4. DB 마이그레이션 0014

`web/src/server/db/schema.ts` 의 `aiPracticeFolder` 정의 뒤(670행 부근)에 세 표를 더하고 `cd web && bun run db:generate -- --name training_chart` 로 `web/drizzle/0014_training_chart.sql`, `web/drizzle/meta/0014_snapshot.json`, `_journal.json`(idx 14, `when` 단조 증가)을 만든다. `drizzle-kit push` 금지. `score` 표는 바꾸지 않는다(D4: 제외 조건 없음).

### 4.1 `training_chart`

| 열 | 타입 | 제약 | 뜻 |
| --- | --- | --- | --- |
| `id` | `varchar(36)` | PK | `tc_` + `crypto.randomBytes(16)` 16진 32자(128비트, 추측 불가) |
| `user_id` | `varchar(36)` | NOT NULL, FK `user.id` ON DELETE CASCADE | 만든 사용자 |
| `md5` | `char(32)` | NOT NULL, unique `training_chart_md5_uniq` | BMS 바이트 md5 |
| `sha256` | `char(64)` | NOT NULL, 색인 `training_chart_sha256_idx` | BMS 바이트 sha256 |
| `title` | `varchar(200)` | NOT NULL | `#TITLE` 과 같다 |
| `play_level` | `int` | NOT NULL | 3.6 |
| `bpm` | `int` | NOT NULL | 기준 BPM |
| `note_count` | `int` | NOT NULL | 플레이 노트 수 |
| `length_ms` | `int` | NOT NULL | 마지막 노트 시각 |
| `generator_version` | `int` | NOT NULL | `GENERATOR_VERSION` |
| `spec` | `json` | NOT NULL | `TrainingSpec`(3.5) |
| `request` | `json` | NOT NULL | `{ focus_key, intensity, plan(정제된 LLM 출력), advice, description }` |
| `profile_snapshot` | `json` | NOT NULL | `{ profile_version, fingerprint, computed_at, targets: { [focus_key]: { value, unit, notes, charts, evidence } } }`(7절 전후 비교의 "전") |
| `bms` | `mediumblob` | NOT NULL | BMS 원 바이트(약 6 KB, gzip 안 함) |
| `provider` | `varchar(16)` | NOT NULL | |
| `model` | `varchar(128)` | NOT NULL | |
| `created_at` | `bigint` | NOT NULL | ms |

색인: `training_chart_user_created_idx (user_id, created_at)`.

### 4.2 `training_folder`

| 열 | 타입 | 제약 |
| --- | --- | --- |
| `user_id` | `varchar(36)` | PK, FK `user.id` CASCADE |
| `folder_id` | `varchar(36)` | NOT NULL, FK `user_folder.id` CASCADE |
| `created_at` | `bigint` | NOT NULL |

사용자의 "훈련곡" 폴더 하나를 가리킨다(이름이 바뀌어도 찾는다, `ai_practice_folder` 와 같은 방식).

### 4.3 `play_profile`

| 열 | 타입 | 제약 |
| --- | --- | --- |
| `user_id` | `varchar(36)` | FK `user.id` CASCADE |
| `window_key` | `varchar(16)` | `recent90`(지금은 하나) |
| `profile_version` | `int` | NOT NULL |
| `fingerprint` | `char(64)` | NOT NULL |
| `data` | `json` | NOT NULL |
| `computed_at` | `bigint` | NOT NULL |

PK `(user_id, window_key)`.

### 4.4 한도·삭제

- 한도 20곡: 생성 요청의 첫 단계에서 `count(*) where user_id = ?`(공급자 호출 전, M18). 이 검사만으로는 동시 요청을 막지 못한다. AI 동시 실행 슬롯은 근거가 되지 못한다: 같은 digest 면 `runAiFeature` 가 사용량·슬롯 확보 전에 캐시 결과를 돌려주고(`feature-runner.service.ts:102-107`, 같은 탭 두 번 누르기가 정확히 이 경로), 슬롯 상한은 운영자 설정으로 5 까지 오른다(`dto/ai.ts:46,52`).
- 그래서 저장 트랜잭션(`compose/training.ts` 의 `insertTrainingChart`)은 첫 문장에서 그 사용자 행을 잠근다: `tx.select({ id: user.id }).from(user).where(eq(user.id, userId)).for('update')`(같은 저장소의 선례 `compose/folder.ts:36` 의 `.for('update')`). 잠근 뒤 `count(*) from training_chart where user_id = ?` 를 하고 `>= TRAINING_LIMIT.CHARTS_PER_USER` 면 아무것도 쓰지 않고 `{ status: 'limit' }` 를 돌려 서비스가 409 `TRAINING_CHART_LIMIT` 을 낸다. 아니면 같은 트랜잭션에서 `training_chart` 를 삽입하고 커밋한다. 폴더 단계(5.3)는 커밋 뒤 폴더 서비스로 한다(폴더 규칙을 compose 에 다시 쓰지 않으려는 것. 폴더를 쓸 수 있는지는 공급자 호출 전에 이미 확인했다. 그래도 폴더 단계가 실패하면 생성은 성공으로 두고 오류 코드만 로그에 남기며, 다음 생성의 `ensureTrainingFolder` 가 빠진 항목을 채운다). 같은 사용자의 생성 저장이 이 잠금으로 직렬화되므로 두 트랜잭션이 모두 19 를 읽고 삽입하는 일은 없다. 다른 사용자끼리는 막지 않는다.
- 한도 초과로 저장이 거절될 때 공급자 호출이 이미 쓰였을 수 있다(캐시 적중이면 쓰이지 않는다). 화면 문구(`pages.ai.training.limitRaced`)에 적는다.
- 시험 `tests/db/training.test.ts`: 19곡 상태에서 `insertTrainingChart` 두 개를 `Promise.all` 로 동시에 부르면 정확히 하나만 성공하고 다른 하나는 `limit`, 끝난 뒤 행이 20 개. 서비스 시험(`tests/service/training/training.test.ts`)은 가짜 db 로 `limit` 결과가 409 가 되는지만 본다.
- 삭제(7.3): `training_chart` 행, 그 사용자의 모든 폴더에서 그 md5 항목. 남기는 것: `chart_pattern` 행(이미 분석된 판의 join 을 유지), `chart` 행, 점수·trace·리플레이(기록), 클라이언트가 받은 파일. `training_folder` 는 폴더가 비어도 남긴다.
- 사용자 삭제는 FK CASCADE 로 세 표가 함께 지워진다.
- DB 시험 `web/tests/db/migration-0014.test.ts`(0013 시험 모양): SQL 문장 목록 고정, 표·색인·FK 존재, `user` 삭제 시 CASCADE.

---

## 5. 전달

### 5.1 공개 GET

- 경로 `GET /api/training/[id]/chart.zip`: 어댑터 `web/src/app/api/training/[id]/chart.zip/route.ts`(세그먼트 `chart.zip` 은 고정 이름 폴더), 라우트 팩토리 `web/src/server/route/training.ts` 의 `createTrainingDownloadRoute`.
- `id` 는 `/^tc_[0-9a-f]{32}$/` 가 아니면 404(DB 를 치지 않음). 행이 없으면 404 `TRAINING_CHART_NOT_FOUND`(봉투 없는 짧은 JSON 이 아니라 상태만, 클라이언트는 본문을 보지 않는다: `NotAZip` 대신 HTTP 오류로 끝난다).
- 응답: `Content-Type: application/zip`, `Content-Disposition: attachment; filename="rbms-training-<id 앞 8자>.zip"`, `Content-Length`, `Cache-Control: private, no-store`(결정: 삭제하면 바로 받을 수 없게. 5 KB 라 DB 비용이 작다).
- 레이트리밋: IP 당 분당 `RATE_LIMIT.TRAINING_DOWNLOAD = 30`(`with-rate-limit.ts` 에 상수 추가).
- 런타임 Node(`node:zlib`, `node:crypto`). `cacheComponents: true` 아래 동적 라우트 처리 방식은 구현 때 `web/node_modules/next/dist/docs/` 의 route handler 문서로 확인한다(02 §8).

### 5.2 `download_url`

- 주소는 **요청의 origin** 으로 만든다. 서버가 `NEXT_PUBLIC_APP_URL` 로 자기 주소를 만들지 않는다는 현행 규칙(`docs/web/architecture.md` 6-1, 이 값은 빌드 시점에 박혀 Preview 배포가 운영 주소를 내려 줄 수 있다)을 그대로 지킨다. 경로 도우미 `trainingDownloadPathOf(id) = `/api/training/${id}/chart.zip``(`web/src/server/dto/training.ts` 에서 export, 폴더·훈련곡 두 도메인이 쓰는 공유 상수라 dto 에 둔다), 절대 주소는 `${origin}${trainingDownloadPathOf(id)}`.
- origin 출처: 라우트가 `new URL(request.url).origin` 을 서비스에 넘긴다. Vercel 에서 `request.url` 은 그 배포가 받은 호스트라 Preview 는 Preview 주소, 운영은 운영 주소가 된다.
- 폴더 항목 응답(`dto/folder.ts` `folderItemSchema`)에 `download_url: z.string().optional()` 을 더한다. 그 폴더 주인이 만든 훈련곡이고 origin 이 주어졌을 때만 필드를 넣고, 아니면 **필드를 뺀다**(빈 문자열을 쓰지 않는다). 선택 필드라서 기존 화면 타입 `FolderItem`(`entities/folder.type.ts:31`)과 서버 출력 타입이 양쪽으로 대입되고(`tests/components/folder-contract.test.ts:23-24`), 항목을 `toEqual` 로 비교하는 기존 시험(`tests/route/folders.test.ts:211`)도 그대로 통과한다.
- `compose/folder.ts` `listItems` 를 `leftJoin(trainingChart, and(eq(trainingChart.md5, userFolderItem.md5), eq(trainingChart.userId, userFolder.userId)))` 로 바꿔 `trainingChartId` 를 고른다(폴더 주인 조건은 `userFolder` 조인으로). `FolderItemRow` 에 `trainingChartId?: string | null`(선택이라 기존 메모리 db `tests/support/folder-memory-db.ts` 도 그대로 맞는다).
- 서비스: `createFolderService` 의 deps 는 **바꾸지 않는다**(그래서 서비스를 직접 만드는 `tests/route/folders.test.ts:22`, `tests/support/ai-score-fixture.ts:121` 이 그대로 컴파일된다). `get(params: { userId; folderId; origin?: string })` 에 선택 인자를 더하고 `toFolderItem(row, origin?)` 이 `...(origin && row.trainingChartId ? { download_url: `${origin}${trainingDownloadPathOf(row.trainingChartId)}` } : {})` 로 만든다. origin 없이 부르는 기존 호출부(`practice-folder.service.ts:324`, `widgets/folder/folder-editor-content.tsx:21`)는 필드 없는 지금 모양을 그대로 받는다.
- `route/folder.ts` 의 `detail`(웹 `GET /api/fe/me/folders/[id]`, 125행)과 `readFolder`(네이티브 `GET /api/players/[playerId]/folders/[folderId]`, 197행)가 `origin: new URL(request.url).origin` 을 넘긴다. 옛 클라이언트는 모르는 필드를 무시한다(02 §1.3, `rbms-ir` DTO 에 `deny_unknown_fields` 없음).
- 계약 픽스처 `contract/server-responses/user-folder-detail.json`(새 폴더): 서버가 실제로 내는 두 모양만 담는다. 훈련곡 항목(`download_url` 있음) 1개 + 일반 항목(필드 없음) 1개. 둘 다 `folderDetailSchema` 를 통과하므로 TS 시험 `web/tests/dto/folder-contract.test.ts`(U7)가 이 파일을 `folderDetailSchema` 로 검증하고, 메모리 db 로 만든 서비스 출력(`get({ ..., origin: 'https://example.test' })`)이 같은 모양인지 본다. Rust 시험(U4)은 같은 파일을 `UserFolder` 로 읽어 첫 항목은 주소, 둘째는 `#[serde(default)]` 로 `''` 인지 본다. 빈 문자열 `''` 이 들어온 경우의 Rust 읽기는 Rust 시험 안의 인라인 JSON 으로 따로 본다(서버는 보내지 않지만 다른 서버 구현 대비).
- 웹 폴더 편집 화면의 서버 프리페치(`folder-editor-content.tsx`)는 origin 을 넘기지 않으므로 필드가 없다. 받기 링크는 U11 이 클라이언트 질의(`/api/fe/me/folders/[id]`, 라우트가 origin 을 넘긴다) 결과에 필드가 있을 때만 보인다. 화면 타입 `FolderItem` 에는 U11 이 `download_url?: string`(선택)을 더한다.

### 5.3 "훈련곡" 사용자 폴더

- 생성 직후(4.4 의 저장 커밋 뒤) `ensureTrainingFolder(userId, locale)`: `training_folder` 가 있고 폴더가 살아 있으면 그것, 없으면 새로 만든다(`is_public: false`). 그 다음 그 사용자의 `training_chart` 중 폴더에 없는 md5 를 모두 `created_at` 내림차순으로 맨 앞에 넣는다. **그래서 새로 만든 폴더에는 기존 훈련곡 전체(최대 19곡)와 새 곡이 함께 들어간다.** 사용자가 훈련곡 폴더를 지우면 `training_folder` 가 CASCADE 로 사라지고(4.2), 다음 생성 때 새 곡만 넣으면 남은 훈련곡은 폴더·`download_url` 이 없어 구동기에서 받을 수 없게 되기 때문이다. 20곡 한도라 폴더 2,000곡 제한과 무관하다. 시험: 훈련곡 3곡 → 폴더 삭제 → 생성하면 새 폴더에 4곡이 최신순. 이름은 **만들 때의 화면 언어**로 `TRAINING_FOLDER_NAME = { ko: '훈련곡', en: 'Training', ja: 'トレーニング' }`(`dto/training.ts`), 설명 `TRAINING_FOLDER_DESCRIPTION` 세 언어. 이후 이름을 다시 바꾸지 않는다(클라이언트는 이름을 그대로 보이고 라틴 문자만 대문자로 바꾼다, `user_folders.rs`). 이름이 겹치면 ` YYYY-MM-DD`(UTC) 를 붙인다(연습 폴더 규칙).
- 폴더가 50개라 만들 수 없으면 409 `TRAINING_FOLDER_UNAVAILABLE`(생성 전 단계에서 확인해 공급자를 부르지 않는다).
- 항목: `{ md5, sha256, title, artist: 'R-BMS', note: 초점 키 라벨 + intent 요약(100자) }` 를 `addItems` 로 맨 앞(새 곡이 위)에 넣는다. 삭제 때 `removeItems`. `note` 는 폴더 주인만 읽는 비공개 값이라 초점(약점) 라벨을 여기에 둔다(3.7 의 공개 제목에는 넣지 않는다).
- 폴더의 `updated_at` 이 바뀌므로 클라이언트의 다음 폴더 받기(시작·로그인·F6·Tables R/A)에서 새 항목이 내려온다(02 §1.3).

---

## 6. 클라이언트(Rust) 변경 — 02 §7.3 의 P2

목표: 내 폴더 안에서 라이브러리에 없는 곡을 흐린 줄로 보이고, 그 줄에서 기존 받기 모달을 열어 `download_url` 로 받는다. 받기·풀기·부분 스캔·재맞춤은 기존 기계를 그대로 쓴다(02 §2.2).

| 파일 | 변경 |
| --- | --- |
| `crates/rbms-ir/src/dto/folder.rs` | `UserFolderItem` 에 `#[serde(default)] pub download_url: String`(`///` 영어 문서). 기존 필드와 같은 방식이라 새 `skip_serializing_if` 는 쓰지 않는다 |
| `crates/rbms-table/src/dto.rs` | `TableEntry` 에 `PartialEq, Eq` derive 추가(`ShownFolder` 가 `PartialEq` 라서) |
| `apps/rbms-player/src/user_folders.rs` | `ShownFolder` 에 `pub(crate) missing: Vec<TableEntry>`. `matched()` 가 라이브러리에 없는 항목을 폴더 순서대로 `TableEntry { md5, sha256, level: String::new(), title, artist, url: String::new(), url_diff: download_url }` 로 만든다. `tidied()` 는 `download_url` 을 `tidy_text(.., DOWNLOAD_URL_MAX_CHARS)`(상수 2,048)로 다듬고, `http://`·`https://` 가 아니면 비운다. 저장본(`.ron`)은 `#[serde(default)]` 덕에 옛 파일도 읽힌다 |
| `apps/rbms-player/src/lib.rs` | `MissingRef` 를 enum 으로: `Table { table, level, entry }`, `Folder { folder, entry }`(둘 다 `usize`, `Copy` 유지). 문서 주석 갱신 |
| `apps/rbms-player/src/tablesrc.rs` | `missing_chart(&MissingRef)` 가 `Folder` 면 `self.user_folders.shown.get(folder)?.missing.get(entry)` |
| `apps/rbms-player/src/stage/select/list.rs` | `SelectFolder::UserFolder(id)` 줄: 가진 곡 뒤에 `missing` 을 `SelectItem::Missing(MissingRef::Folder{..})` 로 붙인다. 표 레벨과 같은 조건(`show_missing_charts && filter.keeps_missing_charts() && search 비었음`), 정렬 규칙도 `level_rows` 와 같은 함수로(제목 순일 때 사이에 끼우는 부분을 공용 도우미로 뽑아 두 곳에서 쓴다, 두 번 쓰이므로 추출). `level_rows` 의 `MissingRef { .. }` 생성은 `MissingRef::Table { .. }` 로 |
| `apps/rbms-player/src/stage/select/download_modal.rs` | `DownloadModal::of`: `Table` 이면 지금대로. `Folder` 면 `table_title = user_folders_title()`, `level_title = 폴더 이름`, 새 필드 `source: ModalSource { Table, Folder { training: bool } }`(`training` = `chart_link == DirectZip` 이고 주소 경로가 `TRAINING_DOWNLOAD_PATH_PREFIX("/api/training/")` 를 포함). 안내 문구 분기: 표면 지금 문구, 폴더+훈련곡이면 `NoteTrainingChart`, 폴더+그 밖이면 `NoteFolderChart`(+ 링크 없으면 `NoteFolderNoLink`). 버튼 규칙은 그대로(`GetChart` 는 `DirectZip` 일 때만) |
| `apps/rbms-player/src/text/select/download_modal.rs` | `DownloadModalText` 에 `NoteTrainingChart`, `NoteFolderChart`, `NoteFolderNoLink` 세 언어(영어 원문은 아래), 기존 카탈로그 시험이 덮는다 |
| `apps/rbms-player/src/download.rs` | 바꾸지 않는다(`start_download(&TableEntry)` 를 그대로 쓴다). 받은 뒤 `user_folders::rematch` 가 이미 불린다(02 §2.2-6) |

영어 문구(앱 관례대로 대문자, 한국어·일본어는 용어집 `docs/reference/2026-10-24-i18n-glossary.md` 를 따른다):

- `NoteTrainingChart`: "A TRAINING CHART THE SERVER MADE FOR YOU. GET CHART DOWNLOADS IT WITH ITS CLICK SOUNDS."
- `NoteFolderChart`: "THIS CHART IS IN YOUR FOLDER BUT NOT IN THE LIBRARY."
- `NoteFolderNoLink`: "THE FOLDER HAS NO DOWNLOAD LINK FOR IT. GET IT ELSEWHERE AND RESCAN."

시험(Rust)

1. `rbms-ir`(`crates/rbms-ir/src/contract_tests/folders.rs` 에 추가): `contract/server-responses/user-folder-detail.json` 을 `include_str!` 로 읽어 `UserFolder` 로 풀면 두 항목의 `download_url` 이 (주소, 필드 없음 → `''`). 인라인 JSON 으로 `"download_url":""` 도 `''` 로 읽힌다.
2. `user_folders`: `matched()` 가 없는 항목만 `missing` 에 폴더 순서로 넣고 `url_diff` 가 주소, `rematch` 뒤 라이브러리에 생긴 md5 는 빠진다. `tidied()` 가 `javascript:`·`file:` 주소를 비운다.
3. `list`: 사용자 폴더 줄 = 가진 곡 다음 없는 곡(`Missing`), 설정을 끄거나 검색 중이면 없음, 제목 정렬 때 제목 순으로 끼움.
4. `download_modal`: `Folder` 훈련곡은 `GetChart` 켜짐·안내 `NoteTrainingChart`·`table_title` 이 MY FOLDERS 제목, 링크 없는 폴더 항목은 `GetChart` 꺼짐.
5. 번역 카탈로그 시험(기존 `catalog_problems(DownloadModalText::ALL)`), 영어 문구 골든(기존 `(variant, text)` 목록에 추가).

시험이 들어갈 파일: 1 은 `crates/rbms-ir/src/contract_tests/folders.rs`, 2 는 `apps/rbms-player/src/user_folders/tests.rs`, 3 은 `apps/rbms-player/src/stage/select/user_folder_tests.rs`(사용자 폴더 줄)와 `missing_tests.rs`(없는 곡 줄 공용 도우미), 4 는 `apps/rbms-player/src/stage/select/download_modal_tests.rs`, 5 는 `apps/rbms-player/src/text/select/download_modal.rs` 의 기존 시험 모듈.

컴파일 영향(구조체·enum 모양 변경으로 같이 고쳐야 하는 기존 코드, 확인됨): `apps/rbms-player/src/user_folders/tests.rs:66`(`UserFolderItem` 구조체 리터럴), `apps/rbms-player/src/stage/select/tree_tests.rs:171`(`ShownFolder` 리터럴), `apps/rbms-player/src/stage/tables.rs:843`(`missing.entry` 필드 읽기), `apps/rbms-player/src/tablesrc.rs:1088`(`missing_chart`), `apps/rbms-player/src/stage/select/download_modal.rs:142-144`(`missing.table`·`missing.level` 읽기). `MissingRef` 를 enum 으로 바꾸면 필드 읽기는 모두 `match` 또는 새 도우미 `MissingRef::entry_index()` 로 바꾼다. 시험 출력 문자열(`format!("missing {}", ...)`)은 지금 값이 그대로 나오게 한다.

릴리스: 병합·CI 3 OS 통과 뒤 메인이 `gh workflow run release.yml --ref dev -f bump=patch`(v0.1.10 draft, 한글 노트). 게시는 사용자(D8).

---

## 7. 훈련 결과 회수와 전후 비교

### 7.1 판을 훈련곡에 잇기

- 연결 키는 **sha256** 이다. `score.chart_sha256` 은 NOT NULL 이고 색인 `(user_id, chart_sha256, played_at)` 이 있지만 `score.chart_md5` 는 NULL 허용·무색인이다(`schema.ts:65-66, 117-123`). `training_chart.sha256` 에는 색인이 있다(4.1).
- 점수: `score.chart_sha256 = training_chart.sha256 and score.user_id = training_chart.user_id` 면 그 사용자의 훈련곡 판이다(조회 때 조인, 열 추가 없음). 목록의 `plays`·`best_ex_ratio`·`last_played_at`, 상세의 `TrainingPlay`, 기록 질문 `training` 필드, 주간 요약 `training`, 대시보드 `TRAINING` 표지가 모두 이 조인을 쓴다.
- 판 분석: `compose/trace.ts` 가 `TraceServiceDeps` 에 `trainingSectionsOf(chartSha256: string) => Promise<TrainingSectionSpan[] | null>`(spec 의 `sections` 에서 `{index, kind, focus_key, from_us, to_us}`)를 주고, `trace.service.ts` 의 `analyze`(190-192행)가 `row.chartSha256` 으로 불러 `analyzeTrace({ ..., training })` 에 넘긴다(업로드·재계산 모두 이 함수를 지난다). 패턴 조회(`loadPattern(trace.chart.md5)`)만 지금처럼 md5 다. 요약의 `metrics.training_sections`(2.2.8)가 채워진다. 훈련곡을 지운 뒤 다시 계산하면 `null` 이 된다(기록은 남는다). 이 배선은 U7 이 한다.
- 프로필: 훈련곡 판도 다른 판과 같이 들어간다(D4). 다만 모든 훈련곡 판은 한 군집 `training` 으로 묶여 K·가중에 최대 한 군집분만 보탠다(2.5.2). 서로 다른 훈련곡 20곡으로 한 약점 셀의 표본 등급·유의성을 채울 수 없게 하려는 것이다.
- 옵션: 레인·손 초점(`lane:*`, `hand:*`) 훈련곡은 `lane_hint` 로 특정 레인에 노트를 몰지만, RANDOM 계열(`Off` 가 아닌 `random`) 옵션을 켜면 화면 레인이 바뀌어 그 구간이 의도한 손·레인을 훈련하지 않는다. 그래서 (1) 만들기 폼과 상세에 레인·손 초점이면 `pages.ai.training.optionNotice`("RANDOM 계열 옵션을 끄고 치세요", 세 언어)를 보이고, (2) `TrainingPlay` 에 `random` 을 그대로 보이며, 구간 추이(`ai-training-section-trend.tsx`)는 레인·손 초점 구간에서 `random !== 'Off'` 판을 흐린 점으로 그리고 추세선에서 뺀다. 전후 비교(`BeforeAfter`)의 레인·손 값은 판 요약의 "본 레인" 기준 지표(`trace-analysis.ts` 의 7K 지표는 옵션 적용 뒤 화면 레인)라 RANDOM 판도 물리 레인·손의 실력을 그대로 잰다. 그래서 전후 비교에서는 빼지 않는다.

### 7.2 API

| 경로 | 응답 |
| --- | --- |
| `GET /api/fe/me/ai/training-charts` | `{ charts: TrainingChartSummary[], used, limit, can_create, needs_key: boolean }` |
| `POST /api/fe/me/ai/training-charts` | 본문 `{ focus_key: string, intensity }`, 응답 `TrainingChartDetail`(201) |
| `GET /api/fe/me/ai/training-charts/[id]` | `TrainingChartDetail` |
| `GET /api/fe/me/ai/training-charts/[id]/removal` | `{ removes: { training_chart: 1, folder_items: n }, keeps: { chart_pattern: 0|1, scores: n, traces: n, replays: n } }` |
| `DELETE /api/fe/me/ai/training-charts/[id]` | 같은 모양 + `removed: true` |

- `TrainingChartSummary = { id, title, created_at, focus_keys, kinds, note_count, bpm, play_level, length_ms, download_url, plays, best_ex_ratio, last_played_at }`.
- `TrainingChartDetail = Summary & { description, advice, sections: { index, focus_key, kind, from_ms, to_ms, target_nps, actual_nps, lane_hint, intent }[], corrections, plays: TrainingPlay[], before_after: BeforeAfter[] }`.
- `TrainingPlay = { score_id, played_at, clear, ex_score, ex_ratio, bp, random, sections: { index, ex_ratio, bp, mean_err_ms, fast, slow }[] | null }`(trace 가 없으면 `null`). 최신순 최대 `TRAINING_PLAYS_SHOWN(50)`.
- `BeforeAfter = { focus_key, unit, before: { value, notes, charts, evidence }, after: { value, notes, charts, plays, evidence } | null, delta: number | null }`. `before` 는 `profile_snapshot.targets`, `after` 는 `weaknessProfileService.measureFocus({ userId, keys, sinceMs: created_at })`(생성 뒤 판만, 최대 `PROFILE_MAX_PLAYS`, 표본 조건 못 채우면 `null`).
- 모두 세션 필요, `private, no-store`, 분당 30회. POST 만 `maxDuration = 300`. 어댑터 `web/src/app/api/fe/me/ai/training-charts/route.ts`(GET `list`, POST `create`), `.../[id]/route.ts`(GET `detail`, DELETE `remove`), `.../[id]/removal/route.ts`(GET `removal`).
- **소유자만**: 상세·removal·DELETE 의 조회와 삭제는 모두 `where id = ? and user_id = session.user.id` 로 건다. 다른 사용자의 id(폴더 응답·다운로드 주소로 id 가 노출될 수 있다)는 행이 없는 것과 같게 404 `TRAINING_CHART_NOT_FOUND` 로 답한다(존재 여부를 알리지 않는다). 목록도 `user_id` 로만 거른다. 공개 GET(5.1)만 id 만으로 zip 을 준다. 시험 `tests/route/training.test.ts`: ALICE 의 훈련곡 id 로 BOB 이 상세·removal·DELETE 를 부르면 셋 다 404 이고 행이 남는다.
- 라우트 팩토리: 새 파일 `web/src/server/route/training-charts.ts` 의 `createAiTrainingChartRoutes(resolveDeps: () => AiTrainingChartRouteDeps = compose, waitForRequest: () => Promise<void> = connection, readLocale: () => Promise<Locale> = getLocale)` → `{ list, create, detail, removal, remove }`. 타입 `AiTrainingChartRouteDeps = { authService: Pick<AuthService, 'resolveSessionUser'>; trainingChartService: Pick<TrainingChartService, 'list' | 'create' | 'detail' | 'removal' | 'remove'> }`. 세션·오류 래퍼는 `route/ai.ts` 의 `createAiRouteWrappers` 와 `AiHandler` 를 쓴다. 지금 둘은 export 되지 않으므로(`route/ai.ts:67, 134`) **U6(W2)이 두 선언 앞에 `export` 만 붙인다**(동작 변화 없음). 분당 한도는 `enforceUserRateLimit('ai-training', user.id, RATE_LIMIT.TABLE_RECORDS)`(분당 30, 기존 점수 AI 기능 `route/ai.ts:335` 과 같은 상수), 응답은 `aiPrivateJson`, 실패 변환은 `unwrapAiResult`·`toAiAppError`(이미 export). 공개 다운로드는 같은 디렉터리의 `route/training.ts` `createTrainingDownloadRoute`(5.1).
- `TrainingChartSummary.download_url` 도 라우트가 넘긴 요청 origin 으로 만든다(5.2).

### 7.3 화면

- `/ai?tab=training`(`widgets/ai/ai-training-panel.tsx`): 사용량(`used/limit`), 키 상태, 만들기 폼(`ai-training-create-form.tsx`: 초점 선택 = `auto` + 프로필 약점 중 `training_kinds` 가 있는 것, 강도 3택, 설명 문장), 목록(`ai-training-list.tsx`: 제목·초점·만든 날·판 수·최고 EX·받기 주소 복사·삭제). 20곡이면 만들기 대신 "먼저 지우세요" 안내와 목록.
- 삭제 확인(`ai-training-removal-dialog.tsx`): `removal` 응답으로 "지워지는 것: 훈련곡 1, 폴더 항목 n / 남는 것: 판 n, 리플레이 n, 이미 받은 파일" 을 보이고 확인 뒤 DELETE(HANDOFF 5절 "지우라는 요청은 개수를 보여 확인").
- 상세 `/ai/training/[id]`(`app/(app)/ai/training/[id]/page.tsx`, `widgets/ai/ai-training-detail-content.tsx`): 구간 띠(`features/ai/ai-training-section-strip.tsx`: 구간 길이 비례 CSS 띠, 종류 색 `--chart-*`, 목표/실제 NPS), 판별 구간 EX 추이(`ai-training-section-trend.tsx`: 구간마다 작은 SVG 선, x = 판 순서), 전후 비교(`ai-training-before-after.tsx`: 초점마다 전·후 값과 차, 등급), 보정 목록, 조언. `proxy.ts` 의 `/ai/:path*` matcher 가 이미 세션을 보호한다.
- 문구 접두사 `pages.ai.training.*`.

---

## 8. 대시보드

### 8.1 분기 방식 (03 §8.1 의 A)

- `web/src/app/(app)/page.tsx`: 격자 안을 `<Suspense fallback={<HomeBranchSkeleton />}><HomeBranch /></Suspense>` 로 바꾼다. `HomeIntroPanel` 은 손님 쪽으로 옮긴다.
- `web/src/widgets/home/home-branch.tsx`(서버, `import 'server-only'`): 세션 쿠키가 없으면 DB 없이 손님. 있으면 `readSessionUser()` 를 try/catch(`table-snapshot-session.tsx:12-18` 선례), 사용자면 `<MemberDashboardContent user={user} />`, 아니면 `<HomeIntroPanel /><HomeContent />`(지금 손님 홈 그대로). 쿠키 이름 확인은 `proxy.ts` 가 쓰는 함수·상수를 재사용한다(구현 때 확인).
- `HomeBranchSkeleton`: 손님·회원 어느 쪽에도 맞는 중립 스켈레톤(패널 4개 높이).
- 클라이언트 패널은 `useSessionUser()` 와 서버가 준 `initialUserId` 를 `table-snapshot-workspace.tsx:88` 방식으로 맞추고, 어긋나면(다른 탭에서 로그아웃) 스켈레톤을 보인다(`useAlignedViewer(initialUserId)`, `entities/dashboard.query.ts` 에 export).
- 로그인 후 `window.location.assign('/')`, 로그아웃은 `router.refresh()` 라 서버 분기가 다시 돈다(03 §3.2).
- `ShellPage` 제목은 `pages.route.home.title` 그대로(정적 셸), 회원 대시보드 안 첫 패널이 "내 대시보드" 를 말한다.

### 8.2 섹션 목록과 데이터 원천

격자는 기존 `DASHBOARD_GRID_CLASS_NAME`(1/2/3/4열). 너비: 한 칸, 넓음 = `DASHBOARD_WIDE_PANEL_CLASS_NAME`(62rem 부터 2칸), 꽉 = `DASHBOARD_FULL_PANEL_CLASS_NAME`. 순서는 왼쪽 위부터 읽는 순서다.

| # | 패널(파일 `widgets/home/`) | iidx-rank 대응 | 보이는 것 | 데이터 원천 | 로딩 |
| --- | --- | --- | --- | --- | --- |
| 1 | `member-profile-panel.tsx` | 내 정보 | 이름, `@login_id`, 대표 단위 배지(A-단위 점선), 종합 순위 `N위 / 전체`, 마지막 플레이 시각, 내 페이지·설정 링크 | 기존 `usePlayer`, `useMyDans` + 새 `overview.rank` | 서버 프리페치(`usePlayer`, overview) |
| 2 | `member-summary-panel.tsx`(넓음) | (없음) | 타일 8개: 총 판, 플레이 곡, 클리어 곡, HARD 이상, FULL COMBO 이상, 30일 플레이 시간, 연속 플레이 일(현재/최고), 이번 주 판 | 새 `overview.tiles` | 서버 프리페치 |
| 3 | `member-lamp-panel.tsx` | 램프 현황 + DJ 랭크 분포 | `LampDistribution`(플레이한 곡 기준, A-SCR 곡 수) + 등급 분포 세로 막대 9개(F, E, D, C, B, A, AA, AAA, MAX) | 기존 `usePlayerStats` + 새 `overview.grades` | 서버 프리페치 |
| 4 | `member-table-progress-panel.tsx`(넓음) | 랭크별 램프 | 표 3종 토글(insane1, stella-st, stella-sl), 전체 막대 + 레벨별 램프 누적 막대·`played/total`·클리어 수 | 새 `GET /api/fe/me/dashboard/tables/[tableId]/progress` | 클라이언트(첫 표만 즉시) |
| 5 | `member-calendar-panel.tsx`(넓음) | (없음) | 26주 활동 달력(일별 판 수 5단계 농도, KST), 연속 일, 활동 일 수 + 아래 30일 플레이 시간 막대(`PlayDayChart`) | 새 `GET /api/fe/me/dashboard/calendar?days=182` + 기존 `usePlayerPlayDays` | 서버 프리페치(일별), 달력은 클라이언트 |
| 6 | `member-recent-panel.tsx`(넓음) | 최근 플레이 20곡 | 판당 20행 `ScoreRow`(A-SCR, DJ 등급 라벨(`ex_score`·노트 수 → `getRateStep` → `shared/lib/dj-rank.ts` 라벨, 노트 수가 없으면 생략), 훈련곡이면 `TRAINING` 표지: 훈련곡 목록의 sha256 과 화면에서 대조), "전체 보기" → `/players/{login_id}` | 기존 `usePlayerRecent({page:1, limit:20})` | 서버 프리페치 |
| 7 | `member-dan-course-panel.tsx` | (단위) | 체계별 보유·표시 단위, 코스 묶음별 클리어 `n/전체`, 최근 코스 기록 5 | 기존 `useMyDans` + 새 `GET /api/fe/me/dashboard/courses` | 클라이언트 |
| 8 | `member-weakness-panel.tsx` | 노트레이더 자리 | 약점 상위 3(키 라벨·값·등급), 밀도 한계 미니 곡선(2.7 의 차트 작은 판), "약점 프로필 보기" | 프로필 `GET /api/fe/me/ai/profile` | 클라이언트, 뷰포트 진입 뒤 |
| 9 | `member-training-panel.tsx` | (없음) | 최근 훈련곡 5(제목·초점·판 수·전후 차), `used/20`, "훈련곡 만들기" | `GET /api/fe/me/ai/training-charts` | 클라이언트, 뷰포트 진입 뒤 |
| 10 | `member-week-panel.tsx` | (없음) | 이번 주 판·활동일·램프 갱신·첫 클리어·앞 주 대비 + 다음 도전 곡 3 | 기존 `useAiWeekly(null)`, `useAiRecommend(첫 표)` | 클라이언트, 뷰포트 진입 뒤 |
| 11 | `member-goal-panel.tsx` | (없음) | 활성 목표 진행 막대(`Progress`), 약점 목표 포함 | 기존 `useAiGoalList()` | 클라이언트, 뷰포트 진입 뒤 |
| 12 | `member-rival-panel.tsx`(넓음) | 게시판 활동 자리 | 라이벌 최근 판 10(라이벌 이름 부제), 라이벌별 공통 곡 승/패/무 | 새 `GET /api/fe/me/dashboard/rivals` | 클라이언트, 뷰포트 진입 뒤 |
| 13 | `member-folder-replay-panel.tsx` | (없음) | 최근 수정 폴더 5(곡 수), 내 최근 리플레이 5 → `/replays/{id}` | 기존 `useGetFolderList` + 새 `GET /api/fe/me/dashboard/replays` | 클라이언트 |
| 14 | `member-ai-status-panel.tsx` | (없음) | AI 키 등록 여부, 오늘·이달 사용량(`AiUsageMeter`), 판별 trace 보관 사용량, 판정 습관 한 줄(FAST 비율·평균 오차·오프셋 제안, 표본 부족이면 `need` 문구), 게이지 조언 한 줄(추천 게이지 결정론 값) | 기존 `useAiUsageSummary`, `useAiCredentialList`, 기존 trace 사용량(있으면), 기존 결정론 GET `useAiJudgeHabit()`·`useAiGaugeAdvice()`(`entities/ai.query.ts:295, 309`, 공급자 호출 없음. 해석 POST 는 부르지 않는다) | 클라이언트, 판정 습관·게이지는 뷰포트 진입 뒤 |
| 15 | `member-community-panel.tsx`(꽉) | (손님 홈 요소) | 전체 최근 활동 10 + 서버 현황 한 줄 | 기존 `useActivityRecent`, `useStatsSummary`(캐시됨) | 클라이언트 |

- 8·9·10·11·12 의 지연 로딩은 `shared/hooks/use-in-view.ts`(IntersectionObserver, 외부 시스템 동기화라 `useEffect` 허용. 이미 있는 훅이 있으면 그것)로 `enabled` 를 켠다.
- 노트레이더(03 §9-2)는 웹에 레이더 값이 없어 약점 패널로 대체한다(결정, 11절).
- 최근 플레이는 판 단위로 둔다(곡당 최신 보기는 넣지 않는다). R-BMS 의 기존 `/players` 최근 목록과 같은 단위라 "전체 보기" 와 이어지고, 곡당 최고는 패널 3·4 의 램프·등급이 이미 보인다(M26).
- AI 리포트 헤드라인은 넣지 않는다. 리포트 GET 이 저장된 해석을 돌려주는지는 이 설계에서 확인하지 않았고, 해석을 새로 부르면 공급자 호출이 생기기 때문이다. 대신 패널 8 이 같은 근거(프로필 약점)를 결정론으로 보인다. 저장 해석이 GET 에 있으면 나중에 패널 8 아래 한 줄로 더할 수 있다(부채 후보 (9)).

### 8.3 새 서버 집계 API

라우트 팩토리 `web/src/server/route/dashboard.ts`(`createDashboardRoutes`), 서비스 `web/src/server/service/domain/dashboard/dashboard.service.ts`(`createDashboardService(deps)`), compose `web/src/server/compose/dashboard.ts`(`composeDashboard(db, deps)` → `compose/index.ts` 스프레드), DTO `web/src/server/dto/dashboard.ts`. 모두 세션 필요, HTTP `private, no-store`, 사용자당 분당 60회(`RATE_LIMIT.DASHBOARD = 60`).

| 경로 | 응답 | 계산 |
| --- | --- | --- |
| `GET /api/fe/me/dashboard/overview` | `{ tiles: { total_plays, played_charts, cleared_charts, hard_or_better, full_combo_or_better, play_ms_30d, active_days_30d, streak_days, best_streak_days, this_week_plays, last_played_at }, grades: { grade, count }[], rank: { position: number | null, total_players } }` | 타일: `chart_best` 범위는 기존 `chartBestScope(db, true)`(03 §4.1), 램프 기준은 `MIN_CLEAR_LAMP_ID`·HARD·FC 상수 재사용, 30일·연속·이번 주(KST 월요일)는 `score.played_at`(`score_player_idx`), 플레이 시간은 `player-play-days.ts` 규칙. 등급: `chart_best ⨝ score(score_id)` 의 `ex_score`·`notes` → `getRateStep` → `DASHBOARD_GRADE_GROUPS`(27단계를 F~MAX 9묶음, `shared/lib/dj-rank.ts` 라벨 접두 기준). 순위: `getLeaderboardPositionsCached()`(아래) |
| `GET /api/fe/me/dashboard/calendar?days=` | `{ days: { date, plays, play_ms }[], total_plays, active_days, streak_days, best_streak_days }` | `days` 1~`DASHBOARD_CALENDAR_DAYS_MAX(371)`, 기본 182. `player-play-days.ts` 를 창 길이 인자로 일반화해 재사용(30일 경로는 그대로) |
| `GET /api/fe/me/dashboard/tables/[tableId]/progress` | `{ table_id, name, symbol, total, played, lamps: LampCount[], levels: { level, total, played, cleared, hard_or_better, lamps: LampCount[] }[] }` | 기존 `tableSnapshotService.getRecords` + 표 레벨 묶음(`groupTableByLevel` 류, 03 §8.3). 응답 수 KB |
| `GET /api/fe/me/dashboard/courses` | `{ groups: { group, total, played, cleared }[], recent: { course_hash, name, clear, ex_score, played_at }[] }` | `course_best` 를 사용자로 + 코스 카탈로그(묶음 이름은 카탈로그 서비스가 주는 값) |
| `GET /api/fe/me/dashboard/rivals?limit=` | `{ rivals: { login_id, name }[], recent: ActivityRow[], versus: { login_id, common, wins, losses, draws }[] }` | `rival` → 사용자 집합 → `listRecentActivity` 의 `user_id IN`(limit 기본 10, 최대 20), 승패: 내 `chart_best` 와 라이벌 `chart_best` 를 `chart_sha256` 으로 조인해 EX 비교(`chartBestScope` 같은 범위) |
| `GET /api/fe/me/dashboard/replays?limit=` | `{ replays: { id, created_at, chart: { md5, sha256, title, level }, clear, ex_score }[] }` | `replay.user_id` 색인, `data` 열은 고르지 않는다(03 §6) |

- 순위: `getLeaderboardPositionsCached()`(`read-cache.ts`, `'use cache'`, `cacheLife('minutes')`, 태그 `leaderboard-players`)가 종합 순위(정렬 `cleared` 내림차순, 같으면 `played` 내림차순. 기존 `compose/fe.ts:103` 의 `order.cleared`. `includeScratchAuto` 범위는 기존 종합 순위 화면이 넘기는 값과 같게 하고, 구현 때 그 호출 인자를 확인한다) 전체를 `{ userId → position }` 으로 한 번 계산한다. 순위 쿼리와 정렬 규칙을 공용 함수로 뽑아 두 곳이 쓴다(두 번 쓰이므로 추출).
  - 동률: `(cleared, played)` 가 같은 사람은 **공동 순위**(경쟁 순위 1, 1, 3). 기존 목록의 정렬은 동률 순서가 정해지지 않았으므로(같은 값의 줄 순서가 DB 에 맡겨진다) 대시보드 숫자가 새로고침마다 흔들리지 않게 공동 순위로 정한다. 목록 화면은 바꾸지 않는다.
  - `total_players` = 같은 범위에서 `chart_best` 행이 하나 이상인 사용자 수(순위가 매겨지는 집합과 같다). `position` 은 그 사용자에게 `chart_best` 가 없으면 `null`.
- 날짜 기준: overview 의 `play_ms_30d`·`active_days_30d`·`this_week_plays`·`streak_days`·`best_streak_days` 는 KST 오늘 0시(`todayStart`)를 기준으로 계산하고, 이번 주는 그 주의 KST 월요일 0시부터다. `streak_days` = 오늘(오늘 판이 없으면 어제)부터 거꾸로 판이 있는 연속 KST 날 수, `best_streak_days` = 최근 `DASHBOARD_STREAK_WINDOW_DAYS(371)` 일 안 가장 긴 연속(화면 라벨 "최근 1년 최고"). calendar 의 `streak_days`·`best_streak_days` 는 같은 정의를 그 요청 창(`days`) 안에서 계산한다.
- 서버 캐시: overview·calendar·tables progress·courses·replays 는 `read-cache.ts` 에 `getDashboard*Cached({ userId, loginId, todayStart, ... })`(`'use cache'`, `cacheLife('minutes')`, 태그 `CACHE_TAG.playerDashboard(loginId)`). **overview·calendar 키에는 KST 오늘 0시 `todayStart` 를 넣는다**(`getPlayerPlayDaysCached` `read-cache.ts:299` 와 같은 방식). 그래야 자정·월요일 0시가 지나면 점수 제출이 없어도 새 키로 다시 계산된다. courses·tables progress·replays 의 값(묶음별 클리어 수, 최근 기록 목록)은 날짜가 지나도 바뀌지 않으므로 넣지 않는다. 무효화: `revalidateOnScoreSubmit`, `revalidateOnCourseSubmit`, `revalidateOnReplayUpload`, `revalidateOnDanAward` 에 같은 태그를 더한다. rivals 는 캐시하지 않는다(라이벌의 제출로 바뀌므로). `docs/web/architecture.md` 5-2·5-3 표를 같은 작업에서 고친다.
- 서버 프리페치 함수: `web/src/server/service/dashboard-reads.ts`(`import 'server-only'`, `readDashboardOverview(user)` 등, `public-reads.ts` 모양). `ssr-no-self-fetch.test.ts` 규칙대로 모든 프리페치는 `queryFn` 을 서비스 직접 읽기로 바꾼다. 세션에서 이미 `id`·`login_id` 를 알므로 `requireUserId` 를 거치지 않는다(03 §7.6 제안).

### 8.4 웹 파일 배치(FSD)

- `web/src/widgets/home/home-branch.tsx`(서버), `home-branch-skeleton.tsx`, `member-dashboard-content.tsx`(서버: 프리페치 + `HydrationBoundary` + 격자), `member-dashboard-grid.tsx`(클라이언트: 패널 배치·정렬 장치), 8.2 의 패널 15개(클라이언트, 1파일 1컴포넌트).
- `web/src/features/dashboard/`(순수): `stat-tile-grid.tsx`, `grade-distribution-chart.tsx`, `activity-calendar.tsx`(CSS grid 7행 × 주, 농도 `DASHBOARD_CALENDAR_LEVELS = [0, 1, 3, 6, 10]`), `table-level-progress-list.tsx`(`LampStackBar` 재사용), `course-progress-list.tsx`, `rival-versus-list.tsx`, `replay-row.tsx`, `folder-read-row.tsx`, `density-limit-mini.tsx`(2.7 차트의 작은 판을 props 로).
- `web/src/entities/dashboard.type.ts`, `dashboard.query.ts`(`queryOptions` 팩토리: `dashboardOverviewQueryOptions()`, `dashboardCalendarQueryOptions(days)`, `dashboardTableProgressQueryOptions(tableId)`, `dashboardCoursesQueryOptions()`, `dashboardRivalsQueryOptions(limit)`, `dashboardReplaysQueryOptions(limit)` + 훅).
- `web/src/shared/constants/query-key.ts`: `DASHBOARD: { ALL: ['dashboard'], OVERVIEW: ['dashboard','overview'], CALENDAR: (days) => ['dashboard','calendar',days], TABLE_PROGRESS: (tableId) => ['dashboard','table-progress',tableId], COURSES: ['dashboard','courses'], RIVALS: (limit) => ['dashboard','rivals',limit], REPLAYS: (limit) => ['dashboard','replays',limit] }`.
- `web/src/entities/auth.query.ts` `USER_SCOPED_QUERY_KEYS` 에 `QUERY_KEY.DASHBOARD.ALL` 과 `QUERY_KEY.DAN.MINE`(03 §7.2 의 누수 의심)을 더한다.
- 문구 `pages.dashboard.*`(세 언어 `pages.json`). 숫자·날짜는 기존 `formatCount`·KST 형식(03 §7.4 의 알려진 어긋남은 그대로).

### 8.5 성능

- 첫 HTML 에 들어가는 프리페치는 1·2·3·5(일별)·6 만(`usePlayer`, overview, stats, play-days, recent). 모두 `'use cache'` 라 적중 시 DB 0회, 실패 시 overview 가 약 5회(타일 3 + 등급 1 + 순위 캐시).
- 나머지는 클라이언트 지연 로딩. AI digest(10·11)와 프로필(8)은 뷰포트 진입 뒤에만 요청해 분당 30회 한도와 DB 비용을 아낀다.
- DB 풀 `CONNECTION_LIMIT = 5`(03 §7.6): 서버 프리페치는 `Promise.all` 로 병렬이지만 캐시 적중이 대부분이라 연결을 오래 잡지 않는다. 운영 응답 시간은 배포 뒤 측정(10.2 부채 후보).

---

## 9. 구현 단위와 웨이브

전제: 웨이브 안 단위는 worktree 격리로 병렬, 메인이 `git apply`(`--3way` 금지)로 병합·전체 게이트·커밋(HANDOFF 5절). 한 웨이브 안에서 소유 파일이 겹치지 않는다. 같은 파일을 다른 웨이브가 다시 고치는 것은 허용한다(직렬화). 에이전트는 커밋·푸시하지 않고, `rbms-player` 를 실행하면 scratch HOME 과 `--server http://127.0.0.1:9`.

- **기존 시험 규칙**: 한 단위의 변경으로 깨지는 기존 시험·시험 보조 파일은 그 단위가 고친다. 아래 소유 목록에 확인된 파일을 적었지만, 목록에 없는 파일이 깨지면 같은 웨이브의 다른 단위 소유가 아닌지 확인하고 아니면 고친 뒤 메인에 보고한다. 다른 단위 소유 파일이면 고치지 말고 멈춰 보고한다(메인이 순서를 정한다).
- **호환 먼저**: 다른 웨이브가 호출부를 가진 함수·타입은 선택 인자·선택 필드로만 넓힌다(`analyzeTrace` 의 `training`, `folderItemSchema.download_url`, `FolderItemRow.trainingChartId`, `folderService.get` 의 `origin`). 그래야 각 단위 게이트(`bun run typecheck` 는 `tsconfig.json` 의 `include` 가 `**/*.ts` 라 `tests/` 까지 본다)가 그 단위 안에서 통과한다.

DB 시험 공통 절차(아래 "DB 게이트"): Docker `mysql:8.4.11` 을 3317 같은 빈 포트로 띄우고 `DATABASE_URL=mysql://root:<임시값>@127.0.0.1:3317/rbms_test bun run db:migrate` 뒤 `RBMS_DB_E2E=1 DATABASE_URL=... bun test <파일>`, 끝나면 컨테이너 삭제. `.env*` 는 읽지 않는다.

### W1 (서로 독립)

**U1 마이그레이션 0014** — sonnet xhigh
- 소유: `web/src/server/db/schema.ts`, `web/drizzle/0014_training_chart.sql`, `web/drizzle/meta/0014_snapshot.json`, `web/drizzle/meta/_journal.json`, `web/tests/db/migration-0014.test.ts`
- 내용: 4절 세 표.
- 게이트: `cd web && bun run typecheck` + DB 게이트 `tests/db/migration-0014.test.ts`
- 완료: 생성 SQL 이 4절 열·제약과 일치, 시험 통과, `db:generate` 를 다시 돌려도 새 파일이 생기지 않음.

**U2 분석 엔진 v3** — opus high
- 소유: `web/src/server/service/domain/analysis/{analysis-constants.ts, play-metrics.ts, chart-features.ts, pattern-tags.ts, play-cells.ts, local-density.ts, bp-runs.ts, tag-combos.ts}`, `web/src/server/service/domain/trace/trace-analysis.ts`, `web/tests/service/analysis/{play-metrics,chart-features,trace-analysis,local-density,bp-runs,tag-combos}.test.ts`, `web/tests/support/play-feature-fixture.ts`, `web/tests/service/trace.test.ts`, `web/tests/service/ai/play-features.test.ts`(버전 상수·고정 요약 갱신만)
- 내용: 2.2, 2.3, 2.4. `pattern-tags.ts` 는 소프란 사건 시각을 내보내는 도우미 export 만 더한다(분류 규칙 불변). `analyzeTrace` 의 `training` 은 선택 인자(기본 `null`)로 더하고 호출부(`trace.service.ts`)는 건드리지 않는다. `chart-features.ts` 에 `readChartFeatureExtras`(2.3)를 export 한다.
- 게이트: `cd web && bun test tests/service/analysis tests/service/trace.test.ts tests/service/ai/play-features.test.ts && bun run typecheck`
- 완료: 2.4 표의 필드가 모두 계산되고 시험(2.2 각 절마다 손으로 만든 작은 trace 로 값 고정, 6만 노트 합성 trace 요약 바이트 수 출력), 버전 3·2, 기존 필드 값 불변(기존 시험 통과), `readChartFeatureExtras` 가 v1 모양 특징(`nps` 없음)에 `null`, v2 모양에 값을 돌려주는 시험.

**U3 훈련곡 합성기·작성기·골든** — opus high
- 소유: `web/src/server/dto/training.ts`(3.3 의 LLM 출력 스키마·`TRAINING_LIMIT`·종류·힌트·강도·`TRAINING_INTENSITY_TITLE`·폴더 이름 상수만), `web/src/server/service/domain/training/{training-constants.ts, prng.ts, spec.ts, synthesize.ts, bms-writer.ts, chart-timing.ts, pattern-document.ts, wav.ts, crc32.ts, zip.ts}`, `contract/training-chart/*`(`.spec.json`·`.bms`·`.pattern.json` 세 벌과 `basic-160.zip`), `web/tests/service/training/{spec,synthesize,bms-writer,chart-timing,wav,zip,golden}.test.ts`, `apps/rbms-cli/src/chart_pattern.rs`(mod 한 줄), `apps/rbms-cli/src/chart_pattern/training_contract_tests.rs`, `crates/rbms-download/src/tests/extract.rs`(훈련곡 zip 시험 하나 추가만)
- 내용: 3.3(스키마), 3.5~3.10. `correctTrainingPlan` 의 구간 고르기·마디 분배(3.5 의사코드)를 포함한다. 생성 요청 스키마(`trainingChartCreateSchema`)와 `trainingDownloadPathOf` 는 약점 키 도구(U6)가 필요해 U7 이 더한다.
- 게이트: `cd web && bun test tests/service/training && bun run typecheck`; `cargo test -p rbms-cli --locked training_contract`; `cargo test -p rbms-download --locked training`; `cargo clippy -p rbms-cli -p rbms-download --all-targets --locked -- -D warnings`; `cargo fmt --all --check`
- 완료: 세 골든이 TS·Rust 양쪽 통과, 저장 방식 골든 zip 이 TS 바이트 비교와 Rust `extract_zip`(차트 1·wav 2, `.bms` 바이트 일치) 양쪽 통과, 합성 결과가 분류기(`classifyPattern`)로 의도한 태그를 갖는지 시험(예: trill 구간 노트의 90% 이상이 `trill`), 같은 사양·시드 바이트 동일, deflate zip 을 `node:zlib.inflateRawSync` 로 되풀어 내용·CRC 일치, 60초 이하·7키 판정 시험, 마디 분배 불변식 시험(무게·BPM 무작위 1,000조합을 고정 시드로 돌려 (a)~(c) 를 확인), `allowed.keys` 밖 구간 버림과 `kind` 대체가 `corrections` 에 남는 시험.

**U4 클라이언트 내 폴더 받기** — opus high
- 소유: `crates/rbms-ir/src/dto/folder.rs`, `crates/rbms-table/src/dto.rs`, `apps/rbms-player/src/{user_folders.rs, lib.rs, tablesrc.rs}`, `apps/rbms-player/src/stage/select/{list.rs, download_modal.rs}`, `apps/rbms-player/src/text/select/download_modal.rs`, `contract/server-responses/user-folder-detail.json`, 시험 파일 `crates/rbms-ir/src/contract_tests/folders.rs`, `apps/rbms-player/src/user_folders/tests.rs`, `apps/rbms-player/src/stage/select/{user_folder_tests.rs, missing_tests.rs, download_modal_tests.rs, tree_tests.rs}`, 컴파일 영향 파일 `apps/rbms-player/src/stage/tables.rs`(843행 `missing.entry` 읽기), 그 밖 **컴파일 오류로 드러나는 모든 파일**(`MissingRef` 생성·필드 읽기, `UserFolderItem`·`ShownFolder`·`TableEntry` 구조체 리터럴. 고친 파일 목록을 메인에 보고). 이 단위는 W1 에서 Rust 를 바꾸는 유일한 단위(U3 의 `rbms-cli`·`rbms-download` 시험 파일 제외)라 겹치지 않는다
- 내용: 6절.
- 게이트: `cargo test -p rbms-ir --locked folder`; `cargo test -p rbms-player --lib --locked user_folders`; `cargo test -p rbms-player --lib --locked download_modal`; `cargo test -p rbms-player --lib --locked select::list`; `cargo clippy -p rbms-player -p rbms-ir -p rbms-table --all-targets --locked -- -D warnings`; `cargo fmt --all --check`
- 완료: 6절 시험 1~5 통과, 옛 저장본(`download_url` 없는 `.ron`) 읽기 시험, 새 `#[allow]`·주석 없음.

**U5 대시보드 서버 집계** — sonnet xhigh
- 소유: `web/src/server/dto/dashboard.ts`, `web/src/server/service/domain/dashboard/*`, `web/src/server/compose/dashboard.ts`, `web/src/server/compose/index.ts`, `web/src/server/compose/fe.ts`(순위 정렬 공용 함수 추출만), `web/src/server/service/domain/fe/player-play-days.ts`(창 길이 인자화), `web/src/server/route/dashboard.ts`, `web/src/server/service/read-cache.ts`, `web/src/server/service/dashboard-reads.ts`, `web/src/server/lib/with-rate-limit.ts`, `web/src/app/api/fe/me/dashboard/**/route.ts`, `web/tests/service/dashboard/*.test.ts`, `web/tests/route/dashboard.test.ts`, `web/tests/db/dashboard.test.ts`
- 내용: 8.3. `with-rate-limit.ts` 에 `DASHBOARD` 와 `TRAINING_DOWNLOAD` 두 상수를 함께 넣는다(U7 이 쓴다).
- 게이트: `cd web && bun test tests/service/dashboard tests/route/dashboard.test.ts tests/service/fe-player-play-days.test.ts && bun run typecheck` + DB 게이트 `tests/db/dashboard.test.ts`
- 완료: 6개 경로 응답이 DTO 로 검증, 인증 없으면 401, 캐시 태그 무효화 시험, 30일 일별 결과 불변.

### W2 (U1·U2·U5 뒤)

**U6 약점 프로필 + 판 분석 기능 1~4·8** — opus high
- 소유: `web/src/server/service/domain/analysis/{weakness-profile.ts, profile-stats.ts, weakness-key.ts}`, `web/src/server/service/domain/ai/{weakness-profile.service.ts, play-report.service.ts, replay-coach.service.ts, play-compare.service.ts, judge-habit.service.ts, gauge-advice.service.ts, play-digest.ts, play-feature.constants.ts}`, `web/src/server/service/domain/trace/trace.service.ts`, `web/src/server/compose/{trace.ts, ai-play-features.ts, ai-score-features.ts}`(판정 습관 배선만), `web/src/server/dto/ai-play.ts`, `web/src/server/dto/ai.ts`(판정 습관 뷰 스키마 추가 필드만), `web/src/server/route/ai.ts`(프로필 라우트 + `createAiRouteWrappers`·`AiHandler` 앞에 `export` 만 붙이기, 7.2), `web/src/app/api/fe/me/ai/profile/route.ts`, `web/tests/service/analysis/{weakness-profile,profile-stats,weakness-key}.test.ts`, `web/tests/service/ai/{play-features,weakness-profile,output-language}.test.ts`, `web/tests/route/{ai-profile,ai-play-features}.test.ts`, `web/tests/db/ai-play-features.test.ts`, `web/tests/support/ai-memory-db.ts`
- 내용: 2.5, 2.6 의 기능 1·2·3·4·8, 프로필 라우트, 약점 키 도구(2.5.5). `trace.service.ts` 에서는 `analyzedPlaysInWindow` 추가만 하고 `analyze`(190-192행)·`recomputeStale`·`adminUsage` 는 건드리지 않는다(U7·U12 가 쓴다). `compose/trace.ts` 의 `analyzedPlaysInWindow` 질의가 `training_chart` 를 `(user_id, sha256)` 로 left join 해 `isTraining` 을 낸다(표는 U1 이 이미 만든다).
- 게이트: `cd web && bun test tests/service/analysis tests/service/ai tests/route/ai-profile.test.ts tests/route/ai-play-features.test.ts && bun run typecheck` + DB 게이트 `tests/db/ai-play-features.test.ts`
- 완료: 통계 도구 시험(t 분포 알려진 값, BH 예제, PAVA 예제), 프로필 결정론 시험(같은 입력 같은 출력, 곡 반복 가중 상한, 표본 등급 경계), 가족 표(2.5.3)대로 셀이 두 가족에 나뉘는 시험, timing 셀의 판 전체 평균이 `timing_by_hand` 합인 시험(여러 태그를 가진 노트가 한 번만 들어감), SCRATCH AUTO 시험(auto 판만 있으면 밀도 `insufficient`, 섞이면 밀도·스크래치 셀은 auto 판을 빼고 `auto_plays` 를 셈, 건반 태그 셀은 넣음), 훈련곡 군집 시험(서로 다른 훈련곡 20곡이 K 에 1 만 보탬), `parseWeaknessKey` 의 13종 문법·잘못된 태그·길이 초과 거절, 지문 캐시(같으면 재계산 안 함), 다섯 기능 digest 에 새 키, `refine` 이 모르는 키를 버림.

**U8 대시보드 웹 화면** — sonnet xhigh
- 소유: `web/src/app/(app)/page.tsx`, `web/src/widgets/home/{home-branch.tsx, home-branch-skeleton.tsx, member-dashboard-content.tsx, member-dashboard-grid.tsx, member-*-panel.tsx}`(8.2 의 1~7, 10~15; 8·9 는 U11), `web/src/features/dashboard/*`(새 파일), `web/src/entities/{dashboard.type.ts, dashboard.query.ts, auth.query.ts}`, `web/src/shared/constants/{query-key.ts, dashboard.ts}`, `web/src/shared/hooks/use-in-view.ts`, `web/messages/{ko,en,ja}/pages.json`, `web/tests/components/member-dashboard*.test.tsx`, `web/tests/components/entity-query-wiring.test.ts`
- 내용: 8.1, 8.2(8·9 제외), 8.4, 8.5. 격자 자리는 8·9 를 위해 비워 두지 않는다(U11 이 끼운다).
- 게이트: `cd web && bun test tests/components/member-dashboard tests/components/home-recent-activity-panel.test.tsx tests/components/entity-query-wiring.test.ts tests/lib/ssr-no-self-fetch.test.ts tests/lib/i18n-messages.test.ts && bun run typecheck && bun run lint`
- 완료: 손님 홈 HTML 불변(기존 시험), 회원 분기 시험(세션 있음/없음/읽기 실패), 패널마다 로딩·빈·오류 상태, 세 언어 키 일치.

**U12 재계산 운영 경로** — sonnet xhigh
- 소유: `web/scripts/analysis-recompute.ts`, `.github/workflows/web-db.yml`, `web/tests/scripts/analysis-recompute.test.ts`. 서비스·compose 파일은 고치지 않는다(이미 공개된 `coverage`·`adminUsage`·`recomputeStale` 만 쓴다, 2.8).
- 내용: 2.8.
- 게이트: `cd web && bun test tests/scripts/analysis-recompute.test.ts && bun run typecheck`; `uv run -q --with pyyaml python -I -c "import yaml; yaml.safe_load(open('.github/workflows/web-db.yml'))"`
- 완료: 가짜 서비스로 count·apply·이어 하기 시험, 시간 상한 시험(가짜 `now` 가 배치마다 3분씩 늘면 세 번째 배치 전에 멈추고 `stopped elapsedMs N, remaining N` 한 줄을 찍고 성공 코드), 모든 출력 줄에서 숫자를 `N` 으로 바꾸면 2.8 의 고정 문장 네 가지 중 하나와 같은지(가짜 서비스가 `login_id` 를 담은 `top` 을 돌려줘도 찍히지 않음), `ANALYSIS_RECOMPUTE_MAX_MS` 가 workflow 작업 제한보다 작은지 시험(`web-db.yml` 을 텍스트로 읽어 `timeout-minutes: (\d+)` 정규식으로 꺼내 비교, 새 의존성 없음), workflow 두 action 이 기존 모양과 같다.

### W3 (W2 뒤)

**U7 훈련곡 서버** — opus high
- 소유: `web/src/server/dto/training.ts`(생성 요청 `trainingChartCreateSchema`, `trainingDownloadPathOf`, 뷰 스키마 추가), `web/src/server/dto/folder.ts`, `web/src/server/service/domain/training/training.service.ts`, `web/src/server/service/domain/ai/training-plan.service.ts`, `web/src/server/service/domain/folder/folder.service.ts`, `web/src/server/compose/{training.ts, index.ts, folder.ts, trace.ts}`, `web/src/server/service/domain/trace/trace.service.ts`(`analyze` 의 `trainingSectionsOf` 배선만), `web/src/server/route/{training.ts, training-charts.ts, folder.ts}`(`folder.ts` 는 `detail`·`readFolder` 가 origin 을 넘기는 것만), `web/src/app/api/training/[id]/chart.zip/route.ts`, `web/src/app/api/fe/me/ai/training-charts/route.ts`, `.../training-charts/[id]/route.ts`, `.../training-charts/[id]/removal/route.ts`, `web/src/server/lib/{error-code.ts, error-message.ts, error.ts}`, `web/messages/{ko,en,ja}/server.json`, `web/tests/service/training/{training,training-plan}.test.ts`, `web/tests/route/{training,folders}.test.ts`(`folders.test.ts` 는 origin 시험 추가만), `web/tests/dto/folder-contract.test.ts`, `web/tests/service/folder.test.ts`, `web/tests/support/folder-memory-db.ts`, `web/tests/db/training.test.ts`
- 내용: 3.1~3.4, 5절, 7.1·7.2. 새 오류 코드 `TRAINING_CHART_NOT_FOUND`(404), `TRAINING_CHART_LIMIT`(409), `TRAINING_FOLDER_UNAVAILABLE`(409) 세 파일 + server.json 세 언어. `createFolderService` 의 deps 는 바꾸지 않는다(5.2). 그래서 같은 웨이브 U9 가 쓰는 `tests/support/ai-score-fixture.ts` 는 두 단위 모두 고치지 않으며 병합 순서 제약이 없다.
- 게이트: `cd web && bun test tests/service/training tests/route/training.test.ts tests/route/folders.test.ts tests/dto/folder-contract.test.ts tests/components/folder-contract.test.ts tests/service/folder.test.ts && bun run typecheck` + DB 게이트 `tests/db/training.test.ts`
- 완료: 가짜 어댑터로 생성 전 과정(한도·키 없음·프로필 부족·`output_invalid`·폴더 없음/있음/이름 겹침/50개, 폴더 재생성 때 기존 훈련곡 전체가 들어감), 요청 검사(`focus_key` 65자·모르는 접두사·잘못된 태그는 400, 문법은 맞지만 프로필에 없는 키는 422 `weakness_profile` 이고 공급자 호출 0회), 한도 경쟁 DB 시험(4.4, 19곡에서 동시 두 저장 중 하나만), 소유자 시험(타인 id 로 상세·removal·DELETE 가 404, 7.2), zip 라우트(형식 아닌 id 404, 지운 뒤 404, 헤더), 폴더 응답 `download_url`(주인 아닌 폴더·origin 없는 호출에는 필드 없음, 주소의 origin 이 요청 origin), 훈련곡 판 trace 의 `training_sections`(sha256 조회), 삭제 개수.

**U9 점수 기반 기능 5·6·7·9·10** — opus high
- 소유: `web/src/server/service/domain/ai/{recommend.service.ts, practice-folder.service.ts, goal-plan.service.ts, record-query.service.ts, record-query.compiler.ts, weekly-summary.service.ts, table-progress.ts, score-feature.constants.ts}`, `web/src/server/compose/{ai-score-features.ts, ai-record-query.ts}`, `web/src/server/dto/ai.ts`, `web/src/server/route/ai.ts`, `web/src/entities/ai.type.ts`(`AI_GOAL_KINDS` 와 그 라벨, `AI_PRACTICE_SOURCES` 화면 쪽 대응만), `web/messages/{ko,en,ja}/pages.json`(`pages.ai.goalForm.goalKind`·출처 라벨 키만), `web/tests/service/ai/{score-table-features,score-history-features,record-query,record-query-compiler,table-progress,output-language}.test.ts`, `web/tests/route/ai-score-features.test.ts`, `web/tests/components/ai-score-features-contract.test.ts`, `web/tests/db/{ai-score-features,ai-record-query}.test.ts`
- 내용: 2.6 의 기능 5·6·7·9·10. 곡 특징의 새 필드는 `readChartFeatureExtras`(2.3)로만 읽는다. 약점 키 입력은 U6 의 `weaknessKeySchema` 를 쓴다.
- 게이트: `cd web && bun test tests/service/ai tests/route/ai-score-features.test.ts tests/components/ai-score-features-contract.test.ts tests/lib/i18n-messages.test.ts && bun run typecheck` + DB 게이트 `tests/db/ai-score-features.test.ts tests/db/ai-record-query.test.ts`
- 완료: 추천 점수 식 시험(약점 일치·밀도 초과 감점), v1 모양 특징(`nps`·`combos` 없음, `isStale`) 픽스처로 추천·연습 폴더·목표 계획이 던지지 않고 감점·밀도 일치가 0 인 시험, 약점 출처 후보와 프로필에 없는 키의 422, 두 새 목표 종류 진행, 기록 질문 `tag`·`training` 컴파일(매개변수 바인딩, `training` 은 `chart_sha256` 조건), 주간 `training`·`focus`.

### W4 (W3 뒤)

**U10 AI 웹 화면 1(프로필·기존 기능 갱신)** — opus high
- 소유: `web/src/entities/{ai.type.ts, ai.query.ts}`, `web/src/shared/constants/query-key.ts`(AI 추가), `web/messages/{ko,en,ja}/{pages.json, ai.json}`, `web/src/widgets/ai/{ai-hub-tabs.tsx, ai-profile-panel.tsx, ai-report-panel.tsx, ai-judge-habit-panel.tsx, ai-gauge-advice-panel.tsx, ai-coach-panel.tsx, ai-compare-panel.tsx, ai-recommend-panel.tsx, ai-practice-folder-panel.tsx, ai-goal-panel.tsx, ai-goal-plan-section.tsx, ai-weekly-panel.tsx, record-query-filter-editor.tsx}`, `web/src/features/ai/{ai-density-limit-chart.tsx, ai-weakness-list.tsx, ai-tag-timing-chart.tsx, ai-combo-grid.tsx, ai-hand-balance.tsx, ai-bp-context-list.tsx, ai-sample-badge.tsx, ai-segment-chart.tsx, ai-goal-form.tsx}`, `web/tests/components/ai-*.test.tsx`, `web/tests/components/{ai-contract.test.ts, ai-score-features-contract.test.ts}`
- 내용: 2.7, 2.6 의 화면 쪽(약점 버튼 → 연습 폴더 `weakness` 출처·훈련곡 탭으로 이동, 새 목표 종류 폼, 기록 질문 새 필드), `AI_DATA_NEEDS` 에 `weakness_profile` 과 문구, 탭 `profile`(탭 `training` 은 상수만 더하고 패널은 U11), 프로필 화면의 `auto_plays`·`training_plays` 표시와 도움말 `pages.ai.profile.trainingClusterNote`(2.5.2).
- 게이트: `cd web && bun test tests/components tests/lib/i18n-messages.test.ts && bun run typecheck && bun run lint`
- 완료: 프로필 화면 시험(빈·부족·정상·한계 없음), SVG `aria-label` 문장, 계약 시험 갱신 통과.

### W5 (W4 뒤)

**U11 훈련곡 화면 + 대시보드 AI 패널 + 폴더 받기 표시** — sonnet xhigh
- 소유: `web/src/widgets/ai/{ai-training-panel.tsx, ai-training-create-form.tsx, ai-training-list.tsx, ai-training-removal-dialog.tsx, ai-training-detail-content.tsx, ai-hub-tabs.tsx}`, `web/src/features/ai/{ai-training-section-strip.tsx, ai-training-section-trend.tsx, ai-training-before-after.tsx}`, `web/src/app/(app)/ai/training/[id]/page.tsx`, `web/src/widgets/home/{member-weakness-panel.tsx, member-training-panel.tsx, member-dashboard-grid.tsx}`, `web/src/features/dashboard/density-limit-mini.tsx`, `web/src/entities/{ai.type.ts, ai.query.ts, folder.type.ts}`(`folder.type.ts` 는 `FolderItem` 에 `download_url?: string` 선택 필드만), `web/src/shared/constants/query-key.ts`(`AI.TRAINING` 추가), `web/src/widgets/folder/*`(항목에 `download_url` 이 있으면 "받기" 링크), `web/messages/{ko,en,ja}/{pages.json, ai.json}`, `web/tests/components/{ai-training*,member-dashboard-ai*,folder-download*}.test.tsx`
- 내용: 7.3, 8.2 의 8·9 패널, 폴더 화면 받기 링크, 레인·손 초점 옵션 안내(`pages.ai.training.optionNotice`)와 구간 추이의 RANDOM 판 흐림(7.1), 한도 경쟁 문구(`pages.ai.training.limitRaced`, 4.4).
- 게이트: `cd web && bun test tests/components tests/lib && bun run typecheck && bun run lint`
- 완료: 만들기 폼(키 없음 막힘, 20곡 안내, 레인·손 초점이면 옵션 안내), 삭제 대화상자가 개수를 보이고 확인 뒤에만 DELETE, 상세 화면 시험(RANDOM 판이 레인·손 초점 구간 추세에서 빠짐), 대시보드 두 패널 지연 로딩.

**U13 문서** — sonnet xhigh
- 소유: `docs/web/architecture.md`, `docs/web/components.md`, `docs/backend/api-spec.md`, `docs/play-options.md`(13절 내 폴더 받기), `docs/acknowledge/2026-11-04-adr-*.md`(11절 결정마다), `docs/acknowledge/README.md`, `docs/quality-assurance/2026-11-04-ai-training-dashboard.md`(실기 점검표 3.27), `docs/quality-assurance/2026-10-14-test-debt.md`(10.2 부채 추가)
- 내용: 새 경로·표·캐시 태그·무효화·버전·web-db action·클라이언트 동작. 사용자 trace·차트 배치는 적지 않는다.
- 게이트: `git diff --check -- docs`
- 완료: 9절 각 단위의 결과가 문서에 반영, 링크 깨짐 없음(`rg -n '\]\((?!http)' docs/...` 로 상대 경로 존재 확인).

메인 통합 게이트(웨이브마다 병합 뒤 메인이 실행): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(Rust 를 바꾼 웨이브만), `cd web && bun run verify && bun run format:check && bun run build`, DB 통합 `RBMS_DB_E2E=1 bun test tests/db`, 계약 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 뒤 diff 없음 + `bun test tests/dto/client-requests.test.ts`.

---

## 10. 시험 전략 · 남은 위험 · 운영 반영

### 10.1 시험 전략

- 순수 함수 우선: 밀도·실패 run·조합·통계·합성·작성·zip·wav 는 모두 입력 → 출력이 정해진 함수라 작은 손 입력으로 값을 고정한다(`describe`/`test` 설명 한국어).
- 교차 계약: 훈련곡 골든(3.10, TS ↔ Rust 파서), 훈련곡 zip(3.9, TS 작성기 ↔ Rust `extract_zip`, 저장 방식 골든), 폴더 응답(5.2, TS ↔ Rust DTO), 기존 클라이언트 요청 계약은 그대로.
- 경쟁·권한: 한도 20곡의 동시 저장(4.4, DB 시험), 훈련곡 API 의 소유자 검사(7.2, 라우트 시험).
- 결정론: 같은 사양·시드 → 같은 BMS 바이트·md5. 같은 판 목록 → 같은 프로필(정렬 안정성: 동률은 키 사전순).
- 떠는 시험 금지: 시각(`Date.now`)은 deps 의 `now()` 로 주입, PRNG 는 시드 고정, 전역 토스트는 시험 시작 때도 비운다(HANDOFF 7절).
- 화면: happy-dom + `withLocale`, 세 언어 키 일치 시험(`i18n-messages.test.ts`), SVG 는 `aria-label` 과 요소 수로 검사(스냅샷 금지).
- DB: 마이그레이션 0014, 훈련곡 저장·삭제 CASCADE, 대시보드 집계 SQL, 프로필 캐시 upsert, 기록 질문 새 필드 SQL.
- 클라이언트 실기는 QA 3.27(U13)로 사용자에게: 생성 → F6 로 폴더 받기 → 내 폴더의 흐린 줄 → GET CHART → 플레이 → 점수·trace 업로드 → 웹 상세의 구간 결과.

### 10.2 남은 위험과 테스트 부채 후보

| 위험 | 내용 | 대응 |
| --- | --- | --- |
| 요약 크기 | v3 요약이 판당 30 KB 를 넘으면 프로필 재계산(최대 300판) 때 DB 전송이 수 MB | U2 에서 측정, 넘으면 `PLAY_COMBO_TOP` 으로 자름. 프로필은 지문 캐시 |
| 재계산 시간 | 7,903 패턴 + 전체 trace 재계산이 `web-db.yml` 작업 제한 10분을 넘을 수 있음 | 8분 상한 뒤 이어 하기(2.8), count 로 남은 수 확인, 남으면 사용자 `!` 를 다시 |
| 재계산 전 낡은 기간 | 버전을 올린 코드가 운영에 나간 뒤 재계산이 끝날 때까지 판 요약·곡 특징이 낡은 것으로 읽힌다 | 10.3 의 순서로 기간을 줄이고, 그 동안의 동작을 알려진 동작으로 둔다(아래) |
| 통계 해석 | 곡 군집 t 검정은 곡 수가 적으면 보수적, 곡 난이도와 패턴의 교락은 잔차로만 상쇄 | 등급·표본 등급을 함께 보이고 AI 에는 strong 만. 다변량 모형은 부채 |
| 합성 의도와 분류 차이 | 합성한 trill 이 분류기에서 다른 태그를 더 받을 수 있음 | U3 시험이 종류별 태그 비율 하한을 고정 |
| LLM 품질 | Ollama 는 best effort 구조화(01 §6.2) | Zod·refine·서버 보정으로 형태 보장, 보정 목록을 화면에 보임 |
| 공급자 호출 뒤 실패 | 한도·폴더 문제가 저장 단계에서 드러나면 호출이 헛됨 | 한도·폴더·프로필 검사를 호출 전에, 트랜잭션 재검사는 경쟁만 |
| `cacheComponents` 와 zip 라우트 | 동적 라우트 처리 방식 미확인(02 §8) | 구현 때 Next 문서 확인, 라우트 시험 |
| 훈련곡의 공개 노출 | 판이 랭킹·검색에 들어가 다른 사용자가 훈련곡 제목을 본다(D4) | 제목은 서버 고정 형식(`R-BMS Training · 날짜 · 강도`, 3.7)이라 약점·LLM 문장이 들어가지 않는다. 초점은 비공개 폴더 `note` 와 소유자 전용 API 에만 |
| 훈련곡의 프로필 무게 | 서로 다른 훈련곡이 같은 약점 셀을 채울 수 있음 | 훈련곡 판은 한 군집으로 묶어 K·가중을 한 군집분으로 제한(2.5.2). 훈련 효과를 일반 곡 판으로 확인하는 한계를 화면에 적음 |
| 레인·손 초점과 옵션 | RANDOM 계열이면 의도한 레인이 바뀜 | 옵션 안내 문구, 구간 추이에서 RANDOM 판 흐림(7.1) |
| 클라이언트 폴더 줄 증가 | 일반 폴더도 없는 곡 줄이 보이게 됨(행동 변화) | `show_missing_charts` 설정을 따른다. 릴리스 노트에 적는다 |
| 운영 응답 시간 | 회원 `/` 프리페치 비용 미측정 | 배포 뒤 측정을 부채로 |

테스트 부채 후보(`docs/quality-assurance/2026-10-14-test-debt.md` 에 3.145~): (1) 프로필 다변량 분리(태그·밀도 교락), (2) 일반 LN 놓기 타이밍(데이터 없음), (3) 5K·14K·9K 분석, (4) 회원 `/` 운영 응답 시간 측정, (5) 옛 클라이언트 빌드로 `download_url` 응답 실기, (6) Next 런타임 차이로 zip deflate 바이트가 바뀌는지(내용은 같음), (7) 프로필 재계산 비용 실측, (8) `stream` 태그의 레이아웃 의존(01 §2.3 추정) 확인, (9) 리포트 GET 이 저장 해석을 주면 대시보드 패널 8 아래 헤드라인 한 줄(8.2), (10) 운영 재계산 한 배치(100건)의 실제 소요 시간(2.8 의 2분 여유 가정 확인).

### 10.3 운영 반영 순서

버전 상수(`PLAY_ANALYSIS_VERSION` 3, `ANALYSIS_VERSION` 2)는 U2(W1) 가 올리지만 재계산 action 은 U12(W2) 가 만든다. 그래서 **W1 코드는 W2 와 함께 푸시한다**(웨이브마다 로컬 커밋은 하되, 버전을 올린 커밋이 재계산 경로 없이 운영에 나가지 않게 한다).

1. U1 만 담은 마이그레이션 커밋 푸시 → 메인이 `web-db.yml` `status` → 사용자 `!` `gh workflow run web-db.yml -f action=migrate` → 메인 `status` 로 0014 적용 확인.
2. W1 과 W2 의 코드 커밋을 W2 병합·통합 게이트 뒤 한 번에 푸시. `gh run list` 로 3 OS ci 와 web CI 를 확인한다(진행 중 ci 가 있으면 끝난 뒤 푸시).
3. 그 배포가 운영에 반영된 직후 메인이 `analysis-recompute-count` 실행 → 남은 수를 사용자에게 보인다.
4. 사용자 `!` `gh workflow run web-db.yml -f action=analysis-recompute` → 메인이 다시 `analysis-recompute-count` 로 0 확인(8분 상한으로 남으면 4 반복).
5. W3·W4·W5 코드를 웨이브마다 푸시하고 CI 를 확인한다. 이 웨이브들은 분석 버전을 바꾸지 않으므로 재계산이 다시 필요 없다(훈련곡 판의 `training_sections` 는 새 판·재계산 때 채워지고, 훈련곡이 운영에 생기기 전 판에는 해당이 없다).
6. 클라이언트 변경(U4, W1)이 든 커밋의 CI 통과 뒤 메인이 `gh workflow run release.yml --ref dev -f bump=patch` → draft v0.1.10, 한글 노트(내 폴더 받기, 훈련곡, 일반 폴더의 없는 곡 줄) → 사용자가 받아 확인 뒤 게시. 훈련곡 서버(W3)가 운영에 나간 뒤에 draft 를 만든다(받을 곡이 있어야 실기 확인이 된다).

알려진 동작(2 의 배포부터 4 가 끝날 때까지): 판 기능 1~4·8 은 한 번의 읽기에서 낡은 판을 최대 `PLAY_ANALYSIS_READ_RECOMPUTE_MAX(10)` 개만 다시 계산하고 나머지는 `stale` 로 빼므로(`trace.service.ts:169, 277`) digest 의 판 수가 줄고 표본 부족 안내가 늘 수 있다. 약점 프로필은 `stale` 이 0 이 아니면 저장하지 않으므로 매번 다시 계산된다(2.5.1). 점수 기능 5·6·7 은 낡은 곡 특징을 `isStale` 로 받아 밀도·조합 일치를 0 으로 둔다(2.3). 오류는 나지 않는다. 이 기간을 사용자에게 알리고 QA 3.27 에 적는다.

---

## 11. 메인 추천으로 정한 세부 결정 (사용자에게 알릴 것)

### 11.0 사용자 질문 "클라이언트 수정도 필요한 거야?" 에 대한 답

필요하다. 지금 클라이언트는 내 폴더에 있어도 라이브러리에 없는 곡을 줄로 보여 주지 않고 `(가진 곡/전체)` 숫자만 보이며, 곡을 받는 모달은 난이도표 항목에서만 열린다(02 §1.3, §2.3). 서버만 바꾸면 훈련곡은 내 폴더에 숫자로만 잡히고, 사용자가 웹에서 zip 을 받아 직접 풀고 다시 스캔해야 한다(02 §7.3 의 P0). "구동기에서 바로 받아 플레이" 는 내 폴더의 없는 곡 줄과 받기 모달 분기(6절, 파일 약 8개)가 있어야 된다. 받기·풀기·스캔 기계는 이미 있어 재사용한다.

### 11.1 결정 목록

| 번호 | 결정 | 이유 |
| --- | --- | --- |
| M1 | 판 요약에 평균이 아니라 합칠 수 있는 개수·합·제곱합을 저장 | 여러 판을 정확히 합쳐 평균·표준편차·검정을 계산 |
| M2 | 국소 밀도는 중심 2초 창 NPS, 문맥은 1초·8초 창 비로 burst/sustained | 고정 1초 창의 경계 분할(01 §5.1)을 없애고 순간 대 지속을 가름 |
| M3 | 약점 검정은 곡 군집 t 검정 + Benjamini–Hochberg(q 0.10), 가족 둘(정확도·타이밍) | 같은 판·곡 노트의 상관과 다중비교 위양성(01 §3.8 추정)을 다룸 |
| M4 | 밀도 한계는 등위회귀 곡선에서 EX −0.10, BP 8% 기준 | 단조 가정으로 표본 잡음을 줄이고 설명 가능한 숫자 하나로 |
| M5 | 프로필 창 90일·최대 300판, 지문 캐시 표 `play_profile` | 매 요청 재계산 비용을 없애고 업로드 훅 없이 최신 유지 |
| M6 | 약점 키 문법 하나를 리포트·연습 폴더·목표·추천·훈련곡이 공유 | "약점 → 훈련 대상" 을 기계가 잇는다(01 §5.2) |
| M7 | `ANALYSIS_VERSION` 도 2 로 올려 곡 특징에 NPS·조합 추가 | 추천·연습 폴더가 프로필과 같은 단위로 곡을 고름. 재계산은 한 번에 |
| M8 | 훈련곡은 1분 = 카운트인 1마디 + 노트, BPM 은 4의 배수(100~220), 구간 3~12 | 1분 고정이 정수 마디로 맞고 Rust 시간 계산과 일치(02 §4.5) |
| M9 | `#LNMODE 1` 고정, 지뢰 없음, 노트 키음 `hit.wav` 하나 + 카운트인 `count.wav` | LN 종류 통제(02 §4.3), 키음 클릭(D5) |
| M10 | `#SUBTITLE` 에 id 를 넣어 md5 를 사용자·곡마다 다르게 | `training_chart.md5` unique, 다른 사용자 곡과 섞이지 않음 |
| M11 | 생성 직후 서버가 `chart_pattern` 을 직접 저장, 훈련곡을 지워도 패턴은 남김 | 훈련곡 판이 `matched` 로 분석되고 지운 뒤에도 과거 분석이 흔들리지 않음 |
| M12 | 다운로드 응답 `Cache-Control: private, no-store` | 지우면 바로 못 받게(5 KB 라 비용 작음) |
| M13 | BMS 원 바이트를 DB 에 저장하고 zip 은 요청 때 조립 | md5 는 BMS 바이트로 고정, wav 는 코드 상수 |
| M14 | "훈련곡" 폴더 이름은 만들 때 화면 언어(훈련곡 / Training / トレーニング), 이후 고정 | 클라이언트는 이름을 그대로 보임 |
| M15 | 폴더 응답의 `download_url` 은 폴더 주인이 만든 훈련곡일 때만 넣는 선택 필드, 주소는 요청 origin 으로 | 다른 사람 곡 주소가 새지 않음, 옛 클라이언트·기존 화면 타입·시험과 호환, Preview 가 운영 주소를 내리지 않음 |
| M16 | 클라이언트는 일반 폴더의 없는 곡도 흐린 줄로 보임(`show_missing_charts` 를 따름) | 표 레벨과 같은 규칙. 링크 없으면 받기 버튼만 꺼짐 |
| M17 | 훈련곡 생성 LLM 결과는 같은 digest 면 캐시 재사용, 곡은 시드(생성 시각)로 매번 다름 | 호출 수 절약, 반복 생성도 새 곡 |
| M18 | 보관 한도·폴더·프로필 검사를 공급자 호출 전에 | 실패할 요청에 호출·사용량을 쓰지 않음 |
| M19 | 대시보드 분기는 서버(A안), 노트레이더 자리는 약점 패널 | 기존 선례, 웹에 레이더 값이 없음(03 §4.4) |
| M20 | 대시보드 15패널, 첫 화면 5개만 서버 프리페치·나머지 지연 로딩, AI 계열은 뷰포트 진입 뒤 | 03 §7.6 비용과 분당 30회 한도 |
| M21 | 대시보드 날짜는 KST(이번 주도 KST 월요일), 주간 요약 digest 는 기존 UTC | 기존 화면의 일별 기준과 맞춤, 주간 digest 는 캐시 키 호환 |
| M22 | 회원 대시보드에도 커뮤니티 최근 활동·서버 현황을 맨 아래 한 패널로 남김 | "최대한" 요청, 캐시된 데이터라 비용 없음 |
| M23 | `USER_SCOPED_QUERY_KEYS` 에 `DASHBOARD.ALL` 과 `DAN.MINE` 추가 | 로그아웃 뒤 다음 계정에 남는 누수 방지(03 §7.2) |
| M24 | 재계산 스크립트는 8분 상한(작업 제한 10분보다 짧게) 뒤 이어 하기, 배치 100, 출력은 숫자만, 새 서비스 메서드 없이 기존 공개 메서드만 | Actions 시간·공개 로그, 같은 웨이브 단위와 파일이 겹치지 않음 |
| M25 | 훈련곡 PLAYLEVEL 은 밀도(구간 최대 NPS) 정수, 난이도표 레벨과 무관 | 정직한 표기, 상세 화면에 설명 |
| M26 | 대시보드 최근 플레이는 판 단위 20행에 DJ 등급 라벨을 더하고, 곡당 최신 보기는 넣지 않음. 판정 습관·게이지 조언 한 줄은 AI 상태 패널에(결정론 GET), 리포트 헤드라인은 넣지 않음 | 기존 `/players` 최근 목록과 같은 단위, 공급자 호출 없이 보일 수 있는 것만 |
| M27 | **훈련곡의 공개 제목은 `R-BMS Training · 만든 날 · 강도` 고정**, 초점(약점) 이름과 LLM 문장은 넣지 않음 | 판이 랭킹·검색에 공개되므로(D4) 제목으로 약점이 드러나지 않게. 초점은 비공개 폴더 `note`·상세 화면에서 |
| M28 | 훈련곡 판은 프로필에서 한 군집으로 묶음 | 합성 차트만으로 약점 셀의 표본 등급·유의성을 채우지 않게. 훈련 효과는 일반 곡 판으로 확인됨 |
| M29 | SCRATCH AUTO 판은 밀도 한계·스크래치 셀에서 빼고, 건반 셀에는 넣음 | trace 에 자동 레인 노트가 없어 밀도가 낮게 잡힘. 현행 태그 규칙과 같음 |
| M30 | 한도 20곡은 저장 트랜잭션에서 사용자 행을 `FOR UPDATE` 로 잠근 뒤 셈 | 캐시 적중 경로는 동시 실행 슬롯을 잡지 않아 슬롯으로는 막을 수 없음 |
| M31 | 훈련곡 연결 키는 sha256(점수·trace 모두), 패턴 조회만 md5 | `score.chart_md5` 는 NULL 허용·무색인 |
| M32 | 레인·손 초점 훈련곡은 RANDOM 계열을 끄라고 안내하고, 구간 추이에서 RANDOM 판을 흐리게 표시(전후 비교에서는 빼지 않음) | 판 지표가 화면(물리) 레인 기준이라 전후 비교는 옵션과 무관하게 유효 |
| M33 | 버전을 올린 W1 코드는 재계산 action 이 든 W2 와 함께 푸시하고, 그 직후 count → 사용자 `!` apply | 낡은 요약으로 도는 기간을 최소로 |

---

## 12. 검토 반영 (2026-11-04, 1차 검토 지적 25건)

지적마다 실제 코드로 확인한 결과와 처리를 적는다. "반영" 은 문서를 고쳤다는 뜻이고, 고친 절을 함께 적었다. 반영하지 않은 부분은 이유를 적었다.

| # | 지적(요지) | 코드 확인 | 처리 |
| --- | --- | --- | --- |
| 1 | U7 의 필수 `download_url`·`downloadUrlOf` deps 가 U7 소유가 아닌 시험·화면 타입을 깨 W3 typecheck 실패 | 맞음. `tests/components/folder-contract.test.ts:23-24` 가 `FolderDetail` ↔ `FolderDetailOutput` 을 서로 대입, `tests/route/folders.test.ts:22, 211` 과 `tests/support/ai-score-fixture.ts:121` 이 서비스를 직접 생성, `tsconfig.json` `include` 가 `**/*.ts` | 반영(5.2, 9절 전제·U7·U11). 소유를 옮기는 대신 **변경 모양을 바꿨다**: `download_url` 은 선택 필드(없으면 뺌), `createFolderService` deps 는 그대로, `get` 에 선택 인자 `origin`. 세 시험·화면 타입이 고치지 않고 통과하므로 U9 와의 병합 순서 제약도 사라졌다. 화면 타입의 선택 필드는 U11 |
| 2 | 재계산 상한 20분이 작업 제한 10분보다 길다 | 맞음. `.github/workflows/web-db.yml:25` `timeout-minutes: 10` | 반영(2.8, 9절 U12, 10.2, M24). 상한 8분·배치 100, workflow 제한은 그대로. 상한 경로 출력 시험과 상한 < 작업 제한 시험 추가 |
| 3 | 동시 실행 슬롯이 20곡 한도 경쟁을 막는다는 근거가 틀림 | 맞음. `feature-runner.service.ts:102-107` 은 슬롯·사용량 전에 캐시 결과를 돌려준다, 슬롯 상한 5(`dto/ai.ts:46,52`) | 반영(4.4, 9절 U7, 10.1, M30). 저장 트랜잭션 첫 문장에서 `user` 행 `FOR UPDATE`(선례 `compose/folder.ts:36`) 뒤 count, 동시 두 저장 DB 시험 |
| 4 | 재계산 전 옛 모양 특징을 기능 5·6 이 `features.nps` 로 읽으면 TypeError | 맞음. `chart-pattern.service.ts:209` 는 `hasTagShares` 만 보고 `isStale` 로 돌려준다 | 반영(2.3, 2.6 기능 5·6, 9절 U2·U9). 접근자 `readChartFeatureExtras`(Zod `safeParse`), `null`·`isStale` 이면 감점·일치 0, v1 픽스처 시험 |
| 5 | `focus_key` 가 임의 문자열 | 맞음. `ai_result.subject_key varchar(128)`(`schema.ts:582`) | 반영(2.5.5, 3.1, 3.2, 2.6 기능 6·7, 9절 U6·U7). `weakness-key.ts` 의 `parseWeaknessKey`·`weaknessKeySchema`(64자), 요청 스키마 `trainingChartCreateSchema`, 프로필에 없는 키는 호출 전 422 |
| 6 | SCRATCH AUTO 판 취급이 프로필에 없음 | 맞음. `sample.ts:26-81` 이 auto 판을 스크래치 태그에서 빼고, 요약에 `scratch_auto`(`trace-analysis.ts:174`) | 반영(2.5.2, 2.5.6, 9절 U6, M29). 밀도·스크래치 셀은 auto 판 제외, 건반 셀은 포함(현행 규칙), `auto_plays` 표시 |
| 7 | `countAnalysesBehind` 는 공개 메서드가 아니고 `countStaleFeatures` 는 중복 | 맞음. `trace.service.ts:104`(deps.db 안), 공개는 `adminUsage().analysis`(361-366행), `coverage().stale_features`(176-179행) | 반영(2.8, 9절 U12). 새 메서드 없이 두 공개 메서드를 쓰고, U12 소유에서 서비스·compose 파일을 뺐다. 스크립트 배선(`listCoverageTables: () => []`) 명시 |
| 8 | U4 소유 밖 Rust 시험·코드가 컴파일 실패 | 맞음. `user_folders/tests.rs:66`, `tree_tests.rs:171`, `tables.rs:843`, 그 밖 `tablesrc.rs:1088`, `download_modal.rs:142-144` | 반영(6절 시험 파일·컴파일 영향 목록, 9절 U4). 예외 문구를 "컴파일 오류로 드러나는 모든 파일" 로 넓힘 |
| 9 | 인증 훈련곡 라우트의 팩토리·래퍼 출처가 없음 | 맞음. `route/ai.ts:67, 134` 비공개, export 는 `aiPrivateJson`·`toAiAppError`·`unwrapAiResult` | 반영(7.2, 9절 U6·U7). U6(W2)이 `createAiRouteWrappers`·`AiHandler` 에 `export` 만 붙이고, U7 이 새 `route/training-charts.ts` 의 `createAiTrainingChartRoutes` 와 deps 타입을 만든다. 같은 웨이브 U9 와 `route/ai.ts` 가 겹치지 않는다 |
| 10 | U6·U9 가 깨는 기존 시험이 소유 목록에 없음 | 맞음. `table-progress.test.ts`, `output-language.test.ts`, `ai-score-features.test.ts`, `ai-play-features.test.ts` 확인 | 반영(9절 전제의 기존 시험 규칙, U6·U9 소유·게이트). `output-language.test.ts` 는 W2 의 U6, W3 의 U9 가 차례로 소유 |
| 11 | 옛 형식 항목이 필수 스키마 검증에 실패 | 맞음(초안 기준) | 반영(5.2). 1번 처리로 `download_url` 이 선택 필드가 되어 픽스처는 서버가 실제로 내는 두 모양(필드 있음/없음)만 담고 TS·Rust 가 같은 파일을 쓴다. `''` 읽기는 Rust 시험의 인라인 JSON 으로 분리 |
| 12 | 서버가 `NEXT_PUBLIC_APP_URL` 로 자기 주소를 만듦 | 맞음. `docs/web/architecture.md` 6-1 의 규칙과 빌드 시점 고정 설명 | 반영(5.2, 7.2, M15). 라우트가 `new URL(request.url).origin` 을 넘기고 경로 도우미 `trainingDownloadPathOf` 만 공유한다. 아키텍처 규칙은 바꾸지 않는다 |
| 13 | TS zip → Rust `extract_zip` 경계 시험이 없음 | 맞음. `crates/rbms-download/src/extract.rs:264`, 기존 시험 도우미 `tests/extract.rs` 의 `unpack` | 반영(3.9, 3.10, 9절 U3, 10.1). 저장 방식 골든 `basic-160.zip` + Rust 시험. md5 대신 `.bms` 바이트 비교(새 의존성 없음) |
| 14 | 버전을 올린 W1 이 재계산 경로(W2) 없이 운영에 나감 | 맞음. `trace.service.ts:169, 277` 은 읽기당 10판만 다시 계산 | 반영(10.3, 10.2, M33). W1 은 W2 와 함께 푸시, 직후 count → `!` apply, 그 사이 동작을 알려진 동작으로 기록 |
| 15 | 마디 분배가 모호하고 `refine` 의 보정 기록이 저장되지 않음 | 맞음. `feature-runner.service.ts:148-161` 은 `refine` 결과를 그대로 저장하고 캐시 적중 때 다시 부르지 않는다 | 반영(3.4, 3.5, 9절 U3). 의사코드·불변식·종료 근거, `refine` 은 거절·글자 다듬기만, 구간 버리기·종류 대체는 `correctTrainingPlan` 이 매번 `corrections` 에 남김 |
| 16 | 검정 가족 배정 누락, timing 판 평균이 태그 중복 집계 | 맞음(01 §3.4 의 태그별 중복 집계) | 반영(2.5.3 가족 표, 2.5.4 timing 행, 9절 U6). 판 평균은 `timing_by_hand` 합(중복 없는 분할) |
| 17 | `analyzeTrace` 인자가 필수면 U2 게이트가 깨짐 | 맞음. 호출부 `trace.service.ts:192`, `tests/support/play-feature-fixture.ts:33` | 반영(2.2, 7.1, 9절 U2·U7). 선택 인자 기본 `null`, 배선은 U7 |
| 18 | overview 캐시 키에 날짜가 없고 연속일 창·동률·`total_players` 정의 없음 | 맞음. `getPlayerPlayDaysCached`(`read-cache.ts:299`) 는 `todayStart` 를 받는다, `compose/fe.ts:103` 정렬에 동률 규칙 없음 | 대부분 반영(8.3). overview·calendar 키에 `todayStart`, 연속일 창 371일, 공동 순위, `total_players` 정의. **courses 키에는 넣지 않았다**: 묶음별 클리어 수와 최근 코스 기록은 날짜가 지나도 값이 바뀌지 않으므로 날짜 키는 캐시 적중만 줄인다 |
| 19 | 판정 습관·리포트 헤드라인·게이지 조언·DJ 등급이 대시보드에 없음 | 맞음. 결정론 GET `useAiJudgeHabit`·`useAiGaugeAdvice` 가 있다(`entities/ai.query.ts:295, 309`) | 부분 반영(8.2 패널 6·14, M26). 판정 습관·게이지 한 줄은 패널 14 에, 최근 판에 DJ 등급. **리포트 헤드라인은 넣지 않았다**: GET 이 저장 해석을 주는지 이 단계에서 확인하지 않았고 새로 해석을 부르면 공급자 호출이 생긴다(부채 (9)). 곡당 최신 보기는 넣지 않은 이유를 8.2 에 적음 |
| 20 | LLM `title_suffix` 가 공개 제목에 들어가 약점이 드러남 | 맞음(D4 로 판이 공개) | 반영(3.3, 3.7, 5.3, 10.2, M27). LLM 스키마에서 `title_suffix` 삭제, 제목은 `R-BMS Training · 날짜 · 강도` 고정, 초점은 비공개 `note` 에. 사용자에게 알릴 결정으로 M27 에 둠 |
| 21 | 레인·손 초점 훈련곡이 RANDOM 계열에서 무의미 | 일부 맞음. 요약의 7K 지표는 "본 레인"(옵션 적용 뒤) 기준이다(`trace-analysis.ts` `analyzeTrace` 문서 주석, 요약에 `random` 있음 172행) | 부분 반영(7.1, 9절 U11, M32). 안내 문구와 구간 추이의 RANDOM 판 흐림은 반영. **전후 비교에서 RANDOM 판을 빼지는 않았다**: 전후 비교의 레인·손 값은 물리 레인 기준 실력이라 옵션과 무관하게 유효하고, 빼면 표본만 준다 |
| 22 | 서로 다른 훈련곡이 약점 셀의 K 를 채움 | 맞음(군집 = `chartSha256`, 훈련곡은 곡마다 다름) | 반영(2.5.1 `isTraining`, 2.5.2, 7.1, 9절 U6, M28). 훈련곡 판 전체를 한 군집으로, D4 는 유지 |
| 23 | 상세·removal·DELETE 소유자 검사 규칙 없음 | 맞음(초안 규칙 누락) | 반영(7.2, 9절 U7). `user_id` 조건, 타인 id 는 404, 라우트 시험 |
| 24 | 폴더를 지운 뒤 다시 만들면 기존 훈련곡이 빠짐 | 맞음(`training_folder.folder_id` CASCADE) | 반영(5.3, 9절 U7). 폴더에 없는 그 사용자의 훈련곡을 모두 최신순으로 넣고 시험 |
| 25 | 훈련곡 판 연결을 NULL 허용·무색인 `score.chart_md5` 로 함 | 맞음. `schema.ts:66` nullable, 색인은 `(user_id, chart_sha256, played_at)` 등 | 반영(7.1, 2.6 기능 9·10, 8.2 패널 6, M31). 점수·trace 모두 sha256, 패턴 조회만 md5 |
