# R-BMS 메모리 누수 / 무한 성장 / 성능 핫패스 감사

- 대상 커밋: `c6f0885` (dev, clean)
- 조사 시간: 약 15분 (상한 준수)
- 방법: 코드 통독 + `cargo build --release -p rbms-audio --example decode_probe` 실측 1건
- **레퍼런스 구현 대조 미수행**: 이번 관점(누수/핫패스)은 Rust 소유권·cpal/wgpu 런타임 고유 문제라 Java 원본과 1:1 대응되는 지점이 거의 없어, 시간 상한 안에서 rbms 쪽 코드만 근거로 판정했다. 레퍼런스 구현 측 인용이 필요한 항목은 아래 "미조사 범위"에 남긴다.

---

## 1. 누수·무한 성장 후보 전수 판정

| # | 후보 | 판정 | 근거 |
|---|---|---|---|
| 1 | 폰트 레이아웃 캐시 (`TextEngine.cache`) | **확정 (무한 성장)** | `font.rs:50,89` — 상한·evict 없음, `set_ui_family`/`reset_ui_family`(`font.rs:225,237`)의 폰트 교체 시에만 `clear()` |
| 2 | 폰트 글리프 런 캐시 (`TextEngine.runs`) | **의심 (색상축 성장)** | `font.rs:54,113` — 키가 `(CacheKey, packed RGB)`. 글리프×사용색 조합만큼 성장, 색이 애니메이션되면 비선형 |
| 3 | 오디오 키음 뱅크 (`AudioEngine.bank`) | **아님 (곡 전환 시 해제)** | `engine.rs:20,75` — 제거 API 없음. 다만 엔진 자체를 `self.audio = None`(`main.rs:1099`, `app_select.rs:869`) / `self.preview_audio = None`(`app_select.rs:339,522`)으로 통째 드롭 → 뱅크 동반 해제 |
| 4 | 믹서 보이스의 `Arc<SampleData>` 반환 | **아님 (해제됨)** | `mixer.rs:102-103`(stop), `mixer.rs:145-146`(재생 종료) 둘 다 `v.sample = None`. 단 §2-6 RT 이슈 참조 |
| 5 | 프리뷰 코디네이터 스레드 | **의심 (일시적 무한 증가)** | `app_select.rs:474` detach 후 join 없음. `spawn_keysound_decode`(`main.rs:328-346`)가 추가로 최대 8개 → **포커스 정착 1회당 최대 9스레드**, 상한 없음 |
| 6 | 프리뷰 채널 (`mpsc<PreviewMsg>`) | **의심 (일시적 대용량)** | `app_select.rs:469` 무제한 mpsc가 `DecodedAudio`(디코드된 f32 PCM 전체)를 실어 나름. 곡 전체 키음이 채널에 적체 가능 |
| 7 | cpal 출력 스트림 (프리뷰) | **의심 (핸들 churn)** | `app_select.rs:422,457` — 호버한 곡마다 `AudioEngine::new()`로 **새 디바이스 스트림 오픈**, `app_select.rs:522`에서 드롭. 반복 open/close |
| 8 | 폴더 스캔 스레드 | **아님** | `main.rs:796-800` 앱 시작 시 1회, `app_select.rs:701` 재스캔 시 1회. 결과 1건 전송 후 종료 |
| 9 | BGA/커버 텍스처 (wgpu) | **아님 (단일 슬롯 재사용)** | `gpu.rs:178-187` 256×256 텍스처 1개를 생성 후 `write_texture`로 덮어씀(`gpu.rs:247-259`). 곡 이동 시 텍스처 생성 없음 |
| 10 | 인스턴스 버퍼 재할당 | **아님 (단조 증가·상한 있음)** | `gpu.rs:279-287` — 초과 시에만 `next_power_of_two`로 재생성, 축소 안 함. 피크 쿼드 수로 유계 |
| 11 | `quads` Vec | **아님** | `gpu.rs:344` `clear()` (capacity 유지 = 프레임당 재할당 없음, 의도된 동작) |
| 12 | `cached_select` 씬 | **아님 (중복 보유)** | `app_select.rs:99-128` — 전곡 title/level String을 `self.songs`와 별개로 1벌 더 보유. 유계지만 라이브러리 규모에 비례 |
| 13 | 리플레이/스코어 Vec, 난이도표 fetch 캐시, 로그 버퍼 | **미조사** | `scores.rs`, `replay.rs`, `tables.rs`, `tablesrc.rs`, `rbms-ir/src/http.rs` 미통독 |

