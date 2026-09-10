# G6 시스템 사운드 — G8 배선 지시

대상 스펙: `docs/plan/2026-09-09-phase-g-spec.md` §8, 테스트 §11 11행.

G6 이 소유·완성한 파일은 아래 둘뿐이다.

- `apps/rbms-player/src/syssound.rs`
- `apps/rbms-player/src/syssound/tests.rs`

`syssound` 모듈은 자족적이다. 설정도 스테이지도 참조하지 않고, 폴더 경로 하나와 가이드음 토글 하나만 밖에서 받는다.
아래 지시는 전부 **G8 소유 파일에 대한 편집**이며 G6 은 한 줄도 건드리지 않았다.

---

## 0. 공개 표면 (G8 이 쓰는 것 전부)

```rust
pub(crate) const SYSTEM_SOUND_NAMESPACE: IdNamespace;  // base 0x0090_0000, len 0x100
pub(crate) const SYSTEM_SOUND_COUNT: usize;            // 22
pub(crate) const SYSTEM_SOUND_GAIN: f32;               // 1.0 — 음량은 System 버스가 담당

pub(crate) enum SystemSound { Scratch, FolderOpen, FolderClose, OptionChange, OptionOpen, OptionClose,
    PlayReady, PlayStop, ResultClear, ResultFail, ResultClose, CourseClear, CourseFail, CourseClose,
    GuidePg, GuideGr, GuideGd, GuideBd, GuidePr, GuideMs, Select, Decide }

impl SystemSound {
    pub(crate) const ALL: [SystemSound; SYSTEM_SOUND_COUNT];
    pub(crate) fn file_stem(self) -> &'static str;   // "scratch", "f-open", ...
    pub(crate) fn slot(self) -> usize;
    pub(crate) fn is_guide(self) -> bool;
    pub(crate) fn is_bgm(self) -> bool;              // Select | Decide
    pub(crate) fn sample_id(self) -> u32;
}

pub(crate) fn guide_for_judge(judge: rbms_judge::Judge) -> SystemSound;
pub(crate) fn result_sound(cleared: bool) -> SystemSound;
pub(crate) fn course_result_sound(cleared: bool) -> SystemSound;
pub(crate) fn resolve_sound(dir: &Path, sound: SystemSound) -> Option<(PathBuf, String)>;

pub(crate) struct SystemSoundCue { pub(crate) id: u32, pub(crate) gain: f32 }

pub(crate) struct SystemSoundSet;
impl SystemSoundSet {
    pub(crate) fn silent() -> SystemSoundSet;
    pub(crate) fn load(dir: &Path) -> SystemSoundSet;
    pub(crate) fn load_optional(dir: Option<&Path>) -> SystemSoundSet;
    pub(crate) fn set_guide_enabled(&mut self, enabled: bool);
    pub(crate) fn guide_enabled(&self) -> bool;
    pub(crate) fn is_resolved(&self, sound: SystemSound) -> bool;
    pub(crate) fn resolved_count(&self) -> usize;
    pub(crate) fn install(&self, engine: &mut AudioEngine);
    pub(crate) fn cue(&self, sound: SystemSound, gain: f32) -> Option<SystemSoundCue>;
    pub(crate) fn play(&self, engine: &mut AudioEngine, sound: SystemSound, gain: f32);
    pub(crate) fn stop(&self, engine: &mut AudioEngine, sound: SystemSound);
}
```

스펙 §8 원안 시그니처와 다른 두 곳:

| 원안 | 실제 | 이유 |
|---|---|---|
| `play(&self, engine: &rbms_audio::Engine, ...)` | `play(&self, engine: &mut AudioEngine, ...)` | 타입 이름이 `AudioEngine` 이고 `play_on` 이 `&mut self` 를 요구한다 |
| (없음) | `install(&self, engine)` 분리 | 디코딩과 믹서 뱅크 적재를 나눠야 오디오 장치 없이 테스트가 된다. `install` 은 `&self` 라 스트림 재오픈(뱅크가 비워짐) 뒤 다시 불러도 된다 |

---

## 1. 선행 편집 — `syssound.rs` 최상단 `#![allow(dead_code)]` 삭제

`apps/rbms-player/src/syssound.rs` 20행 근처의

```rust
#![allow(dead_code)]
```

한 줄은 W2 병렬 창에서 이 모듈이 어디에서도 호출되지 않아 `-D warnings` 가 깨지는 것을 막기 위한 것이다.
**아래 배선을 마친 직후 이 한 줄을 삭제**하고 `cargo clippy --workspace --all-targets -- -D warnings` 를 다시 돌린다.
배선 후에도 남는 미사용 항목이 있으면 그것은 지우거나 실제로 배선한다(허용으로 덮지 않는다).

---

## 2. 설정 두 필드 (G8 소유: `crates/rbms-config`)

시스템 사운드 폴더와 가이드음 토글은 오디오 계열이므로 `crates/rbms-config/src/audio.rs` 의 `AudioOptions` 에 넣는다
(스펙 §8 은 `PlaySettings` 라 적었으나 현 트리에 그 이름의 구조체는 없고, 음량·버스가 전부 `AudioOptions` 에 있다).

