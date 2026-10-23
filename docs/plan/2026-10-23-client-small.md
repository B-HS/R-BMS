# 클라이언트 작은 묶음 — SCRATCH AUTO 색(결정 8), Esc 결과 화면(결정 9), 패널 키 스킨 opt-in(결정 5) 설계서

> 최종 갱신 2026-10-23 · 대응 커밋 1169efb(dev) · 작성 2026-10-23 · 상태: 설계(구현 전). PROCESS 체크리스트 "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" 의 F1(설계) 중 F2 묶음 몫이다.
> 사용자 결정 원문(2026-10-23): "1-11 추천대로 / 12. st, sl 둘 다 구현 / 13 - 16 추천대로 / 17. … / 18. DP도 포함 / 19. 맞어 / 20. 추천대로 / 21. …". 결정 5·8·9 는 "1-11 추천대로" 에 들어 있다. 결정 요약은 [PROCESS.md](../PROCESS.md) 의 같은 체크리스트, 조사 근거는 [reference/2026-10-23-stella-course-dan-survey.md](../reference/2026-10-23-stella-course-dan-survey.md) 의 client-1-3 절과 [reference/option-panels.md](../reference/option-panels.md) 1.10절 14.
> 이 문서는 결정을 바꾸지 않는다. 결정이 다루지 않는 세부는 8절 "메인 결정 필요" 표에 추천안 하나와 근거·대안을 적었다. 판정·점수 의미가 원본과 달라지는 곳은 4절 표에 따로 표시했다.

## 1. 범위와 결정 번호 대응

| 결정 | 사용자 결정(PROCESS 요약) | 이 문서의 구현 단위 | 바뀌는 사용자 체감 |
| --- | --- | --- | --- |
| 8 | SCRATCH AUTO 노트 색 = PLAY 전역 RGBA 한 값(외부 스킨 포함, 알파 = 불투명도, 저장값 없으면 `#30E050`). 조사의 추천: LN 몸통은 지금의 0xA0 비율 유지, 메뉴는 PLAY 메뉴 CHART 의 SCRATCH AUTO 행 옆 색 행(16진 입력·프리셋·채널), 스킨 없는 내장 필드도 같은 값 | U-A | PLAY 메뉴 CHART 에 `SCRATCH AUTO COLOR` 행과 채널 접이가 생긴다. 바꾸면 어느 스킨에서든, 내장 필드에서도 턴테이블 초록 대신 그 색이 그려진다. 바꾸지 않으면 지금과 화소가 같다 |
| 9 | 노트를 1개 이상 친 판의 Esc 는 로그인과 상관없이 결과 화면, 코스 도중 Esc 는 FAILED 코스 결과까지, 결과 판정 수는 저장값과 같게. 조사의 추천: 진입은 즉시(닫힘 연출 없이), 이중 전송 금지(결과 화면 전송 경로 하나만), 노트를 하나도 치지 않은 판은 지금처럼, 앱 종료는 지금대로 | U-B | Esc 로 나간 판이 FAILED 결과 화면을 거친다. 코스는 그 스테이지를 FAILED 로 접고 코스 결과(불합격)까지 간다. 저장·전송되는 값은 지금과 같다 |
| 5 | panels-1: 스킨 헤더의 확장 지원 표시가 있을 때만 재배정 키 표, 없으면 원본 `MusicSelectKeyProperty` 표 그대로 | U-C | ModernChic 같은 외부 스킨에서 패널 키가 원본 역할로 돌아간다(죽은 칸은 아무 일도 하지 않는다). 기본 스킨은 지금과 같다 |

세 단위는 서로 다른 파일이 대부분이라 병렬로 나눈다(5절). 겹치는 파일은 `apps/rbms-player/src/stage/play/mod.rs` 하나이고 고치는 줄 범위가 떨어져 있다(5.4절).

범위 밖: 결정 15 의 ZIP 해제 크레이트(F4 묶음 몫). 이 묶음은 새 의존성을 하나도 추가하지 않는다. 웹·DB·마이그레이션 변경도 없다.

## 2. 현재 코드 사실(다시 확인한 것, HEAD 1169efb)

### 2.1 SCRATCH AUTO 초록(결정 8)

| 사실 | 위치(직접 열어 확인) |
| --- | --- |
| 노트 색 상수 `AUTO_LANE_NOTE = Color { r: 0x30, g: 0xe0, b: 0x50, a: u8::MAX }` | `crates/rbms-render/src/skin_render/notes.rs:141` |
| 몸통 색 상수 `AUTO_LANE_LONG_BODY` = 같은 초록, `a: 0xa0` | `notes.rs:144` |
| `AUTO_NOTE_IMAGE: NoteImage = NoteImage::Solid(AUTO_LANE_NOTE)` | `notes.rs:147` |
| `AUTO_LONG_IMAGES`(머리·끝·HCN 머리·끝은 노트색, 나머지 몸통은 0xA0) | `notes.rs:150-157`(조사 보고의 150-156 은 끝 줄이 하나 짧다) |
| 일반 노트 그리기: `NoteKind::Normal if notes.plays_itself(lane_index)` → `self.put(r, &AUTO_NOTE_IMAGE, at, alpha, None)` | `notes.rs:1028-1031` |
| 롱노트 그리기: `if self.notes.plays_itself(lane_index) { (&AUTO_LONG_IMAGES, None) }` | `notes.rs:1099` |
| `put()` 이 `alpha × self.opacity × 255` 로 알파를 정하고 `NoteImage::Solid(color)` 는 `lane_tinted(*color, alpha, tint)` = `faded`(색 알파 × 그리기 알파 / 255) | `notes.rs:943-951`, `:970`, `faded :780-782`, `lane_tinted :788-791` |
| `LaneNotes.auto_lanes` 필드와 `LaneNotes::new` 기본값 | `notes.rs:303-308`, `:319-337` |
| 앱에서 `LaneNotes` 를 만드는 곳은 둘뿐: 플레이 `stage/play/mod.rs:1186-1196`(auto_lanes 는 `:1194`), PLAY·SKIN 메뉴 미리보기 `stage/play/skin_preview.rs:777-787`(`:785`). 조사 보고가 함께 든 `stage/capture/panels.rs:148`·`ghost.rs:246` 은 `SessionOptions.auto_lanes` 를 채울 뿐 그리지 않는다(정정) | 각 파일 |
| 내장 필드: `PlayfieldView.auto_lanes`, 몸통 `AUTO_LANE_LONG_BODY`, 노트 `AUTO_LANE_NOTE` | `crates/rbms-render/src/playfield.rs:30-31`, `:56-57`, `:105`, `:127` |
| `PlayfieldView` 를 필드 전부로 짓는 곳: `playfield.rs:290`(테스트 도우미), `:776`(테스트), `crates/rbms-render/tests/golden_play.rs:65-74`, `crates/rbms-render/examples/render_frame.rs:35-44`, `apps/rbms-player/src/stage/play/mod.rs:1698-1707`. 구조체는 `Default` 가 없다 | 각 파일 |
| 화소 단언 테스트: `playfield.rs:321-331`(`a_note_the_chart_plays_itself_is_drawn_green`), `skin_render/notes/tests.rs:410-445`, `:1035`, `apps/rbms-player/src/stage/play/screen_tests/default_pack_tests.rs:395-428`(`is_auto_lane_green`) | 각 파일 |
| 골든 `golden_play.rs:73` 은 `auto_lanes: &[]` 라 이 색과 무관 | 같은 파일 |
| 설정: `PlayOptions.scratch_auto: bool`, 기본 `false`. 이 구조체는 `#[serde(default)]` | `crates/rbms-config/src/schema.rs:536-591`(필드 `:580`), 기본값 `:593-622` |
| 색 직렬화: `SkinColor` 의 `Serialize`/`Deserialize` 는 `crates/rbms-skin/src/loader.rs:146-` 에 있고 RON 에서는 `(r: .., g: .., b: .., a: ..)`, 알파를 빼면 불투명(테스트 `crates/rbms-config/src/tests.rs:384-393`) | 각 파일 |
| 설정 묶음 업로드 계약 `contract/client-requests/settings-blob.json` 은 RON 본문 전체를 담는다(`content` 문자열). 필드를 더하면 이 파일이 바뀐다(`apps/rbms-player/src/app_result/record_tests/contract_requests.rs:933-941` `a_settings_upload`, 갱신은 `RBMS_UPDATE_CONTRACT=1`, `:28`). 웹은 이 본문을 문자열로만 다룬다 | 각 파일 |
| PLAY 메뉴 CHART 묶음의 SCRATCH AUTO 행: `SettingId::ScratchAuto` 되돌리기 `apps/rbms-player/src/stage/play_menu/rows.rs:402`, 행 목록 `listed() :210-232`, 묶음은 `SettingTab::PlayChart`(`crates/rbms-config/src/settings.rs:427`) | 각 파일 |
| 기존 색 행 UI 부품: 프리셋 16색 `COLOR_PRESETS`(`apps/rbms-player/src/skin_select.rs:175-192`, 비공개), 프리셋 한 칸(`step_for` 의 `SkinRow::Color` 가지 `:1274-1288`), 채널 `ColorChannel`(`:194-236`, `get`/`set`/`label` 비공개), `CHANNEL_STEPS`(`:165`, 비공개), `hex_color`(`:239`, `pub(crate)`), `color_value`(`:1954-1956`, 비공개), 16진 편집 `NumberInput::begin_hex_edit`·`typed_color`(`apps/rbms-player/src/stage/play_menu.rs:339-346`, `:396-400`) | 각 파일 |
| PLAY 메뉴 미리보기는 SCRATCH AUTO 를 그리지 않는다 | `docs/play-options.md:346`(7.4절 표) |
| ADR 2026-10-22 색 행 4절이 "SCRATCH AUTO 초록은 사용자 색보다 우선한다(기존 동작 유지)" 라고 적었다 | `docs/acknowledge/2026-10-22-adr-skin-colour-rows.md:41` |

### 2.2 Esc 와 결과 화면(결정 9)

