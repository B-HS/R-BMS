# G9 bmson — G8 배선 지시

> 대상: 스펙 `docs/plan/2026-09-09-phase-g-spec.md` §2 보류항목·§9.2 G9 행, 결정 10(`docs/acknowledge/2026-09-09-enhancement-decisions.md`) = **bmson 은 Phase G 범위 안**.
> 소유 파일(G9 가 이미 작성 완료): `crates/rbms-parser/src/bmson.rs`, `crates/rbms-parser/src/bmson/{convert.rs,tests.rs}`, `crates/rbms-parser/tests/bmson/*.bmson`.
> 아래 패치는 전부 **G9 소유가 아닌 파일**에 대한 지시다. G9 는 이 파일들을 한 줄도 건드리지 않았다.
> 검증 상태: `cargo test -p rbms-parser` 196 passed / 0 failed, `cargo clippy -p rbms-parser --all-targets -- -D warnings` 0, `cargo fmt -p rbms-parser -- --check` clean.

## 0. 새로 쓸 수 있는 것

| 심볼 | 경로 | 용도 |
|---|---|---|
| `parse(&[u8]) -> Result<BmsonChart, BmsonError>` | `rbms_parser::bmson` | 문서 디코드 + 파일 md5/sha256 |
| `BmsonChart::{mode, pulses_per_measure, rank, judgerank_percent, total_for_notes, to_model, to_model_in_mode}` | 〃 | 모델 매핑 |
| `BmsonChart` 필드 `{version, info, lines, bpm_events, stop_events, scroll_events, sound_channels, key_channels, mine_channels, bga, md5, sha256}` | 〃 | 원문 접근 |
| `BmsonInfo, BmsonNote, SoundChannel, KeyChannel, MineChannel, MineNote, BpmEvent, StopEvent, ScrollEvent, BarLine, Bga, BgaHeader, BgaEvent, BmsonError` | 〃 | 타입 |
| `EXTENSION`(= `"bmson"`), `DEFAULT_RESOLUTION`(= 240), `MODE_HINTS` | 〃 | 상수 |

`to_model()` 이 돌려주는 것은 BMS 경로의 `rbms_chart::to_model` 과 **같은 `rbms_model::Model`** 이다. 따라서 `shuffle::apply` · `resolve_long_note_flavour` · `contains_undefined_long_note` · `count_playable_notes` · `note_density` · `keysound_jobs` 는 그대로 쓰면 된다. **bmson 은 `BmsSource` 를 거치지 않는다**(§6 참조).

---

## 1. 스캔 확장자 필터

**파일**: `crates/rbms-library/src/lib.rs`
**위치**: `pub fn is_chart(p: &Path) -> bool` 전체 교체.

```rust
/// Whether a path carries one of the chart extensions the scanner reads: the BMS family, and the
/// JSON bmson format the parser decodes on its own path.
pub fn is_chart(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("bms" | "bme" | "bml" | "pms" | "bmson"))
}
```

**근거**: 스펙 §9.2 G9 행이 `is_chart` 확장을 G8 에 위임. 이 줄이 없으면 bmson 파일은 **스캔 자체에 잡히지 않아** 나머지 배선이 전부 죽은 코드가 된다. `"bmson"` 리터럴은 `rbms_parser::bmson::EXTENSION` 과 같은 값이며, 그 동치는 `bmson::tests::the_extension_is_the_one_the_scanner_filters_on` 이 고정한다.

---

## 2. 증분 스캐너 — 파일 하나를 행으로 (필수)

**파일**: `crates/rbms-library/src/scan.rs` (G1 소유 파일 — 아래 "소유권 주의" 참조)
**위치 ①**: `fn parse_chart(job: &Found) -> Option<(SongRow, Option<DetailRow>)>` 의 `let bytes = std::fs::read(&job.source).ok()?;` **바로 아래**에 분기 3줄 삽입.

```rust
    if is_bmson(&job.source) {
        return parse_bmson_chart(job, &bytes);
    }
```

**위치 ②**: `fn parse_chart` **바로 아래**에 함수 두 개 추가.

