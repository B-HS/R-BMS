# 레퍼런스 구현과의 의도적/보류 차이 (M0/M1 적대적 리뷰 결과)

> 워크플로우 `wfgr2tk4o`(21 에이전트) 결과. 18개 발견 중 14개 확정, 그러나 검증자 판정상 **거의 전부 현 corpus(727곡)에 미발현**. 아래는 처리 결정.

## 수정 완료 (이번 커밋)
| # | 항목 | 파일 | 수정 |
|---|---|---|---|
| 1 | 데이터필드 토크나이저가 비영숫자 필터 후 재페어링 → 위치 어긋남 | parser/lib.rs | **위치 기준 페어링**, 잘못된 쌍=빈칸(레퍼런스 구현 `parseInt36==-1→empty`와 일치) |
| 14 | `#BASE 62`에서 헤더 id는 base36 하드코딩, 객체값은 base62 → 불일치 | parser/lib.rs | `#BASE` 사전 스캔 + 헤더 id도 `src.base` 사용 |
| 11 | LNOBJ가 mine/LN으로 덮인 슬롯을 변환 → 손상 | chart/lib.rs | 추적 슬롯이 `Normal`일 때만 변환 |
| 8 | BPM≤0 / 음수 STOP → inf/NaN 전파 | chart/lib.rs | BPM≤0은 직전값 폴백, 음수 STOP은 0 클램프 |

검증: 회귀 테스트 4종 추가, 전체 30 테스트 통과, corpus 727/727 파싱·MD5 불변, 노트수 불변(α=2582, A+=2418, FELYS=812).

## 보류 (현 corpus 미발현 — 추후 필요 시)
| # | 항목 | 이유 |
|---|---|---|
| 2 | 중첩 `#IF`를 모든 프레임으로 게이트(레퍼런스 구현는 최내곽만) | corpus 최대 중첩 깊이 1, #RANDOM 사용 4곡뿐. **#RANDOM은 런타임 랜덤이라 해시·점수 무관** |
| 3 | `#RANDOM` 없는 고아 `#IF`를 비활성화(레퍼런스 구현는 유지) | 에디터가 항상 짝을 생성, 발현 희박 |
| 4 | `#ELSE/#ELSEIF` 구현(레퍼런스 구현는 미구현) | 우리 쪽이 spec-complete. corpus에 #ELSE 0건 |
| 5,10 | ch03 inline BPM의 G–Z 글자 처리 차이 | 실차트는 00–FF(hex)만 사용 → 우리 base16이 정확 |
| 6 | `#BASE 62`는 레퍼런스 구현에 없음 | 우리 확장. #14로 자체 일관성 확보. corpus 0건 |
| 7 | ch06(POOR layer)가 base BGA 필드 덮음 | BGA 렌더러 미구현(필드 write-only). BGA 단계에서 분리 |
| 9,12 | `#LNTYPE 2`(MGQ 연속형 LN) 미구현 | 사실상 사장된 레거시. corpus 0건. LNTYPE1/#LNOBJ는 구현·테스트됨 |
| 13 | 고아 `#LNOBJ` 꼬리를 Normal로 방출 | 권위 있는 동작 불명, Normal이 안전 폴백 |

## CN/HCN 판정 차별화 — ✅ 종단 판정 적용 (2026-06-03)

> 충실 포팅 스펙·구현 = `docs/reference/cn-hcn-judgment.md` (레퍼런스 구현 `JudgeManager` 라인 대조).

`judge/matcher.rs`가 `LnKind`를 운반해 **CN/HCN을 2-판정(head@press + end@release)으로** 처리한다(레퍼런스 구현 `updateMicro` ×2). 이른 릴리스는 end 윈도우로 판정(head로 capping 안 함), 미히트는 head+end 2 Miss. `count_playable_notes`/`total_notes`/gauge 분모를 CN/HCN=2로 일관. **`Cn`/`Hcn`에만 게이트**해 LN/Normal은 byte 불변. 합성 픽스처 5종으로 고정.
**잔여:** HCN 연속 게이지(`hcnmduration` ±0.5, Phase 7), CN deferral/재홀드, BSS. 노트-카운트 분모는 레퍼런스 구현 BMS 모델이 외부 라이브러리라 미검증 — `JudgeManager` 2×updateMicro 증거 기반 + 내부 일관(→ 스펙 "미검증 경계").

## 판정 윈도우·게이지 발산 (2026-09-09 감사)

> 근거: `docs/plan/2026-09-09-enhancement-plan.md` §1.1(판정·게이지 관점 감사) + Phase A 구현·적대적 리뷰 결과(`scratchpad/phase-a/{core,review-core,fix-core}.md`). 대상 커밋 `c6f0885`. 이전까지 이 문서에는 판정 윈도우·게이지 섹션이 없었다(누락형 stale) — 이번 신설로 해소.

**일치(발산 아님, 확인 완료):** 7K NOTE 윈도우, 7K 게이지 6종 전 수치·TOTAL/LIMIT_INCREMENT 공식·guts·사망 후 동결·클리어 판정·EX스코어, 空POOR 노트 미소실·콤보 유지, 후보 게이트, 見逃し 스윕 경계, autoplay PG, CN/HCN 머리 판정.