| 사실 | 위치 |
| --- | --- |
| `escape_key` 는 ESCAPE 설정(Immediate/Hold/Double) 모두 `leave_run` 으로 간다 | `apps/rbms-player/src/stage/play/mod.rs:652-680` |
| 길게 누르기 경과 호출: 장면 경로 `:1430`, 장면 없는 경로 `:1554` | 같은 파일 |
| 장면 경로의 Esc: `scene_escape :1460`. 닫히는 중이면 `enter_result`, 남은 노트가 없으면 `stop_finished`, 그 밖은 `escape_key` | 같은 파일 |
| `leave_run :709-728` 순서: PlayStop 소리 → `save_settings` → 연습이면 `Back` → `has_ended()`(`:731`)면 `enter_result` → 코스면 `app_result::leave_course` → 단일 곡은 `keeps_a_left_run && !has_judged_nothing` 일 때 `record_left_run` → `leave_play()`(선곡) | 같은 파일 |
| `has_judged_nothing :739-743`: PG·GR·GD·BD 가 0 이고 쥔 롱노트가 없음. `keeps_a_left_run :1482-1484`: 오토플레이·리플레이 보기가 아님 | 같은 파일 |
| 장면의 실패 가지가 곡 소리를 끈다: `SceneStep::Failed \| Finished { failed_too: true }` → `stop_chart_sounds` + PlayStop | `mod.rs:1417-1420`(조사 보고의 1415-1418 은 두 줄 어긋났다, 정정) |
| `enter_result(state, shared) :70-187`: `play.summary()`·`play.record()`(조정 안 된 값)로 `record_run`, 결과 화면 전송(`spawn_profile_submits`, `spawn_score_submit` 또는 `IrStatus::Sending`), `keep_run_record`, 코스면 `fold_course_stage` → 다음 스테이지 또는 `finish_course` | `apps/rbms-player/src/app_result.rs` |
| `record_left_run :489-495`: `summary().left_unfinished()` 로 `record_run`, `send_without_a_screen`(프로필 + 그 판 하나의 기록 동기화), 메시지 `LEFT_RUN_KEPT`(`:477`). 문서 주석 `:480-488` 이 "No result screen is shown" 이라고 적었다 | 같은 파일 |
| `leave_course :595-612`: 친 노트 0 이면 `abandon_course(Sending::Now)`, 아니면 `record_left_run` → `fold_course_stage` → `keep_finished_course(Now)` → `end_course`(선곡). 메시지 `LEFT_COURSE_KEPT :580` | 같은 파일 |
| `send_without_a_screen :636-643` 은 `record_retried_run :624-632` 도 쓴다 | 같은 파일 |
| `keep_run_cut_short_by_quitting :545-576`(앱 종료)은 화면 없이 남긴다. 이번에 바꾸지 않는다 | 같은 파일 |
| `fold_course_stage :786-812` 는 `survived: !summary.failed` 를 넘기고 `CourseRun::advance`(`crates/rbms-course/src/run.rs:170-184`)는 살아남지 못한 스테이지에서 `CLEAR_FAILED` 로 `CourseStep::Failed` 를 돌려준다. `finish_course :821-829` 는 `CourseResultState::of` 후 `keep_finished_course(Now)` 로 코스를 남기고 보낸다(한 길) | 각 파일 |
| `PlaySummary::left_unfinished`(남은 노트를 늦은 POOR 로, 램프 FAILED, `failed = true`) | `crates/rbms-play/src/session.rs:577-588` |
| `PlaySession::record() :872-874` = `PlayRecord::of(model, judge, self.summary(), &self.gauge_log)`. `PlayRecord` 는 `summary` 를 품는다 | `session.rs`, `crates/rbms-play/src/record.rs:421-460` |
| 게이지 이력: `GaugeLogRecorder::closed :216-229` 는 게이지 사망(`failed_at_ms`)일 때만 끝까지 0 을 채운다. Esc 판은 0 을 채우지 않는다 | `record.rs:168-233` |
| 결과 화면 데이터: `ResultRun::of`(`apps/rbms-player/src/stage/result.rs:669-708`)는 `shared.run_records.last()`(= `keep_run_record` 가 넣은 `PlayRecord`)에서 `ResultInput::from_record`(`skin_host/result/snapshot.rs:195-203`)로 읽는다. 내장 화면은 `ResultView.counts = summary.counts` | 각 파일 |
| 결과 화면 진입 때 스킨 장면 타이머는 `start_skin_scene_clock`(`stage/scene_life.rs:95-98`, `skin_timers.clear()`)이 비운다. 플레이 화면의 `on_exit`(`mod.rs:1529-1539`)이 로드 중단·텍스처 해제를 한다 | 각 파일 |
| 원본: Esc → `player.stopPlay()`(`ControlInputProcessor.java:205-206`). `stopPlay`(`BMSPlayer.java:938-970`): 남은 노트가 있으면 `STATE_FAILED`, `TIMER_FAILED`, 소리 정지, `PLAY_STOP`. `STATE_FAILED` 가지(`:693-742`): `skin.getClose()` 경과 뒤(`:711`) `createScoreData()`(`:717`), 게이지 로그 꼬리에 0 채움(`:722-727`), 점수가 있으면 RESULT(`:737-738`), 없으면 선곡(`:739-741`). `createScoreData`(`:858-861`)는 코스가 아니고 PG~BD 합이 0 이면 `null`. 오토플레이는 `stopPlay` 가 `STATE_FINISHED` 로 보낸다(`:953-955`) | `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/` |

뒤집히는 테스트(직접 읽어 확인): `app_result/record_tests.rs:936`(`leaving_mid_song_keeps_a_failed_run_with_the_rest_as_poor` 가 "Result·Stay 가 아님" 단언), `:965`(`leaving_mid_song_signed_in_sends_the_run` 이 `submit_rx.is_none()` 과 기록 동기화 단언), `:1184`(`leaving_mid_course_keeps_the_stage_and_a_failed_course` 가 "Result·CourseResult 가 아님", `course_run.is_none()` 단언), `app_result/record_tests/contract_requests.rs:746-776`(`a_run_left_mid_song` 이 `submit_rx.is_none()` 후 동기화로 받음). `stage/play/tests.rs:82-94` 의 `leaves_the_chart` 는 "Stay 가 아님" 이라 결과 화면도 참이다(뒤집히지 않음, 조사 보고의 "확인 못함" 해소).

### 2.3 패널 키 표(결정 5)

| 사실 | 위치 |
| --- | --- |
| 현재 표 `BEAT_TABLE`·`POPN_TABLE` 은 원본에서 10칸을 바꾼 확장 표이고, 어느 스킨에서나 쓴다 | `apps/rbms-player/src/stage/select/keys.rs:98-125`, 바뀐 칸 `CHANGED_SLOTS :391-402` |
| 원본 표(이름) `REFERENCE_BEAT`(테스트 상수) | `keys.rs:333-343` |
| 패널별로 누름을 받는 역할 `PLAY_PANEL_PRESSES`(7), `ASSIST_PANEL_PRESSES`(8), `DETAIL_PANEL_PRESSES`(7) | `keys.rs:127-155` |
| `KeyLayout::of(mode)` 는 턴테이블 유무만 본다. 호출: `stage/select/skinned.rs:268`(스킨 선곡의 입력), `stage/select/guide.rs:242`(키 안내). 스킨 없는 내장 선곡은 `BUILT_IN_KEY_LAYOUT = KeyLayout::Beat`(`stage/select/mod.rs:105`, 목록 역할 FolderOpen·FolderClose 만 읽는다) | 각 파일 |
| 2026-10-22 이전(커밋 `7746932^`) 표와 역할: `OptionDpUp/Down`, `Option2Up`, `JudgeArea`, `MarkNote`, `BpmGuide`, `NoMine` 이 있었다. `press_act` 에서 `OptionDpDown → DoubleOption NEXT`, `OptionDpUp → DoubleOption PREVIOUS`, `Option2Up → Random PREVIOUS`, 네 죽은 칸은 `AssistSwitch` 의 `row() == None` 이라 아무 일도, 소리도 없었다 | `git show 7746932^:apps/rbms-player/src/stage/select/{keys,panel}.rs` |
| `SettingEvent::DoubleOption`(이벤트 54)은 지금도 있다(스킨 단추가 실행한다). DP OPTION 값은 플레이에 적용되지 않는다 | `stage/select/events.rs:86-88`, `docs/reference/option-panels.md` 1.4절 |
| `press_act` 와 `AssistSwitch` | `stage/select/panel.rs:66-131` |
| 패널 2 턴테이블 `Hidden`, 패널 3 턴테이블 `JudgeAlgorithmUp/Down` 은 확장 표에만 있다. 원본은 턴테이블을 패널 1 의 TARGET 으로만 읽는다(`MusicSelectInputProcessor.java:62-330`, option-panels.md 3.1절) | `keys.rs:107-108` |
| 키 안내의 패널 줄 `panel_rows`: `SELECT + TURNTABLE = HIDDEN+ ON/OFF`, `START+SELECT + TURNTABLE = JUDGE ALGORITHM` 을 늘 쓴다. 테스트 `stage/select/overlay_tests.rs:602-624` 가 6줄과 `[5]` 를 단언한다 | `guide.rs:225-237` |
| 스킨 헤더: `SkinDef` 는 `#[derive(Deserialize)] #[serde(default)]` 이고 `deny_unknown_fields` 가 없다. R-BMS 확장 헤더 키 선례 `color` 가 있다. JSON 은 `from_value`(`crates/rbms-skin/src/loader.rs:818-825`, serde_json), Lua 는 `skin_def_from_lua`(`loader/from_lua.rs:122-136`, 같은 serde 파생을 Lua 값 위에서 강제 변환) | `crates/rbms-skin/src/model.rs:585-631` |
| Lua 스킨은 이름·작성자·행을 헤더 패스에서, 나머지를 본체 패스에서 가져온다 | `loader/lua_skin.rs:186-207` |
| 선곡 화면이 그리는 문서는 `shared.skins.document(SKIN_TYPE_MUSIC_SELECT) -> Option<&LoadedSkin>` 이고 `LoadedSkin.def: SkinDef` 를 가진다 | `apps/rbms-player/src/skin_select.rs:716`, `loader.rs:487-492` |
| beatoraja 는 JSON(`JSONSkinLoader.java:99`, `:226` `setIgnoreUnknownFields(true)`)과 Lua(`LuaSkinLoader.fromLuaValue :169-207`, 필드 이름이 맞는 키만 대입) 모두 모르는 키를 버린다 | `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/skin/` |
| 기본 스킨 선곡 헤더 `screen.header()` | `assets/skins/rbms-default/lua/select/main.lua:15-27`, 확장 id 표 `lua/props.lua:167-177` |
| 패널 테스트 픽스처 `panel_document()`(`stage/select/panel_tests.rs:54-73`)와 선곡 픽스처 `wheel_document()`(`stage/select/skinned_tests.rs:99`)에는 헤더 표시가 없다. 그래서 표시를 요구하면 지금의 패널·안내 테스트가 모두 원본 표로 바뀐다(5.3절에서 픽스처에 표시를 넣는다) | 각 파일 |

## 3. 설계

### 3.1 U-A: SCRATCH AUTO 색(결정 8)

#### 3.1.1 설정

`crates/rbms-config/src/schema.rs` 의 `PlayOptions` 에 필드 하나를 `scratch_auto` 바로 뒤에 더한다.

```rust
/// The colour the notes SCRATCH AUTO plays are drawn in, on every skin and on the built-in field,
/// its alpha being how opaque they are. `None` is the shipped green.
pub scratch_auto_color: Option<SkinColor>,
```

- 기본값 `None`(`Default for PlayOptions` 에 `scratch_auto_color: None`). 구조체가 `#[serde(default)]` 라 이 필드가 없는 옛 `settings.ron` 은 그대로 읽힌다. 스키마 버전(`CURRENT_SCHEMA_VERSION`)은 올리지 않는다. 마이그레이션 없음.
- `sanitise` 는 손대지 않는다(u8 네 개라 범위 밖 값이 없다).
- 직렬화: `None` 은 RON 에 `scratch_auto_color: None,` 한 줄로 적힌다. 설정 업로드 계약 `settings-blob.json` 이 이 한 줄만큼 바뀐다(8절 M8-5). 계정 설정 묶음은 웹이 문자열로만 보관하므로 웹 변경은 없다. 옛 빌드가 새 묶음을 내려받으면 serde 파생이 모르는 필드를 버리므로 읽힌다.
- 리플레이·trace·점수 본문에는 싣지 않는다(표시 전용, 판정과 무관).

#### 3.1.2 그리기(rbms-render)

`crates/rbms-render/src/skin_render/notes.rs`:

```rust
pub const AUTO_LANE_NOTE: Color = Color { r: 0x30, g: 0xe0, b: 0x50, a: u8::MAX };
const AUTO_LONG_BODY_ALPHA: u8 = 0xa0;
pub const fn auto_long_body(note: Color) -> Color
pub const AUTO_LANE_LONG_BODY: Color = auto_long_body(AUTO_LANE_NOTE);
const fn auto_long_images(note: Color) -> [NoteImage; LONG_IMAGES]
```