### 재현·측정 방법 제안

- **폰트 캐시(1·2)**: 플레이 1곡(EX 스코어 0→5000, 콤보 0→3000, GREEN 수치 변동) 후 디버그 오버레이 `RAM`(`app_play.rs:383, 752`, `memory_stats` 크레이트 이미 의존) 값을 곡 시작/종료로 비교. 곡을 10회 반복하며 RAM이 단조 증가하면 확정.
- **프리뷰(5·6·7)**: 곡선택에서 방향키를 20프레임 간격 이상으로 100회 이동(디바운스 `PREVIEW_DEBOUNCE_FRAMES = 20`, `main.rs:59` 충족)한 뒤 RSS와 스레드 수(`ps -M <pid>` / Activity Monitor) 확인. 스레드 수가 이동 횟수에 비례해 남으면 확정.
- **뱅크(3)**: 곡 진입 → Esc → 다른 곡 진입을 100회 반복 후 RSS. 현재 코드 기준으로는 평탄해야 정상(회귀 감시용 baseline).

---

## 2. 성능 핫패스

### 2-1. 곡선택 커서 이동 = 라이브러리 전체 재구축 (가장 큰 건)

`select_key()`에 **`self.sel`이 포함**된다.

```
app_select.rs:79   (self.select_gen, self.sel, self.record_modal, self.scores.records.len(), self.config.score_graph)
app_select.rs:86-92 키가 바뀌면 build_select_view() 전체 재실행
app_select.rs:99-128 select_items 전체를 map → title.clone(), level.clone() 로 SelectRow Vec 생성
```

반면 실제로 그려지는 행은 가시 구간뿐이다 — `select.rs:288 for idx in start..(start + visible).min(m)`.

즉 **커서 1칸 이동마다 전곡 String 2개씩 힙 할당**. 방향키 홀드 시 60fps로 반복되므로, 1만 곡 라이브러리에서 초당 120만 회 할당. 주석(`app_select.rs:82-84`)은 "매 프레임 재할당 방지"를 의도했다고 적혀 있지만, `sel`이 키에 들어가 있어 **입력 중에는 캐시가 사실상 무효**다.

### 2-2. UI 스레드 동기 블로킹 (프리뷰 파일 경로)

`#PREVIEW` 파일이 있는 곡은 포커스 정착 시 렌더 스레드에서 전부 처리한다.

| 단계 | 위치 |
|---|---|
| 디바이스 스트림 오픈 | `app_select.rs:422` `AudioEngine::new()` → `engine.rs:29-47` (`build_output_stream` + `play()`) |
| 파일 읽기 | `app_select.rs:412` `std::fs::read` |
| 심포니아 디코드 | `app_select.rs:433` `eng.load` → `engine.rs:65` `decode_bytes` |

autoplay 폴백 경로(`app_select.rs:474`)는 올바르게 백그라운드로 뺐는데, 파일 프리뷰 경로만 메인 스레드에 남아 있다. 수 MB ogg면 포커스 정착마다 수십~수백 ms 프레임 스톨.

### 2-3. 프리뷰 로더 스레드·채널 무제한

- 코디네이터(`app_select.rs:474`)는 `cancel`을 **2곳에서만** 확인(`476`, `498`). 그 사이의 `parse_with` → `detect_mode` → `to_model` → `Player::new` + 전곡 autoplay `update`(`481-496`)는 취소와 무관하게 끝까지 돈다.
- 키음 디코드 풀은 호출마다 최대 8스레드(`main.rs:328-330`).
- 채널은 무제한 `mpsc`(`app_select.rs:469`)이며 디코드된 PCM 전체를 실어 나른다.
- 결과: 빠른 스크롤 시 **(1 + 8) × 정착 횟수**만큼의 스레드가 동시에 CPU를 물고, 렌더 스레드를 굶긴다.

### 2-4. 오디오 콜백 RT-safety

콜백 본문은 `engine.rs:122-134`.

