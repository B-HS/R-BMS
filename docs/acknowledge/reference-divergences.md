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
| J6 | 24K(KEYBOARD) 모드 자체 없음 | 미구현 | ±30/90/200, (-320,240), MS(-200,650) | M | Phase D 예정(모드 추가 포함) |
| J7 | 7K LN 종단 윈도우가 실은 5K(FIVEKEYS) 값 | 5K 값 오적용 | (120,160,200,(-280,220)), MS 없음 | S | **Phase A 수정 완료** |
| J8 | PMS LN 종단이 7K(=5K) 복제 | 7K 표 | 120/150/217/283 | S | **Phase A 수정 완료** |
| J9 | 스크래치 NOTE/LN 윈도우·후보 게이트 없음(키와 동일) | 키와 동일 | 7K scr ±30/70/160,(-290,230); longscratch 130/170/210,(-290,230); 5K 별도 | M | **Phase A 수정 완료** |
| J10 | LN 릴리스 마진이 전모드 `LN_MARGIN=200_000` | 전모드 동일 | 7K/5K/KB 0, PMS 200000, `longnoteMarginRate` 설정 | S | **Phase A 수정 완료** |
| J11 | plain LN 미릴리스 확정 방식 | 마진 후 종단 윈도우 재판정 | `lnstartJudge`(머리 판정)로 확정 | S | **Phase A 수정 완료** |
| J12 | JUDGE WIDTH — judgerank×rate를 PG~BD 일괄, 클램프 없음, 키/스크 공통 | 위와 동일 | PG/GR/GD 3개만, MS 상한·단조 클램프, 키/스크 분리 | M | **Phase A 수정 완료** |
| J13 | `#RANK` 범위 밖이 clamp(0,4) → 25/125 | clamp | NORMAL(75) 폴백 | S | **Phase A 수정 완료** |
| J14 | `#DEFEXRANK` 미파싱 | 미파싱 | judgerank×75/100, `<=0`은 NORMAL 75 폴백 | S | **Phase A 수정 완료**(리뷰에서 `<=0` 분기 추가 정정) |
| J15 | 기본 TOTAL이 헤더 없으면 200 고정 | 200 고정 | `max(260, 7.605n/(0.01n+6.5))`, KB는 별도 floor | S | **Phase A 수정 완료** |
| J16 | 지뢰가 판정 대상 제외, 데미지 없음 | 데미지 없음 | 눌린 채 통과 시 `gauge.addValue(-damage)` | S | **Phase A 수정 완료**(데미지 스케일 자체는 아래 보류 참조) |
| J17 | JudgeAlgorithm이 Duration 고정 | Duration 고정 | Combo(기본)/Duration/Lowest/Score 선택 | M | 보류 — Phase D 예정 |
| J18 | 판정 완료 노트 재타격이 후보 제외(무판정) | 무판정 | MS 범위면 空POOR | S | **Phase A 수정 완료**(리뷰에서 커서 전진 후 재타격 경로 사망 결함 추가 발견·수정) |
| J19 | fast/slow `dm==0`이 LATE로 집계 누락 | 누락 | EARLY | S | **Phase A 수정 완료** |
| J20 | 게이지가 단일 `Gauge`만 갱신(9종 병렬 아님) | 단일 | 9종 전부 갱신, 선택만 표시 | M | 보류 — Phase D 예정 |
| J21 | 게이지 세트가 7K 6종뿐 | 7K 6종 | 7K 9종(CLASS 계열 추가) + 5K·PMS·KB·LR2 각 9종 | M | 보류 — Phase D 예정 |
| J22 | MODIFY_DAMAGE(EXHARD/HARD_LR2 등) 없음 | 없음 | EXHARD_5/HARD_LR2/EXHARD_LR2 | S | 보류 — Phase D 예정(J21 후) |
| J23 | PMS 판정 규칙(`MissCondition`·`JudgeWindowRule.PMS` fixjudge) 없음 | 데이터만 반영, 엔진 미배선 | per-index fixjudge + fixmin/fixmax 클램프 | M | 부분(Phase A: judgeVanish/MissCondition 데이터화만) — 엔진 배선은 Phase D 예정 |
| J24 | CN deferral / HCN 연속 게이지 / BSS·MSS 없음 | 없음(문서상 Phase 7) | `JudgeManager` 해당 로직 | L | 보류 — Phase D 예정 |
| J25 | lnmode 강제(LN→CN/HCN)가 차트 `#LNMODE`만 | 차트 값만 | `PlayerConfig.lnmode` 0~3으로 강제 가능 | S | 보류 — Phase D 예정 |
| J26 | GAS(게이지 자동전환)/bottom shiftable gauge 없음 | 없음 | `gaugeAutoShift` 5모드 + `bottomShiftableGauge` | M | 보류 — Phase D 예정(J20 후) |

**파급:** J1+J2로 어긋나 있던 IR 제출의 `pr/ms`(epr/lpr/ems/lms) 4필드는 Phase A 수정으로 정합됐다(`app_play.rs`, 앱 수정 없이 그대로 정확해짐).
**Phase A 리뷰에서 추가로 드러난 결함(계획 표에는 없던 것)**: 재타격(空POOR) 판정이 판정 사다리(PG→BD→MS)를 타 BD 안이면 空POOR가 아예 안 나던 결함, `JudgeEngine::from_model`이 `apply_mode` 결과를 덮어써 5K/PMS가 7K 윈도우를 쓰던 결함(`from_model_for_mode` 신설로 해결), `scaled()`의 하한 클램프가 `max(1)`이라 judgerank 0에서 원본과 다르게 붕괴하지 않던 결함 — 전부 수정 완료.
**보류(1차 출처 미확인)**: 지뢰 데미지의 채널값→게이지 퍼센트 변환 배율. 레퍼런스 구현 `BMSDecoder`가 바이너리(jbms-parser.jar)이고 이 환경에 JRE가 없어 디컴파일 불가. 현재는 채널 id 값을 그대로 데미지로 취급(레포 기존 결정, `docs/reference/_appendix-raw.md:371-372` 근거). 확정하려면 JRE 환경에서 `BMSDecoder` 지뢰 채널 파싱을 직접 대조해야 한다.

## 기각 (not-a-bug)
- `#SETRANDOM/#RONDAM` 수용(우리 확장, 무해)
- 데이터라인 감지 colon@byte5(현 구현 충분)
- mechanics.md §2 STOP 공식 본문이 `/192` 누락(코드는 정확)
- 미종결 LN(LN 채널 홀수 개) → dangling LongStart 잔존(허용)

## 설계 입장
**#RANDOM 계열 제어흐름은 레퍼런스 구현과 bug-for-bug 호환하지 않는다.** 차트 해시는 raw 바이트 기준이라 점수·IR·리플레이 키는 영향 없고, #RANDOM 분기는 본래 플레이 시 무작위다. 우리 구현은 #ELSE/#ELSEIF/중첩을 spec에 맞게 더 완전히 처리한다 — 이는 의도된 개선이다.
