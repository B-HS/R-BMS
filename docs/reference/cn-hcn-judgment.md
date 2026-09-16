# CN / HCN 판정 — 레퍼런스 구현 대조 구현 스펙

> **상태(2026-09-16): 현재 구현 기준.** CN/HCN 2-판정, CN deferral/재홀드, HCN 연속 게이지, scratch BSS/MSS 및 IR `lntype` 보고는 완료했습니다. Phase H `cargo test --workspace`는 exit 0이며 등록 테스트는 3,040개입니다.

> 로드맵 Phase 1과 Phase 7의 판정 동작을 실제 `rbms-judge` matcher에 반영한 구현 스펙입니다.
> **역사 상태(대체됨):** 아래 최초 조사 시점에는 rbms가 모든 롱노트를 단일 경로로 처리했다. 현재 구현은 다음 상태 절을 따른다.
> **역사 검증(2026-06-03):** 당시 CN/HCN 2-판정 최초 적용은 합성 픽스처 5종과 전체 887 통과로 확인했습니다. 이는 현재 Phase H의 3,040개 등록 테스트와 별개인 당시 수치입니다.

## 원본 위치
`<reference>/play/JudgeManager.java` (903줄). 윈도우 정의는 `bms/model` rule, 게이지는 `GrooveGauge`.

## LN 종류 (BMSModel)
- `#LNTYPE 1` = TYPE_LONGNOTE(레거시), `#LNTYPE 2` = MGQ(미사용).
- `#LNMODE` = 0(undefined→타입에 위임), 1=LN, 2=CN(charge), 3=HCN(hell charge). rbms `chart::to_model`이 `lnmode`→`LnKind`(0/1→Ln, 2→Cn, 3→Hcn)로 이미 매핑(테스트됨).
- IR `lntype` 인코딩(백엔드 contract `data-model.md`/`compatibility.md`) = **0=LN, 1=CN, 2=HCN** = `LnKind` 순서. (`ir_map::ir_lntype` 구현·테스트 완료)

## 판정 윈도우 (rule.getJudge)
- `nmjudge` = NoteType.NOTE (노트 머리).
- `cnendmjudge` = NoteType.LONGNOTE_END (키 LN/CN 종단).
- `smjudge` = NoteType.SCRATCH, `scnendmjudge` = NoteType.LONGSCRATCH_END (스크래치 LN 종단).
- `nreleasemargin` = rule.longnoteMargin, `sreleasemargin` = rule.longscratchMargin (LN 늦은 릴리스 유예).
- rbms는 key `ln_end`와 scratch `ln_scratch_end`를 분리하고, LN과 CN/HCN에 서로 다른 종단 로직을 적용합니다.

## 머리(press) 판정 — JudgeManager 436-471
- 공통: 대상 노트 추출 후 `nmjudge`로 `judge` 산출. `judgeVanish[judge]`(PG/GR/GD/BD는 true, POOR=false)면 "소실"로 처리.
- **LN(443-459):** `judgeVanish[judge]`면 `lnstartJudge=judge`, `lnstartDuration=dmtime`, `processing=pair`로 홀드 시작(레이저색=8). 아니면 즉시 확정.
- **CN/HCN(460-471):** `judgeVanish[judge]`면 `processing=pair`로 홀드 시작. 그 다음 **항상 `updateMicro(ln, judge, judgeVanish[judge])` 호출** — 즉 머리 판정을 즉시 기록.
- rbms matcher: plain LN 머리는 `holding=true`로 보류하고, CN/HCN은 머리 판정을 즉시 카운트합니다.

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

## HCN 연속 게이지 — JudgeManager 299-345 (완료)
- `passing` = 현재 통과 중인 HCN(머리 통과 시 set, 종단 통과 시 clear). `inclease` = 이번 프레임 눌림 여부(또는 autoplay/auto-hit).
- 눌림 유지: `mpassingcount += dt`; `> hcnmduration`마다 `gauge.update(1, 0.5)` (게이지 +0.5), `mpassingcount -= hcnmduration`.
- 안 눌림: `mpassingcount -= dt`; `< -hcnmduration`마다 `gauge.update(3, 0.5)` (게이지 -0.5, BAD 취급).
- ⇒ rbms `matcher::tick_hell_charges`가 `LaneHold.passing`과 `passing_us`로 이를 구현합니다. 200ms를 넘길 때 프레임당 최대 한 번 half-GREAT/half-BAD를 적용하고 나머지 시간을 유지하며, 판정 수·combo·EX·timing에는 영향을 주지 않습니다.

## rbms 구현 (✅ 완료)
1. **플럼빙:** `matcher.rs`의 `JNote`에 `ln: Option<LnKind>` 추가, `from_model`이 `LongStart{ln}`에서 운반. 공개 `from_pairs`/`new` 시그니처 유지(내부 `from_triples` 도입, pair-빌드는 `LnKind::Ln` 기본). `is_charge(ln)`로 CN/HCN 게이트.
2. **2-판정 모델:** CN/HCN은 **head(press 시 즉시 카운트) + end(release 시 별도 카운트)** = 2 판정/노트. `total_notes`·`count_playable_notes`가 CN/HCN을 2로 카운트(분모 일관). `press`: CN/HCN head면 `apply(judge)` 즉시. `release`: CN/HCN은 `final = end_judge`(ln_end 윈도우, head로 capping 안 함), LN은 `worse(head,end)` 유지. `update`: CN/HCN over-hold면 end 1판정, 미히트면 head+end 2 Miss; LN은 1.
3. **CN deferral/재홀드:** 이른 BAD/POOR release는 `release_us`/`end_judge`로 보류합니다. `longnoteMargin` 안의 re-grab은 보류를 취소하고, 만료한 `update`는 저장된 종단 판정으로 확정합니다. 늦은 release와 GOOD 이상 종단은 즉시 확정합니다.
4. **BSS/MSS:** scratch owner 방향을 기록합니다. BSS는 반대 방향 press로 charge note를 종단하고, 같은 방향 press는 보류 release를 re-grab합니다. 다른 방향 release와 end window 안의 mid-spin release는 무시합니다. MSS는 종단 뒤 owner를 해제해 다음 spin이 어느 방향으로도 시작할 수 있습니다.
5. **HCN 연속 게이지:** `tick_hell_charges`가 held/GOOD-or-better end에는 half-GREAT, unheld에는 half-BAD를 200ms마다 프레임당 한 번 적용합니다. remainder·통계 불변·결정성 테스트가 이를 고정합니다.
6. **검증:** 2026-06-03 당시 2-판정 픽스처 5종과 전체 887 통과를 기록했습니다. 현재 기준은 2026-09-16 Phase H `cargo test --workspace` exit 0, 등록 테스트 3,040개이며, deferral/re-grab, HCN tick, BSS/MSS까지 matcher 테스트가 포함됩니다.

## 남은 실제 제약
- 레퍼런스 구현의 **노트 카운트 분모**(CN을 passnote 2로 세는지)는 BMS 모델이 **외부 라이브러리**(레퍼런스 구현 소스 트리 밖)라 원본 대조 불가. rbms는 `JudgeManager`의 **2× `updateMicro`(head+end) 증거**에 근거해 CN=2로 카운트하고, `count_playable_notes`↔judge `total_notes`↔gauge 분모를 **내부 일관**되게 맞췄다. 실제 CN/HCN 차트가 corpus에 거의 없어 today 영향은 미미. 실차트 확보 시 회귀 추가 권장.
