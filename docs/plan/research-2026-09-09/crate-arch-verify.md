# 검증 보고서 — workspace-quality (크레이트 구조·Rust 관용성) findings 반증 시도

대상 보고서: `scratchpad/research/workspace-quality.md` (findings 01~20)
검증 방식: 인용된 file:line 을 직접 열람, 수치 재계산, clippy 실측(`cargo clippy --workspace --all-targets`), beatoraja 원본 파일 존재 확인.
검증 시각 기준 커밋: `c6f0885` (branch dev, clean)

## 1. 요약 판정표

| id | 판정 | 한 줄 근거 |
|----|------|-----------|
| 01 | confirmed | `Renderer` 는 정확히 `size/clear/fill_rect` 3개 (render/src/lib.rs:61-65). 텍스트조차 1px 스캔라인 `fill_rect` 로 그림 |
| 02 | confirmed | `SkinConfig` pub 필드 정확히 40개, 단일 RON 로드 |
| 03 | partially | 필드 수는 93개(약 100 아님). 나머지 주장은 정확 |
| 04 | confirmed | frame() 373-783(410줄), load() 9-149, enter_result() 198-372 |
| 05 | confirmed | main.rs:949 부터 Stage 8분기 인라인, 기존 `*_input` 은 4개뿐 |
| 06 | partially | const/serde 부재는 사실. 단 `note_for_mode`/`ln_end_for_mode` 로 모드별 테이블 선택은 이미 존재 |
| 07 | confirmed | PlayerConfig 28필드 / PlaySettings 24필드 + 수동 양방향 매핑 (수치 정확) |
| 08 | confirmed | 상수 4개 + SETTING_TABS 6탭 하드코딩 인덱스, 인용 라인 정확 |
| 09 | confirmed | `#xxx02:nan` → `parse::<f64>()` 가 NaN 허용 → sort_by unwrap 패닉 |
| 10 | partially | unwrap 은 실재하나 `active=true` 는 항상 `sample=Some` 과 동시 설정 → 현재 도달 불가 |
| 11 | **refuted** | matcher.rs:238 `end_us.is_some()` 가드가 `holding=true` 를 감싸므로 264/293 unwrap 은 도달 불가 |
| 12 | confirmed | ir_map.rs 는 winit 의존 0, rbms-ir 은 rbms-* 의존 0 → 이관 가능 |
| 13 | confirmed | scores/replay/scan/tablesrc 전부 bin 타깃에 있고 lib 타깃 자체가 없음 |
| 14 | confirmed | `Result<_, String>` 정확히 7곳(보고서 8곳은 1건 과다), thiserror/anyhow 의존 0 |
| 15 | confirmed | theme.rs:164-166, font.rs:159-161 thread_local RefCell |
| 16 | confirmed | lib/bin 51건 실측 일치. CI 의 clippy 는 `continue-on-error: true` |
| 17 | confirmed | serde `"1"` vs `"1.0.228"` 혼재, workspace.dependencies 미등록 |
| 18 | confirmed | 인용한 cfg(test) 시작 라인 전부 ±1 이내 일치 |
| 19 | partially | render 는 `rbms_chart::scroll` 을 **본문에서** 실제 사용(playfield.rs:1) → 단순 제거 불가 |
| 20 | confirmed | play/lib.rs:87 `pub judge: JudgeEngine`, 다른 필드는 private + 접근자 존재 |

## 2. 반증에 성공한 항목

### 11 — dangling LongStart 판정 패닉 (refuted)

보고서는 `matcher.rs:264` / `:293` 의 `end_us.unwrap()` 이 "end 없는 LongStart 차트에서 패닉"한다고 주장한다. 실제 코드는 두 unwrap 모두 `holding == true` 인 노트에서만 실행되고, `holding = true` 는 다음 가드 안에서만 세팅된다.

```
crates/rbms-judge/src/matcher.rs:238  if self.lanes[lane].notes[idx].end_us.is_some() {
crates/rbms-judge/src/matcher.rs:241      note.holding = true;
```

- `:264` 는 `l.notes.iter().position(|n| n.holding)?` (matcher.rs:262) 로 걸러진 노트에만 접근한다.
- `:293` 은 `if n.holding {` (matcher.rs:292) 블록 안이다.

