# 레퍼런스 구현과의 의도적/보류 차이 (M0/M1 적대적 리뷰 결과)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c. 이번 갱신에서 코드와 대조한 범위: 새 절 "옵션 패널·램프 등급·스킨 색·확장 id (2026-10-22)". 그 전 갱신(2026-10-19, e2c829f 이후 작업 트리)이 대조한 범위: 새 절 "기록 목표의 진행 중 값"과 "Esc 포기 판 저장" 절의 "저장하지 않는 판"·"코스 도중 Esc" 행. 그 전 갱신(2026-10-15, ea698a9)이 대조한 범위: 2026-10-14 이후 절(Esc 포기 판 저장 ~ 연습과 #TOTAL)과 아래에 표시한 죽은 서술. 그 전 절(2026-06 ~ 09)의 함수 이름은 대조하지 않았다.

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
| 13 | 고아 `#LNOBJ` 꼬리를 Normal로 방출 | 원본은 이제 확인됐다(아래 "롱노트 정리" 절): 버린다. 아직 맞추지 않았다. 고아 `#LNOBJ` 가 있는 차트는 원본보다 노트가 하나씩 많다 |

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
| D-D6 | **assist 1(AUTO SCRATCH) 런의 리플레이 저장** | (2026-10-14 대체: `saves_replay` 는 없어졌고, 사람이 친 판은 어시스트와 상관없이 모두 리플레이를 저장한다 → [모든 판 저장·전송 ADR](2026-10-14-adr-save-and-send-every-played-run.md). 아래는 당시 서술) 저장한다(`saves_replay(assist) = assist < 2`) | `MusicResult.java:317-321` 이 `resource.isUpdateScore()` 로 게이트하고, assist 를 올리는 모든 분기가 `score=false` 로 만들므로(`BMSPlayer.java:203-252`) assist 1 에서도 저장하지 않는다 | 결정 12 는 assist 2 에 대해서만 리플레이 차단을 문구화했다. assist 1 의 리플레이는 rbms 가 유지하는 편의 기능 |
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
- 미종결 LN(LN 채널 홀수 개) → dangling LongStart 잔존(허용) — 2026-10-15 에 뒤집었다: 원본은 디코드 끝에서 지운다. "롱노트 정리" 절

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

## Phase E — 스킨 완전 커스터마이징 (2026-09-10, 착수 전 선반영)

> 이 절과 바로 아래 "적대 리뷰 반영" 절은 JSON 스킨 시스템(2026-09 ~ 10-08)의 기록이다. 스킨 시스템은 2026-10-09 Lua 스킨 호환으로 바뀌었다([Lua 스킨 호환 결정](2026-10-09-lua-skin-compat-decisions.md), `docs/skin.md`). 여기 적힌 함수·설정 이름 가운데 E-D13 의 `AppShared::skin_now_ms()`(지금은 µs 단위 `AppShared::skin_now_us()`, 장면 시작부터 잰다)와 E-D17 의 `max_call_micros`·`max_frame_micros`(지금 예산은 `crates/rbms-skin/src/lua/budget.rs` 의 프레임 50 ms·호출당 100만 명령 등, 정본 `docs/skin.md` 의 예산 표)는 코드에 없다. 이력으로만 읽는다.

> 근거: `docs/plan/2026-09-09-phase-e-spec.md` §2.1.1 / §2.1.2 / §2.2 / §5.4 / §5.6. 아래 5건은 사양이 **레퍼런스 구현을 직접 읽고 확정한 차이**이며, 코드가 아직 없는 시점(웨이브 0 스캐폴드)에 미리 기록한다. 실현 브랜치를 함께 적어 두었으니, 해당 단계가 끝나면 "상태" 칸만 갱신하면 된다.
> 다섯 건 중 화면 결과가 달라지는 것은 E-D4 하나뿐이고(축소·확대 시 보간 품질), 나머지는 동형 매핑·관대화·내부 구조·보안 추가다.

| # | 항목 | rbms | 레퍼런스 구현 | 사유/영향 | 실현 |
|---|---|---|---|---|---|
| E-D1 | **배치 병합 범위** — 드로우 콜을 텍스처별로 모으지 않는다 | 제출 순서를 절대 재배열하지 않고, **직전 드로우와 `(TextureId, BlendMode, TextureFilter, 회전 유무, 현재 클립)` 이 전부 같은 연속 구간만** 한 드로우 콜로 병합한다. `push_clip`/`pop_clip`/`clear` 는 무조건 flush 경계 | 오브젝트마다 즉시 그린다(스프라이트 배처가 상태 변경 시 flush) | **화면 동형, 내부 구조 차이.** 스킨은 그리는 순서가 곧 z-order 이므로 텍스처별 전역 수집은 반투명 오브젝트가 섞인 순간 z-order 를 깨뜨린다. 상위 계획 §2 E1 의 "per-texture 배칭" 문구는 사양 §2.2 가 대체했다(결정 문서에 기록) | E-prim |
| E-D2 | **blend 3 을 가산으로 매핑** | `BlendMode::Add` (= blend 2 와 동일) | `Skin.java:636-645` 가 `glBlendEquation(FUNC_SUBTRACT)` 를 걸었다가 `setBlendFunction(SRC_ALPHA, ONE)` 직후 **즉시 `FUNC_ADD` 로 되돌린다** → 실제 화면은 가산 | **의도적 동형 매핑.** 감산식을 구현하면 오히려 원본과 화면이 달라진다. 그래서 rbms `BlendMode` 에 `Subtract` 변형 자체를 두지 않는다. 실존 blend 값은 2/4/9 뿐이고 그 외 정수는 전부 `Alpha` | E-prim |
| E-D3 | **center 범위 밖 값을 클램프** | `Destination.center` 가 0~9 밖이면 `0`(중앙)으로 클램프 | `SkinObject.java:80-81` 의 `CENTERX`/`CENTERY` 를 배열 인덱스로 바로 쓰므로 범위 밖 값은 예외 | **rbms 가 더 관대한 쪽.** 저작 실수가 있는 스킨이 통째로 죽지 않게 한다. 0~9 자체의 환산(정규화 비율 → y-down 픽셀)은 사양 §2.1.2 표대로 패리티 | E-prim |
| E-D4 | **TYPE_BILINEAR 셰이더를 하드웨어 Linear 로 근사** | `dstfilter != 0` 이고 dst 크기 ≠ 소스 리전 크기면 `TextureFilter::Linear`, 그 외(필터 0 또는 1:1)는 `Nearest` | `SkinObject.java:634-636` 이 그 조건에서 `TYPE_BILINEAR` 를 세우고, `Skin.java:82-86` 의 `setFilter` 는 `TYPE_LINEAR`/`TYPE_FFMPEG`/`TYPE_DISTANCE_FIELD` 에만 하드웨어 Linear 를 건다 — `TYPE_BILINEAR` 는 **전용 셰이더** | **유일한 시각적 차이.** 셰이더 재현은 하지 않는다(스킨 커스터마이징의 본질과 무관한 비용). 확대·축소 시 보간 품질만 미세하게 다르다. `TYPE_FFMPEG`(동영상)·`TYPE_DISTANCE_FIELD`(SDF 폰트)는 Phase E 범위 밖이라 만나면 `Linear` + 1회 warn | E-screen |
| E-D5 | **해석된 경로가 스킨 루트를 벗어나면 거부** | 와일드카드·filemap·customfile 로 해석한 경로를 `canonicalize` 해 스킨 루트 밖이면 `SkinError::PathEscape`. `..` 세그먼트와 심볼릭 링크 탈출 모두 막는다. Lua 는 파일 API 자체가 없다 | 해당 검사 없음(`SkinLoader.getPath` 는 해석 결과를 그대로 연다) | **레퍼런스에 없는 보안 추가.** 스킨은 사용자가 인터넷에서 받아 넣는 제3자 데이터이므로, 스킨 파일이 홈 디렉터리의 임의 파일을 텍스처로 읽어 화면에 띄우는 경로를 원천 차단한다. 정상 스킨에는 영향이 없다 | E-load |

### Phase E — 구현 단계에서 추가된 이탈 (2026-09-10, 웨이브 3 통합 시점)

> 위 표는 착수 전에 사양이 확정한 5건이다. 아래는 E-prim/E-timer/E-prop/E-load/E-screen/E-ui 가 실제로 코드를 쓰면서 생긴 것으로, 렌더러·로더·플레이어 배선이 합류하는 시점에 등록한다.