- `auto_long_body(note)` = `Color { a: ((note.a as u16 * AUTO_LONG_BODY_ALPHA as u16) / u8::MAX as u16) as u8, ..note }`(`u16::from` 는 const 가 아니므로 `as` 로 넓힌다. 곱이 65,025 를 넘지 않아 u16 에 든다). 출하 초록은 255 × 160 / 255 = 160 = 0xA0 이라 지금의 `AUTO_LANE_LONG_BODY` 와 비트까지 같다.
- `AUTO_NOTE_IMAGE`·`AUTO_LONG_IMAGES` 상수를 지운다. `auto_long_images(note)` 는 지금 상수의 몸체(몸통 전부 `Solid(auto_long_body(note))`, `LONG_END`·`LONG_START`·`HELL_END`·`HELL_START` 는 `Solid(note)`)를 매개변수로 옮긴 것이다. `NoteImage` 는 `Copy` 라 반복 배열식이 const fn 안에서 된다.
- `LaneNotes` 에 `pub auto_note: Color` 를 `auto_lanes` 바로 뒤에 더하고 `LaneNotes::new` 에서 `auto_note: AUTO_LANE_NOTE`. 문서 주석의 `AUTO_LANE_NOTE` 언급을 "drawn in [`LaneNotes::auto_note`]" 로 고친다.
- `:1028-1031`: `self.put(r, &NoteImage::Solid(notes.auto_note), at, alpha, None)`.
- `:1099`: 함수 안에서 `let auto_images = auto_long_images(self.notes.auto_note);` 를 먼저 두고 `(&auto_images, None)`. 틴트는 지금처럼 `None`(색 행 틴트가 AUTO 노트에 닿지 않는 의미색 규칙 유지).
- `crates/rbms-render/src/skin_render/frame.rs:37` 의 `pub use super::notes::{…}` 에 `auto_long_body` 를 더한다.

`crates/rbms-render/src/playfield.rs`:

- `PlayfieldView` 에 `pub auto_note: Color`(`auto_lanes` 뒤). `:56` 의 구조 분해에 더한다.
- `:105` 몸통: `if plays_itself(lane) { auto_long_body(auto_note) } else { … }`.
- `:127` 노트: `_ if plays_itself(lane) => auto_note,`.
- `PlayfieldView` 를 필드 전부로 짓는 다섯 곳(2.1절 표)에 `auto_note: AUTO_LANE_NOTE` 를 넣는다. 앱(`stage/play/mod.rs:1698-1707`)만 설정 값을 넣는다.

불투명도 합성 규칙(결정 8 "알파 = 불투명도"): 노트 알파 = 사용자 알파 × 그리기 알파(스킨 노트 객체의 dst 알파 + 오프셋, 기본 스킨의 Note Opacity 행) / 255. 몸통 알파 = 사용자 알파 × 0xA0 / 255 에 같은 그리기 알파를 곱한다. 지금 `put()` 의 `faded` 가 하는 곱 그대로다.

#### 3.1.3 앱

`apps/rbms-player/src/app_play.rs`(`scratch_auto_lanes :153` 옆):

```rust
pub(crate) const SCRATCH_AUTO_SHIPPED_COLOR: SkinColor =
    SkinColor::rgba(AUTO_LANE_NOTE.r, AUTO_LANE_NOTE.g, AUTO_LANE_NOTE.b, AUTO_LANE_NOTE.a);
pub(crate) fn scratch_auto_color(play: &rbms_config::PlayOptions) -> SkinColor
```

- `scratch_auto_color` = `play.scratch_auto_color.unwrap_or(SCRATCH_AUTO_SHIPPED_COLOR)`. 초록 값을 다시 적지 않고 렌더러 상수에서 유도한다.
- `stage/play/mod.rs:1186-1196` 의 `LaneNotes { … auto_note: Color::from(scratch_auto_color(&shared.config.play)), … }`, `:1698-1707` 의 `PlayfieldView { … auto_note: …같은 식… }`, `stage/play/skin_preview.rs:777-787` 도 같은 식(미리보기 세션은 auto_lanes 가 비어 화소 변화 없음, 8절 M8-4). `Color::from(SkinColor)` 는 `crates/rbms-render/src/skin_render/mod.rs:82` 에 있다.

#### 3.1.4 PLAY 메뉴

행 배치(CHART 묶음): `… SCRATCH AUTO` → `SCRATCH AUTO COLOR`(색 행) → `SCRATCH AUTO CHANNELS`(접이, 닫힌 채 열림) → 펴면 `SCRATCH AUTO COLOR R`, `G`, `B`, `A`(0~255 숫자 행) → 그다음 CHART 의 나머지 행. SCRATCH AUTO 가 꺼져 있어도 행은 보인다(8절 M8-7).

`apps/rbms-player/src/stage/play_menu/rows.rs`:

- `Row` 에 변종 둘: `AutoColor`, `AutoChannel(ColorChannel)`.
- `Fold` 에 `AutoChannels` 를 더하되 `Fold::ALL`(스킨 묶음 접이 넷)에는 넣지 않는다. `label()` = `"SCRATCH AUTO CHANNELS"`, `help()` = 새 상수, `holds()` 는 `AutoChannels` 에 늘 `false`. `Folds` 에 `auto_channels: bool`, `is_open`/`toggle` 에 가지 추가.
- 상수: `SCRATCH_AUTO_COLOR_LABEL = "SCRATCH AUTO COLOR"`, `SCRATCH_AUTO_COLOR_HELP = "The color SCRATCH AUTO's notes are drawn in, on every skin. Its alpha is how opaque they are. ENTER types a hex color, 0 gives back the green"`, `SCRATCH_AUTO_CHANNELS_HELP = "The red, green, blue and alpha of SCRATCH AUTO's color. ENTER shows or hides the rows"`, 채널 행 도움말 `SCRATCH_AUTO_CHANNEL_HELP = "One channel of SCRATCH AUTO's color, 0 to 255"`.
- `listed()`: `tab == SettingTab::PlayChart` 일 때 `tab_rows` 를 넣은 뒤 `Row::Setting(SettingId::ScratchAuto)` 의 위치 바로 뒤에 `[Row::AutoColor, Row::Fold(Fold::AutoChannels)]` 와(열려 있으면) `ColorChannel::ALL.map(Row::AutoChannel)` 을 끼운다. `tab_rows` 가 ScratchAuto 를 내지 않는 설정(현재 없음)이면 묶음 끝에 붙인다.
- `number_of()`: `Row::AutoChannel(channel)` → `Some(NumberValue::of(CHANNEL_STEPS, i64::from(channel.get(current)), i64::from(channel.get(SCRATCH_AUTO_SHIPPED_COLOR))))`, `current = scratch_auto_color(&shared.config.play)`.
- `help()`: 두 변종과 새 접이에 위 상수.
- `is_read_only` 는 그대로(두 변종 모두 잠기지 않음).

`apps/rbms-player/src/skin_select.rs`(공통화, 두 번째 사용처가 생겨서):

- `pub(crate) fn preset_after(current: SkinColor, delta: i32) -> Option<SkinColor>`: `step_for` 의 `SkinRow::Color` 가지(`:1274-1288`)의 프리셋 찾기·한 칸·알파 유지를 그대로 옮기고, `step_for` 는 이 함수를 부른다(동작 불변).
- `CHANNEL_STEPS`, `ColorChannel::{label, get, set}`, `color_value` 를 `pub(crate)` 로.

`apps/rbms-player/src/stage/play_menu.rs`:

- `fn put_scratch_auto_color(&mut self, ctx: &mut FrameCtx<'_>, color: Option<SkinColor>)`: `ctx.shared.config.play.scratch_auto_color` 가 바뀌면 `self.preview.settings_changed(ctx.shared)`. 문서를 다시 읽지 않는다(엔진 값이라 다음 프레임에 반영). 파일 쓰기는 메뉴를 떠날 때의 기존 `save_settings`(`row_key` 의 Escape)와 끌기 끝의 저장이 한다.
- `step`: `Row::AutoColor` → `preset_after(current, delta)` 를 `Some` 으로 저장. `Row::AutoChannel(_)` → `number_of(…).stepped(delta, wide)` 를 `set_number`. `Row::Fold(Fold::AutoChannels)` 는 기존 접이 가지가 처리한다.
- `set_number`: `Row::AutoChannel(channel)` → `Some(channel.set(current, steps as u8))`(`steps` 는 `CHANNEL_STEPS` 범위 0~255 로 이미 묶여 온다. `u8::try_from(steps)` 가 실패하면 움직이지 않는다).
- `enter_focused`: `Row::AutoColor` → `self.numbers.begin_hex_edit(row, &hex_color(current))`.
- `commit_edit`: `Row::AutoColor` → `typed_color` 가 색이면 `put_scratch_auto_color(Some(color))`. 색이 아니면 기존처럼 행이 `NOT A COLOR` 를 잠시 말한다.
- `reset_focused`(0 키): `Row::AutoColor` → `None`(DEFAULT). `Row::AutoChannel(channel)` → 그 채널만 출하 초록의 채널 값으로(결과는 `Some`).
- `line`: `Row::AutoColor` → 라벨 `SCRATCH AUTO COLOR`, 값 `color_value(shared.config.play.scratch_auto_color)`(`DEFAULT` 또는 `#RRGGBBAA`). `Row::AutoChannel(channel)` → 라벨 `format!("{SCRATCH_AUTO_COLOR_LABEL} {}", channel.label())`. `Row::Fold(Fold::AutoChannels)` → 값 `fold_value(open, ColorChannel::ALL.len())`(`folded()` 를 거치지 않는다).
- `draw_rows` 의 `color_of`: `Row::AutoColor` → `Some(Color::from(current))`(저장값이 없으면 출하 초록 견본).
- `reveal_fold`(`:265-272`): 접이 아래 행을 셀 때 `Row::Skin(_) | Row::AutoChannel(_)` 을 함께 센다.
- `row_note`: 바꾸지 않는다.
- 위 함수들의 `match row` 가 남김없는 곳(`set_number`, `step`, `reset_focused`, `line`, `help_of` 를 거치는 `help`)에 두 변종을 넣는다. 지금 `Row::Group(_) | Row::PreviewBpm | Row::Fold(_) => false` 같은 가지에 묶지 않고 위 동작을 준다.

### 3.2 U-B: Esc 결과 화면(결정 9)

#### 3.2.1 rbms-play: 남은 노트를 접은 기록

`crates/rbms-play/src/record.rs` 의 `GaugeLogRecorder` 에:

```rust
pub(crate) fn closed_left(&self) -> GaugeLog
```

- `closed()` 와 같되 0 채움의 시작이 `failed_at_ms` 가 아니라 "다음에 찍을 표본의 시각" `self.log.len() as i64 * GAUGE_LOG_INTERVAL_MS` 이다. 끝은 `closed()` 와 같은 `play_time_ms + GAUGE_LOG_INTERVAL_MS`(상한 `GAUGE_LOG_PADDING_END_MS`). `failed_at_ms` 가 이미 있으면(게이지 사망 뒤) `closed()` 를 그대로 돌려준다. 원본 `BMSPlayer.java:722-727` 이 Esc 로 FAILED 가 된 판에도 0 을 채우는 것과 같다(8절 M9-5).

`crates/rbms-play/src/session.rs`(`record() :872` 옆):

```rust
pub fn left_record(&self) -> PlayRecord
```

- `PlayRecord { summary: self.summary().left_unfinished(), gauge_log: self.gauge_log.closed_left(), ..self.record() }`. `failed_at_ms` 는 `None` 그대로(게이지가 끝낸 판이 아니다).
- 문서 주석에 "the record of a run left before its end: the summary [`PlaySummary::left_unfinished`] gives, and the gauge history padded with zeroes from where it stopped, as the reference pads a run it failed" 를 적는다.

#### 3.2.2 app_result: 결과 화면 한 길

> 구현과 다름: `record_run`(이력 행·전송 본문)은 `left_record` 가 아니라 `play.record()` 로 샘플된 그대로의 게이지 이력을 쓴다. 0 으로 채운 이력은 결과 화면용 `keep_run_record` 에만 쓴다. 판정 trace 도 샘플된 그대로다([ADR](../acknowledge/2026-10-23-adr-escape-shows-the-result.md)).

`apps/rbms-player/src/app_result.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunEnd {
    Reached,
    LeftPartWay,
}

pub(crate) fn enter_result(state: &mut PlayState, shared: &mut AppShared) -> Transition
pub(crate) fn enter_result_of_a_left_run(state: &mut PlayState, shared: &mut AppShared) -> Transition
fn show_result(state: &mut PlayState, shared: &mut AppShared, end: RunEnd) -> Transition
```