```rust
/// Read one bmson chart into the row and the detail the database stores for it. bmson states its
/// own mode instead of leaving it to be detected, and its TOTAL is already absolute on the model.
fn parse_bmson_chart(job: &Found, bytes: &[u8]) -> Option<(SongRow, Option<DetailRow>)> {
    let chart = rbms_parser::bmson::parse(bytes).ok()?;
    let mode = chart.mode();
    let model = chart.to_model();

    let duration_us = model.timelines.last().map(|tl| tl.time_us).unwrap_or(0);
    let length_ms = duration_us / US_PER_MS;
    let (bpm_min, bpm_max) = bpm_range(&model);
    let density = note_density(&model, model.meta.total);
    let title = if model.meta.title.is_empty() {
        job.source.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string()
    } else {
        model.meta.title.clone()
    };

    let row = SongRow {
        path: job.path.clone(),
        md5: model.md5.clone(),
        sha256: model.sha256.clone(),
        title,
        subtitle: model.meta.subtitle.clone(),
        artist: model.meta.artist.clone(),
        subartist: model.meta.subartist.clone(),
        genre: model.meta.genre.clone(),
        maker: String::new(),
        level: model.meta.play_level.clone(),
        difficulty: model.meta.difficulty,
        mode: mode_id(mode),
        judge: model.meta.rank,
        total: model.meta.total,
        init_bpm: model.init_bpm,
        min_bpm: bpm_min as i32,
        max_bpm: bpm_max as i32,
        length_ms,
        notes: count_playable_notes(&model) as i32,
        long_notes: long_note_count(&model) as i32,
        stagefile: chart.info.eyecatch_image.clone(),
        banner: chart.info.banner_image.clone(),
        backbmp: chart.info.back_image.clone(),
        preview: chart.info.preview_music.clone(),
        folder: job.folder.clone(),
        favorite: 0,
        date: job.mtime,
        adddate: crate::songdb::now_secs(),
        size: job.size,
        feature: feature_bits(&model, false),
        content: content_bits(&model, job.has_text, !chart.info.preview_music.is_empty(), length_ms),
    };
    let detail = DetailRow { duration_us, peak_density: density.peak, avg_density: density.avg, end_density: density.end, density: density.bins };
    Some((row, Some(detail)))
}

/// Whether a path is a bmson chart, which is decoded from JSON instead of the BMS line grammar.
fn is_bmson(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(rbms_parser::bmson::EXTENSION)).unwrap_or(false)
}
```

**근거**: 스캐너가 bmson 바이트를 BMS 문법으로 읽으면 헤더가 하나도 안 잡혀 제목·BPM·노트수가 전부 빈 행이 된다. 필드 대응은 레퍼런스 `BMSONDecoder` 가 `BMSModel` 에 넣는 것과 같다 — `maker` 는 bmson 에 대응 필드가 없어 빈 문자열, `difficulty` 는 `ModelMeta::difficulty`(bmson 은 항상 0), `feature_bits` 의 control-flow 인자는 bmson 에 `#RANDOM` 계열이 없으므로 `false`, `total` 은 **이미 절대값**(§7-1)이라 `note_density` 에도 그대로 넘긴다.

**소유권 주의**: 소유권 표는 `scan.rs` 를 G1 에, `is_chart` 가 있는 `lib.rs` 를 G8 에 두었다. 이 분기는 **W2 의 어느 갈래도 소유하지 않는 유일한 bmson 공백**이므로 G8 통합 커밋에서 넣는다. G1 이 `parse_chart` 를 리네임했으면 "바이트를 `SongRow` 로 바꾸는 단 하나의 함수" 를 앵커로 삼는다.

---

## 3. 구 스캔 경로와 상세 계산

`crates/rbms-library/src/lib.rs` 의 `scan_folder`/`compute_chart_detail` 을 G8 이 **삭제**(G1 의 `scan_into` 로 대체)한다면 이 절은 건너뛴다. 남긴다면 아래 두 패치가 필요하다.

**파일**: `crates/rbms-library/src/lib.rs`
**위치 ①**: `pub fn scan_folder` 안, `} else if is_chart(&p) && let Ok(bytes) = std::fs::read(&p) {` 블록 전체. bmson 이면 갈라서 `SongEntry` 를 만든다(§2 의 `is_bmson` 과 같은 헬퍼를 이 파일에도 둔다).

```rust
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
                let entry = if is_bmson(&p) {
                    let Ok(chart) = rbms_parser::bmson::parse(&bytes) else {
                        continue;
                    };
                    let model = chart.to_model();
                    SongEntry {
                        title: if model.meta.title.is_empty() { name } else { model.meta.title.clone() },
                        subtitle: model.meta.subtitle.clone(),
                        artist: model.meta.artist.clone(),
                        genre: model.meta.genre.clone(),
                        maker: String::new(),
                        level: model.meta.play_level.clone(),
                        difficulty: model.meta.difficulty,
                        init_bpm: model.init_bpm,
                        rank: model.meta.rank,
                        total: model.meta.total,
                        mode: chart.mode(),
                        md5: model.md5.clone(),
                        stagefile: chart.info.eyecatch_image.clone(),
                        banner: chart.info.banner_image.clone(),
                        preview: chart.info.preview_music.clone(),
                        path: p,
                    }
                } else {
```

