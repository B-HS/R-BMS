# 완전성 비평 — 19개 보고서 vs 사용자 원 요청 9항목

작성 2026-09-09. 대상: `scratchpad/research/*.md` 19건(원보고서 9 + 검증 9 + 웹 1) 전량의 헤딩·요약·판정표·"미조사 범위"·"놓친 항목" 절 통독.
본 문서는 사실 재조사가 아니라 **커버리지 감사**다. 새 주장에는 file:line 근거를 붙였고, 보고서 인용은 보고서 파일명으로 표기한다.

---

## 0. 원 요청 9항목 커버리지 스코어카드

| # | 원 요청 | 담당 보고서 | 커버리지 | 판정 |
|---|---|---|---|---|
| 1 | 스킨 완전 커스터마이징 고도화 | skin-system, skin-render-inventory (+verify 2) | beatoraja 스펙 추출·현 하드코딩 인벤토리는 깊음. **채택 포맷 결정·마이그레이션 비교표 없음**, effort 추정이 상충 | 부분 |
| 2 | IIDX(~34) 기능 전수 구현 확인 | iidx-feature-gap, iidx-30-34-web, iidx-features-verify | IIDX 20~30대 공통 사양 대조는 유효. **31~34 1차 출처 확보 실패(remywiki 403)**, 웹 스니펫 기반 | 미달 |
| 3 | beatoraja 판정 동일성 + 세팅 조절 | judge-gauge, judge-parity-verify | 윈도우/게이지/설정축 대조 매우 깊음(18 findings, refuted 0) | 충족 |
| 4 | crate 분리·역할 적절성 | workspace-quality, crate-arch-verify | 의존 그래프·경계 결함 확정. **rbms-ir 1,649 LOC 미통독** | 대체로 충족 |
| 5 | 파일 길이·Rust 관용성 | workspace-quality | clippy 51·test 889·unsafe 0 실측, 분할안 3건 | 충족 |
| 6 | 메모리 누수 | memory-perf, memory-perf-verify | 13후보 전수 판정. **실측 0건**(전부 정적) | 부분 |
| 7 | UI/UX(프리징·프리뷰·디버그·사전옵션·타깃 그래프) | ux-flow, ux-flow-verify, feature-inventory | 21 findings, 디버그는 오히려 refuted. **IR/커스텀 서버 배선은 trait 표면까지만** | 부분 |
| 8 | 성능 | memory-perf, feature-inventory | 핫패스 구조 확정. **프레임타임/스캔시간 실측 0건** | 부분 |
| 9 | 사전 설정 화면 vs 인게임 옵션 | ux-flow §5 | 3방식 비교 + B(곡선택 오버레이) 권고로 명확히 답함 | 충족 |