`AudioOptions` 필드 추가:

```rust
    /// Folder holding the system sound set. `None` leaves every system sound silent.
    pub sound_folder: Option<String>,
    /// Play the per-judgment guide cues while a chart runs.
    pub guide_se: bool,
```

`impl Default for AudioOptions` 에 대응 추가:

```rust
            sound_folder: None,
            guide_se: false,
```

`AudioOptions` 는 이미 컨테이너 `#[serde(default)]` 이므로 기존 `settings.ron` 후방호환은 그대로다. `sanitise()` 는 손댈 것이 없다(빈 문자열 폴더는 §3 의 `sound_folder_path()` 가 `None` 으로 접는다).

설정 행(descriptor)은 `crates/rbms-config/src/settings.rs` 의 AUDIO 탭에 두 행을 추가한다.

- `SOUND FOLDER` — 폴더 선택 행. 기존 라이브러리 폴더 행과 같은 방식(`rfd`)을 쓰고, 값이 바뀌면 §4 의 `reload_system_sounds()` 를 부른다.
- `GUIDE SE` — on/off 행. 값이 바뀌면 `shared.syssound.set_guide_enabled(shared.config.audio.guide_se)` 를 부른다.

---

## 3. `AppShared` 필드 (`apps/rbms-player/src/lib.rs`)

`struct AppShared` (현재 `lib.rs:377`) 에 `audio_*` 필드 무리 바로 뒤로 추가한다.

```rust
    /// The system sound set read from the configured folder. Silent throughout when no folder is
    /// set, which is what a fresh install holds.
    syssound: SystemSoundSet,
```

`use` 추가(`mod` 선언부 아래 `use` 무리):

```rust
use syssound::{SYSTEM_SOUND_GAIN, SystemSound, SystemSoundSet};
```

`AppShared` 를 만드는 자리(생성자 리터럴)에 다음을 넣는다.

```rust
        syssound: {
            let mut set = SystemSoundSet::load_optional(sound_folder_path(&config).as_deref());
            set.set_guide_enabled(config.audio.guide_se);
            set
        },
```

`lib.rs` 에 헬퍼 하나를 추가한다(빈 문자열 = 미지정).

```rust
/// The configured system sound folder, with a blank setting read as "not configured".
fn sound_folder_path(config: &Config) -> Option<PathBuf> {
    config.audio.sound_folder.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(PathBuf::from)
}
```

설정 화면이 폴더를 바꿨을 때 다시 읽는 메서드도 `impl AppShared`(`app_play.rs` 쪽 블록) 에 둔다.

```rust
    /// Re-read the system sound set after the SOUND FOLDER row changed, and hand it to the running
    /// stream so the next cue is heard without a restart.
    pub(crate) fn reload_system_sounds(&mut self) {
        self.syssound = SystemSoundSet::load_optional(sound_folder_path(&self.config).as_deref());
        self.syssound.set_guide_enabled(self.config.audio.guide_se);
        if let Some(engine) = self.audio.as_mut() {
            self.syssound.install(engine);
        }
    }
```

---

## 4. 믹서 뱅크 적재 — `open_audio_with` (`apps/rbms-player/src/app_play.rs`)

`open_audio_with` 의 `Ok((audio, report))` 갈래에서 `self.apply_audio_gains();` **바로 뒤**에 넣는다.

```rust
                if let Some(engine) = self.audio.as_mut() {
                    engine.set_chart_gain(self.chart_gain);
                }
```

이 블록 **앞**에 한 줄:

```rust
                self.syssound.install(self.audio.as_mut().expect("engine just stored"));
```

`expect` 를 피하려면 아래 형태로 기존 블록과 합친다(권장).

```rust
                self.apply_audio_gains();
                let syssound = std::mem::replace(&mut self.syssound, SystemSoundSet::silent());
                if let Some(engine) = self.audio.as_mut() {
                    engine.set_chart_gain(self.chart_gain);
                    syssound.install(engine);
                }
                self.syssound = syssound;
```

`install` 이 `&self` 라 재오픈(`reopen_audio`)이 같은 경로를 지나면 자동으로 다시 채워진다. 별도 처리 불필요.

또한 `app_play.rs` 의 `released_namespaces` 는 **손대지 않는다**. 시스템 사운드는 자기 네임스페이스에 있고 스테이지 전환으로 비우지 않는다.

---

## 5. 재생 헬퍼 (`apps/rbms-player/src/app_play.rs`, `impl AppShared`)

호출부가 매번 `if let Some(engine)` 를 쓰지 않도록 한 줄 헬퍼를 둔다.

```rust
    /// Raise a system sound on the shared stream. Silent when the stream is not open or the set has
    /// no file for it.
    pub(crate) fn play_system_sound(&mut self, sound: SystemSound) {
        let syssound = std::mem::replace(&mut self.syssound, SystemSoundSet::silent());
        if let Some(engine) = self.audio.as_mut() {
            syssound.play(engine, sound, SYSTEM_SOUND_GAIN);
        }
        self.syssound = syssound;
    }
```

