# ADR: 기본 스킨 선곡의 MASTER 슬라이더(rate 17, 숫자 57)는 진짜 마스터 audio.master 를 움직이고, 미리듣기 키음은 Key 버스로 울린다

> 최종 갱신 2026-10-29 · 대응 커밋 `435839e`
> 상태: 유효(M11 은 메인 추천, 사용자 미확인. 사용자는 증상만 보고했고 해법은 정하지 않았다) · 결정일 2026-10-29 · 결정자 메인(M11)과 구현 단위 U4(세부) · 커밋 `435839e`
> 관계: 레퍼런스(beatoraja)와 id 뜻이 달라진다([skin.md](../skin.md) 1.3절의 "그 밖의 차이"). 확장 id 는 만들지 않았다. 설계 [plan/2026-10-29-feature-upgrade.md](../plan/2026-10-29-feature-upgrade.md), 조사 [reference/2026-10-29-feature-upgrade/select-volume.md](../reference/2026-10-29-feature-upgrade/select-volume.md).

## 배경

- 사용자 보고(2026-10-29): "사운드 조절 (곡선택화면)이 제대로 먹히긴하나 ? 안먹는 느낌인데".
- 슬라이더 → 설정 → 믹서 경로는 고장 나지 않았다. 드래그한 값은 그 프레임 끝에 오디오 스트림에 들어가고 손을 떼면 파일에 저장된다.
- 근본 원인은 세 슬라이더가 움직이는 버스와 선곡에서 실제로 들리는 소리가 타는 버스가 어긋난 것이었다.

| 슬라이더(수정 전) | 움직이던 값 | 선곡에서 영향받던 소리 |
| --- | --- | --- |
| MASTER (rate 17) | `audio.system`(시스템 버스). 진짜 마스터 `audio.master` 가 아님 | 짧은 시스템 효과음뿐. 미리듣기는 그대로라 "안 먹는다" |
| KEY (rate 18) | `audio.key` | 없음. 자동 재생 미리듣기의 키음도 BGM 버스로 울렸다 |
| BGM (rate 19) | `audio.bg` | 미리듣기 전체. 셋 중 이것 하나만 들렸다 |

- 보조 원인: 스킨의 MASTER 숫자는 시스템 음량(기본 50%)을 보이는데 SETTINGS 의 MASTER VOL(기본 100%)은 다른 값이었다. 음량이 선형 이득이라 50% 에서 100% 가 +6 dB 뿐이라 위쪽 절반의 변화가 작게 들린다.

## 결정

### 1) 메인 추천(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M11 | 기본 스킨 MASTER 슬라이더(rate 17)와 NUMBER 57 을 진짜 마스터 `audio.master` 로 다시 묶는다(레퍼런스 id 뜻과 다름을 문서화한다). 자동 재생 미리듣기는 키음을 Key 버스, BGM 을 Bg 버스로 나눈다. 선형 이득 곡선은 유지한다 |

### 2) 구현(구현 단위가 정함, 사용자 미확인)

| 항목 | 내용 |
| --- | --- |
| 쓰기 | `skin_host/writers.rs` 에 앱 쪽 열거형 `VolumeTarget { Master, Bus(VolumeBus) }`. `rate_bus` 를 `rate_volume` 으로 바꾸고 17 → Master, 18 → Key, 19 → Background. `set_volume` 이 `VolumeTarget` 을 받고 값이 NaN 이면 마스터는 `DEFAULT_MASTER_VOLUME`, 버스는 `DEFAULT_BUS_VOLUME` 으로 대체한다. `carry_out` 은 흐름 그대로(`apply_audio_gains` 가 마스터 이득도 건다, 설정 저장 표시) |
| 읽기 | `skin_host/system.rs` 의 `Volumes` 에 `master` 필드와 `Volumes::of_audio(&AudioOptions)`. 숫자 57 과 rate 17 은 `master`, `main_state.volume_sys` 는 `system` 을 읽는다. 공급처 두 곳(`skin_screen.rs`, `skin_screen/preview.rs`)의 같은 생성자를 `of_audio` 하나로 모았다 |
| Lua | `rbms-skin` 의 `VolumeBus` 와 Lua `volume_sys`·`set_volume_sys`·`volume_key`·`volume_bg` 는 그대로다. 시스템 음량은 SETTINGS 의 SYSTEM VOL 과 Lua `set_volume_sys` 로만 바꾼다 |
| 미리듣기 | `stage/select/preview.rs` 의 자동 재생 일정을 `(at_us, wav, source)` 로 바꿔 `PlayEvent.source` 를 보존하고 `play_sink::bus_for(source)` 로 버스를 고른다. `#PREVIEW` 클립은 섞인 음원이라 Bg 그대로다. `pump_fade` 와 `release_bus` 는 `put_buses_at` 하나를 통해 `preview_bus_gains(audio, level)` 로 Key·Bg 두 버스에 같은 레벨을 곱하거나 설정값으로 되돌린다. 일정 추출은 `autoplay_preview_schedule` 로 분리했고 매직넘버 `1_000_000` 은 `AUTOPLAY_PASS_TAIL_US` 가 되었다 |
| 라벨 | 기본 스킨 `select_other.lua` 의 MASTER / 마스터 / マスター, KEY / 키음 / キー音, BGM 은 새 동작과 맞아 Lua·카탈로그를 고치지 않았다 |
| 곡선 | 선형 이득 유지. 바꾸면 저장값의 뜻이 바뀐다 |