| # | 항목 | rbms(감사 시점) | 레퍼런스 구현 | 효력 | 상태 |
|---|---|---|---|---|---|
| J1 | **見逃し POOR 게이지 슬롯** — 스윕 미스가 idx5(MS)로 들어가 게이지 페널티 과대 | `Judge::Miss`(idx5)로 push | 코드 4(PR) → value[4] | S | **Phase A 수정 완료** |
| J2 | 空POOR 카운트가 `counts[]`에 미집계 → IR `ems=0` | 별도 카운터만 | `addJudgeCount(5)` → ems/lms | S | **Phase A 수정 완료**(J1과 한 묶음) |
| J3 | combocond 테이블 — Poor·Miss 모두 콤보 리셋(空POOR 마스킹) | 모드 무관 동일 | 7K만 空POOR 콤보 유지, 5K/PMS는 끊음 | S | **Phase A 수정 완료** |
| J4 | 5K NOTE 윈도우가 7K 표 사용 | 7K 표 | ±20/50/100/150, MS(-150,500) | S | **Phase A 수정 완료** |
| J5 | PMS NOTE 윈도우가 7K 복제 | 7K 표 | ±20/50/117/183, MS(-175,500) | S | **Phase A 수정 완료** |
| J6 | 24K(KEYBOARD) 모드 자체 없음 | 미구현 | ±30/90/200, (-320,240), MS(-200,650) | M | **Phase D 해소(판정·게이지 경로)** — `Mode::KEYBOARD_24K` 신설, `judge.ron` KEYBOARD 행·`GaugeSetId::Keyboard`·`JudgeProperty::defaults_for_mode` 분기까지 연결. `Mode::ALL` 미포함이라 UI 도달 경로는 Phase G |
| J7 | 7K LN 종단 윈도우가 실은 5K(FIVEKEYS) 값 | 5K 값 오적용 | (120,160,200,(-280,220)), MS 없음 | S | **Phase A 수정 완료** |
| J8 | PMS LN 종단이 7K(=5K) 복제 | 7K 표 | 120/150/217/283 | S | **Phase A 수정 완료** |
| J9 | 스크래치 NOTE/LN 윈도우·후보 게이트 없음(키와 동일) | 키와 동일 | 7K scr ±30/70/160,(-290,230); longscratch 130/170/210,(-290,230); 5K 별도 | M | **Phase A 수정 완료**. 정/역 2키(BSS·MSS)는 **Phase D 해소** — `ScratchDir` 가 판정 엔진에서 앱 입력·리플레이까지 배선됨 |
| J10 | LN 릴리스 마진이 전모드 `LN_MARGIN=200_000` | 전모드 동일 | 7K/5K/KB 0, PMS 200000, `longnoteMarginRate` 설정 | S | **Phase A 수정 완료** |
| J11 | plain LN 미릴리스 확정 방식 | 마진 후 종단 윈도우 재판정 | `lnstartJudge`(머리 판정)로 확정 | S | **Phase A 수정 완료** |
| J12 | JUDGE WIDTH — judgerank×rate를 PG~BD 일괄, 클램프 없음 | 위와 동일 | PG/GR/GD 3개만, MS 상한·단조 클램프 | M | **Phase A 수정 완료**. 다만 표의 "키/스크 분리" 서술은 **오기였다** — 아래 D-D1 참조 |
| J13 | `#RANK` 범위 밖이 clamp(0,4) → 25/125 | clamp | NORMAL(75) 폴백 | S | **Phase A 수정 완료** |
| J14 | `#DEFEXRANK` 미파싱 | 미파싱 | judgerank×75/100, `<=0`은 NORMAL 75 폴백 | S | **Phase A 수정 완료**(리뷰에서 `<=0` 분기 추가 정정) |
| J15 | 기본 TOTAL이 헤더 없으면 200 고정 | 200 고정 | `max(260, 7.605n/(0.01n+6.5))`, KB는 별도 floor | S | **Phase A 수정 완료** |
| J16 | 지뢰가 판정 대상 제외, 데미지 없음 | 데미지 없음 | 눌린 채 통과 시 `gauge.addValue(-damage)` | S | **Phase A 수정 완료**(데미지 스케일 자체는 아래 보류 참조) |
| J17 | JudgeAlgorithm이 Duration 고정 | Duration 고정 | Combo(기본)/Duration/Lowest/Score 선택 | M | **Phase D 해소** — 기본값을 `Combo` 로 전환(`algorithm.rs` `#[default]`), JUDGE 탭에 4종 노출. 구 리플레이는 `ReplayJudge.algorithm` 결측 시 `Duration` 으로 읽혀 원 판정 그대로 재생된다 |
| J18 | 판정 완료 노트 재타격이 후보 제외(무판정) | 무판정 | MS 범위면 空POOR | S | **Phase A 수정 완료**(리뷰에서 커서 전진 후 재타격 경로 사망 결함 추가 발견·수정) |
| J19 | fast/slow `dm==0`이 LATE로 집계 누락 | 누락 | EARLY | S | **Phase A 수정 완료** |
| J20 | 게이지가 단일 `Gauge`만 갱신(9종 병렬 아님) | 단일 | 9종 전부 갱신, 선택만 표시 | M | **Phase D 해소** — `GrooveGauge` 가 9개를 병렬 갱신하고 선택 슬롯만 클리어를 정한다 |
| J21 | 게이지 세트가 7K 6종뿐 | 7K 6종 | 7K 9종(CLASS 계열 추가) + 5K·PMS·KB·LR2 각 9종 | M | **Phase D 해소** — 5세트 45원소를 `gauge_tables.rs` 에 이식, pin 테스트가 전 원소를 고정 |
| J22 | MODIFY_DAMAGE(EXHARD/HARD_LR2 등) 없음 | 없음 | EXHARD_5/HARD_LR2/EXHARD_LR2 | S | **Phase D 해소** — 게이지 생성 시 1회 적용(`GrooveGauge.java:274-291` 구조 그대로) |
| J23 | PMS 판정 규칙(`MissCondition`·`JudgeWindowRule.PMS` fixjudge) 없음 | 데이터만 반영, 엔진 미배선 | per-index fixjudge + fixmin/fixmax 클램프 | M | **Phase D 해소** — `JudgeWindowRule` 의 fixjudge·fixmin/fixmax 클램프와 `judge_vanish`/`MissCondition::One` 엔진 배선까지 완료. `passnotes` 도 소비한 판정만 센다(`JudgeManager.java:640-646`) |
| J24 | CN deferral / HCN 연속 게이지 / BSS·MSS 없음 | 없음(문서상 Phase 7) | `JudgeManager` 해당 로직 | L | **Phase D 해소** — deferral·plain LN 헤드 확정·HCN 200 ms 틱·BSS/MSS 전부 배선 |
| J25 | lnmode 강제(LN→CN/HCN)가 차트 `#LNMODE`만 | 차트 값만 | `PlayerConfig.lnmode` 0~3으로 강제 가능 | S | **Phase D 해소** — 적용 정본은 차트 변환부(`rbms-chart`)이고, 앱이 모델을 만들 때 해소하므로 노트 수·`#TOTAL` 폴백·게이지 분모가 전부 같은 값을 본다. `JudgeEngine::set_ln_mode` 는 원본 flavour 를 보존해 멱등한 방어 경로로만 남는다 |
| J26 | GAS(게이지 자동전환)/bottom shiftable gauge 없음 | 없음 | `gaugeAutoShift` 5모드 + `bottomShiftableGauge` | M | **Phase D 해소** — 5모드 + 0..2 클램프, `GaugeAutoShift::None` 의 stage failed 전이까지 앱에 배선 |