`else` 가지에는 **지금 있는 BMS 블록을 그대로** 넣는다(`title` 계산만 위에서 만든 `name` 을 쓰도록 고친다). 블록을 닫고 나서:

```rust
                };
                out.push(entry);
                count.fetch_add(1, Ordering::Relaxed);
```

**위치 ②**: `pub fn compute_chart_detail` 의 첫 세 줄(`let src = ...` ~ `let model = to_model(&src, mode);`)을 형식별로 가른다. bmson 이면 `let model = chart.to_model();` 과 `let total_value = model.meta.total;` 를 쓰고, 이후 밀도·BPM·길이 계산은 **한 줄도 바뀌지 않는다**.

**근거**: 두 함수 모두 `is_chart` 가 통과시킨 파일을 BMS 로 단정한다. `total_value` 만 주의하면 나머지는 포맷 무관이다.

---

## 4. 플레이 로드 경로

**파일**: `apps/rbms-player/src/app_play.rs`
**위치 ①**: `impl` 밖(파일 하단의 자유 함수들 옆)에 디코더를 추가한다.

```rust
/// What the load path needs from a chart file, whatever format it is written in.
struct DecodedChart {
    mode: rbms_model::Mode,
    model: rbms_model::Model,
    /// The file hash a replay is checked against.
    md5: String,
    /// The long-note mode the chart states, in `#LNMODE` codes.
    lnmode: i32,
    /// The bmson document, kept only because bmson states TOTAL as a percentage of the mode default
    /// and that percentage is not final until the long-note flavour is resolved.
    bmson: Option<rbms_parser::bmson::BmsonChart>,
}

/// Decode a chart file of either format into the play model. `None` when the bytes are not a chart
/// this build can read, which the caller reports instead of playing silence.
fn decode_chart(bytes: &[u8], path: &str) -> Option<DecodedChart> {
    let is_bmson = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(rbms_parser::bmson::EXTENSION))
        .unwrap_or(false);
    if is_bmson {
        let chart = rbms_parser::bmson::parse(bytes).ok()?;
        let mode = chart.mode();
        let model = chart.to_model();
        let (md5, lnmode) = (chart.md5.clone(), chart.info.ln_type as i32);
        return Some(DecodedChart { mode, model, md5, lnmode, bmson: Some(chart) });
    }
    let src = rbms_parser::parse_with(bytes, Default::default());
    let mode = rbms_chart::detect_mode(&src, path);
    let model = to_model(&src, mode);
    Some(DecodedChart { mode, model, md5: src.md5.clone(), lnmode: src.headers.lnmode, bmson: None })
}
```

**위치 ②**: `pub(crate) fn load(&mut self) -> Option<LoadedChart>` 안, `let src = rbms_parser::parse_with(&bytes, Default::default());` 과 그 다음 줄 `let mode = rbms_chart::detect_mode(&src, &self.chart_path);` **두 줄을 교체**한다.

```rust
        let decoded = match decode_chart(&bytes, &self.chart_path) {
            Some(d) => d,
            None => {
                notify(Level::Error, format!("chart could not be read: {}", self.chart_path));
                return None;
            }
        };
        let mode = decoded.mode;