| # | 항목 | rbms | 레퍼런스 구현 | 사유/영향 |
|---|---|---|---|---|
| E-D6 | **`image`/`imageset` 의 `w`/`h` 가 0 이면 텍스처 전체** | `w` 또는 `h` 가 `0` 이면 소스 텍스처 전체를 리전으로 읽는다 | `-1` 만 "전체" 로 해석하고 `0` 은 그대로 0 크기 | **rbms 가 더 관대한 쪽.** 0 크기 리전은 아무것도 그리지 않아 무의미하므로, 그것을 "지정 안 함" 으로 읽는다. 정상 스킨에는 영향이 없다 |
| E-D7 | **image-index 를 정수 프로퍼티 공간에서 읽는다** | `imageset`/`image` 의 `ref`(어느 이미지를 고를지) id 를 `SkinStateSource::integer` 로 읽는다 | image-index 전용 프로퍼티 공간이 따로 있다 | **id 공간 통합.** rbms 는 정수 프로퍼티 하나로 답하며, 두 공간이 겹치는 id 가 없어 실제 스킨에서 충돌하지 않는다. 겹치는 문서를 만나면 이 결정을 다시 본다 |
| E-D8 | **미지원 stretch 모드는 경고 후 STRETCH** | trimmed 계열·`FitWidth`/`FitHeight`/`NoExpanding` 은 1회 경고를 남기고 `STRETCH` 로 그린다 | 각 모드를 개별 구현 | **시각적 차이 있음(해당 모드를 쓴 스킨만).** 오브젝트가 사라지는 것보다 늘어나 그려지는 쪽이 저작자 의도에 가깝다고 보았다 |
| E-D9 | **Note/Judge/Gauge 계열 오브젝트를 아직 그리지 않는다** | `Note`/`Judge`/`Gauge`/`gaugegraph`/`judgegraph`/`bpmgraph`/`hiterrorvisualizer`/`timingvisualizer`/`pmchara`/`songlist` 오브젝트는 경고 1줄을 남기고 드롭한다 | 전부 그린다 | **미구현.** 이들은 프로퍼티 id 밖의 플레이 세션 데이터(레인별 노트 목록, 판정 팝업 카운트, 게이지 노드)를 원천으로 한다. 그때까지 플레이 화면은 스킨이 주변 크롬을, 내장 `render_playfield_view` 가 노트를 그리는 하이브리드가 아니라 **문서가 선택되면 문서만** 그린다(§E-D11) |
| E-D10 | **골든 PNG 를 커밋한다** | 사양 §9.1 의 A 안 확정. `crates/rbms-render/tests/golden/` 에 4장(합계 32 KB, 스킨 픽스처 20 KB) | 해당 없음(rbms 내부 결정) | 블록 시그니처만으로는 "한 자리 숫자가 틀린 프레임" 을 구분하지 못한다. 200 KB 예산 안이므로 실제 픽셀을 커밋한다. 결정 문서 E2 의 "3장 이하" 는 예산과 무관한 임의 수치였고 예산 기준으로 정정했다(2026-09-10) |
| E-D11 | **문서가 선택된 화면은 내장 레이아웃을 전혀 그리지 않는다** | 게이트(`render_{play,select,result,decide,keyconfig}_screen`)가 `true` 를 돌려주면 그 프레임은 문서만 그린다. 브라우저의 클릭 영역(`Hot`)도 이때는 등록되지 않으므로 스킨 브라우저는 키보드 전용이다 | 화면마다 스킨이 전권을 갖는 것은 같다 | **사양 §6-3 대로.** 폴백은 "문서 없음/로드 실패" 단위이지 오브젝트 단위가 아니다. 문서가 그리지 않는 것(E-D9)은 그냥 그려지지 않는다. 클릭 영역은 문서 오브젝트를 `Hot` 으로 대응시키는 표가 생기면 해소된다 |
| E-D12 | **문서를 그리기 직전 프레임을 검게 지운다** | `AppShared::draw_*_skin` 이 `Canvas::clear(Color::BLACK)` 후 게이트를 부른다 | 스킨이 배경 오브젝트를 갖는 전제 | 내장 화면은 각자 자기 프레임을 지우는데 게이트는 그 앞에서 가로채므로, 지우지 않으면 이전 프레임이 비어 있는 구석에 남는다 |
| E-D13 | **`skin.time()` 과 타이머의 시계가 앱 기동 이후 경과 ms** | `AppShared::skin_now_ms()` = `clock.elapsed()` 의 ms | 레퍼런스도 단조 증가 밀리초 시계 | 원천만 다르고 성질은 같다. 중요한 것은 프로퍼티·타이머·보간이 **같은 시계**를 본다는 것이고, 그 하나는 이 함수뿐이다 |
| E-D14 | **레인별 `BOMB`/`KEYON`/`HOLD` 타이머는 아직 켜지지 않는다** | `PlayTimers` 가 켜는 것은 PLAY/READY/FAILED/JUDGE_1P/COMBO_1P/FULLCOMBO_1P/GAUGE_INCLEASE_1P/GAUGE_MAX_1P 뿐 | 레인 단위 타이머도 켠다 | **미구현.** E-D9 와 같은 원천(레인별 입력·노트 상태)이 필요하다 |


### Phase E — 적대 리뷰 반영에서 추가된 이탈 (2026-09-10)

> 스킨 파이프라인 적대 리뷰(critical 4 / major 8 / minor 9)를 반영하며 생긴 것. 대부분은 레퍼런스와의 **일치 복구**(알파 0 조기 반환, 커스터마이즈 op 로드시 판정, ArraySerializer 의미, 11/22칸 소수점 매핑, padding 필드, isSignvisible 26칸 한정, dstfilter/NO_RESIZE 화면 픽셀 기준)이라 여기에 적지 않는다. 아래는 **레퍼런스에 없는 rbms 고유 결정**만이다.

| # | 항목 | rbms | 레퍼런스 구현 | 사유/영향 |
|---|---|---|---|---|
| E-D15 | **Lua 패턴 함수를 샌드박스에서 제거** | `string.find`/`match`/`gmatch`/`gsub` 와 `string.dump` 를 `nil` 로 지운다 | 표준 `string` 라이브러리 전체 노출 | **보안 추가.** 패턴 매칭은 C 안에서 도는데 인스트럭션 훅은 VM 인스트럭션에서만 발화하고 할당도 없어, `(.-)` 캡처 여러 개짜리 패턴 하나가 두 예산 어디에도 안 걸린 채 초 단위로 돈다(실측 120자 subject 에서 2초 이상). 프레임 루프에서 도는 코드라 제거가 유일한 확실한 차단이다. `format`/`sub`/`rep`/`upper`/`lower` 등 문서가 실제로 쓰는 것은 그대로 |
| E-D16 | **Lua 청크를 텍스트 모드로 고정** | `Chunk::set_mode(Text)` — 바이너리 청크 거부 | 모드 지정 없음(libgdx 경유라 해당 없음) | **보안 추가.** Lua 5.4 의 언덤프는 신뢰할 수 없는 바이트코드를 검증하지 않는다. 현재는 문서가 UTF-8 문자열로만 들어와 실현 경로가 없지만 `LuaSandbox::compile` 이 `pub` 이라 계약으로 고정한다 |
| E-D17 | **Lua 에 벽시계 예산 2종 추가** | 표현식 1개당 `max_call_micros`(기본 20 ms), 프레임당 `max_frame_micros`(기본 4 ms). 프레임 예산은 draw 조건뿐 아니라 `value`/`floatvalue`/`text` 표현식까지 **같은 카운터**로 소비한다 | 예산 없음 | **성능·보안 추가.** 인스트럭션 수는 시간이 아니고, 값 표현식은 오브젝트 수만큼 평가된다. 초과분은 오브젝트가 기본값(숨김 또는 미갱신)으로 떨어질 뿐 스킨이 죽지 않는다 |
| E-D18 | **include 총 확장량 상한** | 깊이 8(기존)에 더해 로드 1회당 확장 1024개(`MAX_INCLUDE_EXPANSIONS`). 같은 대상은 1회만 읽고 파싱해 캐시한다 | 상한 없음 | **보안 추가.** 깊이만 막으면 폭이 안 막힌다 — fan-out 6 × 깊이 8 짜리 수 KB 문서가 실측 18초를 먹었고, 로더는 스킨을 고른 프레임에서 동기로 돈다. 정상 스킨은 캐시 덕분에 오히려 빨라진다 |
| E-D19 | **숫자 자릿수 상한 16** | `value` 의 `digit` 을 1..=16 으로 클램프한다 | 상한 없음(`digit` 그대로 배열 크기) | **rbms 가 더 방어적인 쪽.** i32 는 10자리 + 부호이고 소수는 8자리 + 점 + 부호라 16이면 남는다. 프레임마다 자릿수 배열을 힙에 잡지 않기 위한 고정 크기 저장의 전제이기도 하다 |
| E-D20 | **스킨 파일을 워커 풀에서 읽는다** | 문서가 선언한 이미지·폰트를 `spawn_skin_asset_decode` 로 디코드하고, 전부 도착한 프레임에서 화면을 컴파일한다. 그때까지 그 화면은 내장 레이아웃을 그린다 | 로드 시점에 동기 디코드 | **성능.** 실제 배포 스킨은 소스가 수십~수백 장이고 2048x2048 도 흔해, 프레임 루프에서 디코드하면 곡이 시작되는 첫 프레임이 초 단위로 멈춘다. 화면 전환 직후 몇 프레임 동안만 내장 레이아웃이 보인다 |
| E-D21 | **글리프 아틀라스 페이지에 레이아웃 세대별 키** | 페이지가 커지거나 비워지면 새 등록 키를 쓰고, 직전 페이지는 `GlyphAtlasBinding::end_frame` 까지 살려 둔다 | 해당 없음(rbms 내부 구조) | **지연 백엔드 정합성.** 프레임을 모았다가 끝에 제출하는 백엔드(GPU)에서는, 한 프레임 안에서 페이지가 커지면 앞서 기록한 쿼드의 UV 가 새 페이지를 가리켜 글자가 깨진다. 글리프 추가만으로는 좌표가 안 움직이므로 그때는 같은 키에 재업로드한다 |