**파급:** J1+J2로 어긋나 있던 IR 제출의 `pr/ms`(epr/lpr/ems/lms) 4필드는 Phase A 수정으로 정합됐다(`app_play.rs`, 앱 수정 없이 그대로 정확해짐).
**Phase A 리뷰에서 추가로 드러난 결함(계획 표에는 없던 것)**: 재타격(空POOR) 판정이 판정 사다리(PG→BD→MS)를 타 BD 안이면 空POOR가 아예 안 나던 결함, `JudgeEngine::from_model`이 `apply_mode` 결과를 덮어써 5K/PMS가 7K 윈도우를 쓰던 결함(`from_model_for_mode` 신설로 해결), `scaled()`의 하한 클램프가 `max(1)`이라 judgerank 0에서 원본과 다르게 붕괴하지 않던 결함 — 전부 수정 완료.
**보류(1차 출처 미확인)**: 지뢰 데미지의 채널값→게이지 퍼센트 변환 배율. 레퍼런스 구현 `BMSDecoder`가 바이너리(jbms-parser.jar)이고 이 환경에 JRE가 없어 디컴파일 불가. 현재는 채널 id 값을 그대로 데미지로 취급(레포 기존 결정, `docs/reference/_appendix-raw.md:371-372` 근거). 확정하려면 JRE 환경에서 `BMSDecoder` 지뢰 채널 파싱을 직접 대조해야 한다.

## Phase D — 판정 패리티 완성 (2026-09-10)

> 근거: `docs/plan/2026-09-09-phase-d-spec.md`, 결정 12(`docs/acknowledge/2026-09-09-enhancement-decisions.md`). 위 표의 J6·J9(정역 2키)·J17·J20~J26 과 C-D1·C-D3 이 이 단계에서 해소됐다. 아래는 그 과정에서 **새로 만들어졌거나 이번에 정정된 차이**만 적는다.

| # | 항목 | rbms | 레퍼런스 구현 | 사유/영향 |
|---|---|---|---|---|
| D-D1 | **JUDGE WIDTH 의 스크래치 rate 적용 범위 (J12 정정)** | 네 판정 테이블(note/scratch/longnote/longscratch)을 **전부 key rate** 로 만들고, scratch rate 는 후보 게이트(`JudgeWindowSet::candidate_gate`)에만 쓴다 | `JudgeManager.java:198` 이 `new JudgeWindow(rule, judgerank, keyJudgeWindowRate)` 로 **단 한 벌**을 만들고, `scratchJudgeWindowRate` 는 `:185-197` 의 `smjudge`(후보 게이트) 에만 쓴다 | **패리티(발산 아님).** 이전 표의 J12 "키/스크 분리" 서술이 오기였다 — rbms 가 스크래치 테이블에 scratch rate 를 곱하던 것이 실제 발산이었고, 이번에 레퍼런스대로 되돌렸다. SCR PG 50% 설정 시 7K 스크래치 PG 가 (-15000,15000) 이던 것이 원래대로 (-30000,30000) |
| D-D2 | **LR2 게이지 세트를 사용자 설정으로 노출** | JUDGE 탭 `GAUGE SET = AUTO / LR2` | `GrooveGauge.java:123-149` 는 코스 제약(`CourseDataConstraint`)으로만 LR2 세트에 도달 | rbms 에 코스 모드가 없어 도달 경로 자체가 없다. 설정 행으로 대체(스펙 §11-4 의 명시적 설계 결정) |
| D-D3 | **`dj_rank` 의 `max_ex == 0` 처리** | 밴드 0(F) | `ScoreDataProperty.java:101` `rate = totalnotes == 0 ? 1.0f` → AAA | 노트 0개 차트는 실사용 경로가 아니다. rbms 현행 유지, `crates/rbms-render/src/result.rs` 의 `dj_rank_zero_max_is_lowest` 가 고정 |
| D-D4 | **KEYBOARD 게이지 세트의 init 혼재** | ASSIST_EASY_KB `init 30`, EASY_KB/NORMAL_KB `init 20` 을 원문 그대로 | `GaugeProperty.java:107-115` 원문이 그렇다 | 오타처럼 보이지만 **수정하지 않는다**(스펙 §2 명시). 24K 는 아직 UI 도달 경로가 없어 실영향 0 |
| D-D5 | **PMS/KEYBOARD 의 빈 스크래치 테이블을 키 테이블로 대체** | `JudgeProperty::PMS`/`::KEYBOARD` 의 `scratch`/`ln_scratch_end` 에 각각 note/ln_end 표를 복사 | `JudgeProperty.java:35,38,46,49` 는 `new long[]{}` — 빈 배열이라 `getJudge` 가 루프를 한 번도 돌지 않아 **항상 0(PERFECT GREAT)** 을 돌려준다 | POPN_9K 는 스크래치 레인이 없어 무해. KEYBOARD_24K 는 스크래치 레인(24,25)이 있으나 `Mode::ALL` 에 없어 도달 불가 — 24K 를 실제로 여는 Phase G 에서 "빈 표 = 항상 PG" 의미론을 그대로 이식할지 결정한다. `windows.rs` 의 `rows_without_a_scratch_lane_stand_in_their_key_tables` 가 현행을 고정 |
| D-D6 | **assist 1(AUTO SCRATCH) 런의 리플레이 저장** | 저장한다(`saves_replay(assist) = assist < 2`) | `MusicResult.java:317-321` 이 `resource.isUpdateScore()` 로 게이트하고, assist 를 올리는 모든 분기가 `score=false` 로 만들므로(`BMSPlayer.java:203-252`) assist 1 에서도 저장하지 않는다 | 결정 12 는 assist 2 에 대해서만 리플레이 차단을 문구화했다. assist 1 의 리플레이는 rbms 가 유지하는 편의 기능 |
| D-D7 | **GAS 로 전환된 런의 게이지 보고** | 실제로 클리어를 정한 게이지(`PlaySummary::finished_gauge`)를 기록·제출한다 | `BMSPlayer.java:884` `score.setGauge(gauge.isTypeChanged() ? -1 : gauge.getType())` | 로컬 `ScoreRecord.gauge` 도 IR `options.gauge`(`rbms_ir::GaugeType`)도 `-1` 에 해당하는 값이 없다. "NORMAL 게이지로 HARD 램프" 같은 모순을 피하려면 전환 후 게이지를 보고하는 편이 정확하다 |
| D-D8 | **리플레이 이벤트의 스크래치 방향이 IR 와이어에 없다** | 로컬 `ReplayEvent.backward` 로 기록·재생 | 해당 없음(rbms 고유 포맷) | `rbms_ir::ReplayEvent` 에는 방향 필드가 없어, 업로드/다운로드를 거친 리플레이는 전부 정회전으로 재생된다. 필드 추가는 IR 계약 변경이라 별건 |
| D-D9 | **구 세대 어시스트 기록의 클리어 램프** | `rule_version < 2` 이면서 `assisted` 인 기록은 셀렉트 LED 집계에서 제외 | `PlayDataAccessor.java:427-430` 은 이미 강등된 램프만 올린다 | Phase D 이전 빌드는 램프를 강등하지 않고 저장했다. 그 기록을 그대로 올리면 어시스트 클리어가 정규 클리어 LED 로 소급 상승한다 |
| D-D10 | **스코어 기록을 LN MODE 로 키잉** | `ScoreRecord.ln_mode` — 차트가 `#LNMODE` 를 말하지 않을 때만 실제 LN MODE, 그 외에는 `CHART` | `PlayDataAccessor.java:200-205` `containsUndefinedLongNote() ? lnmode : 0` | **패리티**(신규 발산 아님). 구 기록은 `CHART` 로 읽혀 서로 비교되므로 기존 베스트가 사라지지 않는다 |

