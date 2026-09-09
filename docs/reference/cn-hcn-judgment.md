# CN / HCN 판정 — beatoraja 대조 구현 스펙 (후속 작업용)

> 로드맵 Phase 1(`LnKind` 판정 전파 + CN 종료 판정) / Phase 7(HCN 연속 게이지)의 **충실 포팅 스펙**.
> 현재 rbms는 모든 롱노트를 단일 경로(`judge/matcher.rs::release` = `worse(head, end)`, 단일 `ln_end` 윈도우)로 처리해 LN/CN/HCN을 구분하지 않는다. 차이는 `docs/acknowledge/beatoraja-divergences.md` "진행 중" 참조.
> **상태(2026-06-03):** ✅ **구현됨** — 2-판정 모델(아래 §구현)을 `LnKind::Cn`/`Hcn`에만 게이트해 적용. 검증된 LN/Normal 경로는 byte 불변(회귀 0), CN/HCN 합성 픽스처 5종으로 고정(`crates/rbms-judge/src/lib.rs` `cn_*`/`hcn_*`/`ln_remains_*`). **HCN 연속 게이지(§HCN)는 Phase 7로 잔여.** (lntype IR 보고 = `ir_map::ir_lntype`는 별개로 완료.)

## 원본 위치
`/Users/gkn/beatoraja/src/bms/player/beatoraja/play/JudgeManager.java` (903줄). 윈도우 정의는 `bms/model` rule, 게이지는 `GrooveGauge`.

## LN 종류 (BMSModel)
- `#LNTYPE 1` = TYPE_LONGNOTE(레거시), `#LNTYPE 2` = MGQ(미사용).
- `#LNMODE` = 0(undefined→타입에 위임), 1=LN, 2=CN(charge), 3=HCN(hell charge). rbms `chart::to_model`이 `lnmode`→`LnKind`(0/1→Ln, 2→Cn, 3→Hcn)로 이미 매핑(테스트됨).
- IR `lntype` 인코딩(백엔드 contract `data-model.md`/`compatibility.md`) = **0=LN, 1=CN, 2=HCN** = `LnKind` 순서. (`ir_map::ir_lntype` 구현·테스트 완료)

## 판정 윈도우 (rule.getJudge)
- `nmjudge` = NoteType.NOTE (노트 머리).
- `cnendmjudge` = NoteType.LONGNOTE_END (키 LN/CN 종단).
- `smjudge` = NoteType.SCRATCH, `scnendmjudge` = NoteType.LONGSCRATCH_END (스크래치 LN 종단).
- `nreleasemargin` = rule.longnoteMargin, `sreleasemargin` = rule.longscratchMargin (LN 늦은 릴리스 유예).
- rbms 현황: `JudgeWindows::ln_end_for_mode` 단일 윈도우만 보유. **추가 필요:** LN end vs CN end가 같은 LONGNOTE_END 윈도우를 쓰되 **로직이 다름**(아래).

## 머리(press) 판정 — JudgeManager 436-471
- 공통: 대상 노트 추출 후 `nmjudge`로 `judge` 산출. `judgeVanish[judge]`(PG/GR/GD/BD는 true, POOR=false)면 "소실"로 처리.
- **LN(443-459):** `judgeVanish[judge]`면 `lnstartJudge=judge`, `lnstartDuration=dmtime`, `processing=pair`로 홀드 시작(레이저색=8). 아니면 즉시 확정.
- **CN/HCN(460-471):** `judgeVanish[judge]`면 `processing=pair`로 홀드 시작. 그 다음 **항상 `updateMicro(ln, judge, judgeVanish[judge])` 호출** — 즉 머리 판정을 즉시 기록.
- rbms 현황(matcher `press`): LN 머리는 `holding=true`+`head_judge=Some(judge)`만, 확정은 release까지 보류. **CN은 머리 판정을 즉시 카운트해야** 함(beatoraja와 차이).

## 종단(key-up) 판정 — JudgeManager 508-572
키가 떨어질 때 `processing != null`이면:
- 윈도우 = 스크래치면 `scnendmjudge` 아니면 `cnendmjudge`. `dmtime = end.microTime - releasePressTime`, `judge`=윈도우 인덱스.
- **CN/HCN(517-542):**
  - 스크래치(BSS): `judge != 4 || key != sckey` 면 `release=false`(중간 떼기 무시, 계속 홀드 가능). 같은 스크 키로 정확히 종단(judge==4)이면 처리.
  - `if judge >= 3 && dmtime > 0`(= BAD/POOR 범위이며 **종단보다 이르게** 뗌): **deferral** — `releasetime=now`, `lnendJudge=judge`로 보류(아직 확정 안 함; 다시 누르면 회복 가능).
  - else: **즉시 확정** = release 윈도우 `judge`로 `updateMicro`(POOR 포함), `processing=null`.
  - ⇒ **CN 종단은 릴리스 타이밍 자체가 판정**이다. 머리 판정과 `worse` 결합이 아니라 release `judge`로 확정(이른 릴리스는 BAD/POOR).