## Esc 포기 판 저장 (2026-10-14, 0.1.4)

> 사용자 결정(2026-10-14) "Esc 중도 포기 판도 저장·전송: 램프 FAILED, 남은 노트는 놓친 것(見逃し POOR)". 동작 전체는 `docs/play-options.md` 10.2절의 "도중 포기(Esc)" 행.

| 항목 | rbms | 레퍼런스 구현 | 사유/영향 |
|---|---|---|---|
| 결과 화면 | 거치지 않는다. 기록·리플레이를 남기고 로그인했으면 그 판 하나를 기록 동기화로 보낸 뒤 곧장 선곡으로 간다(`PlayState::leave_run` → `app_result::record_left_run`). 판이 이미 실패·끝으로 닫히는 중이면(길게 누르기 도중 게이지가 바닥난 경우 등) 이 길을 타지 않고 결과 화면으로 가서 닫힌 대로 남는다. 남은 노트가 없는 판(판정할 노트가 0 개인 차트 포함)도 결과 화면으로 간다(원본과 같다) | `BMSPlayer.stopPlay` 가 `STATE_FAILED` 로 바꿔 결과 화면을 거쳐 저장한다(`BMSPlayer.java:938-970`) | Esc 는 이 플레이어에서 바로 나가는 키이고 ESCAPE 설정(즉시·길게·두 번)이 실수를 막는다. 판의 결과는 기록 상세(R)와 상태 줄로 볼 수 있다 |
| 남은 노트 | 아직 판정하지 않은 노트(롱노트의 남은 끝 포함)를 늦은 POOR(見逃し)로 판정 수에 더하고 BP 에도 더한다(`PlaySummary::left_unfinished`). PG~POOR 합이 노트 수와 같다 | 판정 수에는 넣지 않고 `minbp` 에만 더한다(`BMSPlayer.java:909`) | 사용자 결정. 점수 서버의 판정 수 검사(노트를 소비한 판정 ≤ 노트 수)를 그대로 통과한다 |
| 저장하지 않는 판 | 오토플레이, 리플레이 보기, 연습, 그리고 노트를 하나도 치지 않은 판이다: PG·GR·GD·BD 가 모두 0 이고 쥔 롱노트가 없는 판이며, 지나간 노트의 POOR 나 빈 POOR 만 있어도 같다(`PlayState::has_judged_nothing`, 2026-10-19 사용자 지시, [ADR](2026-10-15-adr-escape-keeps-every-run.md) 결정 7). 2026-10-15 에는 판정한 노트가 없는 판도 저장했다(그날의 사용자 결정은 메인이 낸 "판정 없는 판 제외"의 거부였고, 이번 제외는 사용자가 직접 지시한 것이다). ~~코스 도중의 Esc 무기록~~ 은 2026-10-15 에 대체됐다 | 코스가 아닌 판에서 PG·GR·GD·BD(early·late 합) 가 모두 0 이면 점수를 만들지 않는다(`BMSPlayer.java:857-860`) | 기준 네 판정은 원본과 같다. 다른 점 둘: 쥔 롱노트가 있으면 친 판으로 남긴다(원본은 그 판에 판정이 없어 남기지 않는다), 코스의 스테이지는 친 노트가 없으면 곡 기록으로 더하지 않는다(원본의 이 검사는 코스를 거르지 않는다). 사용자 결정 "실패기록도 다 서버로 보내야지"(10-15)의 범위 안에서, 친 노트가 없는 판만 사용자가 뺐다 |
| 코스 도중 Esc | 남은 노트가 없으면 정상 판정으로 코스가 이어진다. 있으면 그 스테이지를 FAILED 곡 기록으로, 코스를 끝낸 스테이지와 함께 FAILED 코스 기록으로 저장·전송하고 선곡으로 나간다. 그 스테이지에서 노트를 하나도 치지 않았으면 그 곡은 더하지 않고 코스가 그 자리에서 끝난다: 친 스테이지가 있으면 그 스테이지들로 FAILED 코스를 남겨 보내고, 첫 스테이지였으면 코스 기록도 없다(`app_result::leave_course` → `abandon_course`) | `stopPlay` 가 FAILED 로 결과를 거쳐 코스 결과로 간다. 남은 곡의 노트를 BP 에 더한다(`MusicResult.java:183-196`) | 결과 화면은 거치지 않는다. 아직 치지 않은 스테이지의 노트는 코스 BP 에 더하지 않는다(테스트 부채) |
| 리플레이 | 나간 시점까지의 입력으로 저장한다. 재생하면 곡 끝까지 가며 남은 노트는 見逃し POOR 가 되지만 게이지는 기록과 다르게 끝날 수 있다 | 같다(나간 시점까지) | 알려진 한계 |
| 플레이 중 앱 종료(창 닫기·Cmd+Q·메뉴 Quit·Dock, S19 2차 REC2-2) | Esc 와 같은 규칙으로 판과 코스를 남긴다(`App::keep_what_quitting_cuts_short`). 끄는 중에는 보내지 않고 다음 실행의 기록 동기화가 보낸다 | `MainController.dispose` 는 설정 저장과 자원 해제만 하고 진행 중인 판을 저장하지 않는다(`MainController.java:590-612`) | 사용자 정책 "사람이 친 판은 실패·포기·종료 포함 전부 저장·전송" |
| 플레이 중 앱 종료 때의 설정 (S19 3차 R3-REC-1) | 판을 남기기 전에 Esc 처럼 설정을 저장한다. 그 판에서 키로 바꾼 하이스피드·커버·리프트·히든과 AUTO ADJUST 가 옮긴 판정 오프셋이 남는다. 플레이 화면이 아닌 곳에서 끌 때는 쓰지 않는다(읽지 못한 설정 파일을 기본값으로 덮지 않기 위해) | `dispose` 의 `saveConfig` 가 판정 오프셋(`JudgeManager.java:701` 이 플레이어 설정에 바로 쓴다)은 저장한다. 하이스피드·커버·리프트·히든은 `LaneRenderer` 가 `PlayConfig` 사본(`LaneRenderer.java:90`)을 바꾸고 판이 끝나는 길의 `BMSPlayer.saveConfig` 만 되돌려 쓰므로 곡 도중 창을 닫으면 잃는다 | 오프셋은 원본과 같다. 하이스피드·셰이드는 원본과 달리 남긴다: 같은 판을 Esc 로 나가면 남는 값이 종료로는 사라지는 rbms 안의 불일치를 없앤 것이다 |
| 코스 스테이지 사이에서 나감(LOADING 의 Esc, 결정 화면에서 나감, 스테이지 차트를 못 읽음·라이브러리에 없음, 앞 코스가 남은 채 새 코스 시작, S19 2차 REC2-1·리뷰 5) | 코스를 끝내고, 친 스테이지가 있으면 FAILED 코스로 남겨 보낸다(`app_result::abandon_course`). 서 있던 스테이지는 아무것도 더하지 않는다 | 스테이지 로딩은 `BMSPlayer` 안이라 Esc 는 `stopPlay` 로 코스 결과를 거친다 | 결정 9 의 코스 도중 포기와 같은 취급. 전에는 코스가 끝나지 않아 다음 단일 곡이 그 코스에 접혔다 |

## 플레이 수리 — 오토플레이·일반 롱노트 집계·포커스 (2026-10-15, S19)