**미해소로 남긴 것**: TARGET 행(§7 13행)은 값 저장만 하고 페이서 계산은 Phase F, 24K 의 렌더 레인 레이아웃·차트 스캔은 Phase G, `gauge.ron` 의 비-7K 세트 수치 1차 출처 대조는 `gauge_tables.rs` 의 컴파일 내장 표(레퍼런스 `GaugeProperty.java` 직접 이식 + pin 테스트)로 대신했다.

## Phase D — 적대 리뷰 23건 반영 (2026-09-10)

> D0~D5 산출물에 대한 적대 리뷰(critical 1·major 12·minor 10)를 전부 수정했다. 요약은 `docs/history/2026-09-09-phase-d-judge-parity.md`. 여기서는 그 과정에서 확정된, **레퍼런스와의 관계가 명확한 항목**만 file:line 과 함께 남긴다. 위 D-D1(§79행)은 이 리뷰의 critical 지적("scratch JUDGE WIDTH 가 네 테이블 전부에 적용됨")을 되돌린 결과다 — 새 항목이 아니라 갱신이므로 여기서 재서술하지 않는다.

| # | 항목 | rbms | 레퍼런스 구현 | 상태 |
|---|---|---|---|---|
| D-F1 | **`JudgeAlgorithm` 기본값** | `#[default] Combo`(`crates/rbms-judge/src/algorithm.rs`), `crates/rbms-config` 의 `JudgeOptions::default()` 도 동일 | `JudgeManager.java` 의 `defaultAlgorithm = {Combo, Duration, Lowest}` — `Combo` 가 0번째 | **패리티 달성.** C-D1 이 "Phase D 에서 해소" 라 적었던 것의 실제 반영. 구 리플레이는 `ReplayJudge.algorithm` 결측 시 `Duration` 으로 읽혀 원 판정 그대로 재생된다(하위호환, 신규 발산 아님) |
| D-F2 | **PMS 판정 규칙이 매처에도 적용됨** | `JudgeEngine::apply_mode`/`from_model_for_mode` 가 `JudgeWindowRule::for_mode(mode)` 로 룰을 뽑아 `window_set` 을 만든다(`crates/rbms-judge/src/matcher.rs:338-342`) | `JudgeManager.java:198` 이 모드별 `JudgeWindowRule`(Normal/PMS, `JudgeProperty.java:209-224`) 로 윈도우를 만든다 | **패리티 달성.** 이전에는 `window_set` 호출부가 `JudgeWindowRule::Normal` 을 하드코딩해 POPN_9K 도 Normal 스케일을 탔다 |
| D-F3 | **KEYBOARD_24K 의 판정 표 폴백** | `JudgeProperty::defaults_for_mode` 가 `crate::data::KEYBOARD_24K_KEY` 를 `Self::KEYBOARD` 로 매핑(`crates/rbms-judge/src/windows.rs:327-331`) | `JudgeProperty.java` 는 모드 이름으로 상수 테이블을 직접 선택하므로 폴백 개념이 없다 | rbms 고유의 데이터 로드 실패 폴백 경로에 대한 보강(패리티 아님, rbms 내부 정합성). `data/judge.ron` 에 KEYBOARD 행이 있는 한 도달하지 않는다 |
| D-F4 | **LN MODE 가 리플레이·스코어 키에 기록됨** | `ReplayEvent`/`ReplayHeader` 에 `ln_mode`·`gauge_set`·`gauge_auto_shift`·`bottom_shiftable_gauge` 필드 신설(`crates/rbms-store/src/replay.rs`), 구 리플레이는 `REPLAY_LEGACY_*` 상수로 읽는다 | 해당 없음(rbms 고유 리플레이 포맷 — 레퍼런스 IR 와이어와 별개) | D-D10(§88행)과 짝 — 스코어는 이미 LN MODE 로 키잉되고 있었고, 이번에 리플레이 포맷도 같은 축으로 맞췄다. 순수 rbms 포맷 확장이라 레퍼런스 대조 대상이 아니다 |
| D-F5 | **역회전 스크래치(BSS/MSS)가 앱까지 배선됨** | `apps/rbms-player/src/app_input.rs` 의 `lane_input_for` 가 `ScratchDir::Backward` 를 판별해 `apps/rbms-player/src/app_play.rs` 를 거쳐 `JudgeEngine::press_dir`(`crates/rbms-judge/src/matcher.rs`) 까지 전달 | `JudgeManager.java:358-375` 백스핀 스크래치 종료·재파지 취소 | **패리티 달성.** 리뷰 전에는 엔진에 `press_dir` 이 있었지만 앱 입력 경로가 방향을 버리고 있었다 |
| D-F6 | **assist 1(AUTO SCRATCH) 런의 리플레이 저장 — 재확인** | 여전히 저장한다(`apps/rbms-player/src/lib.rs`) | `MusicResult.java:317-321` + `BMSPlayer.java:203-252` 는 assist 를 올리는 모든 분기에서 저장하지 않는다 | **의도적 미해소.** D-D6(§84행)과 동일 사안 — 리뷰가 다시 지적했으나 결정 12 의 범위(assist 2 만 차단)를 재확인하고 유지했다. rbms 가 유지하는 편의 기능으로 계속 기록 |