| 항목 | 판정 |
|---|---|
| 락 | 없음 (rtrb SPSC + `AtomicU64`) — 양호 |
| 로그/syscall | 데이터 콜백에는 없음. `eprintln!`은 **에러 콜백**(`engine.rs:135`)이라 RT 경로 아님 |
| 할당 | **있음** — `engine.rs:126-128` `scratch.resize(data.len(), 0.0)`. 첫 콜백과 호스트 버퍼 크기 변경(디바이스/라우트 전환) 시 콜백 안에서 힙 할당 |
| Arc 해제 | `mixer.rs:103,146`에서 `v.sample = None`. **현재는 안전** — `bank`(`engine.rs:20`)가 모든 `Arc`를 엔진 수명 내내 보유하므로 콜백에서는 refcount 감소만 발생하고 `free`는 일어나지 않는다. 향후 `bank`에서 개별 제거(unload) API가 생기면 즉시 RT free로 전환되는 잠재 위험 |
| 명령 유실 | **있음(무음 유실)** — `engine.rs:95,100,104`가 전부 `let _ = self.producer.push(...)`. 8192 슬롯 링이 가득 차면 Play/Stop이 조용히 사라지고 카운터도 없다 |
| 보이스 delay 루프 | `mixer.rs` mix 내부에서 `v.delay > 0`이면 출력 프레임마다 1씩 감소. 대기 보이스 수 × 프레임 수의 순수 오버헤드(`skip = min(delay, frames)`로 상수화 가능) |

### 2-5. 판정 루프 복잡도

- `Matcher::press`(`matcher.rs:203-220`): `l.cursor`부터 시작해 `dm > gate_early`에서 break — **판정창 내 노트 수로 유계**. 양호.
- `Matcher::release`(`matcher.rs:262`): `l.notes.iter().position(|n| n.holding)` — **레인 처음부터 전수 스캔**. LN을 잡고 있지 않은 일반 키업에서도 레인 전체를 훑고 `None`을 반환한다. 곡 길이에 비례하는 O(n).
- `Matcher::update`(`matcher.rs:284`): 매 프레임 `events: Vec<Judge>` 생성. `Vec::new()`는 push 전까지 할당하지 않으나, 판정이 발생하는(=밀도 높은) 프레임마다 힙 할당.
- `Player::update`(`rbms-play/src/lib.rs:173-208`): `bg_cursor`/`action_cursor` 전진 방식 — 양호.

### 2-6. 프레임당 문자열 할당

`format!`/`fit_text`가 프레임마다 새 `String`을 만든다: `hud.rs:46,47,81,85,87,90,126,128`, `select.rs:225,230,231,238,313,315`, `font.rs:186`(`fit_text`는 항상 새 String 반환, `font.rs:186-201`). 프레임당 20~30건. 할당 자체보다, 이 문자열들이 **§1-1 폰트 캐시에 영구 적재된다**는 점이 본질적 비용이다.

### 2-7. 렌더/GPU (양호)

- draw call은 프레임당 최대 2회(BGA 1 + 인스턴스 1) — `gpu.rs:312-322`.
- `fill_rect`는 인스턴스 push만(`gpu.rs:348-353`), CPU 래스터·텍스처 업로드 없음.
- 텍스트는 **매 프레임 shaping 하지 않는다** — `ensure()`가 캐시 히트면 즉시 반환(`font.rs:70-74`), 글리프 래스터도 `(CacheKey, rgb)` 캐시(`font.rs:113-122`). 설계는 옳고, 문제는 캐시에 상한이 없다는 것뿐.

### 2-8. 실측

`cargo build --release -p rbms-audio --example decode_probe` 성공 후:

```
./target/release/examples/decode_probe samples/preview-demo/preview.wav
  instructions retired : 29,092,066
  cycles elapsed       : 18,914,260
  peak memory footprint: 1,737,040 bytes (1.74 MB)
```

**이 수치는 대표성이 없다** — 저장소에 커밋된 유일한 오디오 픽스처가 8 kHz 모노 짧은 클립(`decode.rs:240-241`)이라 실제 BMS 키음 부하(수백~수천 개 44.1 kHz 파일)와 무관하다. GUI 실행이 필요한 프레임타임/RSS 추세는 헤드리스 환경 제약과 읽기 전용 규칙(샘플 곡 미보유)으로 **실측 불가**.

---

## 3. 우선순위 개선안