> 감사 PLAY-1·2·8·13. 판정·점수·램프·게이지의 의미는 바뀌지 않는다. 원본과 달랐던 동작을 원본 쪽으로 맞춘 것이다.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 오토플레이와 지뢰 | 오토플레이가 사람의 키 누름과 같은 `press`/`release` 로 판정해, 일반 노트를 친 레인이 끝까지 눌린 채 남았고 그 레인의 지뢰가 게이지를 깎았다(지뢰 있는 실제 차트 29개 중 26개가 HARD 오토플레이에서 사망, NORMAL 에서도 7개가 FAILED) | 오토플레이는 `auto_presstime` 만 바꾸고 노트를 그 자리에서 PGREAT 로 판정한다. 지뢰는 실제 키 상태가 눌려 있을 때만 깎는다(`JudgeManager.java:244, 251-283`) | `JudgeEngine::auto_press`/`auto_release` 가 키 상태를 건드리지 않고 판정한다 |
| 오토플레이 스크래치 CN/HCN | 끝의 릴리스가 BSS 규칙(`release_dir` 의 방향·창 검사)에 걸려 무시돼, 끝이 POOR 이고 홀드가 남아 다음 머리까지 POOR(CN·HCN 모드 각 1,629 차트가 오토플레이 만점이 아니었다) | CN/HCN 끝을 그 시각에 PGREAT 로 직접 판정한다(`:272-283`) | `auto_release` 가 방향·창 검사 없이 끝을 PGREAT 로 확정한다. 사람 입력의 BSS 규칙은 그대로다 |
| 오토플레이 HCN 게이지 | (사람 키 경로라 누른 것으로 쳐서 우연히 맞았다) | 틱의 증가 조건에 `autoplay` 가 있다(`:289-293`) | `JudgeEngine::set_autoplay(true)`. 키 상태를 안 올리는 대신 이 플래그로 증가가 유지된다 |
| 일반 롱노트의 집계 | 머리를 누르는 입력이 결과를 돌려줘 결과 화면의 판정 분포·FAST/SLOW·타이밍 히스토그램과 AUTO ADJUST 가 롱노트를 머리와 끝 두 번 셌다. 끝까지 쥐고 있다가 시계가 확정한 롱노트는 한 번도 못 셌다 | 끝에서 한 번만 `updateMicro`(판정음·자동 조정 포함)(`:426-438, 697-703`) | `JudgeResult::counted` 가 거짓인 결과는 세지 않고, 시계가 확정한 롱노트는 그때 센다. 판정음도 센 판정에서만 난다(CN 의 끝 판정음이 새로 난다) |
| 포커스를 잃은 창의 키 | 창이 포커스를 잃으면 창의 눌림 집합만 비웠다. winit 0.30 은 포커스를 잃을 때 합성 key-up 을 X11·Windows 에서만 보낸다(`WindowEvent::KeyboardInput::is_synthetic` 문서, macOS 쪽 소스에 합성 키 이벤트가 없다). macOS 에서는 눌러 둔 롱노트가 끝까지 쥔 것으로 PGREAT 가 됐다(실제 창에서 Cmd+Tab 으로 확인한 것은 아니다) | 포커스 상실을 따로 다루지 않고 윈도우 라이브러리(libGDX)가 주는 키 이벤트에 맡긴다(원본 소스에 포커스 처리가 없다). 그 라이브러리가 해제를 주는지는 확인하지 못했다 | `Focused(false)` 가 눌린 키마다 key-up 을 화면에 넘긴다(`Stage::release_keys_lost_with_focus`). 리플레이에도 기록된다 |

## 롱노트 정리 — 디코더가 원본처럼 겹침·짝 없음을 정리한다 (2026-10-15, S19)

> 감사 PLAY-11·PLAY-12와 사용자 결정 7·8. 원본 디코더 `jbms-parser`(`beatoraja/lib/jbms-parser.jar`)는 소스가 없어 클래스 파일을 바이트코드까지 읽어 확정했다(JRE 없이 읽는 `docs/utils/class-disassembler.py <클래스 파일> [메서드]`). 근거 메서드: `Section.lambda$makeTimeLines$9`(LN 채널), `$7`(일반 노트·LNOBJ), `$10`(지뢰), `BMSDecoder.decode` 끝의 정리 루프, `TimeLine.getTotalNotes(int)`.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 같은 레인 LN 몸통 안의 일반 노트 | 그대로 남아 칠 수 없는 노트가 됐다(홀드가 끝날 때까지 스윕이 막혀 늦은 POOR). 오토플레이의 POOR 459(ts5)·83(cqc)도 이것이었고, 오토플레이는 PLAY-1 수리로 노트를 그 자리에서 판정하게 되어 사라졌지만 사람이 치는 판에는 남아 있었다 | LN 꼬리를 읽을 때 머리와 꼬리 사이의 모든 노트를 레인에서 지우고, 지운 것이 일반 노트면 배경음으로 옮긴다(`$9`). 지뢰는 몸통 안이면 추가하지 않는다(`$10`) | `rbms-chart/long_note.rs` 가 같은 시점(꼬리)에 같은 일을 한다. 일반 노트는 배경음(`bgnotes`)으로, 지뢰는 버린다 |
| LN 머리와 같은 칸의 일반 노트 | 머리가 덮어써 키음이 사라졌다 | 샘플이 머리와 다르면 배경음으로 옮기고, 같으면 버린다(`$9`) | 같다 |
| 앞 LN 이 끝난 칸에서 시작하는 LN 객체 | 꼬리를 덮어써 앞 LN 이 짝을 잃었다 | 앞 LN 의 [머리, 꼬리] 안(양끝 포함)에 든 객체는 삼켜지고 그 짝도 삼킨다. 앞 LN 이 남는다(`$9`) | 같다 |
| 끝나지 않은 LN 머리 | 머리가 모델에 남아 곡 DB 노트 수·기본 TOTAL 은 세고 판정 엔진은 세지 않았다(코퍼스 18개 차트) | 디코드 끝에서 경고하고 레인에서 지운다(`BMSDecoder.decode`). 그 뒤의 노트는 그대로 친다 | 같다 |
| 노트 수의 정의 | `rbms-chart` 와 BMSON 변환기가 같은 식을 따로 가졌다 | `TimeLine.getTotalNotes(lntype)`: 일반 노트 1, LN 머리 1, 꼬리는 CN·HCN 일 때만(미정의 LN 은 LN MODE 가 LN 이 아닐 때) | `rbms_model::NoteKind::counts_as_playable_note` 한 곳. 판정 엔진의 `total_notes` 와 같은 값이 나온다(`crates/rbms-play/tests/note_count.rs`) |

**기록의 의미:** 이 두 정리가 걸리는 차트(감사 때 11,336 차트 라이브러리에서 몸통 안에 일반 노트가 있는 차트 약 15개, 노트 수가 어긋난 차트 18개)는 곡 DB `notes`·`long_notes`·밀도, 기본 TOTAL, 판정 엔진의 `total_notes`·`max_ex` 가 바뀐다. 곡 DB 는 `PARSER_VERSION` 3 이 다음 스캔을 전체 재스캔으로 만든다(같은 릴리스 안에서 4·5 로 더 올라 지금은 5, 아래 "연습과 #TOTAL" 절). 점수 기록의 규칙 세대(`SCORE_RULE_VERSION`)는 이 변경만으로 올리지 않았다(결정은 [몸통 안 일반 노트 ADR](2026-10-15-adr-long-note-body-notes.md)와 [짝 없는 머리 ADR](2026-10-15-adr-unpaired-long-note-heads.md)).

**아직 원본과 다른 점(이 차트들을 일부러 남겼다):**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 몸통 정리 시점 | 꼬리 앞의 모든 사건이 처리된 뒤라 꼬리 앞의 노트를 전부 정리한다 | 채널 줄을 파일 순서로 처리하므로 같은 마디에서 LN 줄 뒤에 적힌 일반 노트 줄은 몸통 안에 남는다 | 그런 차트는 원본에서도 겹친 노트로 쳐지지 않는다. 채널 줄 순서를 따라가려면 `to_model` 전체를 마디·줄 순서로 바꿔야 한다 |
| 일반 노트·지뢰가 LN 머리/꼬리 칸에 겹침(LN 줄이 앞) | 일반 노트가 그 칸의 LN 반쪽을 지우고 `repair` 가 짝을 지운다 | 일반 노트가 덮어써 짝 링크만 남는다(정의되지 않은 동작에 가깝다) | 모델은 칸 하나에 노트 하나라 짝 링크만 남길 수 없다 |
| 고아 `#LNOBJ` | 일반 노트로 남긴다(13행) | 버린다. 직전 노트가 없거나 이미 LN 의 꼬리면 경고만 한다(`$7`) | 사용자 결정 범위 밖이라 두었다 |
| `#LNOBJ` 가 열린 LN 채널 머리를 닫음 | 닫지 않는다(기존 테스트 `lnobj_does_not_convert_an_ln_channel_start`) | 닫는다(`$7` 의 `LongNote` 이고 짝이 없는 분기) | 위와 같음 |
| 지뢰와 일반 노트가 같은 칸 | 지뢰가 일반 노트를 덮어쓴다 | 칸에 노트가 있으면 지뢰를 넣지 않는다(`$10`) | 위와 같음. 기존 테스트가 지금 동작을 고정한다 |