- `enter_result` = `show_result(state, shared, RunEnd::Reached)`, `enter_result_of_a_left_run` = `show_result(state, shared, RunEnd::LeftPartWay)`. 지금의 `enter_result` 호출처(10곳 남짓)는 손대지 않는다.
- `show_result` 는 지금 `enter_result` 의 몸체를 그대로 옮기고 첫 줄만 바꾼다: `let (summary, record) = match end { RunEnd::Reached => (play.summary(), play.record()), RunEnd::LeftPartWay => (play.summary().left_unfinished(), play.left_record()) };`. 이후 `record_run(state, shared, &summary, &record)`, `ResultView.counts = summary.counts`, `keep_run_record(shared, record)` 가 모두 같은 접은 값을 쓴다. 그래서 결과 화면의 판정 수·스킨 결과 화면(`ResultRun::of` 가 `run_records.last()` 를 읽음)·DB 행·전송 본문이 같은 수다.
- 전송은 `show_result` 안의 결과 화면 전송 하나뿐이다. 이 경로는 `send_without_a_screen`(기록 동기화 시작)을 부르지 않는다. 이중 전송 없음.
- 코스: `show_result` 의 기존 코스 가지가 그대로 돈다. 접은 요약은 `failed = true` 라 `fold_course_stage` 가 `survived: false` 를 넘기고 `CourseRun::advance` 가 `Failed` 를 돌려주며 `finish_course` 가 코스 결과(불합격)를 띄우고 코스를 남기고 보낸다(`keep_finished_course(Sending::Now)`, 한 길). 스테이지 점수는 같은 함수의 결과 화면 전송이 보낸다.
- 지운다: `record_left_run`, `leave_course`, `LEFT_RUN_KEPT`, `LEFT_COURSE_KEPT`(호출처가 없어지고, 화면이 알림을 대신한다. 8절 M9-4). `send_without_a_screen` 은 `record_retried_run` 이 쓰므로 남긴다. `abandon_course` 의 문서 주석 중 `[`leave_course`]` 링크를 "as a course left part-way with Escape ends on its result" 로 고친다.
- `RunEnd` 의 `Reached` 는 끝까지 친 판, 게이지 사망, 닫히는 장면에서의 Esc 를 모두 덮는다(남은 노트를 접지 않는다). `LeftPartWay` 는 남은 노트가 있는 채 사람이 나간 판이다.

#### 3.2.3 플레이 화면: `leave_run`

`apps/rbms-player/src/stage/play/mod.rs:690-728`(문서 주석과 몸체만):

```rust
fn leave_run(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
    self.esc_down_at = None;
    self.esc_pressed_at = None;
    ctx.shared.play_system_sound(SystemSound::PlayStop);
    ctx.shared.save_settings();
    if self.practice.is_some() {
        return Transition::Back;
    }
    if self.has_ended() {
        return enter_result(self, ctx.shared);
    }
    let in_a_course = ctx.shared.course_run.is_some();
    if !keeps_a_left_run(ctx.shared) {
        return if in_a_course { crate::end_course(ctx.shared) } else { ctx.shared.leave_play() };
    }
    if self.has_judged_nothing() {
        return if in_a_course { crate::app_result::abandon_course(ctx.shared, crate::app_result::Sending::Now) } else { ctx.shared.leave_play() };
    }
    ctx.shared.stop_chart_sounds(self.session.model());
    crate::app_result::enter_result_of_a_left_run(self, ctx.shared)
}
```

- 순서는 지금과 같다: 연습 → 끝났거나 닫히는 판 → 남기지 않는 판(오토플레이·리플레이 보기: 지금의 `leave_course` 첫 줄과 단일 곡의 `keeps_a_left_run` 거짓 가지) → 친 노트 0(지금과 같은 처리) → 새 가지.
- 새 가지는 닫힘 연출 없이 곧장 결과로 간다. 소리: PlayStop 은 위에서 이미 울렸고 `stop_chart_sounds` 로 곡 소리를 끊는다(장면의 실패 가지 `:1417-1420` 과 같은 두 가지). 타이머: 결과 장면이 `start_skin_scene_clock` 으로 비운다(2.2절). BGA·로드: 플레이 화면 `on_exit` 이 한다. 키음 네임스페이스 해제(`release_play_audio`)는 결과 화면이 끝나고 선곡·다음 판으로 갈 때 지금의 결과 화면 경로가 하던 대로 한다.
- 장면 경로(`scene_escape`)·길게 누르기 경로(`:1430`, `:1554`)는 모두 `leave_run` 을 거치므로 따로 고칠 곳이 없다.
- 문서 주석 `:690-708` 을 새 동작으로 고쳐 쓴다(Escape 는 결과 화면을 거친다, 이유는 사용자 결정 2026-10-23 결정 9).
- 로그인 여부를 보지 않는다(결정 9 "로그인과 상관없이"). 로그인하지 않았거나 서버가 없으면 결과 화면의 전송이 지금처럼 `IrStatus::Off`·건너뜀이 되고, 남은 판은 기록 동기화가 나중에 보낸다(DB 에는 `record_run` 이 늘 남긴다).

앱 종료(`keep_run_cut_short_by_quitting`)와 코스 스테이지 사이 이탈(`abandon_course`)은 바꾸지 않는다.

### 3.3 U-C: 패널 키 스킨 opt-in(결정 5)

#### 3.3.1 헤더 표시

형식(8절 M5-1): 문서 최상위 정수 키 `rbms_extensions`. 값 1 이상이면 "이 스킨은 R-BMS 확장 id(이벤트 21000~21099, 불린 21100~21199)와 그에 딸린 선곡 패널 키 재배정을 안다". 없거나 0 이면 모른다. JSON 은 `"rbms_extensions": 1`, Lua 는 헤더 패스가 돌려주는 표의 `rbms_extensions = 1`. beatoraja 는 두 형식 모두 모르는 키를 버린다(2.3절). 새 id 는 만들지 않는다.

`crates/rbms-skin/src/model.rs`:

```rust
pub const NO_RBMS_EXTENSIONS: i32 = 0;
pub const RBMS_EXTENSIONS_PANEL_KEYS: i32 = 1;
```

- `SkinDef` 에 `pub rbms_extensions: i32`(`color` 다음, serde 이름은 필드 이름 그대로 `rbms_extensions`), `Default` 에 `rbms_extensions: NO_RBMS_EXTENSIONS`.
- `impl SkinDef { pub fn knows_rbms_extensions(&self) -> bool { self.rbms_extensions >= RBMS_EXTENSIONS_PANEL_KEYS } }`(문서 주석 영어: "Whether the document says it knows this player's own ids, which is what the option panels' reassigned keys are read by").
- JSON 은 serde 파생이 읽는다(`from_value`). Lua 는 `skin_def_from_lua` 의 `Reader` 가 강제 변환으로 읽는다.

`crates/rbms-skin/src/loader/lua_skin.rs:186-191`: `def.name = header.name;` 줄들 옆에 `def.rbms_extensions = header.rbms_extensions;` 를 더한다(헤더 패스 값이 이긴다, 이름·작성자와 같은 규칙). `SkinHeader`(스킨 목록용)에는 더하지 않는다(읽는 곳이 없다).

기본 스킨: `assets/skins/rbms-default/lua/props.lua` 에 `props.rbms_extensions = 1` 을, `assets/skins/rbms-default/lua/select/main.lua` 의 `screen.header()` 표에 `rbms_extensions = props.rbms_extensions,` 를 넣는다. 다른 화면 문서(play·result 등)에는 넣지 않는다(읽는 곳이 선곡뿐이다, 8절 M5-3).

#### 3.3.2 키 표

`apps/rbms-player/src/stage/select/keys.rs`:

- `SelectKey` 에 원본 역할 일곱을 되살린다: `OptionDpUp`, `OptionDpDown`, `Option2Up`, `JudgeArea`, `MarkNote`, `BpmGuide`, `NoMine`(각 변종에 영어 `///`: 원본 이름과 "on a skin that does not know this player's ids" 일 때만 키에 있다는 것).
- 원본 표 둘을 더한다. 값은 커밋 `7746932^` 의 `BEAT_TABLE`·`POPN_TABLE` 그대로다(2.3절):

```rust
const REFERENCE_BEAT_TABLE: KeyTable
const REFERENCE_POPN_TABLE: KeyTable
```

- `KeyLayout` 을 넷으로: `Beat`, `Popn`(지금의 확장 표), `ReferenceBeat`, `ReferencePopn`(원본 표). `table()` 이 각 표를 돌려준다.

```rust
pub(super) fn of(mode: Mode, knows_extensions: bool) -> KeyLayout
pub(super) fn of_browser(shared: &AppShared) -> KeyLayout
```

- `of` = 턴테이블 유무 × `knows_extensions`. `of_browser` = `KeyLayout::of(shared.mode, shared.skins.document(SKIN_TYPE_MUSIC_SELECT).is_some_and(|skin| skin.def.knows_rbms_extensions()))`. 선곡 문서가 아직 없으면(읽는 중) 원본 표다. 그동안은 스킨 선곡의 입력이 돌지 않는다.
- 누름 목록을 두 표의 합집합으로(원본 묻는 순서 그대로, 표에 없는 역할은 `pressed` 가 거짓이라 안전):
  - `PLAY_PANEL_PRESSES: [SelectKey; 10] = [Option1Down, Option1Up, GaugeDown, GaugeUp, OptionDpDown, OptionDpUp, Option2Down, Option2Up, HsFixDown, HsFixUp]`(2026-10-22 이전 목록과 같다)
  - `ASSIST_PANEL_PRESSES: [SelectKey; 12] = [JudgeWindowUp, Constant, JudgeArea, ScratchAuto, LegacyNote, MarkNote, KeepGreenNumber, BpmGuide, LaneCover, NoMine, Lift, Hidden]`
  - `DETAIL_PANEL_PRESSES` 는 그대로.
- 원본 표에서는 패널 2·3 의 턴테이블이 어떤 역할도 갖지 않는다(인덱스 7·8 은 `Up`/`TargetUp`, `Down`/`TargetDown` 뿐). 패널이 열린 동안 목록은 멈추므로 아무 일도 없다. `forget_turntable_changes` 는 그대로 둔다(원본 표에서는 영향 없음).

`apps/rbms-player/src/stage/select/panel.rs` 의 `press_act`:

- `OptionDpDown → setting(SettingEvent::DoubleOption, NEXT)`, `OptionDpUp → setting(SettingEvent::DoubleOption, PREVIOUS)`, `Option2Up → setting(SettingEvent::Random, PREVIOUS)`(원본 역할, 2026-10-22 이전 동작. 8절 M5-2).
- `JudgeArea | MarkNote | BpmGuide | NoMine → None`(죽은 칸: 누름은 패널이 가져가고 아무 일도, 소리도 없다. 2026-10-22 이전과 같다). `AssistSwitch` 에 변종을 되살리지 않는다.
- 모듈 문서 주석(`:16-24`)에 "the keys are the reference's table on a skin that does not say it knows this player's ids, and the reassigned table on one that does" 를 더한다.

호출처:

- `stage/select/skinned.rs:268`: `let layout = KeyLayout::of_browser(shared);`.
- `stage/select/guide.rs:242`(`key_rows`): `KeyLayout::of_browser(shared)`. `panel_rows`(`:225-237`): 마지막 두 줄(`SELECT + TURNTABLE`, `START+SELECT + TURNTABLE`)은 `KeyLayout::of_browser(shared)` 가 확장 표일 때만 넣는다(`KeyLayout::is_extended(self) -> bool` 을 더한다).
- `BUILT_IN_KEY_LAYOUT`(`mod.rs:105`)은 그대로(목록 역할은 두 표가 같다).