| 순위 | 항목 | 효과 | 노력 |
|---|---|---|---|
| 1 | `select_key`에서 `sel` 분리 (행 목록은 `select_gen`만, detail만 `sel` 의존) | 커서 이동 시 O(라이브러리) → O(1) | M |
| 2 | 폰트 레이아웃 캐시에 상한 + LRU evict, HUD 숫자는 자릿수 단위 그리기 | 무한 성장 제거, 캐시 히트율 상승 | M |
| 3 | 프리뷰 엔진 1개 재사용 + `AudioEngine::clear_bank()` 추가 | 호버마다의 cpal 스트림 open/close 제거 | M |
| 4 | 파일 프리뷰(`#PREVIEW`)도 코디네이터 스레드로 이동 | 포커스 정착 스톨 제거 | S |
| 5 | 프리뷰 로더를 세대(generation) 기반 단일 워커로 통합, 파싱 루프 내 cancel 확인 | 스레드 폭증·CPU 경합 제거 | M |
| 6 | `engine.rs` `scratch`를 생성 시 최대 버퍼로 선할당 | 콜백 내 할당 제거 | S |
| 7 | `producer.push` 실패를 `AtomicU64`로 집계 + 디버그 오버레이 노출 | 무음 키음 유실을 관측 가능하게 | S |
| 8 | `Matcher::release`를 `l.cursor`부터 스캔 또는 레인별 `holding_idx` 보유 | O(n) → O(1) | S |
| 9 | `Matcher::update`의 `events` 버퍼를 필드로 승격(재사용) | 프레임당 할당 제거 | S |
| 10 | `mixer` delay를 `min(delay, frames)`로 상수 스킵 | 대기 보이스 오버헤드 제거 | S |

### 측정 하네스 제안

1. **누수 회귀 테스트** (`apps/rbms-player/tests/`): `memory_stats`(이미 의존, `app_play.rs:383`)로 RSS를 읽고, ① 폰트 `draw_text`로 서로 다른 10,000개 문자열을 그린 뒤 RSS 증가분에 상한 assert, ② `AudioEngine` 생성/드롭 100회 후 RSS 평탄성 assert.
2. **폰트 캐시 계측**: `font.rs`에 `#[cfg(any(test, feature = "metrics"))] pub fn cache_stats() -> (usize, usize)`(레이아웃 엔트리 수, 런 엔트리 수)를 노출해 테스트에서 상한을 직접 검증. 디버그 오버레이에도 함께 표시.
3. **프레임타임 로그**: 이미 `fps` EMA가 있으므로(`app_play.rs:379`), 프레임 시간 원본을 링버퍼에 적재해 p50/p99를 오버레이에 추가하고, `--debug` 시 CSV로 덤프. 스파이크와 프리뷰 정착 프레임의 상관을 바로 볼 수 있다.
4. **오디오 언더런/유실 카운터**: `AudioEngine`에 `Arc<AtomicU64>` 2개(콜백 내 `scratch` 재할당 횟수, `producer.push` 실패 횟수)를 두고 오버레이에 노출. 3번의 프레임타임과 함께 보면 "프레임 스톨 → 명령 적체 → 키음 유실" 경로가 재현 가능해진다.
5. **소크 테스트**: 곡선택에서 커서를 스크립트로 100회 이동시키는 헤드리스 경로(`autoplay_preview.rs` 테스트 확장)를 만들고, 1분 간격 RSS·스레드 수를 기록.

---

## 4. 문서 stale

- `docs/PROCESS.md:132` — "**`#PREVIEW` 프리뷰 재생 (TODO — 실재생 미동작)** … 포커스해도 소리 안 남(추후 디버깅)". 실제 코드에는 파일 프리뷰 + autoplay 폴백 프리뷰가 모두 구현·동작 형태로 존재한다(`app_select.rs:286-514`, 커밋 `c6f0885` "Add song-select hover preview"). 코드를 신뢰하고 문서를 갱신해야 한다.

---

## 5. 미조사 범위

- `apps/rbms-player/src/scores.rs`, `replay.rs` — 스코어/리플레이 `Vec` 성장, 파일 I/O 빈도
- `apps/rbms-player/src/tables.rs`, `tablesrc.rs`, `crates/rbms-ir/src/http.rs` — 난이도표 fetch 캐시, HTTP 클라이언트 수명
- `crates/rbms-render/src/cpu.rs` — 헤드리스 CPU 캔버스 경로
- `crates/rbms-parser`, `rbms-chart` — 스캔 시 파싱 비용(곡당 파싱 시간, `scan_folders` 병렬성)
- 실제 GUI 실행 기반 프레임타임/RSS 추세, 곡 전환 100회 루프 후 RSS 실측
- 레퍼런스 구현 Java 원본과의 대조 (본 관점에서는 대응 지점이 희박하나, 키음 뱅크 수명·프리뷰 정책은 `select/`·`play/` 원본과 비교 가치가 있음)