## 입력 시각 판정 — 시간으로 진행되는 판정을 프레임이 아니라 시각 순서로 (2026-10-15, S19)

> 감사 PLAY-3·PERF-1·PERF-2 와 사용자 결정 2([ADR](2026-10-15-adr-event-time-judging.md)). 원본은 프레임 구동이라 같은 입력도 프레임 속도에 따라 결과가 달라진다. rbms 는 결과가 프레임에 매이지 않게 이 부분을 일부러 원본과 다르게 둔다.

| 항목 | 원본 | rbms |
|---|---|---|
| 시간으로 진행되는 판정(놓친 노트, 지연 릴리스 확정, 일반 롱노트 끝, 지뢰, HCN 머리·끝) | 프레임마다 한 번, 그 프레임 시각으로(`JudgeManager.java:212-336, 556-628`) | 각자 기한이 된 순간에, 그 순간의 키 상태로(`JudgeEngine::update` 가 기한 순서로 멈춰 간다). 입력은 엔진을 그 시각까지 진행한 뒤 판정한다 |
| 한 프레임 안의 떼기와 다시 누르기 | 키마다 마지막 변화만 남아 떼기가 판정에 닿지 않는다(`BMSPlayerInputProcessor.java:191-218`) | 둘 다 그 시각에 판정한다. 마진 0 인 모드(7키·5키·24키)에서 판정 창 밖에서 일찍 뗀 롱노트는 다시 누르기가 몇 µs 뒤여도 BAD 로 확정된다 |
| 지뢰 | 프레임 처음의 키 상태로(`:216-247`) | 지뢰가 지나간 순간의 키 상태로 |
| HCN 게이지 틱 | 프레임마다 최대 한 번(`:305-336`) | 잔고가 200 ms 를 넘는 순간마다. 긴 프레임에서도 밀리지 않는다 |
| 게이지 자동 전환(GAS)·FAILED | 프레임마다 판정 뒤에 한 번(`BMSPlayer.java:638-672`) | 게이지가 바뀐 모든 순간 뒤에. NONE 에서 게이지가 비면 그 순간부터 판정하지 않는다 |
| 게이지 이력(0.5초) | 경계를 지난 첫 프레임의 값, 프레임당 최대 하나(`BMSPlayer.java:628-633`) | 경계 순간의 값. 실패 시각도 게이지가 빈 순간 |
| 컨트롤러 입력 | 입력 스레드가 약 1 kHz 로 훑는다(`MainController.java:357-372`) | gilrs 가 이벤트마다 찍은 시각. 한 폴링 안의 누름·뗌이 모두 판정되고, 디바운스는 이벤트 시각 사이로 잰다. 아날로그 턴테이블은 폴링마다 한 번 읽는다. gilrs 의 Windows 백엔드는 8~10 ms 로 폴링해 그만큼 양자화된다 |
| 컨트롤러 디바운스 | 원소마다 받아들인 변화 뒤 16 ms 동안 그 원소를 읽지 않는다(`BMControllerInputProcessor.java:61,136`, 양방향 잠금). 뗀 직후 창 안의 재누름은 창 끝까지 늦는다 | PAD DEBOUNCE(키 설정 파일의 `pad.debounce_ms`, 기본 16 ms, 0~100, SETTINGS INPUT 탭). 버튼·두 키 축 방향은 키보드와 같은 LR2 `inputinterval` 상태 기계(`input_debounce.rs`)다: 누름은 늘 즉시, 누른 뒤 창 안의 뗌만 창 끝까지 미루고, 넘긴 뗌 뒤 2 ms 안의 누름은 튐. 아날로그 턴테이블 방향만 원본의 양방향 잠금을 그대로 둔다(턴테이블 알고리즘이 잠금이 통과시킨 읽기 횟수를 센다). 패드는 한 곳에서 읽어 메뉴 입력에도 같은 디바운스가 걸린다(뗌 뒤 2~16 ms 튀는 버튼은 메뉴에서 두 번 눌릴 수 있다, 테스트 부채 3.71) |
| 키보드 입력 시각 | 1 kHz 폴링 스레드가 읽은 순간의 시각. 단 키 상태는 LWJGL2 키 버퍼가 렌더 루프(`Display.processMessages`)에서만 갱신되어 렌더 루프 주기(기본 240 fps, 약 4.2 ms)로 양자화된다(조사의 추정) | 이벤트 루프가 키를 받은 순간. present 대기는 전용 스레드(`gpu/present.rs`)가 하고 이벤트 루프는 프레임 사이에 창 말고는 기다리지 않으므로, 키는 OS 가 넘긴 즉시 받힌다(LR2 가 vblank 대기 동안 입력을 폴링하는 것과 같은 원리). 렌더 주기로 양자화되지 않는다. 이벤트 루프가 프레임을 갱신·그리는 CPU 시간은 남는다. winit 의 키 이벤트에는 하드웨어 시각이 없다([키보드 시각 ADR](2026-10-15-adr-keyboard-input-timing.md)) |
| 키보드 디바운스 | `KeyboardConfig.duration` 16 ms(0..100), 레인 키만, 1 ms 폴링으로 레벨을 다시 읽는다(`KeyBoardInputProcesseor.java:99-146`). 런처의 한 값이 키보드와 컨트롤러에 같이 들어간다 | KEY DEBOUNCE(`play.key_debounce_ms`, 기본 16 ms, 0~100, SETTINGS INPUT 탭). 뜻은 원본이 아니라 LR2 `inputinterval`(`En_input.cpp:276-292`)이다: 누름은 늘 즉시 받고, 누른 뒤 창 안의 뗌만 창 끝까지 미루며(그때도 떼어져 있으면 창 끝 시각으로), 넘긴 뗌 뒤 2 ms(LR2 의 폴링 간격) 안의 누름은 튐으로 버린다. 원본은 뗀 직후 창 안의 재누름을 창 끝까지 늦춰, 같은 레인 고밀도 연타에서 누름이 SLOW 로 밀린다. 이벤트 시각으로 잰다. 판의 레인 입력에만 걸고 메뉴에는 걸지 않는다. 컨트롤러는 키 설정 파일의 `pad.debounce_ms`(PAD DEBOUNCE)를 따로 두고, 같은 상태 기계를 쓴다(위 행) |

## 기록의 원본 정의 — avgjudge·BP·콤보·풀콤·TOTAL (2026-10-15, S19)

> 감사 REC-1·IR-3·PLAY-4·5·6·7·9·PERF-12 와 사용자 결정 1·3·4·5·6. 규칙 세대 `SCORE_RULE_VERSION` 4(`REFERENCE_RECORD_RULE_VERSION`). 옛 행의 처리는 각 ADR 과 `docs/play-options.md` 10.11절.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| `avgjudge` | 제출은 맞은 입력의 부호 있는 평균, DB 는 그 절댓값 | 노트마다 `|시간차|`, 놓친 노트 1,000,000 µs 의 평균(`BMSPlayer.java:911-930`) | 원본과 같다(`PlayRecord::avg_duration_us`). 부호 있는 평균은 `extra.mean_offset_us`. [ADR](2026-10-15-adr-avgjudge-reference-definition.md) |
| 게이지 사망 판의 `minbp` | 판정한 BAD+POOR+空POOR 만 | 도달하지 못한 노트를 더한다(`:909`) | 원본과 같다(`min_bp_with_unreached`). [ADR](2026-10-15-adr-failed-run-bp-unreached.md) |
| 코스 스테이지의 곡별 `max_combo`·풀콤 | 이어받은 콤보가 섞였다 | 스테이지 `combo` 로(`JudgeManager.java:668-676`) | 원본과 같다. [ADR](2026-10-15-adr-course-stage-combo.md) |
| 풀콤 조건 | `BAD+POOR==0 && max_combo==notes` | 끝에 남은 콤보 == 지나간 노트(`BMSPlayer.java:868`) | 원본과 같다. [ADR](2026-10-15-adr-full-combo-condition.md) |
| 코스 램프 | 마지막 스테이지의 램프 | 마지막 스테이지에서 코스 총 노트 == 코스 최대 콤보면 풀콤 계열, 아니면 게이지 램프(`MusicResult.java:402-428`) | 원본과 같다(`PlaySession::course_lamp`) |
| TOTAL 덮어쓰기 | 어시스트가 아니었다 | 설정이 없다(rbms 확장) | 넓힌 판정과 같은 어시스트, 리플레이에 기록. [ADR](2026-10-15-adr-total-override-assist.md) |