### 3) 선곡에서 들리는 것(수정 후)

| 슬라이더 | 움직이는 값 | 선곡에서 영향받는 소리 |
| --- | --- | --- |
| MASTER (rate 17, 숫자 57) | `audio.master` (기본 100%) | 미리듣기(클립·자동 재생 모두), 시스템 효과음, 선곡 음악 전부 |
| KEY (rate 18, 숫자 58) | `audio.key` | 자동 재생 미리듣기의 키음. `#PREVIEW` 클립에는 닿지 않는다 |
| BGM (rate 19, 숫자 59) | `audio.bg` | `#PREVIEW` 클립과 자동 재생 미리듣기의 BGM |

## 이유

- 라벨(MASTER)과 동작이 맞고 SETTINGS 의 MASTER VOL 과 같은 값을 보인다. 미리듣기·효과음·플레이까지 즉시 들린다.
- 미리듣기는 Bg·Key 버스와 자기 페이드(`preview_volume`)로 돌기 때문에, 이 소리에 닿는 슬라이더는 마스터와 두 버스뿐이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 레퍼런스처럼 미리듣기를 시스템 음량(System 버스)에 따르게 함 | 페이드가 Bg 버스 이득으로 구현돼 있어 System 버스로 옮기면 효과음까지 페이드된다. 소리마다 이득을 매기면 이미 울리는 `#PREVIEW` 클립은 다음 반복 때까지 바뀌지 않는다. BGM 슬라이더가 미리듣기에서 떨어져 지금 유일하게 들리는 슬라이더를 잃는다 |
| 기본 스킨 라벨만 SYSTEM 으로 바꾸고 마스터용 rbms 전용 rate 를 새로 만들어 네 번째 슬라이더를 달기 | 새 property id(생성 표와 체크섬 시험), 스킨 레이아웃 변경까지 범위가 넓다. 라벨만 바꾸면 "안 들린다"는 체감이 남는다 |
| 이득 곡선을 로그/지수로 | 레퍼런스도 선형이고 저장값·`volume_percent`·`step_volume` 의 뜻이 바뀐다. 별도 결정으로 다룬다 |

## 영향과 관련 파일

- 코드: `apps/rbms-player/src/{lib.rs(PreviewMsg::Schedule 의 요소 타입), skin_host/{system,writers}.rs, skin_screen.rs, skin_screen/preview.rs, stage/select/preview.rs}` 와 각 시험 파일. `crates/rbms-skin` 과 `settings_ui.rs` 는 바꾸지 않았다.
- 시험: `writers/tests.rs`(rate 17 → Master, 마스터 단독 보정, 슬라이더 쓰기가 `audio.master` 만 움직임), `system/tests.rs`·`skin_host/tests.rs`, `skin_screen/tests.rs`·`typing_tests.rs`, `stage/select/preview.rs` 의 `a_fade_is_put_on_the_key_and_the_background_bus_alike`·`an_autoplay_preview_keeps_the_source_of_each_event_and_sounds_it_on_that_bus`·`the_master_key_and_bgm_sliders_each_turn_a_preview_down`. 마지막 시험은 실제 `rbms_audio::Mixer` 에 마스터·버스 이득을 넣어 재고, 예전처럼 마스터 슬라이더가 `audio.system` 에 쓰게 되돌리면 "MASTER 를 0 으로 해도 미리듣기가 0.25 로 그대로"라며 실패하는 것을 확인했다. 출력 장치는 쓰지 않았다.
- 외부 스킨: beatoraja 용 스킨이 rate 17 / 숫자 57 을 쓰면 R-BMS 에서는 마스터를 움직이고 마스터를 보인다. 스킨이 `main_state.volume_sys` 로 자기 슬라이더를 만들었으면 그것은 시스템 음량을 보이고 쓰므로 한 스킨 안에서 둘이 다를 수 있다(의도).
- 업데이트 뒤 보이는 변화: 예전 MASTER 슬라이더(= `audio.system`)를 내려 효과음을 줄이던 사용자는 슬라이더가 `audio.master`(기본 100%)를 보이는 것을 본다. 저장된 `audio.system` 값은 그대로 남고, 효과음 음량은 SETTINGS 의 SYSTEM VOL 로 바꾼다.
- 알려진 위험(테스트 부채 3.139·3.140, 실기 QA 3.25 V12): 미리듣기 페이드(약 200 ms) 중 슬라이더를 끌면 `apply_audio_gains` 가 Key·Bg 를 한 프레임 동안 전체 값으로 되돌렸다가 `pump_fade` 가 고친다(엔진의 3 ms 슬루로 희미한 튐이 들릴 수 있다. 고치려면 `AppShared` 가 미리듣기 레벨을 들고 `apply_audio_gains` 가 곱해야 하는데 `settings_ui.rs`(U2 와 공유)를 건드린다). KEY 는 `#PREVIEW` 클립이 있는 차트에서 여전히 아무 효과가 없다. 실제 소리 체감은 출력 장치가 있는 실기에서만 본다.
- 문서: [play-options.md](../play-options.md) 17.6절, [skin.md](../skin.md) 1.3절 "그 밖의 차이"·3.4절.

## 날짜와 사용자 원문

2026-10-29.

- 사용자 보고: "4. 사운드 조절 (곡선택화면)이 제대로 먹히긴하나 ? 안먹는 느낌인데"
- 사용자 결정: 없음(M11 은 메인이 추천안으로 정했다. 설계서 1절의 사용자 답 D1~D6 에 이 항목은 없다).