따라서 `end_us == None` 인 LongStart 는 애초에 `holding` 상태가 되지 못하고, 두 unwrap 은 현재 코드에서 도달 불가능하다. rbms-play 의 `dangling_longstart_*` 테스트와 "견고성이 어긋나 있다"는 대비도 성립하지 않는다. 다만 `end_us: Option<i64>` + `holding: bool` 이라는 이중 표현이 불변식에만 의존한다는 지적 자체는 유효하므로, 권고(타입 레벨로 `LongStart{end_us: i64}`)는 유지할 가치가 있으나 severity 는 high → low(설계 위생) 로 내려야 한다.

## 3. 부분 정정 항목

### 03 — App 필드 수
`sed -n '609,733p' | grep -cE '^    [a-z_]+:'` = **93개**. "약 100 필드"는 과장은 아니나 정확히는 93. `Stage` 가 데이터 없는 enum(main.rs:479-492)이고 필드가 평평하다는 본론은 그대로 성립.

### 06 — 판정 설정 데이터 모델
`JudgeWindows` 가 const 4개(windows.rs:19-52)이고 rbms-judge 의 Cargo.toml 에 serde 가 없다는 것은 사실이다. 그러나 보고서가 "조절 손잡이가 judgerank 스케일 하나뿐"이라고 한 것은 절반만 맞다. `note_for_mode`/`ln_end_for_mode`(windows.rs:57-72) 가 이미 모드별 테이블 선택 축을 제공하며, 주석에 "새 키모드는 여기 한 줄만 추가"라는 데이터화 원칙이 명시돼 있다. 즉 **모드 축은 있고 사용자 설정 축(외부화 + UI 노출)이 없다**로 정정한다. 또 POPN_NOTE/POPN_LN_END 는 현재 7K 값을 그대로 복제한 placeholder 라고 코드 주석이 명시하고 있어(windows.rs:36-38, 46-48), 여기가 실제로 미검증 구간이다.

### 10 — 믹서 RT unwrap
`mixer.rs:133 v.sample.as_ref().unwrap()` 은 실재한다. 다만 `active: true` 는 오직 `mixer.rs:86-96` 의 `self.voices[slot] = Voice { sample: Some(sample), ..., active: true }` 한 곳에서만 설정되고, `stop`(:101-103)과 소진 처리(:145)는 active 를 끄기만 한다. 즉 **현재 도달 가능한 패닉 경로가 없다**. "어긋나면 패닉"이라는 조건부 서술 자체는 맞으나 severity high 는 과대이고, "곡 전체 무음" 결과 서술은 추정이다. medium/S 로 조정하고 근거는 "RT 스레드에 unwrap 이 남아 있다(위생)"로 축소하는 것이 정확하다.

### 19 — rbms-render → rbms-chart 의존
보고서 스스로 "playfield.rs 본문 미통독"이라 밝혔는데, 실제로 통독하면 **본문 1행에서 사용 중**이다.

```
crates/rbms-render/src/playfield.rs:1  use rbms_chart::scroll::{constant_offsets, visible_offsets};
```

(200-201 행의 `closed_form_offset`/`to_model` 은 테스트 전용.) 따라서 "뷰 타입만 필요하므로 model 의존만으로 충분할 가능성이 높다"는 추정은 틀렸다. 의존을 끊으려면 스크롤 오프셋 계산 결과를 인자로 받도록 API 를 바꾸는 실제 리팩터가 필요하다 — 즉 S(하루 미만) 가 아니라 M 에 가깝고, 우선순위도 낮다.

## 4. 확인 과정에서 확정한 수치

| 항목 | 보고서 | 실측 |
|------|--------|------|
| clippy lib/bin 경고 | 51 (parser2/chart4/render7/judge5/play4/table1/player28) | 동일 (요약 라인 실측 일치) |
| clippy 전체(--all-targets, 중복 포함 출력) | 미기재 | 101 warning 라인 |
| App 필드 | 약 100 | 93 |
| PlayerConfig / PlaySettings | 28 / 24 | 28 / 24 (일치) |
| SkinConfig pub 필드 | 약 40 | 40 |
| `Result<_, String>` | 8곳 | 7곳 (table 4, render 1, tablesrc 1, replay 1) |
| thiserror/anyhow 의존 | 0건 | 0건 (전 Cargo.toml grep) |
| unsafe 블록 | 미기재 | 0건 (crates + apps 전체) |
| app_play.rs frame() | 373-783 | 동일, 파일 796줄 |