```

**위치 ③**: 같은 함수의 리플레이 md5 대조 두 곳 `src.md5` → `decoded.md5`.

**위치 ④**: `let mut model = to_model(&src, mode);` → `let mut model = decoded.model;`

**위치 ⑤**: `let lntype = run_lntype(src.headers.lnmode, judge_setup.ln_mode);` → `let lntype = run_lntype(decoded.lnmode, judge_setup.ln_mode);`

**위치 ⑥**: `rbms_chart::shuffle::apply(&mut model, random, seed);` 와 `if model.meta.total <= 0.0 {` **사이**에 삽입.

```rust
        if let Some(chart) = &decoded.bmson {
            model.meta.total = chart.total_for_notes(&mode, rbms_chart::count_playable_notes(&model));
        }
```

**근거**: ①~⑤ 없이는 bmson 파일이 BMS 로 읽혀 노트 0개 차트가 되고, 리플레이 md5 대조와 `#LNMODE` 유래 `lntype`(리플레이·IR 에 그대로 실린다)이 빈 값이 된다. ⑥ 은 bmson TOTAL 이 **퍼센트**라서 필요하다 — CN/HCN 은 롱노트 꼬리도 판정 대상이라, LN MODE 해석 전 노트 수로 계산한 값과 해석 후 값이 다르다. 이 재계산이 BMS 경로의 `default_total_for_mode(...)` 폴백과 같은 타이밍(셔플 뒤)에 놓인다. `decoded.model` 이 `model` 로 부분 이동한 뒤에도 `decoded.lnmode`·`decoded.bmson` 접근은 유효하다.

**주의**: `load()` 의 나머지(키음 디코드 `keysound_jobs(&model.wavmap, &dir)`, `chart_gain(model.meta.volwav)`, BGA, 스킨)는 **한 줄도 바꾸지 않는다**. bmson 의 `sound_channels[].name`·`bga_header[].name` 은 BMS 의 `#WAVxx`/`#BMPxx` 와 같은 "차트 폴더 기준 상대 파일명" 이라 `resolve_file` 의 확장자 폴백이 그대로 맞는다. `volwav` 는 bmson 에 없어 항상 unity(100)로 들어간다.

---

## 5. CLI `chart` 명령

**파일**: `apps/rbms-cli/src/main.rs`
**위치**: `let src = rbms_parser::parse(&bytes);` ~ `let model = rbms_chart::to_model(&src, mode);` 세 줄. bmson 이면 아래로 가른다.

```rust
    let is_bmson = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(rbms_parser::bmson::EXTENSION))
        .unwrap_or(false);
    let (mode, model, wav_defs, measures) = if is_bmson {
        let chart = match rbms_parser::bmson::parse(&bytes) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bmson parse error: {e}");
                return ExitCode::FAILURE;
            }
        };
        let (mode, model) = (chart.mode(), chart.to_model());
        let wav_defs = model.wavmap.len();
        (mode, model, wav_defs, 0)
    } else {
        let src = rbms_parser::parse(&bytes);
        let mode = rbms_chart::detect_mode(&src, path);
        let model = rbms_chart::to_model(&src, mode);
        let (wav_defs, measures) = (src.wav.len(), src.measures.len());
        (mode, model, wav_defs, measures)
    };
```
이후 출력부의 `src.wav.len()` → `wav_defs`, `src.measures.len()` → `measures` 로 바꾼다.

**근거**: `rbms-cli chart <파일>` 은 스캔·플레이 배선을 거치지 않는 독립 진입점이라 별도로 갈라야 한다. bmson 에는 마디 개념이 없어 `measures` 는 0 이다(원한다면 그 줄을 bmson 일 때 감춘다).

---

## 6. `rbms_parser::parse` 에는 bmson 분기를 넣지 않는다

색인(`README.md`)이 이 갈래의 주제를 "`rbms_parser::parse` 디스패치" 로 적었지만, **그 함수에는 넣을 수 없다**.

- `parse` 의 반환형 `BmsSource` 는 `measures: BTreeMap<u32, Measure>` — 마디/채널 격자다. bmson 은 `resolution` 분해능의 **펄스 격자**이고, 마디 경계(`lines`)는 임의 위치에 놓일 수 있어 마디 격자로 되돌릴 수 없다(연속 노트 슬라이스 `c`·`up` 꼬리 키음도 표현할 자리가 없다).
- `BmsSource` → `Model` 변환은 `rbms-chart` 에 있고 `rbms-chart` 가 `rbms-parser` 를 의존한다. 파서 안에서 `Model` 까지 만드는 공용 진입점을 두면 의존 방향이 뒤집힌다. 그래서 bmson 만 `rbms_model::Model` 을 직접 만든다(같은 크레이트에서 `rbms-model` 만 의존).
- 결론: 디스패치는 **호출부**(§2·§3·§4·§5)에서 확장자로 가른다. `crates/rbms-parser/src/lib.rs` 는 G0 이 넣은 `pub mod bmson;` 한 줄 외에 **바뀔 것이 없다**.

---

## 7. 알려진 갭 (Phase G 안에서는 못 막는 것)

| # | 갭 | 원인 | 지금 동작 | 필요한 것 |
|---|---|---|---|---|
| 1 | judgerank 퍼센트 | `ModelMeta` 에 judgerank-type 축이 없다. `rank`(#RANK 표) 와 `defexrank`(NORMAL 75 에 곱하는 퍼센트) 둘 뿐이고, bmson 의 `judge_rank` 는 **곱하지 않고 그대로 쓰는 퍼센트**다(`BMSPlayerRule.java:62-64` `BMSON_JUDGERANK` arm). `defexrank` 로 우회하면 `trunc(d*75/100)` 때문에 값에 따라 1 씩 어긋난다(예: 11 → 10) | `judge_rank < 5` 는 `#RANK` 인덱스로 **정확히** 이식(레퍼런스도 그 구간은 `BMS_RANK` 로 취급), 음수는 NORMAL(2), 5 이상은 **100 % 인 `#RANK 3`** 으로 고정. bmson 의 기본값 100 은 이 경로에서 정확하다 | `rbms-model`+`rbms-judge` 에 judgerank-type(또는 `judgerank_percent: Option<i32>`) 추가 후 `JudgeWindowRule::judgerank_for` 확장. **두 크레이트는 Phase G 전 갈래 read-only** 라 별도 Phase. 값은 `BmsonChart::judgerank_percent()` 로 이미 꺼낼 수 있다 |
| 2 | 키음 슬라이스 재생 | bmson 의 `c`(continuation) 노트는 한 파일의 구간 `[start_us, start_us+duration_us)` 만 울린다. `Note::start_us`/`duration_us` 에 정확히 채워 넣었지만 **오디오 엔진이 아직 읽지 않는다**(BMS 는 항상 0 이라 쓰인 적이 없다) | 슬라이스 노트가 파일을 처음부터 끝까지 재생한다 | `rbms-audio` 의 샘플 재생에 오프셋·길이 인자 추가(역시 read-only 트리) |
| 3 | 레이어 이벤트 조건 | bmson `layer_events` 의 `condition`(play/miss)·`interval`·`bga_sequence` 는 레퍼런스가 `Layer` 객체로 다루지만 `TimeLine` 에는 `layer: i32` 한 칸뿐 | `layer_events` 는 그림 인덱스만 반영, `poor_events` 와 시퀀스는 버린다 | BGA 레이어 모델 확장(범위 밖) |
| 4 | `popn-5k` / `keyboard-24k-double` | `rbms_model::Mode` 에 두 모드가 없다 | `mode_hint` 를 못 찾은 것과 같이 BEAT_7K 로 폴백(레퍼런스의 unknown-hint 동작과 동일) | `Mode` 상수 2개 추가(read-only 트리) |
| 5 | 음수 펄스 | 레퍼런스는 검사 없이 음수 시각을 만든다 | `BmsonError::InvalidField { field: "y" }` 로 **파일 전체를 거부** | 없음(의도된 발산, §8 에 기록) |

---

## 8. 문서 갱신 (G8)

`docs/acknowledge/reference-divergences.md` 에 아래를 추가한다.

- **bmson judgerank**: `judge_rank >= 5` 는 레퍼런스가 판정폭 퍼센트로 그대로 쓰지만 rbms 는 100 % 로 고정한다(§7-1). 근거·해소 조건 포함.
- **bmson 음수 펄스**: 레퍼런스는 통과시키고 rbms 는 거부한다(§7-5).
- **bmson `poor_events`/`bga_sequence`**: 모델에 자리가 없어 버린다(§7-3).
- **bmson `scroll`**: 레퍼런스는 이벤트가 놓인 타임라인에만 값을 넣지만, rbms 는 BMS 경로(`rbms_chart::assign_times`)와 같이 **다음 이벤트까지 이어간다**. 같은 렌더러가 두 포맷을 같은 규칙으로 그리기 위한 의도적 통일.

`docs/architecture.md` 의 파서 절에 "bmson 은 `BmsSource` 를 거치지 않고 `rbms_model::Model` 을 직접 만든다" 한 줄을 넣는다(§6).

---

## 9. 검증

배선 후 G8 이 확인할 것.

1. `cargo test -p rbms-parser` — bmson 단위 테스트 55건 포함 196 passed 유지.
2. `cargo test -p rbms-library` — §2 배선 후 bmson 픽스처를 tempdir 에 넣고 스캔하면 행이 1개 늘고 `title`/`notes`/`length_ms` 가 채워지는지(테스트 추가 권장. 픽스처는 `crates/rbms-parser/tests/bmson/*.bmson` 를 복사해 쓰면 된다 — `minimal.bmson` 은 재생 가능한 최소 차트다).
3. 실행 확인: 키음 파일이 함께 있는 bmson 폴더를 스캔 → 곡선택에 뜨는지 → PLAY 진입 시 노트가 보이고 판정이 되는지. 슬라이스 키음은 §7-2 때문에 파일 전체가 울린다(정상).