5키·10키 흐림 불린(21140~21148)과 그 밖의 확장 id 응답은 바꾸지 않는다. 외부 스킨에서도 확장 불린은 계속 답한다(읽는 스킨이 없을 뿐).

## 4. 원본 대비 차이 표

"판정·점수" 열이 "예" 인 줄이 판정·점수 의미가 원본과 다른 곳이다.

| 항목 | R-BMS(이번 설계 뒤) | 원본(beatoraja) | 판정·점수 | 근거 |
| --- | --- | --- | --- | --- |
| SCRATCH AUTO 노트 그리기 | 엔진이 단색으로 그리고 색은 PLAY 전역 RGBA(기본 `#30E050`, 몸통 알파 × 0xA0/255) | 그리지 않는다(배경음으로 옮김, `AutoplayModifier.modify`) | 아니오(표시만) | 결정 8. 기존 차이(play-options.md 10.5)의 색만 사용자 값 |
| Esc 로 남은 노트가 있는 판의 결과 화면 | 결과 화면을 거친다(새로 원본과 같아짐) | `STATE_FAILED` → close 뒤 RESULT | 아니오 | 결정 9 |
| 결과 화면까지 가는 연출 | 즉시(닫힘 연출 없음) | FAILED 닫힘 연출(`skin.getClose()` ms) 뒤 | 아니오 | 결정 9 "진입은 즉시" |
| 결과 화면·저장의 판정 수 | 남은 노트를 늦은 POOR 로 센다(PG~POOR 합 = 노트 수) | 남은 노트는 판정 수에 넣지 않고 BP(`minbp`)에만 더한다(`BMSPlayer.java:909`) | **예**(기존 차이 유지. 2026-10-14 사용자 결정, [판정 수 ADR](../acknowledge/2026-10-14-adr-judge-count-consumed-notes.md)) | 결정 9 "결과 판정 수는 저장값과 같게" |
| Esc 판의 게이지 그래프 | 나간 자리부터 끝까지 0 을 채운다(M9-5 추천) | 같다(`:722-727`) | 아니오 | 원본 |
| 코스 도중 Esc | 그 스테이지를 FAILED 로 접고 코스 결과(불합격)까지(새로 원본과 같아짐). 아직 치지 않은 스테이지의 노트를 코스 BP 에 더하지 않는 것은 그대로 | 코스 결과까지. 남은 곡 노트를 BP 에 더한다(`MusicResult.java:183-196`) | **예**(코스 BP. 기존 차이 유지, 테스트 부채) | 결정 9 |
| 노트를 하나도 치지 않은 판의 Esc(단일 곡) | 아무것도 남기지 않고 선곡 | `createScoreData` 가 null → 선곡 | 아니오(같다) | 2026-10-19 사용자 결정 |
| 코스에서 친 노트 0 인 스테이지의 Esc | 지금처럼 그 스테이지를 접지 않고 앞 스테이지까지의 FAILED 코스를 남겨 보내고 선곡(화면 없음, M9-2) | 코스는 null 이 되지 않아 결과 → 코스 결과 | 아니오(기록 범위는 기존 결정) | 결정 9 "지금처럼" |
| 오토플레이·리플레이 보기 판의 도중 Esc | 지금처럼 선곡(M9-1) | 오토플레이는 `STATE_FINISHED` → 결과, 리플레이는 FAILED → 결과 | 아니오(기록 없음) | 결정 9 의 대상은 사람이 친 판 |
| 확장 표시 없는 스킨의 패널 키 | 원본 `MusicSelectKeyProperty` 표(NEXT_REPLAY 만 없음). 죽은 칸 넷은 무동작, 패널 1 키 4 는 DP OPTION 이전/다음, 키 6 은 RANDOM 이전, 패널 2·3 턴테이블 무동작 | 표 그대로. 죽은 칸은 원본 설정(판정 영역 표시 등)을 뒤집고, DP OPTION 은 플레이에 적용 | 아니오 | 결정 5. DP OPTION 은 R-BMS 가 적용하지 않는 기존 차이 |
| 확장 표시 있는 스킨(기본 스킨)의 패널 키 | 지금의 확장 표 | 해당 없음 | 아니오 | 2026-10-22 ADR |
| 헤더 키 `rbms_extensions` | R-BMS 확장 헤더 키 | 모르는 키로 버린다 | 아니오 | 결정 5 |

## 5. 구현 단위

세 단위는 서로 기다리지 않는다(병렬 가능). 각 단위의 작업자는 커밋·푸시하지 않고, `rbms-player` 를 실행해야 하면 scratch HOME 과 `--server http://127.0.0.1:9` 로만 한다(HANDOFF 5절).

### 5.1 U-A: SCRATCH AUTO 색(결정 8)

| 항목 | 내용 |
| --- | --- |
| 소유 파일(수정 가능) | `crates/rbms-config/src/schema.rs`, `crates/rbms-config/src/tests.rs`, `crates/rbms-render/src/skin_render/notes.rs`, `crates/rbms-render/src/skin_render/notes/tests.rs`, `crates/rbms-render/src/skin_render/frame.rs`(re-export 한 줄), `crates/rbms-render/src/playfield.rs`, `crates/rbms-render/tests/golden_play.rs`(구조체 필드 한 줄), `crates/rbms-render/examples/render_frame.rs`(필드 한 줄), `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/stage/play/mod.rs`(**`:1186-1196` 과 `:1698-1707` 두 블록만**), `apps/rbms-player/src/stage/play/skin_preview.rs`(`:777-787` 만), `apps/rbms-player/src/skin_select.rs`, `apps/rbms-player/src/stage/play_menu.rs`, `apps/rbms-player/src/stage/play_menu/rows.rs`, `apps/rbms-player/src/stage/play_menu/tests.rs`, 새 파일 `apps/rbms-player/src/stage/play_menu/tests/scratch_auto_color.rs`, `apps/rbms-player/src/stage/play/screen_tests/default_pack_tests.rs`, `contract/client-requests/settings-blob.json`(재생성) |
| 읽기 전용 | `crates/rbms-skin/src/dst.rs`(`SkinColor`), `crates/rbms-skin/src/loader.rs`(직렬화), `apps/rbms-player/src/number_row.rs`, `apps/rbms-player/src/stage/preview_menu.rs`, `apps/rbms-player/src/stage/play_menu/tests/colors.rs`(패턴 참고), `apps/rbms-player/src/app_result/record_tests/contract_requests.rs` |
| 선행 단위 | 없음 |
| 다른 묶음과 겹치는 파일 | `apps/rbms-player/src/stage/play/mod.rs`(U-B 와. 줄 범위가 다르다), `apps/rbms-player/src/skin_select.rs` 는 F4(없는 곡) 묶음이 손대는지 통합 때 확인(이 설계서 밖). PLAY 메뉴 `rows.rs`·`play_menu.rs` 는 F5(코스 게이지 설정)가 PLAY 메뉴에 행을 더하면 겹친다(그 설계서와 통합 순서 조정) |
| 병렬 | U-B·U-C 와 병렬 |
| 순서 | 1) rbms-config 필드·테스트 → 2) rbms-render 상수·필드·그리기·테스트 → 3) 앱의 세 곳에 값 연결 → 4) skin_select 공통화(동작 불변 먼저 확인: 기존 `skin_select/color_tests.rs` 통과) → 5) PLAY 메뉴 행·조작·테스트 → 6) 화면 테스트 → 7) `RBMS_UPDATE_CONTRACT=1` 로 `settings-blob.json` 재생성 후 diff 가 `scratch_auto_color: None,` 한 줄인지 확인 |
| 금지·엣지 | 색 행 틴트(`NoteLane.tint`)를 AUTO 노트에 닿게 하지 않는다. 다른 노트·지뢰·스킨 색을 엔진이 칠하지 않는다. 감마 보정 없음. `Option` 이 `None` 일 때 화소가 지금과 같아야 한다(모든 기존 화소 단언이 그대로 통과). 프리셋에 없는 사용자 색에서 → 는 첫 프리셋, ← 는 끝 프리셋(기존 규칙) |

### 5.2 U-B: Esc 결과 화면(결정 9)

| 항목 | 내용 |
| --- | --- |
| 소유 파일 | `crates/rbms-play/src/record.rs`, `crates/rbms-play/src/record/tests.rs`, `crates/rbms-play/src/session.rs`(`left_record` 추가만), `apps/rbms-player/src/app_result.rs`, `apps/rbms-player/src/stage/play/mod.rs`(**`leave_run` 과 그 문서 주석 `:690-728` 만**), `apps/rbms-player/src/app_result/record_tests.rs`, `apps/rbms-player/src/app_result/record_tests/contract_requests.rs`, `apps/rbms-player/src/stage/play/screen_tests/run_tests.rs`(새 테스트 하나) |
| 읽기 전용 | `apps/rbms-player/src/stage/result.rs`, `apps/rbms-player/src/skin_host/result/snapshot.rs`, `apps/rbms-player/src/stage/course_result.rs`, `crates/rbms-course/src/run.rs`, `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/lib.rs`(`end_course :310`), `contract/client-requests/score-abandoned.json`·`trace-abandoned.json`(내용이 바뀌면 안 된다) |
| 선행 단위 | 없음 |
| 다른 묶음과 겹치는 파일 | `apps/rbms-player/src/stage/play/mod.rs`(U-A 와). `apps/rbms-player/src/app_result.rs` 는 F5(코스 제출에 게이지 종류, 코스 게이지 설정)도 손댈 가능성이 크다: `fold_course_stage`·`keep_finished_course`·`build_course_submission` 쪽. 이 단위는 `enter_result`→`show_result`, `record_left_run`·`leave_course` 삭제, `abandon_course` 문서 주석만 바꾼다. F5 와 같은 웨이브로 돌리면 통합 순서를 정한다(5.4절) |
| 병렬 | U-A·U-C 와 병렬 |
| 순서 | 1) rbms-play `closed_left`·`left_record` 와 테스트 → 2) app_result `RunEnd`·`show_result`·삭제 → 3) `leave_run` → 4) 뒤집히는 테스트 고치기·새 테스트 → 5) 계약 테스트 `a_run_left_mid_song` 을 결과 화면 전송으로 고치고 `score-abandoned.json`·`trace-abandoned.json` 이 **재생성 없이** 통과하는지 확인(바뀌면 원인을 찾는다: 같은 `record_run` 본문이어야 한다) |
| 금지·엣지 | 판 제외 조건을 새로 만들지 않는다(친 노트 0 만 제외). 기록 동기화와 결과 화면 전송을 둘 다 타지 않는다. 앱 종료 경로·스테이지 사이 이탈은 바꾸지 않는다. 일반 LN 머리를 쥔 채 나간 판(PG~BD 0 이지만 친 판)은 결과 화면으로 간다. 남은 노트가 없거나 닫히는 판은 지금처럼 `Reached` |

### 5.3 U-C: 패널 키 opt-in(결정 5)