**아직 원본과 다른 점:**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 코스 스테이지의 곡별 기록 램프 | 게이지 램프를 남긴다 | FC 계열이 아니면 Failed, 결과에서 NoPlay 로 바꾼다(`BMSPlayer.java:863-880`, `MusicResult.java:368-371`) | 사용자 결정 5 는 콤보만 정했다 |
| 코스 BP | 스테이지 판정 수의 합(포기한 스테이지의 남은 노트는 POOR 로 들어간다) | 스테이지 `minbp`(도달 못 한 노트 포함)의 합 + 치지 않은 곡의 노트(`MusicResult.java:183-196, 430-436`) | 치지 않은 스테이지의 노트 수를 코스 진행이 모른다. 테스트 부채 |
| 옛 세대(3 이하) 행 | `avgjudge` 는 최고 기록 경쟁에서 빠지고 재전송 때 0, `minbp` 는 판정 수에서 도달 못 한 노트를 계산해 더해 비교, `combo`·램프·TOTAL 판은 그대로 | 해당 없음 | 되살릴 수 없는 값은 섞지 않는다 |

## 연습과 #TOTAL — 2차 감사 수리 (2026-10-15, S19)

> 감사 PLAY2-1·PLAY2-2·PLAY2-3. 판정 규칙은 바뀌지 않는다. 연습 판의 흐름과 차트 해석을 원본 쪽으로 맞추거나 기존 설계(게이지 잠금)를 실제로 지키게 했다.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 연습 판에서 게이지가 빔(GAUGE AUTO SHIFT NONE, HARD 이상·CLASS 계열) | 위 "입력 시각 판정"의 실패 즉시 판정 정지 뒤로 엔진이 멈춰, 구간은 끝까지 흐르는데 판정·놓친 노트·게이지·콤보가 사망 순간에 굳었다(키음·빔은 났다). `PracticeSession::gauge_locked` 는 테스트만 읽었다 | `STATE_FAILED` 를 거쳐 연습 패널로 돌아간다(`BMSPlayer.java:638-672, 735-737`) | 게이지 잠금이 NONE 을 CONTINUE 로 바꿔 구간 끝까지 판정한다(`PracticeSession::gauge_shift`, `PlayState::set_practice`). 다른 GAUGE AUTO SHIFT 는 설정대로다 |
| 연습 FREQUENCY 의 판정 폭 | 엔진이 차트 시각으로 판정해 판정 창·LN 마진·후보 게이트·見逃し 경계·HCN 틱의 실제 시간 폭이 재생 속도에 비례했다(50% 에서 실제 30 ms 늦음이 PGREAT, 200% 에서 10 ms 늦음이 GREAT). 판정이 보고하는 타이밍(FAST/SLOW 값, 자동 오프셋 조정, 노트의 play time)도 차트 시각이었다 | 차트를 재배치하고(`BMSModelUtils.changeFrequency`, `BMSPlayer.java:554-558`) `JudgeManager` 는 같은 창으로 실제 재생 시간에서 판정한다 | 엔진은 여전히 차트 시계로 돌고, `JudgeEngine::set_playback_rate` 가 모든 창·마진·게이트·틱을 그 시계의 길이로 늘이며 보고하는 타이밍은 실제 시간으로 되돌린다. 화면·소리·BGA 의 시간 축(`PracticeClock`)은 그대로다 |
| `#TOTAL` 줄의 명령어 분리와 trim (3차 감사 PARSE3-1) | 첫 유니코드 공백에서 명령어를 잘라 `#TOTAL:300`·`#TOTALS 300` 을 못 읽었다. 값은 유니코드 trim 이라 줄 끝 0x1A·NUL 이 남아 못 읽고(기본 TOTAL), 줄 끝 전각 공백·NBSP 는 지워 읽었다 | `matchesReserveWord` 가 `#TOTAL` 을 접두사로(대소문자 무시) 맞추고, 그 뒤 한 글자를 무엇이든 건너뛴 나머지를 Java `String.trim`(U+0020 이하만)으로 다듬는다(`BMSDecoder` 2257-2311, 1674-1720) | 같다(`TOTAL_COMMAND`, `reference_total_argument`). 줄 앞 공백은 아래 "줄 앞 공백" 행대로 지금도 받아들인다. 코퍼스 11,874 줄 중 해당 0 건. `PARSER_VERSION` 5 |
| 줄 끝 (3차 감사 검증자 추가 1) | `str::lines` 라 `\n`·`\r\n` 만 줄 끝이었다. 단독 `\r`(옛 Mac) 차트는 통째로 한 줄이 되어 제목이 길어지고 노트·TOTAL 이 모두 사라졌다 | `BufferedReader.readLine`: `\n`·`\r`·`\r\n` 모두 줄 끝 | 같다(`reference_lines`). 코퍼스 0 건. `PARSER_VERSION` 5 가 곡 DB 를 다시 읽는다 |
| `#TOTAL` 줄이 여럿 | 줄마다 덮어써 뒤의 숫자 아닌 값·0 이하 값이 앞의 유효한 값을 지웠다(기본 TOTAL 로 떨어짐). `200d`·`200f` 는 못 읽고, Rust 표기 `inf`·`nan` 을 읽었다(`nan` 은 NaN TOTAL 이 됐다) | `CommandWord.TOTAL`: `Double.parseDouble` 이 읽고 0 보다 클 때만 설정, 아니면 경고만 하고 앞 값을 둔다 | 같다(`rbms-parser` 의 `parse_java_double`). 곡 DB 는 `PARSER_VERSION` 4 가 다음 스캔을 전체 재스캔으로 만든다 |

**아직 원본과 다른 점:**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 연습 판의 게이지 사망 | 구간 끝까지 판정한다 | 연습 패널로 돌아간다 | 연습은 구간 끝까지 치는 것이 목적이라는 기존 설계(게이지 잠금, `docs/plan/phase-g-wiring/G4-practice.md`). 2차 감사 검증자 권고. **사용자 결정 대기**(S19 2차 리뷰 2): 판정 규칙은 같지만 판의 흐름이 원본과 다르다. A 원본대로 `is_failed` 면 연습 패널로 돌아간다(`run_is_over`·장면 사실 `failed` 수정) / B 지금처럼 잠금을 유지하고 ADR 로 남긴다 |
| 연습 FREQUENCY 의 반올림 | 창 길이를 차트 시계 µs 로 내림 | 노트 시각을 µs 로 내림(`(long)(time / freq)`) | 1 µs 안의 차이 |
| `#TOTAL` 의 16진 표기(`0x1p8`) | 읽지 않는다 | 읽는다 | 쓰는 차트를 보지 못했다 |
| 줄 앞 공백 (3차 감사 검증자 추가 2) | 줄 전체를 유니코드 trim 한 뒤 보므로 ` #TOTAL 300`·` #00111:0101` 처럼 공백으로 시작하는 `#` 줄도 읽는다 | 줄 앞 trim 이 없어 `charAt(0)=='#'` 가 아닌 줄은 버린다(오프셋 661-669) | 헤더·데이터 줄 전체에 걸쳐 노트 수가 바뀔 수 있어 따로 다룬다. 코퍼스 0 건(`^[ \t\x0b\x0c]+#[0-9A-Za-z]`). 테스트 부채 |
| 다른 헤더의 trim | `#TOTAL` 말고는 줄 끝을 유니코드 trim 한다(줄 끝 전각 공백·NBSP 를 지운다) | Java `String.trim`(U+0020 이하만) | `#TOTAL` 만 이번에 맞췄다. 제목·파일 이름의 줄 끝 전각 공백은 보지 못했다 |

## 기록 목표의 진행 중 값 — 기록 판의 노트별 판정 (2026-10-19)