**리뷰 항목 중 divergence 표에 오르지 않는 것**: `app_play.rs` 800줄 상한 초과(693→833, 구조 위생), `assisted_lamp` catch-all 강도 역전, `ir_sync.rs` 스키마 마이그레이션 누락, `PlaySession::seek`/`Player::release` 의 `auto_lanes` 무시는 전부 **rbms 내부 결함**이었고 레퍼런스와 비교할 대상 자체가 없어(rbms 고유 구조) 여기 신규 행을 만들지 않는다 — 수정 사실은 `docs/history/2026-09-09-phase-d-judge-parity.md` 의 표에 남긴다.

## 기각 (not-a-bug)
- `#SETRANDOM/#RONDAM` 수용(우리 확장, 무해)
- 데이터라인 감지 colon@byte5(현 구현 충분)
- mechanics.md §2 STOP 공식 본문이 `/192` 누락(코드는 정확)
- 미종결 LN(LN 채널 홀수 개) → dangling LongStart 잔존(허용)

## 설계 입장
**#RANDOM 계열 제어흐름은 레퍼런스 구현과 bug-for-bug 호환하지 않는다.** 차트 해시는 raw 바이트 기준이라 점수·IR·리플레이 키는 영향 없고, #RANDOM 분기는 본래 플레이 시 무작위다. 우리 구현은 #ELSE/#ELSEIF/중첩을 spec에 맞게 더 완전히 처리한다 — 이는 의도된 개선이다.

## Phase B — 오디오 클럭 재설계 (2026-09-09)

> 근거: `docs/plan/2026-09-09-phase-b-spec.md`, 구현 요약 `docs/history/2026-09-09-phase-b-audio-clock.md`. 아래 항목은 **레퍼런스 패리티 주장이 아니라 rbms 독자 설계**다 — 레퍼런스 구현의 `AudioDriver`/`PCM` 계열 소스는 이번에도 열지 않았다(spec §8-1, 미확인 사항으로 유지).

| # | 항목 | rbms | 레퍼런스 구현 | 근거/사유 |
|---|---|---|---|---|
| B-D1 | 어택/릴리스 램프 | `ATTACK_MS=1.0`/`RELEASE_MS=3.0` 선형 엔벨로프를 모든 보이스 시작·정지·재트리거·스틸에 적용 | 미대조(패리티 확인 안 함, `PCM.java` 등 미열람) | 클릭 제거(§2 완료정의 5)가 목적. 레퍼런스가 페이드를 쓰는지 여부와 무관하게 rbms가 독자 도입 |
| B-D2 | 마스터 게인 재배치 | `master_gain` 기본 1.0 + `bus_gain[System/Key/Bg]` 기본 0.5(신설) + `chart_gain`(=`#VOLWAV`) — 3단 곱셈 | 레퍼런스는 per-voice `keyvolume`/`bgvolume` 0.5 를 직접 곱함(버스 개념 없음) | 기존 rbms `DEFAULT_MASTER_GAIN=0.5`는 이를 마스터 1회로 근사한 값이었음(Phase A 주석). Phase B가 버스로 정확히 분리하며 마스터를 1.0으로 되돌림. 최종 진폭은 불변(0.5×1.0 = 1.0×0.5, 회귀 테스트로 고정) |
| B-D3 | 룩어헤드 | `playback_ahead + (buffer_frames + LOOKAHEAD_EXTRA_FRAMES) / rate` — 스케줄러가 버퍼 경계보다 미리 예약하는 시간창. 프레임 폴링 간격은 앱이 별도 소유(`schedule_poll_interval_us`) | 레퍼런스의 스케줄링 룩어헤드 방식은 미대조 | rbms 고유 문제(A2: `delay<=0`이면 즉시 발음으로 붕괴)를 해소하기 위한 신설 메커니즘. 레퍼런스와 대응 개념이 있는지 여부와 무관 |
| B-D4 | 오디오 클럭 축 분리 | `audible_us`(보간, 사람이 듣는 위치) vs `scheduled_us`(=audible+lookahead, 예약 전용) 를 엔진이 분리 노출. `rbms-play::Player`도 `update_schedule`(scheduled 축)/`update_judge`(audible 축)로 분리 | 미대조 | 레퍼런스 아키텍처(단일 스레드 폴링 모델)와 직접 비교하지 않았다. rbms는 cpal 콜백 기반 보간 클럭이 필요해 신설한 구조 |
| B-D5 | 보이스 스틸 정책 | 비활성 → Release 중 env 최소 → gain×env 최소(동률 시 최고령) 3단계, 스틸 대상은 "즉시 교체"가 아니라 "피해 보이스 페이드 완료 후 같은 슬롯에 예약 시작"(`Voice.pending`) | 레퍼런스 소스(`AudioDriver.java` 등) 미열람 — 정책 자체를 대조하지 않음(spec §8-1 명시) | rbms 독자 설계. 클릭 없는 스틸(경계 스텝 < 0.01)을 목표로 리뷰에서 재설계됨(초판은 제자리 덮어쓰기로 0.3248 풀스케일 클릭 발생) |