| 항목 | 내용 |
| --- | --- |
| 소유 파일 | `crates/rbms-skin/src/model.rs`, `crates/rbms-skin/src/loader/lua_skin.rs`, `crates/rbms-skin/tests/skin_loader.rs`, `crates/rbms-skin/tests/skin_luaskin.rs`, `apps/rbms-player/src/stage/select/keys.rs`, `apps/rbms-player/src/stage/select/panel.rs`, `apps/rbms-player/src/stage/select/skinned.rs`(`:268` 한 줄), `apps/rbms-player/src/stage/select/guide.rs`, `apps/rbms-player/src/stage/select/panel_tests.rs`, `apps/rbms-player/src/stage/select/skinned_tests.rs`(`wheel_document()` 에 표시 한 줄), `apps/rbms-player/src/stage/select/overlay_tests.rs`, `apps/rbms-player/src/skin_select/default_pack_tests.rs`, `assets/skins/rbms-default/lua/props.lua`, `assets/skins/rbms-default/lua/select/main.lua` |
| 읽기 전용 | `crates/rbms-skin/src/loader.rs`, `crates/rbms-skin/src/loader/from_lua.rs`, `apps/rbms-player/src/stage/select/events.rs`, `apps/rbms-player/src/stage/select/mod.rs`, `apps/rbms-player/src/skin_host/**`, `/Users/hyunseokbyun/development/beatoraja/**`(읽기만), `/Users/hyunseokbyun/Downloads/ModernChic`(읽기만, 복사 금지) |
| 선행 단위 | 없음 |
| 다른 묶음과 겹치는 파일 | `apps/rbms-player/src/stage/select/skinned.rs`·`skinned_tests.rs` 는 F3(폴더 스택)이 크게 손댈 파일이다. 이 단위는 `skinned.rs:268` 한 줄과 픽스처 문서 한 키만 바꾼다. `crates/rbms-skin/src/model.rs` 는 다른 묶음이 손대지 않는 것으로 안다(통합 때 확인) |
| 병렬 | U-A·U-B 와 병렬. F3 와 같은 웨이브면 이 단위를 먼저 합친다(한 줄 충돌만 난다) |
| 순서 | 1) rbms-skin 필드·Lua 복사·테스트 → 2) keys.rs 변종·원본 표·`KeyLayout`·테스트 → 3) panel.rs `press_act` → 4) skinned.rs·guide.rs 호출 → 5) 픽스처 두 문서(`panel_document`, `wheel_document`)에 `"rbms_extensions": 1` 을 넣어 지금 테스트가 확장 표를 계속 시험하게 한 뒤 원본 표 테스트를 새로 더함 → 6) 기본 스킨 헤더 → 7) 기본 팩 테스트 |
| 금지·엣지 | 원본 id·램프 id 재사용 금지, 새 확장 id 없음. 표시가 없을 때 확장 불린 응답을 끄지 않는다. 5키·10키 흐림 불린은 그대로. 사용자가 고친 기본 팩 `select/main.lua` 사본은 표시가 없어 원본 표가 된다(남은 위험 9절) |

### 5.4 겹침과 통합 순서

- `apps/rbms-player/src/stage/play/mod.rs`: U-A 는 `:1186-1196`·`:1698-1707`(값 연결 두 줄), U-B 는 `:690-728`(`leave_run`). 헝크가 수백 줄 떨어져 있어 git 이 자동으로 합친다. 통합 순서는 U-C → U-A → U-B(가장 작은 것부터)로 하고, 각 병합 뒤 `cargo check --locked -p rbms-player` 한 번.
- 같은 웨이브에 F3·F5 가 있으면: U-C 를 F3 보다 먼저, U-B 를 F5 보다 먼저 합친다(F5 가 `enter_result` 를 쓰는 쪽이 새 이름 `show_result` 를 따라가게).

## 6. 테스트 계획

### 6.1 U-A 새 테스트

| 파일 | 테스트 이름 | 단언 |
| --- | --- | --- |
| `crates/rbms-render/src/skin_render/notes/tests.rs` | `the_shipped_scratch_auto_colours_are_the_ones_drawn_before` | `AUTO_LANE_NOTE == Color { r: 0x30, g: 0xe0, b: 0x50, a: 255 }`, `AUTO_LANE_LONG_BODY == Color { r: 0x30, g: 0xe0, b: 0x50, a: 0xa0 }`, `auto_long_body(AUTO_LANE_NOTE) == AUTO_LANE_LONG_BODY` |
| 같은 파일 | `a_long_body_keeps_its_share_of_the_alpha_it_is_given` | `auto_long_body(Color { a: 0x80, .. }).a == 0x50`(128 × 160 / 255 = 80), `a: 0` 이면 0, 색 채널은 그대로 |
| 같은 파일 | `a_note_the_chart_plays_itself_is_drawn_in_the_colour_it_is_given` | 기존 `a_note_the_chart_plays_itself_is_drawn_green_and_leaves_at_the_line`(`:410`) 픽스처로 `LaneNotes { auto_lanes: &auto, auto_note: MAGENTA(FF40C0FF), .. }` 를 그리면 `span(&canvas, middle(0), MAGENTA) == Some((200, 210))`, 초록 화소는 없다 |
| 같은 파일 | `a_long_note_the_chart_plays_itself_takes_the_colour_end_to_end` | `:434` 픽스처로 머리·끝 화소가 주어진 색, 몸통 화소는 그 색 계열(g·b 관계 단언)이고 주어진 색과 다르다 |
| `crates/rbms-render/src/playfield.rs`(tests) | `a_note_the_chart_plays_itself_on_the_built_in_field_takes_the_colour_it_is_given` | `:321-331` 과 같은 필드에 `auto_note: BLUE(4060FFFF)` → 그 레인 화소 == BLUE |
| `crates/rbms-config/src/tests.rs` | `a_settings_file_without_a_scratch_auto_colour_holds_the_shipped_one` | `ron::from_str("(play: (scratch_auto: true))")` 의 `play.scratch_auto_color == None`. `Config::default().play.scratch_auto_color == None` |
| 같은 파일 | 기존 왕복 테스트(`:340-350` 부근)에 `c.play.scratch_auto_color = Some(SkinColor::rgba(255, 64, 192, 128))` 를 더해 왕복 단언 |
| `apps/rbms-player/src/app_play.rs`(tests 모듈, 없으면 그 파일 테스트 모듈을 쓰는 기존 파일) | `scratch_auto_colour_is_the_shipped_green_until_one_is_set` | `scratch_auto_color(&PlayOptions::default()) == SCRATCH_AUTO_SHIPPED_COLOR == SkinColor::rgba(0x30, 0xe0, 0x50, 255)`, 설정하면 그 값 |
| `apps/rbms-player/src/stage/play_menu/tests/scratch_auto_color.rs`(새, `tests.rs:1373-1377` 에 `mod scratch_auto_color;`) | `the_scratch_auto_colour_row_follows_scratch_auto_with_its_channels_folded` | CHART 묶음에서 `Row::Setting(ScratchAuto)` 다음이 `Row::AutoColor`, 그다음 `Row::Fold(Fold::AutoChannels)`, 접이 값 `HIDDEN (4)`, 채널 행 없음. 접이에서 Enter → `SHOWN (4)` 와 R·G·B·A 네 행 |
| 같은 파일 | `the_scratch_auto_colour_row_reads_default_until_a_colour_is_set` | 값 `DEFAULT`, 견본 색 = `#30E050FF` |
| 같은 파일 | `the_arrows_go_round_the_presets_from_the_shipped_green_keeping_the_alpha` | → 한 번에 `Some(40E0FF FF)`(프리셋 9 → 10), ← 두 번에 `Some(A0FF40 FF)`. 채널 A 를 0x80 으로 둔 뒤 → 이면 알파 0x80 유지 |
| 같은 파일 | `a_typed_hex_colour_is_stored_and_one_that_is_no_colour_is_refused` | Enter, `FF000080` 입력, Enter → `Some(rgba(255,0,0,128))`. `XYZ` 는 저장 안 되고 행이 refused |
| 같은 파일 | `the_zero_key_gives_the_colour_back_and_a_channel_its_shipped_value` | 색 행 0 → `None`. 저장값이 있을 때 채널 G 에서 0 → G 만 0xE0 |
| 같은 파일 | `a_channel_row_moves_one_channel_of_the_colour` | 채널 R → 한 번: `Some(rgba(0x31, 0xe0, 0x50, 255))`, Shift → 열 칸 |
| `apps/rbms-player/src/stage/play/screen_tests/default_pack_tests.rs` | `the_turntable_scratch_auto_plays_is_drawn_in_the_colour_the_settings_hold` | `the_turntable_scratch_auto_plays_is_drawn_green_and_counts_for_nothing`(`:395`)와 같은 판에 `config.play.scratch_auto_color = Some(rgba(0xFF, 0x40, 0xC0, 255))` → 그 색 화소 > 0, `is_auto_lane_green` 화소 == 0. 기본 팩과 `RBMS_SKIN_PACK` 팩 모두 |

기존 테스트 중 뒤집히는 것: 없음. 고쳐야 하는 것: `apps/rbms-player/src/stage/play_menu/tests.rs` 의 `rows_without_skin_options()`(`:338-346`)가 CHART 의 행 목록을 정확히 짓는다. `CHART_ROWS` 를 넣은 뒤 `ScratchAuto` 다음에 `Row::AutoColor`, `Row::Fold(Fold::AutoChannels)` 를 끼우게 고친다(이를 쓰는 테스트들이 그대로 통과해야 한다). `every_row_of_the_play_menus_groups_is_listed_once_and_no_other`(`:400`)는 `Row::Setting` 만 세므로 그대로다.

골든·계약: 골든 시그니처는 바뀌지 않는다(`golden_play.rs` 는 `auto_lanes: &[]`, 필드 한 줄만 더함). `contract/client-requests/settings-blob.json` 은 `scratch_auto_color: None,` 한 줄이 늘어난다(재생성, M8-5). 웹 `web/tests/dto/client-requests.test.ts` 는 이 파일의 `content` 를 문자열로 다루는지 확인하고(통과 여부만) 웹 코드는 고치지 않는다.

### 6.2 U-B 새 테스트와 뒤집히는 테스트

| 파일 | 테스트 이름 | 단언 |
| --- | --- | --- |
| `crates/rbms-play/src/record/tests.rs` | `a_history_closed_as_left_falls_to_zero_from_where_it_stopped` | 표본 n 개 뒤 `closed_left()` 의 각 게이지 길이 = `closed()` 가 같은 시각 사망에 내는 길이, 마지막 표본들은 0. 게이지 사망 뒤면 `closed()` 와 같다 |
| `crates/rbms-play/src/session.rs` 테스트 모듈(또는 `crates/rbms-play/tests/` 의 세션 테스트 파일) | `a_record_left_part_way_counts_the_rest_as_poor` | 중간까지 친 세션의 `left_record().summary == summary().left_unfinished()`, `failed_at_ms == None`, 게이지 로그가 0 으로 채워짐 |
| `apps/rbms-player/src/app_result/record_tests.rs` | `leaving_mid_song_shows_a_failed_result_with_the_counts_it_keeps`(기존 `leaving_mid_song_keeps_a_failed_run_with_the_rest_as_poor` 를 이 이름으로 바꾸고 뒤집음) | `matches!(left, Transition::To(Stage::Result(_)))`. DB 단언은 그대로. 추가로 결과 화면이 읽는 기록 `shared.run_records.last().summary.counts`(스킨 결과 화면의 `ResultRun::of` 가 읽는 값)가 DB 행의 단계별 판정 수와 같고, POOR 칸이 `summary.counts[4] + unreached` 다(`stage/result.rs` 는 U-B 읽기 전용이라 접근자를 더하지 않는다) |
| 같은 파일 | `leaving_mid_song_signed_in_sends_the_run_once_from_its_result`(기존 `leaving_mid_song_signed_in_sends_the_run` 대체) | 나간 직후 `shared.submit_rx.is_some()`, `shared.record_sync.is_none()`. 답이 온 뒤 `server.sent_for == [ACCOUNT_ID]`(한 번), 그 판의 동기화 상태 `Accepted` |
| 같은 파일 | `leaving_mid_song_signed_out_still_shows_its_result` | 로그인하지 않은 `signed_out` 에서 `Transition::To(Stage::Result(_))`, DB 행 1, `submit_rx.is_none()` |
| 같은 파일 | `leaving_mid_course_ends_on_a_failed_course_result`(기존 `leaving_mid_course_keeps_the_stage_and_a_failed_course` 대체) | `matches!(leaving, Transition::To(Stage::CourseResult(_)))`, `shared.course_run` 이 `Some` 이고 `failed_at.is_some()`(코스 결과 화면이 쥔다), 스테이지 행 1(FAILED, minbp = `left.min_bp_with_unreached()`), 코스 전송 1회와 그 본문 단언(기존과 같은 값) |
| 같은 파일 | `leaving_a_course_stage_that_judged_nothing_ends_the_course_where_it_stands` | 두 스테이지 코스의 2스테이지에서 친 노트 0 으로 Esc → 결과 화면이 아니고(`!matches!(…, Result \| CourseResult)`), 1스테이지만의 FAILED 코스가 보내진다(기존 동작 고정) |
| 같은 파일 | 기존 `leaving_a_watched_replay_or_autoplay_keeps_nothing`(`:1152`) 에 "결과 화면이 아님" 단언 한 줄 추가 |
| `apps/rbms-player/src/app_result/record_tests/contract_requests.rs` | `a_run_left_mid_song`(`:746`) 고침 | `submit_rx.is_some()` 를 기다려 결과 화면 전송의 `POST /scores` 를 받고 `check_contract("score-abandoned", …)`, trace 는 `traced_from_the_result_screen` 과 같은 길로 받아 `check_contract("trace-abandoned", …)`. 두 계약 파일은 **바뀌지 않아야** 한다 |
| `apps/rbms-player/src/stage/play/screen_tests/run_tests.rs` | `escape_with_notes_left_on_a_skinned_run_goes_straight_to_its_result` | 스킨 플레이에서 노트 하나를 친 뒤 남은 노트가 있을 때 Esc 한 번 → 그 프레임의 전환이 `Stage::Result`, 장면이 `PlayPhase::Failed` 로 가지 않았다(닫힘 연출 없음) |