- **LN(543-571):**
  - 스크래치: `key != sckey` 면 `release=false`.
  - `judge = max(judge, lnstartJudge)` (더 나쁜 쪽), `if |lnstartDuration| > |dmtime|: dmtime = lnstartDuration` (머리 타이밍이 크면 그걸로).
  - `if judge >= 3 && dmtime > 0`: deferral(`lnendJudge=3`). else 즉시 확정.
  - ⇒ **LN 종단은 머리 판정으로 캡**되고(종단이 머리보다 좋을 수 없음), 종단 윈도우는 보조.

## 늦은 종단 / 미릴리스 — JudgeManager 577-628 (LN终端判定 루프)
- **LN(583-589):** `releasetime`가 설정됐고 `releasetime + releasemargin <= now` 면 `lnendJudge`로 확정(유예 후 늦은 릴리스 흡수). 마진 내 재홀드 허용이 LN 특유.
- CN/HCN: deferral 후 재누름으로 회복하거나, pair 도달 시 확정(루프 뒷부분 621-638). HCN은 미릴리스 시 종단에서 처리.

## HCN 연속 게이지 — JudgeManager 299-345 (Phase 7)
- `passing` = 현재 통과 중인 HCN(머리 통과 시 set, 종단 통과 시 clear). `inclease` = 이번 프레임 눌림 여부(또는 autoplay/auto-hit).
- 눌림 유지: `mpassingcount += dt`; `> hcnmduration`마다 `gauge.update(1, 0.5)` (게이지 +0.5), `mpassingcount -= hcnmduration`.
- 안 눌림: `mpassingcount -= dt`; `< -hcnmduration`마다 `gauge.update(3, 0.5)` (게이지 -0.5, BAD 취급).
- ⇒ HCN은 홀드 중 **연속적으로** 게이지가 오르내린다. rbms `gauge.rs`에 등가물 없음 → Phase 7에서 추가.

## rbms 구현 (✅ 적용됨 — `Cn`/`Hcn` 게이트, 2026-06-03)
1. **플럼빙:** `matcher.rs`의 `JNote`에 `ln: Option<LnKind>` 추가, `from_model`이 `LongStart{ln}`에서 운반. 공개 `from_pairs`/`new` 시그니처 유지(내부 `from_triples` 도입, pair-빌드는 `LnKind::Ln` 기본). `is_charge(ln)`로 CN/HCN 게이트.
2. **2-판정 모델:** CN/HCN은 **head(press 시 즉시 카운트) + end(release 시 별도 카운트)** = 2 판정/노트. `total_notes`·`count_playable_notes`가 CN/HCN을 2로 카운트(분모 일관). `press`: CN/HCN head면 `apply(judge)` 즉시. `release`: CN/HCN은 `final = end_judge`(ln_end 윈도우, head로 capping 안 함), LN은 `worse(head,end)` 유지. `update`: CN/HCN over-hold면 end 1판정, 미히트면 head+end 2 Miss; LN은 1.
3. **검증:** 합성 픽스처 5종(`cn_head_and_end_are_two_judgments`·`cn_early_release_judges_end_without_capping_head`·`cn_never_hit_misses_head_and_end`·`hcn_end_is_also_two_judgments`·`ln_remains_single_judgment_worse_of_head_end`). 전체 887 통과·무경고. LN/Normal byte 불변(회귀 0).

## 잔여 (후속)
- **HCN 연속 게이지(Phase 7):** 홀드 동안 `hcnmduration` 주기로 게이지 ±0.5. `gauge.rs`에 연속 증감 API + 엔진에 passing/홀드 시간 누적 상태. (현재 HCN은 종단 판정만 CN과 동일, 연속 게이지 없음.)
- **CN deferral/재홀드(이른 릴리스 후 재누름 회복):** 현 구현은 release 시 즉시 end 판정 확정(beatoraja의 `judge>=3 && dmtime>0` deferral 미구현). 대부분 케이스 동등, 재홀드 회복은 후속.
- **BSS(스크래치 LN):** `scnendmjudge`·중간 떼기 무시·같은 스크키 종단. 스크래치 회전 단순화 항목과 함께 후속.

## 미검증 경계 (정직)
- beatoraja의 **노트 카운트 분모**(CN을 passnote 2로 세는지)는 BMS 모델이 **외부 라이브러리**(beatoraja 소스 트리 밖)라 원본 대조 불가. rbms는 `JudgeManager`의 **2× `updateMicro`(head+end) 증거**에 근거해 CN=2로 카운트하고, `count_playable_notes`↔judge `total_notes`↔gauge 분모를 **내부 일관**되게 맞췄다. 실제 CN/HCN 차트가 corpus에 거의 없어 today 영향은 미미. 실차트 확보 시 회귀 추가 권장.