## 5. 보고서가 놓친 항목 (missed)

1. **CI 의 lint 잡이 `continue-on-error: true` 로 이미 무력화돼 있다** — finding 16 은 "게이트 없음"이라 했지만 실제로는 잡이 존재하는데 실패를 무시하도록 명시돼 있고(`.github/workflows/ci.yml` lint 잡의 rustfmt·clippy 스텝), 게다가 `cargo clippy --workspace` 로 `--all-targets` 가 빠져 테스트 타깃 경고 34건은 CI 에 아예 노출되지 않는다. 수정은 플래그 2개 제거/추가 수준이라 effort 는 finding 16 보다 훨씬 작다.
2. **`apps/rbms-player` 에 lib 타깃이 없다** (`apps/rbms-player/Cargo.toml` 은 `[[bin]]` 만 선언). 그래서 `tests/autoplay_preview.rs` 는 앱 내부 모듈(ir_map/scores/replay/format/keyconfig)에 접근하지 못하고 라이브러리 크레이트만 조합해 테스트한다. finding 12·13 의 "재사용 불가"보다 더 강한 사실이며, `[lib]` 하나 추가만으로도 즉시 통합 테스트 가능해진다.
3. **텍스트 렌더가 글리프를 1px 높이 `fill_rect` 런으로 분해해 매 프레임 인스턴스 버퍼에 밀어 넣는다** (`crates/rbms-render/src/font.rs:123-128`). finding 01 의 "텍스처 부재"가 낳는 구체적 성능 비용이며, `draw_textured_quad` 도입의 정량적 근거가 된다(현재는 글리프 캐시가 런 단위 rect 리스트).
4. **`[workspace.lints]` 테이블이 없다** (`Cargo.toml` 및 전 크레이트 grep 결과 0건). finding 16·17 을 근본에서 해결하는 Rust 2021+ 관용 수단으로, `[workspace.lints.clippy]` 한 블록 + 각 크레이트 `lints.workspace = true` 로 워크스페이스 전역 lint 정책을 코드로 고정할 수 있다.
5. **`unsafe` 가 전 워크스페이스 0건인데 `#![forbid(unsafe_code)]` 선언이 없다** (grep `unsafe ` → crates/apps 0건). 현 상태를 회귀 없이 고정하는 무비용 조치.
6. **`rbms-ir` 과 `rbms-table` 이 각각 reqwest(blocking + rustls)를 따로 선언하고 HTTP 클라이언트 구성도 중복**한다 (`crates/rbms-table/src/lib.rs:140 build_client`, `crates/rbms-ir/Cargo.toml:9`). finding 17 은 버전 드리프트만 짚었으나, 타임아웃·UA·TLS 정책이 두 곳에서 갈릴 수 있는 구조적 중복이다.
7. **`apps/rbms-cli` 가 39줄이고 테스트가 0건**이다 (`apps/rbms-cli/src/main.rs`). 보고서는 이 크레이트를 전혀 다루지 않았다. finding 13 이 제안하는 rbms-store/rbms-library 이관의 첫 소비자로 쓰기 좋은 위치다.
8. **`rbms-chart` 의 measure rate 검증 부재는 finding 09 의 정렬 패닉보다 상류 문제**다 (`crates/rbms-parser/src/lib.rs:191-193` 이 `parse::<f64>()` 결과를 그대로 `measure.rate` 에 저장, `crates/rbms-chart/src/lib.rs:110-111` 이 누적). `total_cmp` 로 바꿔도 NaN 소절은 그대로 모델에 남아 이후 시간 계산(`lib.rs:301`)을 오염시키므로, 파서 단계 거부가 본질적 수정이다.

## 6. 미조사 범위

- `crates/rbms-render/src/playfield.rs` 본문 전체(스크롤 계산이 chart 에 얼마나 깊게 결합돼 있는지 정량화 못 함).
- `apps/rbms-player/src/app_input.rs`, `app_select.rs` 전문 통독(라인 인용 지점만 확인).
- beatoraja 측은 파일 존재·이름만 확인했고 내부 구현(SkinLoader 의 실제 오브젝트 스키마, JudgeAlgorithm 4종의 알고리즘)은 통독하지 않음.
- finding 09 의 NaN 패닉은 코드 경로 추적으로 확정했고 실제 크래시 재현 실행은 하지 않음(레포 수정 금지 제약).