(`self.syssound` 와 `self.audio` 를 동시에 빌릴 수 없어 잠깐 꺼냈다 돌려놓는다. `SystemSoundSet::silent()` 는 할당 22칸짜리 `Vec` 하나뿐이라 프레임당 한두 번 호출에 무해하다. 필드를 `Option<SystemSoundSet>` 로 두고 `take()` 해도 된다 — 둘 중 G8 이 편한 쪽.)

---

## 6. 계기별 호출 지점 (스펙 §8 표 전량)

| 사운드 | 파일 · 함수 | 삽입 위치 |
|---|---|---|
| `Scratch` | `app_play.rs` `AppShared::print_selection` | 함수 첫 줄. 커서가 움직이는 모든 경로(키보드 위/아래, 마우스 행 클릭, 검색 이탈)가 이미 이 함수를 지난다 |
| `FolderOpen` | `stage/select/mod.rs` `SelectState::select_enter` | `Some(SelectItem::Folder { target, .. })` 갈래에서 `shared.select_view = *target;` 앞 |
| `FolderClose` | `stage/select/mod.rs` `SelectState::select_back` | 함수 진입 직후, `SelectView::Root` 조기 반환(= 종료) **뒤** |
| `Select` | `stage/select/mod.rs` `SelectState::select_enter` | `Some(SelectItem::Song(i))` 갈래에서 `Transition::Open(Stage::Loading(..))` 반환 직전 |
| `Decide` | `stage/loading.rs` `LoadingState::finish_song` | Play 로 넘어가는 `Transition` 을 만들기 직전 |
| `PlayReady` | `stage/play/mod.rs` `PlayState::on_enter` | 함수 첫 줄 |
| `PlayStop` | `stage/play/mod.rs` `PlayState::leave_run` | `ctx.shared.save_settings();` 앞 |
| `ResultClear` / `ResultFail` | `stage/result.rs` `ResultState::on_enter` | 첫 줄에서 `result_sound(self.cleared)` |
| `ResultClose` | `stage/result.rs` `ResultState::on_exit` | 첫 줄 |
| `OptionOpen` | `app_options.rs` `options_key` | 패널을 여는 갈래(패널이 닫혀 있다가 열리는 분기)에서 |
| `OptionClose` | `app_options.rs` `close` | 함수 첫 줄 |
| `OptionChange` | `app_options.rs` `options_key` | 값이 실제로 바뀐 갈래(행 좌/우 이동)에서. 행 커서 상하 이동은 `Scratch` 가 아니라 `OptionChange` 를 쓰지 않는다 — 레퍼런스는 값 변경에만 건다 |
| `CourseClear` / `CourseFail` | G3 의 코스 결과 스테이지 `on_enter` | `course_result_sound(cleared)` |
| `CourseClose` | G3 의 코스 결과 스테이지 `on_exit` | |
| `GuidePg` … `GuideMs` | `play_sink.rs` 또는 판정이 확정되는 자리 | `guide_for_judge(judge)` 로 얻어 `play`. 가이드음 게이트는 `SystemSoundSet` 안에 있으므로 호출부는 토글을 검사하지 않는다 |

가이드음만 주의: 레퍼런스는 가이드음을 판정마다 **키음처럼** 재생한다(`BMSPlayer.java:398-410` 의 `setAdditionalKeySound`). rbms 는 System 버스로 보내므로 폭주(1프레임 다중 판정)를 막고 싶으면 호출부에서 프레임당 1회로 접는다. 접는 규칙은 G8 판단이며 `syssound` 는 관여하지 않는다.

---

## 7. 코스 결과 화면이 아직 없을 때

`CourseClear` / `CourseFail` / `CourseClose` 는 G3 의 코스 스테이지가 배선된 뒤에 건다. 그때까지 세 슬롯은 로드는 되지만 호출부가 없다 — `#![allow(dead_code)]` 를 지운 뒤 `SystemSound` 변형 자체는 `ALL` 에서 쓰이므로 경고가 나지 않는다.

---

## 8. 검증

- `cargo test -p rbms-player --lib syssound` — G6 이 넣은 18개 테스트.
- 배선 후 `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
- 실기: 사운드 폴더에 `scratch.wav` / `clear.wav` 만 둔 뒤 곡선택 커서 이동과 결과 화면에서 소리가 나는지, 나머지 20종이 없어도 아무 화면이 죽지 않는지 확인한다.

---

## 9. 남는 제약 (스펙 §13-6)

레퍼런스는 BGM 폴더(`select.wav` 기준)와 효과음 폴더(`clear.wav` 기준)를 나눠 스캔하고 각 루트에서 "세트"를 여럿 찾아 무작위로 고른다. rbms 는 단일 폴더·단일 세트다. `SystemSound::is_bgm()` 이 그 경계를 이미 들고 있으므로, 사용자 폴더 호환이 필요해지면 폴더를 둘로 나누고 `load` 를 `is_bgm` 으로 분기하면 된다. 이번 Phase 에서는 하지 않는다.