> 사용자 요청(2026-10-19) "my best, localbest 등에서는 퍼센트가 아니라 실제 best 기록의 동시간의 기준으로 노트 하나하나당의, 리플레이 플레이 그대로를 기준으로". 판정 규칙은 바뀌지 않고 플레이 중의 "지금까지" 값만 바뀐다. 조사와 구현: [record-target-pace.md](../reference/record-target-pace.md), 결정: [ADR](2026-10-19-adr-record-target-follows-its-run.md), 사용자 문서: `docs/play-options.md` 11.7절.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 스킨의 BEST 칸(최고 기록 대비 점수 차 152/172, rate 112, Lua `rate_best`)의 진행 중 값 | 최고 EX 를 판정된 노트 수에 비례시킨 값 | 로컬 최고 판의 ghost 를 판정된 노트 수(passnotes)만큼 누적한 값. ghost 가 없거나 길이가 노트 수와 다르면 비례(`ScoreDataProperty.java:114-121, 171-172`) | 원본과 같다. 로컬 최고 판의 리플레이를 다시 판정해 얻은 노트별 판정이 있으면 그 판이 지금까지 판정한 노트 수만큼 판정했을 때의 EX, 없으면 비례. 노트 수가 다르면 쓰지 않는 것도 같다 |
| 노트별 판정을 기록하는 순서 | 기록하지 않았다 | 소비한 판정마다 `ghostJudge[passnotes]` 에 적고, `MissCondition::ONE` 의 이른 return 앞이라 세지 않는 소비 POOR 도 자리를 차지한다(`JudgeManager.java:137-140, 639-649`) | 같다(`JudgeEngine::vanished`). 空POOR 는 노트를 소비하지 않으므로 없다 |
| 도중에 떠나거나 게이지로 끝난 판의 도달하지 못한 노트 | - | ghost 의 칸이 POOR(4)로 채워져 EX 0 | 같다(`GhostPace::of_judges`) |
| 연습 | 비례 | ghost 없음(`BMSPlayer.java:441-442`) | 같다 |

**아직 원본과 다른 점:**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 목표 칸의 ghost | LOCAL BEST, 그리고 이 기기에서 친 판이 있는 IR BEST 목표도 그 기록 판의 노트별 판정을 따른다 | 목표(rival) 칸은 늘 비례다. `setTargetScore` 의 rival ghost 인자가 `null`(`BMSPlayer.java:453`). 원본 목표 목록에는 LOCAL BEST 도 없다(`TargetProperty.java:61-76`) | R-BMS 확장. 사용자 요청 |
| ghost 의 출처 | 저장된 리플레이를 작업 스레드에서 다시 판정한다. 노트 수·PGREAT·GREAT 중 하나라도 기록과 다르면 쓰지 않고 비례(허용 오차 없음). 옛 FRAME 시계 리플레이는 판정하지 않고 비례. 로딩이 그 결과를 기다린다 | 점수 DB 의 `ghost` 열을 그대로 읽는다(`ScoreData.java:507-523`) | 점수 DB 마이그레이션을 하지 않는다. 옛 기록에는 그 열이 없다. 재판정 시간과 대기 상한은 테스트 부채 3.75·3.76 |
| 같은 EX 의 최고 기록이 여럿 | 그 EX 를 처음 낸 판, 그 판에 리플레이가 없으면 다음 같은 EX 판(`best_record_with_replay_in_ln_mode`) | 최고 행의 ghost 는 EX 가 엄격히 클 때만 바뀌므로 처음 낸 판의 것(`ScoreData.java:550-564`) | 리플레이가 없는 처음 판을 건너뛰는 것만 다르다 |
| RIVAL 목표, 다른 기기에서 친 IR BEST | 비례 | 비례(rival ghost 가 늘 null) | 서버 리플레이가 판정 알고리즘·LN 종류·시계를 싣지 않아 같은 판정을 보장할 수 없다(`ir_replay.rs` 의 `from_ir_replay`). 테스트 부채 3.79 |
| 비례로 물러날 때 화면 표시 | 없다(콘솔에 `record run <파일> is paced evenly: <이유>` 한 줄) | 해당 없음. ghost 가 없으면 조용히 비례 | **사용자 결정 대기.** 테스트 부채 3.78 |
| 결과 화면의 목표·BEST 값 | 최종 값 그대로 | 최종 값(`MusicResult.java:343`) | 같다 |

## 옵션 패널·램프 등급·스킨 색·확장 id (2026-10-22)

> 사용자 지시(2026-10-22, 요지) "끝까지 다 구현해놔"에 따라 메인이 추천안으로 정했다(사용자 미확인). 판정 규칙과 `SCORE_RULE_VERSION`(4)은 바뀌지 않는다. 결정과 근거: [패널 배치](2026-10-22-adr-option-panel-layout.md), [어시스트 등급](2026-10-22-adr-assist-lamp-grade.md), [확장 id](2026-10-22-adr-rbms-extension-ids.md), [스킨 색 행](2026-10-22-adr-skin-colour-rows.md), [내장 글자](2026-10-22-adr-ui-text-device-pixels.md). 내장 화면의 글자 래스터는 R-BMS 고유 화면이라 원본과 대조할 대상이 없어 아래 표에 넣지 않았다.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 선곡 옵션 패널의 키 표 | 원본 표(`MusicSelectKeyProperty` `BEAT_7K`·`POPN_9K`)에서 `NEXT_REPLAY` 만 뺀 것 | 패널 1 건반 4 `OPTIONDP_DOWN`, 건반 6 `OPTION2_UP`. 패널 2 건반 3 `JUDGEAREA`, 5 `MARKNOTE`, 6 `BPMGUIDE`, 7 `NOMINE`. 패널 2·3 의 턴테이블은 비어 있다(패널 1 만 `TARGET`) | 열 칸을 바꿨다: 패널 1 건반 4 → GAUGE 이전, 건반 6 → HI-SPEED FIX 이전. 패널 2 건반 3 → SCRATCH AUTO, 5 → KEEP GREEN NUMBER, 6 → LANE COVER, 7 → LIFT, 턴테이블 → HIDDEN+. 패널 3 턴테이블 → JUDGE ALGORITHM 이전·다음. 나머지 칸은 원본과 같고 9키도 같은 인덱스를 따른다. 테스트 `keys.rs` 의 `CHANGED_SLOTS` 가 고정한다 |
| SCRATCH AUTO 클리어의 램프 | LIGHT ASSIST(id 3) 하나 | 원본에는 SP 에서 턴테이블을 자동으로 치는 옵션이 없다. 가장 가까운 DP BATTLE AUTO-SC 가 `assist = max(assist, 1)` 로 `LightAssistEasy` 가 된다(`BMSPlayer.java:238-254`). 어시스트 램프에 등급은 없다 | 램프 id 3 안에 등급 0~4(LIGHT ASSIST, ASSIST EASY, ASSIST CLEAR, ASSIST HARD, ASSIST EXHARD). beatoraja·LR2 어느 쪽에도 선례가 없는 R-BMS 확장이다. 램프 id·와이어 이름·강등 규칙은 원본 그대로이고 최고 기록 비교 키만 `(clear, 등급, EX, BP)` 로 넓혔다. 외부 스킨은 0~10 만 본다 |
| 램프 id 2·3 의 표기 | 앱 `ASSIST EASY`·`LIGHT ASSIST`, 기본 스킨 `ASSIST CLEAR`·`LIGHT ASSIST CLEAR`, 웹 `ASSIST`·`L-ASSIST` | `ClearType` `AssistEasy`(2), `LightAssistEasy`(3) | 앱·기본 스킨·웹 모두 `ASSIST`·`LIGHT ASSIST`. 등급 이름 `ASSIST EASY` 와 겹치지 않게 id 2 를 줄인 R-BMS 표기이고 와이어 문자열(`AssistEasy`, `LightAssistEasy`)은 그대로다 |
| 호스트가 답하는 id | 원본이 선언한 id 만(2026-10-09 결정) | `SkinProperty` 최대 옵션 3035·이벤트 388, 스킨 몫 옵션 900~999·이벤트 1000~1999·타이머 10000~19999 | 이벤트 21000~21099 와 불린 21100~21199 도 답한다(SCRATCH AUTO·KEEP GREEN NUMBER 켬끔, 어시스트 등급, 닿지 않는 키). 원본 id 표와 `reference_implements` 는 건드리지 않았다. 표는 [확장 id ADR](2026-10-22-adr-rbms-extension-ids.md) |
| 스킨 맞춤 행의 종류 | property·file·offset(원본과 같다) | `property`, `filepath`, `offset` 셋뿐(`SkinHeader`) | `color` 행을 더했다(헤더 `color` 목록, `skin_config.color`, 저장 `colors`). beatoraja 는 모르는 키를 무시하므로(`setIgnoreUnknownFields(true)`) 같은 스킨이 거기서는 제작자 기본색으로 그려진다 |
| 노트 색 | destination 색을 읽지 않고 알파만 남긴다(원본 `LaneRenderer` 가 `setColor(WHITE)` 로 초기화하는 것과 같다) | 같다 | `note.tint`·`note.long_tint` 가 있는 레인에만 RGBA 를 곱한다. 키가 없으면 이전과 같다. 지뢰·hidden·processed·HCN 닳는 중 몸통·SCRATCH AUTO 초록은 곱하지 않는다 |
| dst 객체의 `tint` | 없음 | 없음 | 키가 있고 저장값이 있을 때만 `color.tinted(tint)`. 불투명도 오프셋을 먼저 더하고 틴트의 알파를 곱한다(노트와 같은 순서) |