뒤집히는 기존 테스트: `record_tests.rs:936`, `:965`, `:1184`, `contract_requests.rs:746` 의 네 개(위 표대로 대체). 그대로 통과해야 하는 것: `leaving_before_any_note_is_judged_keeps_nothing`(`:994`), `leaving_after_only_an_empty_poor_keeps_nothing`(`:1028`), `leaving_after_a_note_went_by_unhit_keeps_nothing`(`:1050`), `leaving_while_holding_the_first_long_note_keeps_a_failed_run`(`:1065`), `leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result`(`:1086`), `leaving_while_holding_a_long_note_counts_it_once`(`:1105`), `leaving_a_course_stage_with_no_note_left_ends_it_as_played`(`:1232`), `stage/play/tests.rs` 의 Esc 테스트들.

골든: 영향 없음. 계약: `score-abandoned.json`·`trace-abandoned.json` 내용 불변(테스트 경로만 바뀜), 설명 문서 `docs/quality-assurance/2026-10-14-test-debt.md:131` 의 경로 설명만 고친다.

### 6.3 U-C 새 테스트

| 파일 | 테스트 이름 | 단언 |
| --- | --- | --- |
| `crates/rbms-skin/tests/skin_loader.rs` | `a_document_that_names_the_rbms_extensions_is_read_as_knowing_them` | `"rbms_extensions": 1` 인 JSON 문서 → `def.rbms_extensions == 1`, `knows_rbms_extensions()`. 키가 없는 문서 → 0, 거짓. `2` 도 참 |
| `crates/rbms-skin/tests/skin_luaskin.rs` | `a_lua_header_that_names_the_rbms_extensions_is_read_as_knowing_them` | 헤더 표에만 `rbms_extensions = 1` 이 있고 본체 패스 표에는 없는 `.luaskin` → 로드한 `def.knows_rbms_extensions()` 참. 헤더에 없으면 거짓 |
| `apps/rbms-player/src/stage/select/keys.rs`(tests) | `the_reference_tables_are_the_references_without_next_replay` | `KeyLayout::ReferenceBeat.table()[i]` 이 `REFERENCE_BEAT[i]` 를 `KEPT_ROLES` + 새 대응 일곱(`OPTIONDP_DOWN`→`OptionDpDown`, `OPTION2_UP`→`Option2Up`, `JUDGEAREA`→`JudgeArea`, `MARKNOTE`→`MarkNote`, `BPMGUIDE`→`BpmGuide`, `NOMINE`→`NoMine`)로 옮긴 것과 같다. `ReferencePopn` 은 목록 역할이 `POPN_9K` 와 같고 패널 역할이 `ReferenceBeat` 의 같은 인덱스와 같다 |
| 같은 파일 | `only_a_skin_that_names_the_extensions_reads_the_reassigned_table` | `KeyLayout::of(Mode::BEAT_7K, true) == Beat`, `of(BEAT_7K, false) == ReferenceBeat`, `of(POPN_9K, false) == ReferencePopn`, `is_extended` 가 앞 둘만 참 |
| 같은 파일 | 기존 `every_panel_role_is_on_a_key_and_every_role_on_a_key_is_read`(`:405` 부근) 고침 | 네 표 모두 "키에 있는 역할은 무엇인가가 읽는다". 누름 목록의 역할은 같은 가족(확장/원본)의 표 중 하나에 있거나 `ASKED_AND_UNASSIGNED = [OptionDpUp]` 이다 |
| 기존 `a_layout_without_a_turntable_reads_the_nine_button_table` | `KeyLayout::of(mode, true)` 로 고침 |
| `apps/rbms-player/src/stage/select/panel.rs`(tests) | `the_reference_roles_of_the_first_panel_step_the_double_option_and_the_random_back` | `press_act(OptionDpDown) == Setting { DoubleOption, NEXT }`, `OptionDpUp` PREVIOUS, `Option2Up == Setting { Random, PREVIOUS }` |
| 같은 파일 | `the_reference_assists_this_player_has_not_got_ask_for_nothing` | `JudgeArea`, `MarkNote`, `BpmGuide`, `NoMine` → `None` |
| `apps/rbms-player/src/stage/select/panel_tests.rs` | `on_a_skin_that_names_no_extensions_the_panels_keys_are_the_references` | `panel_document_naming(None)` 문서의 선곡에서: SELECT + 3·5·6·7 과 턴테이블 두 방향은 설정을 바꾸지 않는다(`changes_nothing`). START + 4 는 `lane_option` 을 다음 값으로, START + 6 은 `random` 을 `AllScratch`(이전)로, START + 7 은 `Mirror`. START+SELECT + 턴테이블은 `judge_algorithm` 불변 |
| 같은 파일 | 픽스처 | `panel_document()` 를 `panel_document_naming(Some(RBMS_EXTENSIONS_PANEL_KEYS))` 로 두고 `browser(tag)` 가 그것을 쓴다. 기존 패널 테스트 전부가 그대로 통과해야 한다 |
| `apps/rbms-player/src/stage/select/overlay_tests.rs` | `the_guide_names_the_keys_as_they_are_bound`(`:602`) | `wheel_document()` 에 표시를 넣으면 그대로 통과. 새 테스트 `the_guide_names_the_panels_turntables_only_on_a_skin_that_names_the_extensions`: 표시 없는 문서에서 `panel_rows(shared).len() == 4`, 마지막 줄 `A + TURNTABLE` |
| `apps/rbms-player/src/skin_select/default_pack_tests.rs` | `the_default_select_skin_names_the_rbms_extensions` | 기본 팩 `select.luaskin` 을 읽은 `def.knows_rbms_extensions()` 참. 다른 화면 문서(`play7.luaskin`)는 거짓(이 단언으로 M5-3 고정) |

뒤집히는 기존 테스트: 픽스처에 표시를 넣으면 없음. 넣지 않으면 `panel_tests.rs` 의 확장 표 테스트 전부와 `overlay_tests.rs:602` 가 뒤집힌다(그래서 5단계에서 픽스처를 먼저 고친다). 골든·계약: 영향 없음(스킨 덤프·캡처 화소 불변, 헤더 키는 그리지 않는다). `rbms-cli skin-dump` 의 경고 0 을 기본 팩에 한 번 확인.

### 6.4 실행 명령