**공통 근거**: `docs/plan/2026-09-09-phase-b-spec.md` §8-1 "보이스 스틸 정책·페이드 유무·리샘플 방식은 미대조 — §3.4의 스틸/램프는 rbms 독자 설계이며 레퍼런스 패리티 주장이 아니다"를 그대로 따른다. 위 5항목 모두 향후 레퍼런스 `AudioDriver`/`PCM` 계열을 직접 열어 대조하기 전까지는 "일치/불일치" 판정 없이 **rbms 설계**로만 기록한다.

## Phase C — 구조 개편 (2026-09-09)

> 근거: `docs/plan/2026-09-09-phase-c-spec.md` §6.1·§6.2 + §9 step 14. 아래 3항목은 Phase C 가 데이터화·enum 화를 하면서 **명시적으로 남긴 차이**다. 어느 것도 판정 결과를 바꾸지 않았다(Phase C 는 동작 보존 리팩터링).

| # | 항목 | rbms | 레퍼런스 구현 | 상태 |
|---|---|---|---|---|
| C-D1 | **후보 선택 기본값** — `JudgeAlgorithm` 의 기본값이 `Duration`(누른 시각과의 `|Δt|` 최소 후보) | ~~`#[default] Duration`~~ → **`#[default] Combo`** | `JudgeAlgorithm.Combo` 가 기본, `defaultAlgorithm = {Combo, Duration, Lowest}` | **Phase D 에서 해소.** 전환을 막던 조건(리플레이가 알고리즘을 기록하지 않음)이 `ReplayJudge.algorithm` 으로 해소돼, 기본값을 `Combo` 로 옮기고 기대값이 바뀌는 매처 테스트만 `Duration` 회귀본과 `Combo` 신규본으로 분리했다 |
| C-D2 | **`JudgeAlgorithm` 을 trait 이 아니라 enum 으로** | `pub enum JudgeAlgorithm { Combo, Duration, Lowest, Score }` + `prefer(...) -> bool` | 레퍼런스도 enum(`JudgeAlgorithm.java`) | 계획 §2 C-4 는 trait 을 적었으나 스펙 §6.2 말미에서 enum 으로 확정. 사유: ① 설정 파일(`rbms-config::JudgeOptions.algorithm`)에 직렬화돼야 하고 ② 레퍼런스가 enum 이며 ③ 동적 확장 요구가 없다. 사용자 정의 알고리즘이 필요해지면 `trait JudgeSelector` 를 추가하고 enum 이 구현하는 형태로 확장한다(Phase C 범위 밖) |
| C-D3 | **게이지가 모드당 6종** — `gauge.ron` 의 `GaugeSet` 은 ASSIST EASY/EASY/NORMAL/HARD/EX-HARD/HAZARD | ~~6종 1행~~ → **5세트 × 9종** | `GaugeProperty` 는 모드당 9종(+ CLASS/EXCLASS/EXHARDCLASS) | **Phase D 에서 해소**(J20·J21). 코스 모드가 없어 CLASS 계열 3종을 *선택*할 수는 없지만, 9개를 전부 만들고 갱신하므로 GAS 가 올라탈 수 있다 |

**참고**: `judge.ron` 은 `BEAT_5K`/`BEAT_7K`/`BEAT_10K`/`BEAT_14K`/`POPN_9K`/`KEYBOARD_24K` 6행으로 이미 전 모드를 담고, 내장 const 표와의 **패리티 가드 테스트**가 파싱 결과 == const 를 필드 단위로 단언한다. 반면 `gauge.ron` 은 `BEAT_7K` 1행뿐이다(다른 모드의 게이지 수치는 아직 1차 출처 대조 전).

**데이터 소비 경로(리뷰 반영)**: 엔진이 실제로 읽는 표는 이제 데이터 파일이다. `JudgeProperty::for_mode` 는 `builtin_judge_tables()` 를, `gauge::params` 는 `builtin_gauge_tables()` 를 조회하고, 행이 없을 때만 컴파일 내장 표(`JudgeProperty::defaults_for_mode` / `gauge::default_params`)로 폴백한다. 패리티 가드는 그 내장 표를 기준값으로 삼으므로 여전히 실질 검증이며, 별도로 "조회 결과 == 데이터 파일" 을 단언하는 테스트가 소비 경로 자체를 고정한다(스펙 §6.1 의 "데이터 로드 실패 시 fallback 겸" 이 실제 폴백이 되었다).

## Phase C — 스펙 대비 의도적 이탈 (2026-09-09, 리뷰 반영)

> 아래는 레퍼런스 구현과의 차이가 아니라 **`docs/plan/2026-09-09-phase-c-spec.md` 대비 이탈**이다. 어느 것도 동작을 바꾸지 않으며, 전부 이 문서에 남기는 조건으로 유지한다.