**요청 대비 전면 누락 축**: 차트 포맷 커버리지(bmson/#SWITCH), 크로스플랫폼·배포, 데이터 스키마 버저닝, 테스트 baseline 실행. 아래 §1.

---

## 1. 어느 보고서도 다루지 않은 갭 (uncovered)

### G-01 bmson 미지원 · `#SWITCH/#CASE/#SKIP` 미지원 — 포맷 커버리지 축 자체가 조사되지 않음

19개 보고서 중 파서/포맷 지원 범위를 다룬 것이 없다(`bmson` 언급은 judge-gauge/judge-parity-verify의 judgerank 변환 경로 "미조사" 한 줄뿐).

- rbms 스캔 필터는 `bms|bme|bml|pms` 4종뿐 — `apps/rbms-player/src/main.rs:393`.
- rbms 제어 구문 리졸버는 `RANDOM|RONDAM|SETRANDOM|ENDRANDOM|IF|ELSEIF|ELSE|ENDIF` 만 — `crates/rbms-parser/src/control.rs:65,71,76,84,90,99,107`. `SWITCH/CASE/SKIP/DEF` 0건.
- beatoraja는 `.bmson` 을 스캔·디코드한다 — `song/SQLiteSongDatabaseAccessor.java:771,833,845-849,1321` (`BMSONDecoder`).

계획 영향: "beatoraja 패리티"를 목표로 하면 bmson은 **크레이트 단위 신규 작업(rbms-parser 확장 + rbms-model 매핑 + judgerank/total 변환)**이고 IIDX/BMS 씬의 실제 곡 자산 커버리지에 직결된다. 로드맵에서 완전히 빠져 있다.

### G-02 rbms-ir 1,649 LOC 미통독 — "커스텀 서버 기록 그래프"(요청 7)의 절반이 공백

- rbms 측: `crates/rbms-ir/src/{dto.rs 938, lib.rs 355, null.rs 227, http.rs 129}`. workspace-quality가 "trait 정의와 매니페스트만 확인, 본문 미통독"이라 명시했고 다른 보고서도 열지 않았다.
- ux-flow §6은 `chart_ranking/player_best/rivals` 가 trait에 존재하나 호출부 0이라는 것까지만 확정. **어떤 프로토콜에 맞춘 DTO인지, 인증·오프라인 큐·재시도·캐시 정책이 있는지 미확인.**
- beatoraja 측 대응은 16개 클래스(`ir/IRConnectionManager.java`, `RankingDataCache.java`, `IRWorkerMain.java`, `IsolatedIRConnection.java` 등) — 별도 워커/격리 연결·랭킹 캐시가 아키텍처로 존재한다.

계획 영향: 타깃/랭킹 그래프 설계(요청 7)는 DTO 스펙과 캐시·비동기 정책을 먼저 확정해야 하는데, 그 근거가 없다.

### G-03 실측 0건 — 요청 6·8의 우선순위 근거가 전부 정적 추론

memory-perf-verify가 "실측 미실시, 01/02의 실제 ms/MB 수치는 검증하지 않았다"고 명시. ux-flow-verify도 "실제 실행 계측(프리징 체감 시간, 표 fetch 실측)" 미조사. feature-inventory §5도 "시간 추정치는 코드 구조 기반 추정".
계획 영향: "곡선택 커서 이동 프리징"과 "폰트 캐시 무한 성장"의 severity 순위가 실제 체감과 어긋날 수 있고, 개선 후 회귀 판정 기준(baseline)도 없다. 사용자 개인 규칙 §20(장시간 파이프라인은 소크 테스트)에도 어긋난다.

### G-04 `cargo test` baseline 미실행 — 889개 테스트의 green 여부 불명

workspace-quality가 `#[test]` 889개·clippy 51경고까지 실측했으나 **테스트 실행 결과는 없다**(judge-gauge·skin-current-verify가 "cargo test 미수행" 명시). 대규모 리팩토링(크레이트 재편·스킨 엔진 신설) 계획의 전제 조건.

### G-05 크로스플랫폼·배포 축 전무

19개 보고서에 "플랫폼" 키워드 0건. 최근 커밋 `b5cff35 Fix Windows config path (HOME -> USERPROFILE fallback)` 가 시사하듯 플랫폼 분기가 실재하지만, cpal 백엔드(WASAPI/ALSA) 차이, wgpu 백엔드 선택, 키코드 매핑, 패키징/배포 경로가 조사되지 않았다.

### G-06 데이터 스키마 버저닝·마이그레이션 부재

`scores.ron` 원자적 저장(feature-inventory #4)은 지적됐지만, **설정/스코어/리플레이 포맷이 바뀔 때의 버전 필드·마이그레이션 정책**은 어느 보고서도 다루지 않았다. 스킨 엔진 신설·판정 설정 외부화·타깃 도입은 전부 저장 스키마를 건드리므로 계획 착수 전에 결정해야 한다.

### G-07 커스텀 판정 설정과 기록 유효성 정책 미조사

judge-gauge §4가 `customJudge`/`judgeWindowRate`/`lnmode` 미노출을 확정했으나, **커스텀 판정으로 낸 스코어를 로컬 베스트·IR에 어떻게 취급할지**(beatoraja의 제출 제한 여부 포함)는 양쪽 모두 미확인. 요청 3의 "세팅에서 조절 가능"을 구현하면 즉시 부딪히는 정책 결정이다.

### G-08 런처(별도 설정 앱) 도입 여부 판단 없음 — 요청 9의 나머지 절반

feature-inventory §1.10이 beatoraja `launcher/` 를 인벤토리했지만, ux-flow §5의 3방식 비교(A 전체화면 / B 곡선택 오버레이 / C 플레이 중)에는 **"게임 실행 전 별도 런처"**가 선택지로 들어 있지 않다. 사용자 요청 문구("게임 시작 전 설정 화면")는 런처를 가리킬 수도 있어 해석이 갈린다.

### G-09 라이브러리 규모 전제 부재

"곡DB(SQLite) 도입"(feature-inventory #1)과 "곡선택 O(N×M)"(memory-perf §2-1)의 severity는 곡 수에 좌우되는데, 실제 대상 라이브러리 규모(수백/수천/수만 곡)를 아무도 확인하지 않았다. 사용자에게 물어야 할 값이다.

---

## 2. 다뤄졌으나 얕은 지점 (shallow)

| id | 항목 | 얕은 이유 |
|---|---|---|
| S-01 | IIDX 31~34(요청 2) | iidx-30-34-web §5가 remywiki WebFetch 403으로 스니펫 인용에 그침. 34 ZINRAI은 로케테스트 사양이라 제품판과 다를 수 있다고 스스로 표기. "전수 확인"이라 부를 수 없다 |
| S-02 | 스킨 포맷 채택 결정(요청 1) | skin-system §3.3에 규모 추정은 있으나 **(a) beatoraja JSON 스킨 호환 (b) LR2 CSV 호환 (c) 자체 RON 확장** 3안의 비용/효익 비교표가 없다. LR2 CSV 106 커맨드·Lua 로더까지 조사해 놓고 채택안을 고르지 않았다 |
| S-03 | 판정 UI 노출 설계 | judge-gauge §4가 "무엇이 없는지"는 확정했으나, 설정 UI가 인덱스 결합(workspace-quality §1.4)이라 항목 추가 비용이 선형이 아니라는 점과 연결되지 않았다 |
| S-04 | 디버그(요청 7) | ux-flow-17이 refuted 되어 오버레이 존재가 확인됐고, 남은 갭은 "판정 ms-off 스캐터 / 활성 보이스 수" 2건으로 축소. 다만 **개발용 진단(오디오 지연 실측 훅, 프레임 히스토그램)** 은 아무도 설계하지 않았다 — G-03 실측의 전제 도구다 |
| S-05 | 오디오 지연 체인 | audio-input이 cpal 버퍼 512프레임/10.7ms를 **추정치**로 명시. 판정 정확성(요청 3)의 실효 상한을 결정하는 값인데 실측되지 않았다 |
| S-06 | 코스(단위인정) | 3개 보고서가 "미구현"만 확정. 게이지 지속·`gaugeAutoShift` 등 코스 전용 규칙(judge-gauge §4)과 묶인 설계 비용은 산정되지 않았다 |

---

## 3. 보고서 간 모순 (cross-report conflicts)

| id | 충돌 | 어느 쪽이 옳은가 |
|---|---|---|
| X-01 | **스킨 텍스처 effort 방향이 정반대.** skin-current-verify 01: "텍스처 전무는 과장, gpu.rs에 UV 샘플링 파이프라인 존재 → effort L보다 낮을 여지". skin-beatoraja-verify(skin-17): "진짜 병목은 `Renderer` trait 3메서드 → 원 finding보다 **더 심각**" | 둘 다 사실의 다른 면(백엔드에는 텍스처 있음 / trait에는 없음). **effort 재산정 필요** — 미해소 |
| X-02 | skin-system(skin-17)은 "rbms는 CPU 캔버스 기반이라 GPU 도입 여부 결정 필요"를 전제. skin-beatoraja-verify가 wgpu 주력 백엔드 존재로 반박 | 검증 쪽이 옳다. **skin-system §3.3 규모 추정이 잘못된 전제 위에 있을 수 있어 재검토 필요** |
| X-03 | SkinConfig 데드 필드 수: skin-render-inventory "19/23·20/24, 데드 15개" vs skin-current-verify "그 수치 아님, 데드 17개(dual_field/dual_gap 포함)" | 검증 쪽 채택 |
| X-04 | `PROCESS.md` stale 판정: skin-render-inventory §6 "PROCESS.md는 스킨 항목을 정확히 기술 — **stale 아님**" vs ux-flow-21/feature-inventory §4/judge-gauge §5 "문서 내부 모순·경로 오류·누락형 stale 다수 확정" | 영역별로 다르다. **문서를 일괄 stale로 처리하면 안 되고 섹션 단위로 갱신**해야 한다 |
| X-05 | panic 위험 등급: workspace-quality가 matcher `end_us.unwrap()`·mixer `sample.unwrap()` 을 high로 올렸으나 crate-arch-verify가 각각 **refuted / medium 하향**(도달 불가) | 검증 쪽 채택. workspace-quality의 "panic 위험 지점" 목록 severity 전반 재조정 필요 |
| X-06 | judge `SEVENKEY_LN_END`: judge-gauge 03 "값 오류" vs judge-parity-verify "partially(값 자체는 FIVEKEYS longnote 복제, 주석 stale)" | 검증 쪽 표현이 정확 |
| X-07 | severity 조정 다수: ux-flow-01 critical→high, ux-flow-19 low→info, ux-flow-17 low→(refuted), audio-input 04 effort 과대(master_gain 한 줄로 완화 가능) | **원보고서 severity를 그대로 우선순위에 쓰면 안 된다.** 검증 보고서의 조정판을 정본으로 삼아야 함 |

---

## 4. 계획 수립에 꼭 필요한 후속 조사 (한 줄 프롬프트)

| gap | 후속 조사 프롬프트 (1줄) |
|---|---|
| G-01 | "rbms-parser/rbms-model 이 지원하는 차트 포맷·제어구문을 beatoraja(bms-model, BMSONDecoder) 대비 전수 대조하고 bmson·#SWITCH 지원 추가 비용을 크레이트 단위로 산정하라" |
| G-02 | "crates/rbms-ir/src/{lib,dto,http,null}.rs 1,649줄 전량 통독 후 beatoraja ir/ 16클래스와 대조해 프로토콜·인증·랭킹 캐시·오프라인 큐 설계 격차를 표로 내라" |
| G-03 | "헤드리스/실기 실행으로 곡선택 커서 100회 이동·1곡 플레이 10회 반복 시 프레임타임과 RSS 추세를 1분 간격 기록해 memory-perf 01/02의 실제 수치를 확정하라" |
| G-04 | "cargo test --workspace 를 실행해 889개 테스트의 green/red baseline 과 실행시간을 기록하고 실패가 있으면 원인별로 분류하라" |
| G-05 | "Windows/Linux 에서의 설정 경로·cpal 백엔드·wgpu 백엔드·키코드 매핑 분기를 전수 확인하고 배포 패키징 현황을 정리하라" |
| G-06 | "settings.ron/scores.ron/replay 파일의 스키마 버전 필드 유무와 하위호환 처리를 확인하고 마이그레이션 정책안을 제시하라" |
| G-07 | "beatoraja 가 customJudge/judgeWindowRate 사용 시 스코어 저장·IR 제출을 제한하는지 소스로 확인하고 rbms 의 판정 설정 노출 시 기록 유효성 정책을 결정하라" |
| G-08 | "beatoraja launcher/ 의 설정 항목과 실행 전 GUI 구조를 통독해 rbms 에 별도 런처를 둘지, 인게임 설정으로 통합할지 비교안을 내라" |
| G-09 | "(사용자 확인) 대상 BMS 라이브러리 규모(곡 수·폴더 수·총 용량)를 확인해 곡DB 도입 우선순위를 확정하라" |
| S-01 | "IIDX 31~34 신규 플레이 옵션·판정 표시·그래프 사양을 bemaniwiki/공식 사이트 1차 출처로 재조사하라(remywiki 403 우회)" |
| S-02 | "스킨 포맷 3안(beatoraja JSON 호환 / LR2 CSV 호환 / 자체 RON 확장)의 구현 비용·기존 스킨 자산 활용도·유지보수 비용 비교표를 작성하라" |
| S-05 | "cpal 실제 출력 버퍼 프레임 수와 오디오 콜백 주기를 런타임 계측해 판정 정확도의 실효 상한을 확정하라" |
| X-01 | "Renderer trait 에 텍스처 드로우 프리미티브를 추가하는 최소 변경(gpu.rs 기존 BGA 파이프라인 재사용 + cpu.rs 대응)의 실제 diff 규모를 산정해 스킨 엔진 effort 를 재확정하라" |

---

## 5. 이 비평의 한계

- 19개 보고서의 **헤딩·요약·판정표·미조사/놓친 항목 절을 전수 통독**했고, 본문 세부는 skin·ux-flow·judge-gauge·workspace-quality·memory-perf·feature-inventory 의 인용 구간만 읽었다. 각 보고서 본문 표의 개별 라인 인용을 재검증하지는 않았다(그것은 verify 보고서들의 역할).
- 새로 제기한 G-01·G-02 는 이번에 직접 grep/ls 로 근거를 확보했다(`main.rs:393`, `control.rs:65-107`, `SQLiteSongDatabaseAccessor.java:771,845-849`, `crates/rbms-ir/src/*` LOC, `beatoraja/ir/` 16파일). 나머지 갭은 "보고서에 없음"을 근거로 한 부재 판정이다.
- 성능·메모리 실측은 이 비평에서도 수행하지 않았다(읽기 전용·시간 상한).