단위마다(작업자):

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p rbms-render
cargo test --locked -p rbms-config
cargo test --locked -p rbms-play
cargo test --locked -p rbms-skin
cargo test --locked -p rbms-player --lib
```

U-A 는 rbms-render·rbms-config·rbms-player, U-B 는 rbms-play·rbms-player, U-C 는 rbms-skin·rbms-player 만 돌리면 된다(위험 비례). U-A 의 계약 재생성은 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 한 번 뒤 같은 테스트를 변수 없이 한 번. 웹 계약 확인은 `cd web && bun run test -- tests/dto/client-requests.test.ts`(웹 코드 변경 없음, 통과만 확인). 통합 게이트는 메인이 `cargo test --locked --workspace` 와 웹 `bun run verify` 를 한 번.

외부 팩 확인(선택): `RBMS_SKIN_PACK=/Users/hyunseokbyun/Downloads/ModernChic cargo test --locked -p rbms-player --lib the_turntable_scratch_auto_plays_is_drawn` (읽기만, 저장소에 복사하지 않음).

## 7. 문서 갱신 목록

| 문서 | 갱신 | 단위 |
| --- | --- | --- |
| 새 ADR `docs/acknowledge/2026-10-23-adr-scratch-auto-colour.md` | 결정 8, 저장 위치·알파 규칙·몸통 0xA0 비율·메뉴 행·기본값 화소 동일, 2026-10-22 색 행 ADR 4절 "SCRATCH AUTO 초록은 사용자 색보다 우선" 의 "고정 초록" 부분만 대체(의미색 규칙은 유지) | U-A |
| `docs/acknowledge/2026-10-22-adr-skin-colour-rows.md` | 4절 41행에 "색은 2026-10-23 ADR 로 PLAY 전역 값" 부분 변경 표시 | U-A |
| `docs/play-options.md` | 7.2절 행 표(CHART 에 두 행), 7.3절 조작, 7.4절 미리보기 표(SCRATCH AUTO COLOR 는 미리보기에 보이지 않음), 10.5절 그리기 행(색은 설정 값) | U-A |
| `docs/skin.md` | 6절 SCRATCH AUTO 줄(색은 PLAY 메뉴 값), 1.3절 의도적 차이 | U-A |
| `docs/reference/skin-color-customization.md` | 41행·290행에 2026-10-23 변경 각주 | U-A |
| `docs/crates.md:239`, `docs/architecture.md:52` | `LaneNotes::auto_note`, `PlayfieldView::auto_note`, `auto_long_body` | U-A |
| `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` | 3.17 X1 기대값에 "색을 바꾸지 않았을 때", 새 3.21 항목(색 바꾸기·알파·외부 팩·내장 필드) | U-A |
| 새 ADR `docs/acknowledge/2026-10-23-adr-escape-shows-the-result.md` | 결정 9. [2026-10-15 Esc ADR](../acknowledge/2026-10-15-adr-escape-keeps-every-run.md) 의 결정 1·2 중 "결과 화면 없이"·기각 표의 "코스 Esc 때 코스 결과 화면을 보인다" 와 [2026-10-14 포기 판 ADR](../acknowledge/2026-10-14-adr-abandoned-run-saved.md) 결정 1 "결과 화면 없이"·이유 "결과 화면을 강제하지 않는다"·기각 표 "결과 화면을 거친다" 를 대체한다. 저장·전송 범위(친 노트 0 제외)는 두 ADR 그대로. 원본 근거 `BMSPlayer.java:938-970`, `:693-742`, `:858-861` | U-B |
| 두 옛 ADR | 상태 줄에 "부분 대체: 2026-10-23 escape-shows-the-result" | U-B |
| `docs/acknowledge/reference-divergences.md:236`, `:239` | 결과 화면은 이제 원본과 같다(연출만 즉시), 코스는 코스 결과까지. 판정 수·코스 BP 차이는 남음 | U-B |
| `docs/play-options.md` 10.2절 "도중 포기(Esc)" 행과 코스 관련 문장, 10.8절 테스트 표 | 새 동작, 메시지 `run left mid-song kept as FAILED`·`course left part-way kept as FAILED` 삭제 | U-B |
| `docs/architecture.md:102`, `:105`, `docs/crates.md:536`, `docs/reference/ai-features/client.md:76` | `record_left_run`·`leave_course` 를 `enter_result_of_a_left_run` 으로. client.md 의 줄 번호(`enter_result :82` → 실제 `:70`, `record_left_run :461` → 삭제)도 정정 | U-B |
| `docs/quality-assurance/2026-10-14-test-debt.md:131` | `score-abandoned` 의 경로 설명 | U-B |
| 새 ADR `docs/acknowledge/2026-10-23-adr-panel-keys-opt-in.md` | 결정 5, 헤더 키 형식, 원본 표 복원 규칙, 2026-10-22 [옵션 패널 ADR](../acknowledge/2026-10-22-adr-option-panel-layout.md) 7절(위험을 남긴다)과 1절(어느 스킨에나 적용)의 부분 변경 | U-C |
| `docs/acknowledge/2026-10-22-adr-option-panel-layout.md`, `2026-10-22-adr-rbms-extension-ids.md` | 상태 줄에 부분 변경 표시, 확장 id ADR 4절(스킨이 읽는 방법)에 헤더 키 한 줄 | U-C |
| `docs/skin.md` | 1.4절 확장 키 표에 `rbms_extensions` 행, 6절 697행(선곡 키 재배정 위험) 해소, 3.4.5절(기본 스킨 선곡 헤더), 머리 줄의 팩 크기 | U-C |
| `docs/play-options.md` 12.2절·12.7절 | 표시 있는 스킨만 재배정, 없는 스킨은 원본 표. 12.7 의 "미확인" 문단 삭제 | U-C |
| `docs/reference/option-panels.md` | 0절 12번, 1.3절, 1.10절 14, 7절 결정 줄 | U-C |
| `docs/quality-assurance/2026-10-14-test-debt.md` 3.90, QA 3.20 Q8 | 해소로 옮기고 외부 팩 실기 항목 남김 | U-C |
| `docs/HANDOFF.md` 4절(74행 Esc 결정), 6절 13 마지막 줄(panels-1) | 해소·새 ADR 링크 | 메인 |
| `docs/acknowledge/README.md` 색인 | 새 ADR 셋 | 메인 |
| `docs/PROCESS.md` F2 | 상태 | 메인 |

## 8. 메인 결정 필요

결정이 다루지 않은 세부다. 추천안을 먼저 둔다. 구현자는 추천안대로 하고, 메인이 바꾸면 그 줄만 고친다.

| # | 결정 지점 | 추천안 | 근거 | 대안 |
| --- | --- | --- | --- | --- |
| M8-1 | 저장 필드 형태 | `play.scratch_auto_color: Option<SkinColor>`, `None` = 출하 초록 | 결정 8 "저장값 없으면 `#30E050`" 을 "저장값 없음" 으로 그대로 표현. 색 행의 `DEFAULT`(저장값 없음) 규칙과 같다 | `SkinColor` 직접 저장(기본 `30E050FF`): DEFAULT 표시와 0 키 "지움" 을 구별 못함 |
| M8-2 | 채널 행 배치 | 색 행 바로 뒤에 접이 `SCRATCH AUTO CHANNELS`(닫힌 채 열림, 펴면 R·G·B·A) | 기존 색 행 UI(채널은 접이)와 같고 CHART 묶음이 닫힌 상태에서 두 행만 는다 | 채널 네 행을 늘 보임 / 채널 없음(16진·프리셋만) |
| M8-3 | 0 키 | 색 행은 저장값 지움(DEFAULT), 채널 행은 그 채널만 출하 초록 값으로 | 기존 색 행·채널 행의 0 키와 같다 | 채널 0 키도 색 전체 지움 |
| M8-4 | PLAY 메뉴 미리보기 | 지금처럼 SCRATCH AUTO 를 그리지 않음. 행의 견본 사각형으로만 확인 | 미리보기는 예시 손이 치는 판이고 턴테이블 손을 빼면 미리보기 체감이 바뀐다(play-options.md 7.4). 변경 최소 | 미리보기 세션에 SCRATCH AUTO 를 적용(예시 차트의 턴테이블 노트가 색으로 보임, 미리보기 판정 바뀜) |
| M8-5 | 설정 업로드 계약 | `settings-blob.json` 을 재생성(`scratch_auto_color: None,` 한 줄 추가) | 다른 설정 필드를 더할 때와 같은 방식이고 rbms-config 에 `skip_serializing_if` 선례가 없다 | `#[serde(skip_serializing_if = "Option::is_none")]` 로 계약 불변(새 패턴) |
| M8-6 | 몸통 알파 | 사용자 알파 × 0xA0/255(조사 추천을 사용자가 "추천대로" 로 받음) | 머리와 몸통 구분 유지, 출하 화소 동일 | 몸통도 사용자 알파 그대로 / 몸통 색 행 따로 |
| M8-7 | SCRATCH AUTO 가 꺼져 있을 때 색 행 | 늘 보임 | 켜기 전에 색을 정할 수 있고 행 목록이 설정에 따라 흔들리지 않는다 | 켜져 있을 때만 보임 |
| M8-8 | 기본 스킨 Scratch Note 색 행(id 12/42)이 AUTO 노트에 닿는지 | 닿지 않음(전용 값만) | 조사 추천, 의미색 규칙(ADR 2026-10-22 4절) | 저장값이 없으면 Scratch Note 색을 따름 |
| M9-1 | 오토플레이·리플레이 보기 판의 도중 Esc | 지금처럼 선곡(결과 화면 없음) | 결정 9 의 대상은 사람이 친 판이고 이 판들은 기록이 없다 | 원본처럼 결과 화면(오토플레이는 FINISHED, 리플레이는 FAILED 결과) |
| M9-2 | 코스에서 친 노트 0 인 스테이지의 Esc | 지금처럼 앞 스테이지까지의 FAILED 코스를 남겨 보내고 선곡(코스 결과 화면 없음) | 결정 9 "노트를 하나도 치지 않은 판은 지금처럼" | 원본처럼 코스 결과(불합격) 화면을 보임(기록은 지금과 같게) |
| M9-3 | Esc 로 온 결과 화면의 다시 하기(R, N, 파란 건반 + START) | 일반 FAILED 결과 화면과 같이 허용(`offers_retry`) | 원본 결과 화면도 다시 하기를 받는다. 코드 분기 없음 | 숨김 |
| M9-4 | 알림 `run left mid-song kept as FAILED`·`course left part-way kept as FAILED` | 지움 | 결과 화면이 판이 남았음을 보인다. 호출처가 없어진다 | 결과 화면 위에 같은 알림을 남김 |
| M9-5 | Esc 판의 게이지 그래프 | 나간 자리부터 끝까지 0 채움(`left_record`) | 원본 `BMSPlayer.java:722-727` 이 FAILED 로 닫는 모든 판에 0 을 채운다. 결과 화면 그래프가 "어디서 멈췄는지" 를 게이지 사망 판과 같은 모양으로 보인다 | 채우지 않음(나간 자리에서 그래프가 끝남, rbms-play 변경 없음) |
| M5-1 | 헤더 표시 형식 | 최상위 정수 키 `rbms_extensions`(1 이상 = 안다). JSON·Lua 같은 이름 | 확장 키 선례 `color` 처럼 최상위, 정수라 뒤에 확장 묶음이 늘면 값을 올릴 수 있다. beatoraja 는 두 형식 모두 모르는 키를 버린다 | 불린 `rbms_extension_ids: true` / 중첩 표 `rbms = { extensions = 1 }` |
| M5-2 | 표시 없는 스킨에서 패널 1 키 4(`OPTIONDP_DOWN`) | 원본 역할 그대로 DP OPTION(이벤트 54) 다음 값. 키 6 은 RANDOM 이전 | 결정 5 "원본 역할", 2026-10-22 이전 동작. 원본 스킨(ModernChic)이 그 칸에 DP OPTION 값을 그리므로 표시 없이 바뀌는 일이 없다. DP OPTION 이 플레이에 적용되지 않는 것은 기존 차이(option-panels.md 1.10절 1) | 키 4 를 죽은 칸으로(아무 일도 안 함) |
| M5-3 | 표시를 넣을 기본 스킨 문서 | `select.luaskin` 만 | 읽는 곳이 선곡뿐이다. 다른 문서에 넣으면 의미 없는 키가 는다 | 기본 팩 문서 전부에 넣음 |
| M5-4 | 선곡 문서가 아직 없을 때(읽는 중) | 원본 표 | 그동안 스킨 선곡 입력이 돌지 않아 체감 차이가 없고, 모르는 쪽이 안전하다 | 확장 표 |

## 9. 남은 위험과 테스트 부채 후보

1. 사용자가 고친 기본 팩 `lua/select/main.lua` 사본(설치 관리자가 덮지 않는 파일)은 헤더 표시가 없어 원본 표가 된다. 기본 스킨 패널 그림은 확장 칸을 그리므로 그림과 키가 어긋난다. 테스트 부채 후보: "고친 기본 팩 select 문서의 표시 누락" — 재현: 기본 팩 `lua/select/main.lua` 를 손으로 고친 뒤 새 빌드 실행, 남은 위험: 패널 2 키 3 이 SCRATCH AUTO 를 바꾸지 않음, 실행 필요 시점: 사용자 보고가 있을 때. 완화안(이번에 하지 않음): 기본 팩 문서 경로면 표시를 가정.
2. JSON 스킨이 `rbms_extensions` 에 정수가 아닌 값을 쓰면 serde_json 이 문서 전체를 거절한다(다른 숫자 필드와 같은 엄격함). beatoraja 용 스킨은 이 키를 쓰지 않으므로 실제 위험은 R-BMS 용 JSON 스킨 작성자뿐이다. skin.md 1.4절에 "정수" 를 적는다.
3. Esc 결과 화면의 판정 분포 그래프(내장 결과 화면의 `judge_dist`, 스킨의 초별 판정 표 `JudgeSeconds`)는 "판정된 입력" 기준이라 남은 노트의 POOR 를 넣지 않는다. 판정 수 칸·DB·전송은 접은 값이다. 이 차이는 끝까지 친 판에서도 있는 기존 의미(입력 분포)라 고치지 않는다. QA 실기에서 사용자가 혼동하면 테스트 부채로 올린다.
4. 결과 화면 진입 때 곡 소리를 `stop_chart_sounds` 로 끊는다. 헤드리스 테스트에 오디오 장치가 없어 소리가 실제로 끊기는지는 자동 확인하지 못한다. QA 실기 항목(새 3.21)으로 둔다: Esc 직후 키음·BGM 이 멈추는지, PlayStop 이 한 번만 울리는지.
5. U-B 의 계약 테스트 `a_run_left_mid_song` 이 결과 화면 전송으로 바뀌면서 `score-abandoned.json` 이 바뀌면 안 된다. 바뀐다면 결과 화면 전송 본문과 저장 본문이 다른 것이니 재생성으로 덮지 말고 원인을 찾는다.
6. F5(코스 게이지·제출 게이지 종류)가 `app_result.rs` 의 코스 경로를 크게 바꾸면 U-B 의 코스 테스트(`leaving_mid_course_ends_on_a_failed_course_result`)의 본문 단언이 같이 움직인다. 통합 순서(5.4절)를 지킨다.
7. 외부 팩(ModernChic)으로 패널 키·SCRATCH AUTO 색을 보는 테스트는 `RBMS_SKIN_PACK` 이 있을 때만 돈다. CI 는 기본 팩만 본다. QA 3.20 Q8·3.17 X1 실기로 확인한다.
8. 설정 묶음을 옛 빌드가 내려받을 때 모르는 필드 `scratch_auto_color` 를 버린다는 판단은 serde 파생 규칙에 기댄 것이고 옛 빌드 바이너리로 시험하지 않았다. 옛 빌드가 다시 올리면 새 필드가 사라진다(색이 출하 초록으로 돌아감, 기록·판정 영향 없음).
9. 실제 GPU 창 확인(색, Esc 체감, 외부 스킨 패널)은 설계 단계에서 하지 않았다. 구현 뒤 QA 수동 점검표에 넣는다.