**아직 원본과 다른 점:**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 외부 스킨 위에서 재배정한 패널 키 | ModernChic 같은 스킨이 죽은 칸 라벨(JUDGE AREA, MARK NOTE, BPM GUIDE, NO MINE)로 그리는 키가 저장되는 설정을 바꾼다: SCRATCH AUTO, KEEP GREEN NUMBER, LANE COVER, LIFT, HIDDEN+, GAUGE 이전, HI-SPEED FIX 이전, JUDGE ALGORITHM. 화면은 변하지 않고 OptionChange 소리만 난다 | 그 칸이 원본 설정을 뒤집는다: 패널 2 는 `showjudgearea`·`markprocessednote`·`bpmguide`·`mineMode`, 패널 1 건반 4·6 은 `optiondp`(54)·`option2p`(43). R-BMS 는 패널 2 의 네 설정을 갖지 않아 전에는 그 칸이 아무 일도 하지 않았다. 패널 1 건반 4 는 값만 저장하고 플레이가 읽지 않았으며 건반 6 은 건반 2 와 같은 RANDOM 이었다 | 스킨이 확장 id 를 안다는 신호 통로가 없어 막지 못했다. **사용자 결정 대기.** 방법 셋(헤더 플래그, 번들 팩 경로, 실행 중 읽기 기록)은 [패널 배치 ADR](2026-10-22-adr-option-panel-layout.md) 7절 |
| 원본이 어시스트로 세는 옵션 | SCRATCH AUTO 와 넓힌 판정·TOTAL 만 센다 | H-RANDOM, S-RANDOM-EX, SPIRAL, ALL-SCRATCH, BPM 가이드, 지뢰 제거 등도 어시스트(`score = false`)(`BMSPlayer.java:200-256`) | 이번 램프 등급 작업의 범위 밖이다. 세면 기존 기록과 서버 ranked 정책이 함께 바뀐다 |
| 코스 최고 기록의 어시스트 등급 | 서버 `course_best.assist_grade` 열은 항상 0 이다 | 해당 없음(R-BMS 확장) | 코스 제출 본문과 코스 DB 에 게이지가 없어 파생할 수 없다. **사용자 결정 대기:** 코스 최고 기록에 등급 정책을 둘지 |

## 코스·선곡 트리·다운로드 (2026-10-23)

> 사용자 결정 1~21(2026-10-23, "1-11 추천대로" 등)과 메인 판단의 결과다. 결정과 근거: [Esc 결과 화면](2026-10-23-adr-escape-shows-the-result.md), [코스 시작 슬롯과 LN 제약](2026-10-23-adr-course-stage-start-and-ln.md), [코스 게이지](2026-10-23-adr-course-gauge-modes.md), [폴더 스택](2026-10-23-adr-select-folder-stack.md), [없는 곡과 다운로드](2026-10-23-adr-missing-charts-and-download.md), [GENOSIDE 2018 코스](2026-10-23-adr-genoside-2018-course-data.md). 이 절의 원본 줄 번호는 각 ADR 이 적은 것을 옮겼고 이번 정리에서 원본 소스를 다시 열지 않았다.

| 항목 | 전 | 원본 | 지금 |
|---|---|---|---|
| 노트를 친 판의 Esc | 결과 화면 없이 선곡, 알림 한 줄 | `stopPlay` 가 남은 노트가 있으면 FAILED 로 닫고 점수 데이터가 있으면 RESULT 로 간다(`BMSPlayer.java:693-742`) | 노트를 하나라도 친 판은 로그인과 상관없이 결과 화면. 남은 노트는 늦은 POOR 로 세고 게이지 그래프는 나간 자리부터 0 으로 채운다(원본 `:722-727`). 코스 도중이면 그 스테이지를 FAILED 로 접어 불합격 코스 결과까지 간다 |
| 코스 스테이지의 시작 게이지 슬롯 | 사용자 GAUGE 와 무관하게 늘 CLASS | 사용자 게이지로 정한다(`GrooveGauge.java:164`) | ASSIST EASY·EASY·NORMAL → CLASS, HARD → EXCLASS, EXHARD·HAZARD → EXHARDCLASS(IIDX 표는 EXCLASS 까지) |
| 코스 곡 사이의 게이지 이월 | 선택 게이지의 끝 값 하나 | 직전 곡 게이지 기록의 마지막 값으로 9슬롯 모두를 세팅한다(`GrooveGauge.java:150-155`) | 9슬롯 모두를 이월한다. 값은 기록 표본이 아니라 판 끝의 정확한 값이다(표본 간격 안의 차이) |
| 코스의 ln·cn·hcn 제약 | 플레이에 적용하지 않음 | 설정의 LN 종류를 강제한다(`MusicSelector.java:501`) | 모든 스테이지가 그 LN 종류로 판정한다. 설정 파일은 바꾸지 않고 복사본에만 적용한다(원본은 `config.setLnmode` 로 설정 자체를 바꾼다) |
| 표 레벨 폴더 제목 | `LV 12` | 표 헤더 symbol + 레벨(`TableDataAccessor.java:185`) | 발광 `★12`, Stella `st3`, Satellite `sl5`. symbol 이 없거나 `*` 이면 `LV 12`. 발광 표의 화면 글자가 바뀐다 |
| 폴더에서 나올 때의 커서 | 목록의 첫 줄 | `BarManager` 가 진입 직전의 막대를 쌓아 두고 나올 때 새 목록에서 찾는다(`BarManager.java:73, 273, 298-302, 375-377, 424-447`) | 폴더 스택으로 같다. 재스캔·표 도착·표 관리 Esc 뒤에도 열린 경로와 커서 막대를 다시 찾는다 |

**아직 원본과 다른 점:**

| 항목 | rbms | 원본 | 이유 |
|---|---|---|---|
| 코스가 쓰는 게이지 표 | 설정 COURSE GAUGE(LR2 기본·BEATORAJA·IIDX)가 정한다. 코스의 게이지 제약은 막대 배지에만 남는다. 코스를 시작할 때 한 번 읽어 모든 스테이지에 쓴다 | 코스 제약이 표를 정한다(`GrooveGauge.java:128-147`) | 사용자 결정 21. 제약이 표를 정하면 발광·Stella 코스(제약 `gauge_lr2`)는 모드와 무관하게 늘 LR2 라서 BEATORAJA·IIDX 모드가 의미가 없다 |
| IIDX 게이지 표 | 있다(Class 시작 100·0% 실패·30% 이하에서 감소량 절반 등). EXHARD 이상은 EX-DAN | 없다 | R-BMS 확장. 수치 출처는 iidx.org 한 곳이고 실제 IIDX 의 내부 단위·반올림은 알 수 없다. 5·9·24키에도 같은 수치다 |
| 노트 하나 못 친 판, 오토플레이·리플레이 보기·연습의 Esc | 결과 화면 없이 선곡(코스 스테이지 사이·앱 종료도 지금대로). 닫힘 연출 없이 곧장 결과 화면 | 오토플레이·리플레이도 결과 화면으로 간다. FAILED 로 닫는 연출을 거친다 | 기록이 없는 판이거나 변경 최소(메인 판단, 사용자 미확인) |
| 곡 받기 | 직접 zip 링크는 앱이 받아 다운로드 폴더에 풀고 그 폴더만 다시 스캔한다. 나머지는 브라우저로 넘긴다. 차분만 받으므로 소리는 곡 URL 에서 따로 받아야 한다 | 두 링크를 곧장 브라우저에 넘긴다(`EventFactory.open_download_site`) | 사용자 결정 15·16 |
| 없는 곡 막대의 검색 | 검색에는 나오지 않는다(검색은 라이브러리만 본다). 필터가 켜지면 빠진다 | 이 작업에서 대조하지 않았다 | 확인하지 못함 |
| DP 코스의 MIRROR | FLIP 이 없다(R-BMS 가 `LaneOption` 을 플레이에 적용하지 않는다) | 이 작업에서 대조하지 않았다 | 범위 밖. 테스트 부채 |
| 코스 제약 NO GOOD·NO GREAT | 적용하지 않고 배지만 남긴다(대상 코스 0개) | 이 작업에서 대조하지 않았다 | 범위 밖. 테스트 부채 |