| # | 스펙 | 구현 | 사유 |
|---|---|---|---|
| C-S1 | §2.2 — 곡목록 상태(`select_view`/`select_items`/`search`/`searching`/`sort`/`sel`/`select_gen`)는 `SelectState` 로, `kc_edit_mode` 는 `KeyConfigState` 로 | 둘 다 `AppShared` 유지 | 목록은 화면보다 오래 산다: 다른 화면이 브라우저로 복귀하고, `Tables`/`Folders` 가 목록을 재구성하며, `Loading` 이 루트로 되돌리고, 디버그 오버레이가 어느 화면에서든 커서를 보고한다. `kc_edit_mode` 는 키컨피그 화면이 방문마다 새로 만들어지므로 화면에 두면 "편집하던 모드 기억" 동작이 사라진다. 이 때문에 `AppShared` 는 스펙이 추정한 ~40 이 아니라 실측 ~69 필드다 |
| C-S2 | §2.3 — 디스패치는 트레이트 오브젝트가 아니라 `impl Stage` 의 메서드별 `match` | `Stage::handler()`/`view()` 단 두 곳에서 전 variant `match` 로 `&mut dyn StageHandler` 를 뽑고, 각 메서드는 그 위임 1줄 | 스펙이 `match` 를 요구한 근거("변형 누락을 컴파일러가 잡는다")는 그대로 보존된다 — `match` 는 여전히 전수이고 여전히 exhaustive 검사를 받는다. 8-arm match 를 메서드마다 7번 반복하지 않을 뿐이다 |
| C-S3 | §4 의존 그래프 — `rbms-config → rbms-model, rbms-chart, rbms-judge, serde, ron` | `rbms-config → rbms-store` 추가 | `write_atomic` 은 설정·점수·리플레이가 모두 쓰는 단일 원자적 쓰기 헬퍼다. 스펙은 이를 `rbms-config` 안에 두라고 적었으나 그러면 `rbms-store` 가 같은 것을 두 번 갖거나 역방향으로 의존해야 한다. 순환은 없고(`rbms-store` 는 serde/ron 만 의존) 중복 구현을 피한다. **스펙 §4·§4.1 을 이 방향으로 정정했다** |
| C-S4 | §1.1 표 — `App`/`AppShared` 해체 | 두 구조체는 크레이트 루트(`lib.rs`)에 유지, 대신 `assets.rs` 와 `stage/select/{preview,scene}.rs` 분리 | 모든 화면·`app_*` 모듈이 크레이트 루트의 **자손**이라 `AppShared` 의 비공개 필드를 볼 수 있다. 두 구조체를 형제 모듈로 옮기면 필드 69개를 전부 `pub(crate)` 로 열어야 하며, 얻는 것은 짧은 파일 하나뿐이다 |

**와이어 변경 1건(리뷰 반영)**: IR 제출의 `judge_algorithm` 이 하드코딩 `"Combo"` → 실제 정책 `session.judge().algorithm().name()`(기본값에서 `"Duration"`) 으로 바뀌었다. 서버는 이 필드를 검증 없는 `varchar(16)` 로 그대로 저장하므로(`web/src/server/dto/score.ts`, `schema.ts`) 계약 영향은 없고, 바뀐 것은 "메타데이터가 클라이언트의 실제 후보 선택 정책과 일치한다" 는 점뿐이다. C-D1 대로 기본값 자체(`Duration`)는 그대로다 — 즉 이제 제출값이 C-D1 을 정직하게 보고한다. `docs/reference/ir-api.md` 의 예시도 함께 정정했다.

## Phase F — 화면·설정 확장 (2026-09-10, 통합 단계 등록)

> 근거: `docs/plan/2026-09-09-phase-f-spec.md`. 아래는 F0~F4 갈래가 합류하면서 오케스트레이터가 등록한 차이다. 어느 것도 판정 결과를 바꾸지 않는다.
>
> **2026-09-10 적대 리뷰 반영**: 밴드 타깃 EX 의 2단 나눗셈 off-by-one(critical), FAST/SLOW 가 리플레이·오토플레이에서 0 으로 나오던 회귀, 옵션 오버레이가 마우스·자동 전이로 브라우저 밖까지 살아남던 문제, SORT 의 진실 출처 이중화, HI-SPEED 상한 규칙 불일치, LANE COVER 100% 에서 hispeed 가 1.0 으로 스냅하던 문제, LEVEL 정렬의 동률 폴백을 수정했다. `ScoreTarget` 은 레퍼런스 고정 레이트 11종 전부(기본 `RATE_AAA`)로 넓혔고, 폰트 다이얼로그·난이도표 파일 파싱·`#PREVIEW` 클립 디코드를 워커로 옮겼다.

| # | 항목 | rbms | 레퍼런스 구현 | 상태 |
|---|---|---|---|---|
| F-D1 | **`dj_rank_label` 의 등급 축이 "밴드 3등분"** | `crates/rbms-render/src/result/grade.rs` 의 `BAND_GRADE_LABELS` 가 8개 밴드를 각각 low/mid/high 3등분해 `-`/무표시/`+` 를 붙인다. 결과적으로 AAA 밴드는 step 24=`AAA-`, 25=`AAA`, 26=`MAX-` 이다 | `TargetProperty.java:118-137` 의 *타깃 이름* 축은 `RATE_A-`=17/27 … `RATE_AAA-`=23/27, `RATE_AAA`=24/27, `RATE_AAA+`=25/27, `RATE_MAX-`=26/27 — 같은 이름이 한 스텝 아래에 있다 | **의도적 차이.** 큰 글자·밴드 색·랭크바 세그먼트가 서로 어긋나지 않는 쪽(밴드 우선)을 택했고, `dj_rank`(밴드 판정) 자체는 손대지 않았다. 레퍼런스의 고정 레이트 타깃 11종은 `apps/rbms-player/src/target.rs` 의 `RATE_TARGETS` 가 원문 비율 그대로 들고 있으므로, "타깃 이름" 축이 필요한 곳에서는 그쪽이 정본이다 |
| F-D2 | **27분위 눈금이 별도 함수** | 눈금은 `draw_rank_bar` 가 아니라 신설 `draw_rank_bar_stepped`(`grade.rs`)에 있고, 결과 화면만 그것을 부른다 | 해당 없음(rbms 내부 배치 문제) | 같은 바를 곡선택의 기록 패널(`crates/rbms-render/src/select.rs`)도 그리고 그 골든이 별도 파일이라, 한 화면의 더 고운 눈금 때문에 다른 화면의 픽셀을 옮기지 않았다 |
| F-D3 | **곡목록 정렬 방향** | `apps/rbms-player/src/stage/select/list.rs` 의 `sort_cmp` 가 CLEAR/SCORE/MISS COUNT/LAST UPDATE 를 **오름차순**(못한 것 먼저)으로, 기록 없는 곡을 **마지막**으로 둔다 | `BarSorter.java:139,159-162,219` 가 정확히 그렇게 한다(`o1 - o2`, 무기록이면 `return 1`) | **패리티 달성.** 통합 단계에서 레퍼런스를 직접 열어 재확인했다. 아직 구현되지 않은 `RivalClear`/`RivalScore` 의 문서 주석도 같은 방향으로 정정해 두었다(`crates/rbms-config/src/sort.rs`) |
| F-D4 | **BPM 정렬 축이 시작 BPM** | `sort_cmp` 의 `SortMode::Bpm` 이 `SongEntry.init_bpm`(헤더 `#BPM`)을 비교한다 | `BarSorter.java:78` 은 `getMaxbpm()` — 차트 전체의 최고 BPM 을 쓴다 | **미해소(데이터 부족).** 헤더 전용 스캔이 만드는 `SongEntry`(`crates/rbms-library/src/lib.rs`)에 최고 BPM 필드가 없다. 곡 DB 에 max BPM 이 생기면 비교자 한 줄로 해소된다 |

**Phase F 미구현으로 남긴 것** (레퍼런스 차이가 아니라 rbms 진행 상태):

- `SortMode::Length` / `SortMode::Duration` — LENGTH 는 곡 길이(`BarSorter.java:96`), DURATION 은 평균 판정 오차(`:201` `getAvgjudge`). 헤더 전용 스캔에도 `ScoreRecord` 에도 없어 둘 다 제목 폴백으로 둔다. 곡 DB 또는 `ScoreRecord.avg_judge` 가 생기면 해소된다.
- `SortMode::RivalClear` / `SortMode::RivalScore` — 라이벌 기록이 붙는 페이즈까지 제목 폴백.
- `ScoreTarget::Rival` 의 INDEX / RANK(n위) / NEXT(자기 위 n번째) 3축(`RivalTargetProperty`) — 현재는 랭킹 캐시에서 점수가 가장 높은 라이벌 하나. 설정 행이 생기면 `target::TargetContext.rival` 을 넓히면 된다.
- **LANE OPTION 의 FLIP / BATTLE / BATTLE AUTO-SC** — 설정 쪽(`LaneOption`, `recorded_value()`, `is_assist()`, 라벨, descriptor 행)은 완비됐지만 런에 반영되지 않는다. 반영에는 ① 모델의 레인 재배치와 SP→DP 모드 승격(5K→10K, 7K→14K), `active_keys`/`active_reverse_keys` 재구성, `auto_lanes` 세팅이 `app_play.rs::load` 에, ② BATTLE 계열 어시스트 게이트가 `lib.rs::ir_submission_block_reason` 에 필요하다. **그때까지 이 행은 오버레이뿐 아니라 설정 화면에서도 감춘다**(`settings.rs::unbuilt_row`, `visible: never`) — 고를 수는 있는데 런에 아무 영향이 없는 행은 "선택된 옵션"으로 읽히기 때문이다. descriptor 자체는 남아 있어 저장값 왕복은 그대로고, 구현이 끝나면 `row(...)` 로 되돌리는 한 줄이면 된다.
- **차트 파싱이 아직 프레임 루프에서 돈다** — `stage/loading.rs` 의 `LoadingTask::Song` 이 `update` 안에서 `AppShared::load()` 를 동기 호출한다(파일 읽기 + `rbms_parser::parse_with` + `to_model` + md5/sha256). 스펙 §F4-2 의 `Loading::Parse` 단계는 아직 없고, 이 구간에는 진행바도 취소 플래그도 없다. 워커로 옮기려면 `load()` 를 "파싱(워커) → 적용(메인)" 두 단계로 쪼개야 하는데, 그 함수가 `AppShared` 의 스킨·오디오·키설정까지 함께 세팅하고 있어 `app_play.rs` 전반의 구조 변경을 동반한다. 키사운드·BGA·표 fetch·폴더 스캔·rfd 다이얼로그 3종·`#PREVIEW` 클립 디코드는 전부 워커로 옮겨져 있으므로, 남은 것은 이 한 구간뿐이다.
- **차트 간 그린넘버 유지(진짜 floating hi-speed)** — 레퍼런스는 `PlayConfig.java:22-26` 에서 `duration`(그린넘버 ms, 기본 500, 1~10000)을 `hispeed` 와 별도로 저장하고 차트 진입마다 `resetHispeed(basebpm)` 로 speed 를 역산한다. `rbms-config` 에 해당 필드가 없어 현재는 "런 내부 고정"까지다. 필드가 생기면 `hispeed_for_green` 을 그대로 재사용한다.
- **PREVIEW FADE 가 단일 값** — config 는 `preview_fade_ms` 하나라 페이드 인/아웃이 같은 길이다(스펙 본문의 진입 200ms / 이탈 150ms 와 다름).
- **곡목록 필터의 레벨·모드·클리어 축이 브라우저 로컬** — 곡선택을 떠나면 초기화된다. 영속·즉시 적용에는 `LibraryOptions` 또는 `AppShared` 에 필드가 필요하다. 같은 이유로 필터 패널은 키보드 전용이다(`Hot` 에 패널 행 variant 가 없어 클릭을 보고할 수 없다).
- **Shift+F3 역정렬은 ShiftRight 로만** — 옵션 오버레이의 홀드 키(`app_options.rs::HOLD_KEY`)가 `ShiftLeft` press/release 를 브라우저보다 먼저 소비한다. 단 브라우저가 텍스트를 받는 상태(검색·기록 모달·필터 패널)에서는 `StageHandler::holds_keys` 가 오버레이를 물리므로 `ShiftLeft` 가 그대로 검색창에 간다.
