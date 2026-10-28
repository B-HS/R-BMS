# 배치 지정 플레이(LAYOUT PICK) 구현 전 조사

> 최종 갱신 2026-10-29 · 대응 커밋 c1dc988
> 범위: 시작 전에 사용자가 레인 순열(예: 7키 `3517246`)을 직접 정하고, 그 배치를 적용한 노트 미리보기를 본 뒤 그 배치로 플레이한다.
> 사용자 결정(전제): (a) 이 판은 RANDOM 판으로 저장한다. 그 배치를 내는 RANDOM 시드를 역탐색해 옵션 RANDOM + 그 시드로 기록한다. 서버·웹·리플레이 형식은 바꾸지 않는다. (b) 미리보기는 그 배치를 적용한 실제 노트를 레인에 그려 스크롤(구간 이동)하고 레인 번호 순서를 보인다.
> 읽기 전용 조사. 시드 측정은 scratch 의 독립 Rust 프로그램(아래 1.4)으로 했고, 그 복제 코드가 저장소의 배치 골든 3개와 같은 값을 냈다.

## 0. 핵심 요약

1. RANDOM 은 차트 내용과 무관한 레인 순열이다. `lane_permutation(Random, mode, seed)` 는 `mode` 와 `seed` 만 읽는다(`crates/rbms-chart/src/shuffle.rs:115-142`). 그래서 "원하는 순열을 내는 시드"는 차트 없이 찾을 수 있고, 찾은 시드로 판을 시작하면 지금의 로드 경로가 그대로 같은 배치를 만든다.
2. 역탐색 비용(1.4 실측, 릴리스 빌드): 5K 120개 순열 전부가 시드 1~783, 7K 5040개 전부가 1~46,287, 9K 362,880개 전부가 1~4,775,533 안에서 나온다. DP 14K 의 (P1, P2) 25,401,600쌍 전부가 1~468,482,176 안에서 나온다(평균 약 2,800만 번째, 0.2초 안팎). **DP 10K 는 14,400쌍 중 12,600쌍(7/8)만 만들 수 있고 1,800쌍은 어떤 시드로도 나오지 않는다**(시드 영역 세 곳에서 같은 집합, 구조적). 24K 는 24! 이 2^53 보다 커 대부분 만들 수 없다.
3. 시작·기록 경로는 기존 "배치 빌리기"(`borrowed_layout.rs`)를 그대로 쓸 수 있다. `LayoutRequest { md5, random_label: "RANDOM", seed, source }` 로 `AppShared::borrow_layout` 을 부르면 기록·리플레이·제출이 RANDOM + 그 시드로 남고, 같은 배치 재시도·버림 규칙도 그대로 따른다.
4. 진입은 선곡의 옵션 패널 1~3(누르고 있는 동안만 열림, 빈 칸 없음)이나 기록 모달(기록이 있어야 열림)보다 **연습 패널처럼 선곡 위에 여는 내장 화면(새 Stage)** 이 맞다. 새 단축키로 연다. 미리보기는 내장 플레이필드 렌더러 `render_playfield_view` 를 그 화면의 첫 그리기로 부르면 rbms-render 를 고치지 않고 쓸 수 있다(이 함수가 화면 전체를 `clear` 하기 때문, 3.2).
5. 차트 파싱과 DP 시드 탐색은 프레임 스레드 밖(작업자 스레드)에서 한다. 선곡의 분석 작업자가 같은 원칙을 문서로 박아 두었다(`docs/play-options.md:1079`, `stage/select/analysis.rs:11-15`).

## 1. 현재 구조

### 1.1 RANDOM 순열 생성(`crates/rbms-chart/src/shuffle.rs`)

| 항목 | 내용과 근거 |
| --- | --- |
| 난수기 | xorshift64(`<<13`, `>>7`, `<<17`). 시드 0 이면 상태를 `0x9E3779B97F4A7C15` 로 둔다(`shuffle.rs:78-96`). 의존성 없음 |
| `below(n)` | `next() % n`(모듈로 편향이 아주 작게 있다, `shuffle.rs:92-94`) |
| 한쪽 레인 | `side_key_lanes(mode, side)`: 플레이어 수로 나눈 한쪽의 레인 중 스크래치가 아닌 것(`shuffle.rs:100-105`). SP 는 side 0 하나, DP 는 양쪽이 각자 자기 절반만 섞는다(P1<->P2 를 넘지 않는다) |
| 섞기 | Fisher-Yates: `for i in (1..len).rev() { swap(i, below(i+1)) }`(`shuffle.rs:107-111`) |
| 시드 사용 | 쪽마다 `Rng::new(seed.wrapping_add(side).wrapping_mul(0x9E3779B97F4A7C15))`(`shuffle.rs:123`). 즉 DP 의 P2 쪽 순열은 **시드 s+1 의 P1 쪽 순열과 같은 모양**이다(1.4 에서 골든으로 확인). 양쪽은 독립이 아니다 |
| 결과 형식 | `perm[old_lane] = new_lane`, 스크래치 레인은 항등(`shuffle.rs:113-142`) |
| 차트 의존 | 없음. 인자는 `option, mode, seed` 뿐 |
| 적용 | `apply` → `apply_perm`: 모든 타임라인의 `notes` 와 `hidden` 을 같은 순열로 옮긴다(`shuffle.rs:148-180`). 길이가 `mode.key` 가 아닌 타임라인은 건드리지 않는다. LN 머리·꼬리는 같은 레인으로 간다 |
| 다른 옵션 | MIRROR·ROTATE·R-RANDOM 도 같은 함수의 레인 순열이다. S-RANDOM·H-RANDOM·ALL-SCRATCH 는 줄마다 섞어 `lane_permutation` 에서는 항등이다(`shuffle.rs:136`, 시험 `srandom_hrandom_allscratch_yield_identity_in_lane_permutation`) |
| 외부 호출 | `lane_permutation` 은 `shuffle.rs` 밖에서 부르는 곳이 없다(`grep` 결과). 레인 순서를 화면에 보이는 곳도 없다 |
| 골든 | `GOLDEN_SEED = 0x001E_D5A7_C3B9_1F42`(`shuffle.rs:832`), 옵션 8개 x 7K·14K·9K 배치 24개(`shuffle.rs:838-883`, 시험 `every_arrangement_lays_the_golden_charts_out_as_it_always_has` `:887`). 기존 함수를 바꾸지 않고 함수를 더하기만 하면 골든은 그대로다 |

모드 상수(`crates/rbms-model/src/mode.rs:23-32`)

| 모드 | key | player | 스크래치 | 섞이는 레인(쪽마다) | 순열 공간 |
| --- | --- | --- | --- | --- | --- |
| BEAT_5K | 6 | 1 | 5 | 0..4 (5개) | 5! = 120 |
| BEAT_7K | 8 | 1 | 7 | 0..6 (7개) | 5040 |
| POPN_9K | 9 | 1 | 없음 | 0..8 (9개) | 362,880 |
| BEAT_10K | 12 | 2 | 5, 11 | 0..4 / 6..10 | 120^2 = 14,400 |
| BEAT_14K | 16 | 2 | 7, 15 | 0..6 / 8..14 | 5040^2 = 25,401,600 |
| KEYBOARD_24K | 26 | 1 | 24, 25 | 0..23 (24개) | 24! ≈ 6.2e23 |

`KEYBOARD_24K` 는 `Mode::ALL` 에 없고 BMS 채널로는 나오지 않으며 bmson `mode_hint` 로만 온다(`mode.rs:17-37`).

### 1.2 시드 제약

- 새 판의 시드는 `fresh_layout_seed`: 시계 나노초 & `LAYOUT_SEED_MASK = (1 << 53) - 1`, 0 이면 1(`apps/rbms-player/src/app_play.rs:18-29`). 이유는 서버 JSON 의 안전 정수(ADR `docs/acknowledge/2026-10-14-adr-layout-seed-range.md`). 서버는 범위 밖 시드를 0 으로 저장하고 판은 받는다.
- 빌린 배치는 시드 0 이면서 OFF 가 아니면 거절한다(`borrowed_layout.rs:155-169` `check_layout_request`).
- 역탐색이 1 부터 올라가면 찾는 시드는 모두 5억 미만이다(1.4). 2^53 미만, 0 아님 조건을 자연히 지킨다. `Rng::new` 의 0 특례는 `(seed+side)*G ≡ 0 (mod 2^64)` 일 때만인데 G 가 홀수라 `seed+side ≡ 0` 이어야 하므로 이 범위에서는 일어나지 않는다.

### 1.3 판 시작과 기록 경로

```
선곡(또는 기록 모달 등) → AppShared::borrow_layout(request, chart_index)          borrowed_layout.rs:205-221
   check_layout_request: 손으로 치는 판인가 → md5 일치 → 라벨 → 시드 0 거절         borrowed_layout.rs:155-169
   end_replay, 알림 LayoutOf "{0}의 배치 ({1})", borrowed_layout = Borrowing{for_next_load: true}
   → Transition::Open(decided_song_stage(index))                                    decide.rs:351-353
Decide/Loading → AppShared::load()                                                  app_play.rs:476-
   carried_seed = retry_seed.take()                                                  app_play.rs:477
   (random, seed) = layout_to_load(decoded.md5, carried_seed)                        app_play.rs:499, 84-101
      빌린 배치(lay_out_as_borrowed, md5 대소문자 무시 비교) > 보는 리플레이 > 내 RANDOM + (carried 또는 fresh)
      빌린 동안 config.play.random = 빌린 옵션, 내 값은 own_play_options 에 보관        borrowed_layout.rs:321-333
   resolve_long_note_flavour(LN MODE) → lay_out_notes(model, random, seed) → shuffle::apply   app_play.rs:512-526, 171-185
   SessionOptions { seed, .. }
결과 → record_run                                                                    app_result.rs:273-
   ChartRun.seed = play.seed()                                                       app_result.rs:281
   제출 options.random = ir_random(config.play.random), sub.seed = chart.seed         app_result.rs:340, 362
   리플레이 random = config.play.random.label(), seed = chart.seed                    app_result.rs:386-387
   점수 로그(FinishedPlay) random, seed                                              app_result.rs:428-429
```

- 기록 필드: 리플레이 `random: String`, `seed: u64`(`crates/rbms-store/src/replay.rs:160-161`), 점수 DB `seed: i64`, `random: i32`(`scoredb.rs:234-236`, `random_id(label)` `scoredb/schema.rs:177`), 제출 `seed: u64`(`crates/rbms-ir/src/dto/score.rs:74`), 옵션 `random: RandomOption`(`dto/score.rs:406`, `random_p2` 는 늘 `None`, `app_result.rs:341`).
- 즉 config.play.random 을 RANDOM 으로, 세션 시드를 찾은 시드로 두기만 하면 모든 기록 경로가 "RANDOM + 그 시드"를 남긴다. 빌린 배치 판이 이렇게 남는다는 시험이 이미 있다(`app_result.rs:1192-1215`).
- 웹은 셔플을 재구현하지 않는다(`web/src` 에 xorshift·순열 코드 없음). 시드는 표시·저장만 한다.
- 수명: 같은 배치 재시도(`retry_seed` 가 빌린 시드와 같음)에서는 유지, 선곡·다음 곡·시드 없는 재시도에서는 버리고 내 RANDOM 으로 돌아온다(`borrowed_layout.rs:301-315`, `stage/play/mod.rs:1468-1471`, `stage/result.rs:1032-1042`).
- `#RANDOM` 분기는 `decode_chart` 가 `ParseOptions::default()` 로 파싱해(`app_play.rs:228-241`, `crates/rbms-parser/src/lib.rs:127-163`) 늘 같은 분기다. 미리보기도 `decode_chart`(또는 `chart_model`, `app_play.rs:244`)를 쓰면 판과 같은 차트가 나온다.

### 1.4 시드 역탐색 실측

scratch 프로그램(저장소 밖)에 `Rng`, `fisher_yates`, 쪽별 시드 식을 그대로 옮겨 쟀다. 옮긴 코드의 출력이 골든과 같았다: 7K `GOLDEN_SEED` → `[5,4,1,2,0,6,3]`(골든 행 `5 4 1 2 0 6 3 7`), 14K P2 쪽 → `[1,5,6,3,4,2,0]`(골든 `9 d e b c a 8` - 8), 그리고 `GOLDEN_SEED + 1` 의 7K 순열이 그 P2 값과 같다. 9K → `[8,7,5,0,6,4,2,1,3]`(골든 `8 7 5 0 6 4 2 1 3`).

| 측정(릴리스 `rustc -O`, Apple Silicon) | 결과 |
| --- | --- |
| 5K: 120개 전부가 나오는 첫 시드 범위 | 1~783 |
| 7K: 5040개 전부 | 1~46,287 (0.5 ms). 무작위 목표 2000개: 평균 5,152번째, 최악 40,714번째 |
| 9K: 362,880개 전부 | 1~4,775,533 (115 ms) |
| 10K: (P1,P2) 쌍 | 14,400 중 **12,600 만** 나온다. 마지막 새 쌍이 시드 1,250,411, 4억까지 늘지 않음. 시드 1부터, 2^52+12345 부터, 987,654,321,987 부터 각 400만 개를 훑은 집합이 서로 같다(구조적 불가능). P1 하나마다 P2 는 최소 90/120 가능. 무작위(도달 가능) 목표 2000개: 평균 12,266, 최악 535,085 |
| 14K: (P1,P2) 쌍 | 25,401,600 전부가 1~468,482,176 안에서 나온다(50%: 1,761만, 90%: 5,858만, 99.99%: 2억 3,453만). 무작위 목표 20개: 평균 2,816만, 최악 8,461만 번째, 합 4.2초 |
| 순열 하나 계산 비용 | `Vec` 할당 포함 7키 순열: 릴리스 약 19 ns, 디버그(`opt-level=0`) 약 225 ns. 실제 `lane_permutation` 은 `Vec` 을 넷 이상 만들고 양쪽을 다 계산하므로 더 비싸다(미측정) |

예시(7키, 문자열 규칙은 2.2): `3517246` = 시드 4761(`perm[old]=new [2,4,0,5,1,6,3]`), `1234567`(OFF 와 같은 모양) = 24088, `7654321`(MIRROR 모양) = 1048, `7123456` = 2272, `2345671` = 14563. 5K `12345` = 225, `54321` = 39. 9K `987654321` = 590,676. `GOLDEN_SEED` 의 7키 문자열은 `5347216`.

결론

- SP(5K·7K·9K): 모든 배치가 가능하다. 1부터 훑으면 최악 9K 477만 번이라 릴리스에서 0.1초 안팎, 7K 는 1 ms 미만이다.
- 14K: 모든 쌍이 가능하지만 최악 4.7억 번이다. 할당 없는 전용 탐색(쪽 0 순열만 매 시드 계산, 시드 s 의 순열이 P1 목표와 같을 때만 s+1 을 계산하거나, 앞 시드 결과를 밀어 쓰기)으로 릴리스 최악 약 3~6초, 평균 0.2~0.4초. 디버그 시험에서는 수십 초가 될 수 있다. 작업자 스레드 + 취소 + 진행 표시가 필요하다.
- 10K: 1,800쌍(1/8)은 RANDOM 으로 만들 수 없다. 결정 (a)를 지키면 그 쌍은 거절해야 한다.
- 24K: 24! ≈ 6.2e23 > 2^53 ≈ 9.0e15 이므로 대부분 만들 수 없다. 지원하지 않는 것이 맞다.
- 기존 `lane_permutation` 을 그대로 반복 호출하면(할당 다수, 양쪽 계산) 14K 최악이 수십 초~분 단위가 될 수 있다(추정). 탐색 함수는 `shuffle.rs` 안에 두어 비공개 `Rng`·`fisher_yates` 를 그대로 쓰고, 결과를 `lane_permutation` 으로 다시 확인하는 구조가 안전하다.

### 1.5 진입 후보가 될 수 있는 기존 화면

| 화면 | 현재 동작 | 배치 지정에 쓸 수 있나 |
| --- | --- | --- |
| 옵션 패널 1~3(`docs/play-options.md` 12절, `stage/select/{keys,panel,events}.rs`) | START/SELECT/상세 키를 누르고 있는 동안만 열린다. 건반 1~7 과 턴테이블이 모두 이미 배정돼 있다(12.2절 표). 내장 선곡에는 패널이 없다 | 맞지 않다. 7자리를 치는 동안 키를 쥐고 있어야 하고 빈 칸이 없다. 외부 스킨 키 표(`REFERENCE_BEAT_TABLE`)와 `CHANGED_SLOTS` 시험도 건드리게 된다 |
| 기록 모달(`stage/select/mod.rs:756-805`, `crates/rbms-render/src/select.rs:123-147`, 단추 `SelectHot::ModalLayout`) | 기록이 있는 차트에서만 열린다. PLAY THIS LAYOUT 단추가 있다 | 주 진입으로는 맞지 않다(기록 없는 차트에서 못 연다). 보조 진입("이 기록의 배치를 편집해서 시작")으로는 좋다 |
| PLAY 메뉴(`stage/play_menu.rs`, 7절) | 독립 화면, 예시 차트를 플레이 스킨으로 그리는 미리보기. 고른 차트와 무관하다 | 맞지 않다. 차트를 모르고, 미리보기가 시간 흐름으로만 돈다(7.4절) |
| 연습 패널(`stage/practice.rs:1-28`, 9절) | 선곡에서 `F4` 로 여는 내장 화면. 차트를 읽어야 해서 독립 화면이 됐다 | 구조가 가장 비슷하다. 다만 연습은 차트를 플레이와 같은 로드 경로(LOADING, 키음 디코드 포함)로 읽는다. 배치 지정은 미리보기에 차트 모델만 필요하다 |
| 선곡 오버레이(`stage/select/overlay.rs:205-251`) | 필터·랭킹·안내·기록 모달·다운로드 모달을 엔진이 스킨 위에 그린다 | 그릴 수는 있으나 키 경로가 복잡하다(`handle_key` 의 모달 분기 `mod.rs:1299-1340`, 스킨 선곡은 건반을 프레임마다 읽는다 `mod.rs:1385-1420`, `holds_keys` `mod.rs:1182-1184`). 또 `render_playfield_view` 가 화면을 지워(3.2) 오버레이로는 그대로 쓸 수 없다 |

### 1.6 미리보기에 쓸 수 있는 렌더러

| 후보 | 내용 | 평가 |
| --- | --- | --- |
| 내장 플레이필드 `render_playfield_view`(`crates/rbms-render/src/playfield.rs:18-120`) | `PlayfieldView { timelines, microtime, hispeed, beam_on, beam_off, constant, legacy_note, auto_lanes, auto_note }` 를 받아 레인 배경, 판정선, 노트, LN 몸통, 지뢰를 그린다. `hispeed` 는 "한 마디가 레인 높이의 몇 배"(`crates/rbms-chart/src/scroll.rs:69-104`, `rxhs = lane_height * hispeed`). `constant` 면 BPM 무시 등속(`scroll.rs:107-`). 플레이 화면이 쓰는 그대로다(`stage/play/mod.rs:1723-1735`) | 추천. `microtime` 을 미리보기 커서로 두면 구간 이동이 된다. **첫 줄이 `r.clear(skin.bg)`**(`playfield.rs:61`)이고 `Renderer::clear` 는 그 프레임에 이미 그린 것을 버린다(`crates/rbms-render/src/lib.rs:425-429`). 그래서 화면의 첫 그리기로 불러야 한다 |
| 내장 `Skin`(`crates/rbms-render/src/skin.rs:200-`, `Skin::build(cfg, mode, w, h)` `:246`) | `SkinConfig` 의 `field_x`, `field_width`, `top_y`, `judge_y`, `scratch_left`, `dual_field` 로 필드 위치를 정한다. `x`, `w`, `fields` 가 `pub` 이라 레인 아래에 번호를 쓸 좌표를 바로 얻는다 | 미리보기 전용 `SkinConfig`(기본값에서 위치만 바꿈)로 `Skin` 을 만들면 된다. DP 는 `dual_field` 로 두 필드 |
| 스킨 미리보기 `PlaySample`(`stage/play/skin_preview.rs:1-60`) | 플레이 스킨 문서(Lua)를 읽고 내장 예시 차트를 적어 둔 손으로 계속 돌린다. 시간이 흐르기만 하고 원하는 곳으로 옮기는 길이 없다 | 맞지 않다. 스킨 로드가 무겁고, 실제 차트·스크롤 지정이 없다 |
| PLAY 메뉴 미리보기(7.4절) | 위 `PlaySample` 을 쓴다. RANDOM 은 미리보기에 보이지 않는다고 문서가 적는다(`docs/play-options.md` 7.4절 마지막 표) | 같은 이유로 맞지 않다 |

### 1.7 문서와 코드의 차이

- `docs/acknowledge/2026-10-24-adr-layout-play.md` 3) 의 검사 순서는 `ArrangementUnknown` 이 `UnknownLayout`(시드 0) 앞이라고 적지만, 코드는 `check_layout_request` 안에서 시드 0 을 먼저 보고(`borrowed_layout.rs:165-167`) `ArrangementUnknown` 은 그 뒤 보는 중 단축키 경로에서만 본다(`borrowed_layout.rs:236-243`). 이번 기능과는 무관하다.
- `docs/play-options.md` 10.15 의 내용과 `borrowed_layout.rs` 의 동작은 일치했다.

## 2. 구현 지점과 설계안

### 2.1 rbms-chart: 역탐색 함수(새 공개 함수 하나, 기존 함수 불변)

위치: `crates/rbms-chart/src/shuffle.rs`. 비공개 `Rng`·`fisher_yates`·`side_key_lanes` 를 그대로 써야 판과 같은 순열이 보장된다.

제안 형태(이름은 예시)

- `pub fn random_seed_for(mode: Mode, wanted: &[usize], cancel: &AtomicBool) -> Option<u64>`: `wanted` 는 `lane_permutation` 과 같은 형식(`perm[old] = new`, 길이 `mode.key`, 스크래치 항등). 반환 시드 s 는 `lane_permutation(Random, mode, s) == wanted` 이고 1 ≤ s < 2^53.
- 탐색: s = 1 부터. 쪽 0 순열을 할당 없이(고정 크기 배열) 계산해 비교한다. DP 는 쪽 0 이 맞을 때만 `s+1` 로 쪽 1 을 계산한다(1.1 의 `seed+side` 관계). 일정 간격마다 `cancel` 을 본다.
- 상한: 모드별 이름 붙은 상수(매직넘버 금지 규칙). 측정값보다 넉넉히 둔다. 예: SP 는 2^23(8,388,608 > 4,775,533), 10K 는 2^21(2,097,152 > 1,250,411), 14K 는 2^29(536,870,912 > 468,482,176). 상한까지 못 찾으면 `None` = "RANDOM 으로 만들 수 없는 배치". 24K 와 섞을 레인이 없는 모드는 바로 `None`.
- 결과를 `lane_permutation` 으로 한 번 더 맞춰 보고 돌려준다(이중 확인).
- 대안: 시드 시작점을 `fresh_layout_seed` 로 두어 판마다 다른 시드를 쓰기. 장점: 같은 배치의 판들이 같은 시드를 공유하지 않는다. 단점: 탐색 시간이 결정적이지 않고, 상한 논리(측정 범위)가 시작점에 따라 달라져 보장이 약해진다. **추천: 1 부터의 결정적 탐색**. 같은 배치 = 같은 시드여도 기록·리플레이에 문제가 없다(시드는 배치 재현용 값이다).
- 10K 의 불가능 쌍은 상한 2^21 이면 릴리스에서 수십 ms 안에 `None` 이 나온다. 미리 "P1 을 고정했을 때 가능한 P2"를 알려주고 싶으면 같은 함수로 시드 1~2^21 을 한 번 훑어 표를 만들 수 있다(선택).
- 함께 필요할 수 있는 공개 함수: `pub fn apply_lane_permutation(model: &mut Model, perm: &[usize])`(지금의 비공개 `apply_perm` 공개판). 시드가 나오기 전(14K 탐색 중)에도 원하는 배치로 미리보기를 바로 그릴 수 있다. 시드가 나온 뒤에는 `apply(model, Random, seed)` 결과와 같다(시험으로 고정).

### 2.2 배치 문자열 규칙

- **문자열의 i 번째 숫자 = 화면 왼쪽부터 i 번째 레인에 오는 원래 레인 번호(1부터)**. 원본 beatoraja 의 표시와 같다: `LaneShuffleModifier.modify` 에서 `random[i]` 는 레인 i 에 놓일 원래 레인이고(`/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/pattern/LaneShuffleModifier.java:58-88`), 스킨 숫자는 `pattern[index] + 1`(`skin/property/IntegerPropertyFactory.java:1477-1478`)이다.
- 이 저장소의 `perm[old] = new` 와는 역순열 관계다: 문자열 d 에서 `perm[d[i] - 1] = i`. OFF = `1234567`, MIRROR = `7654321`(대합이라 두 규칙이 같다), 골든 시드 = `5347216`.
- 숫자는 쪽 안의 번호다. DP 는 P1 문자열과 P2 문자열 두 개(14K 각 7자리, 10K 각 5자리). P2 의 `1` 은 레인 8(14K) 또는 6(10K)이다. 9K 는 9자리.
- 스크래치는 문자열에 넣지 않는다(RANDOM 이 스크래치를 고정하므로).

### 2.3 화면: 선곡 위에 여는 내장 화면 "LAYOUT PICK"(추천)

이유: 차트 모델이 필요하고, 미리보기 영역이 크고, 레인 키를 입력으로 쓰므로 선곡의 키 경로와 겹치면 안 된다. 연습 패널이 같은 이유로 독립 화면이다(`stage/practice.rs:3-8`).

| 항목 | 제안 |
| --- | --- |
| 새 Stage | `Stage::LayoutPick(Box<LayoutPickState>)`, `StageId::LayoutPick`(`stage/mod.rs:70-104` 와 이를 다 나열하는 곳: `stage/mod.rs:218,237,256,275,293`, `discord_presence.rs:274` 부근, `trace_backfill.rs:649`) |
| 여는 길 | 선곡의 새 단축키(`Shortcut` 에 하나 추가, `keyconfig.rs:923-957` 기본 키, 그룹 Browser). 커서가 차트일 때만 연다. 코스 막대·폴더·없는 곡이면 열지 않거나 알림. 열 때 `Transition::Open(Stage::LayoutPick(..))` |
| 기본 키 후보 | 비어 있는 것: `F7`, `F10`, `F11`(단, `app_options.rs:637-655` 시험이 OptionsHold 를 `F7` 로 옮겨 쓴다), 글자 `Y`(시험 외 사용 없음, `app_input.rs:145` 는 원본 키 코드 표), `U`, `E`(`ir_ranking_view.rs:49` 패널 키). 레인 키(`Z S X D C F V`, 10K·14K 의 `M K , L . ; /`)와 START `A`·SELECT `W`, 기존 단축키 `J R I F O T N H` 는 피한다. 사용자 결정 필요 |
| 기록 모달 보조 진입(선택) | 리플레이가 있는 기록에서 "이 배치로 지정 화면 열기": 리플레이의 옵션이 레인 순열 옵션(OFF·MIRROR·RANDOM·R-RANDOM·ROTATE)이면 `lane_permutation` 으로 문자열을 채워 연다 |
| 들어올 때 | 차트 파싱을 작업자에 맡기고 "READING" 표시. 커서 차트의 경로·모드는 `shared.focused_song_index()`(`app_input.rs:605`)와 `library.songs()[i]` 에서 얻는다 |
| 입력(키보드 숫자) | 숫자 줄·넘패드 `1`~`m` 을 치면 왼쪽부터 채운다. 이미 쓴 숫자나 범위 밖 숫자는 받지 않고 그 칸에 1.5초 표시(PLAY 메뉴의 `NOT A NUMBER` 관례). `Backspace` 한 칸 지움, `Delete` 또는 `0` 은 OFF 순서로 되돌림(PLAY 메뉴의 0=기본값 관례, 7.3절) |
| 입력(건반) | 레인 키를 누르면 그 레인 번호를 다음 칸에 넣는다("왼쪽 레인부터 원하는 건반을 차례로 누른다"). 키보드는 `shared.keyconfig.lane_keys(mode)` 로 **그 차트 모드의** 레인을 찾는다(`lane_input_for` 는 마지막 로드 모드의 `active_keys` 를 본다, `app_input.rs:235-238`, `app_play.rs:535`). 패드는 `handle_pad` 의 `PadEvent::Lane { lane, .. }`(`gamepad.rs:392-406`). DP 는 누른 쪽 문자열에 들어간다. 스크래치 레인은 무시 |
| DP 쪽 전환 | `Tab`/`Shift+Tab`. 한쪽만 정하고 다른 쪽을 "아무거나"로 두는 선택지(빈 문자열)를 두면 탐색이 SP 비용으로 줄어든다(추천: 둘 다 허용) |
| 미리보기 조작 | `↑ ↓` 한 마디, `PageUp PageDown` 네 마디(또는 한 화면), `Home End` 처음·끝, 휠. 위치는 `TimeLine.section` 기준으로 옮기고 화면에 마디 번호·시각을 쓴다. 레인 키를 입력에 쓰므로 글자 키는 명령에 쓰지 않는다 |
| 시작 | `Enter`: 문자열이 완성되고 시드를 찾았을 때만. 아니면 이유 표시. 시작 = `borrow_layout`(2.4) |
| 나가기 | `Esc`: 선곡으로(`Transition::Back`). 작업자 취소 |
| 손으로 치는 판 확인 | 출하 기본값이 AUTOPLAY 켬이다(`docs/acknowledge/2026-10-24-adr-layout-play.md` 3), `crates/rbms-config/src/schema.rs:605`). 화면을 열 때 `plays_by_hand()`(`borrowed_layout.rs:267-269`, 지금 비공개)를 보고 "AUTOPLAY 가 켜져 있어 기록되지 않는다/시작할 수 없다"를 미리 보인다 |
| 문구 | 새 `translations!` 카탈로그(`apps/rbms-player/src/text/` 아래, 예: `text/menus/layout_pick.rs`) 세 언어. 단축키 이름, 안내 줄, 거절 이유(만들 수 없는 배치, 24K, 손으로 치는 판이 아님), 탐색 중, 알림 |

대안 비교

| 대안 | 장점 | 단점 |
| --- | --- | --- |
| A. 새 내장 화면(추천) | 키 경로 독립, 미리보기를 화면 첫 그리기로 그려 rbms-render 수정 불필요, 내장·스킨 선곡 둘 다 같은 화면, Lua 수정 없음 | Stage 추가로 나열 지점 몇 곳 수정. 선곡 BGM·미리듣기가 화면 전환에서 어떻게 되는지 확인 필요(PLAY 메뉴와 같을 것, 미확인) |
| B. 선곡 오버레이 모달 | 화면 전환 없음 | `render_playfield_view` 의 `clear` 때문에 `PlayfieldView` 에 "지우지 않음" 필드를 더하거나 함수를 쪼개야 한다(호출처 3곳: `stage/play/mod.rs:1735`, `crates/rbms-render/tests/golden_play.rs:77`, `examples/render_frame.rs:47`, 그리고 골든 PNG 영향 가능). 스킨 선곡의 건반 프레임 읽기와 모달 키 경로가 얽힌다 |
| C. 연습 패널처럼 로드 경로로 들어가 그 자리에서 바로 시작 | 시작할 때 다시 파싱하지 않음 | LOADING(키음·BGA 디코드)을 미리보기 전에 치른다. 연습 쪽 구조(`PracticeChart`)를 넓혀야 하고 저장 규칙(연습은 기록 안 함)과 섞인다 |

### 2.4 시작과 기록: `borrow_layout` 재사용(추천)

- 시작: `shared.borrow_layout(LayoutRequest { md5: 라이브러리 항목 md5, random_label: NoteOption::Random.label(), seed: 찾은 시드, source: 표시 문자열 }, chart_index)`. 돌아오는 `Transition::Open(decided_song_stage)` 를 이 화면에서는 `Transition::To(..)` 로 바꿔 돌려준다(`Transition` 정의 `stage/mod.rs:110-121`: To 는 이 화면을 갈아 끼우고 아래의 선곡은 그대로 둔다).
- 그러면 바뀌는 것 없이: 판 동안 `config.play.random = Random`, 끝나면 내 옵션 복원(`borrowed_layout.rs:321-333`, `app_play.rs:103-110`), 기록·제출·리플레이는 RANDOM + 시드(1.3), 같은 배치 재시도에서 유지, 선곡으로 돌아오면 버림.
- 알림: 지금은 `NoticeText::LayoutOf` = "{0}의 배치 ({1})"(`text/notices.rs:172-176`)가 `source` 와 옵션 라벨로 나온다. `source` 에 "지정 배치 3517246" 같은 번역 문구를 넣거나, 지정 배치용 새 알림 키를 두는 편이 자연스럽다(`borrow_layout` 에 알림을 고를 길이 필요. 작은 변경).
- 고스트: `Borrowing.kept = None` 이므로 TARGET 행 그대로(`borrowed_layout.rs:189-195`).
- 대안: `Borrowing` 과 별도로 `picked_layout: Option<(String md5, u64 seed)>` 를 두고 `layout_to_load` 에 갈래를 더하기. 장점은 출처 구분이 분명함, 단점은 수명·복원 규칙(재시도 유지, 버림, own_play_options)을 다시 구현해야 함. 추천하지 않는다.

### 2.5 미리보기 데이터와 작업자

- 작업자 스레드 하나(선곡 분석 작업자 `stage/select/analysis.rs:21-80` 의 Job/Reply/id 패턴을 따름. 이번 화면 전용이라 요청은 하나뿐).
  1. `std::fs::read(path)` → `app_play::decode_chart(bytes, path)`(같은 `#RANDOM` 분기, 같은 모드 판별, bmson 포함).
  2. LN 표시를 판과 맞추려면 `rbms_chart::resolve_long_note_flavour(&mut model, ln_mode.resolve())`(`app_play.rs:512`)를 플레이어의 LN MODE 로 적용한다. RANDOM 은 레인 단위라 LN 처리와 위치가 무관하다(`borrowed_layout.rs:142-147` 이 노트 단위 옵션만 경고하는 이유).
  3. 원본 모델을 돌려준다. 순열 적용은 프레임 쪽에서 문자열이 바뀔 때만 복제 + `apply_lane_permutation` 으로 한다(타임라인 수 x 레인 수의 옮기기라 가볍다, 미측정). 또는 작업자에 다시 맡긴다.
  4. 시드 탐색은 같은 작업자 또는 별도 작업자에 `cancel: Arc<AtomicBool>` 과 함께 보낸다. 문자열이 바뀌면 이전 탐색을 취소한다.
- 그리기: 화면 `draw` 의 첫 호출로 `render_playfield_view(r, &preview_skin, &PlayfieldView { timelines: &shuffled.timelines, microtime: cursor_us, hispeed: 미리보기 상수(예: 0.25 = 네 마디), constant: 사용자 설정 또는 true, legacy_note: 설정, auto_lanes: scratch_auto_lanes(mode, 설정), beam_on/off: &[] .. })`. 그 위에 레인 아래 번호(`preview_skin.x[lane]`, `w[lane]` 중앙), 입력 칸, 안내 줄, 상태(탐색 중/시드/거절)를 그린다.
- "레인 번호 순서 표시": 각 레인 아래에 그 레인에 오는 원래 번호(문자열의 숫자). 입력 중 빈 칸은 `-`. 레인 위에 원래 번호의 색(흰/검 건반 색)을 두면 읽기 쉽다(선택).

### 2.6 변경 파일 예상

| 파일 | 변경 |
| --- | --- |
| `crates/rbms-chart/src/shuffle.rs` | `random_seed_for`, (선택) `apply_lane_permutation`, 모드별 상한 상수, 시험 |
| `apps/rbms-player/src/stage/layout_pick.rs`(새), `stage/layout_pick/tests.rs`(새) | 화면, 입력, 작업자, 그리기 |
| `apps/rbms-player/src/stage/mod.rs` | `Stage`/`StageId` 변형과 나열 지점 |
| `apps/rbms-player/src/stage/select/mod.rs` | 새 단축키 처리(`run_shortcut` 의 `match`, `mod.rs:1102-1152`) |
| `apps/rbms-player/src/keyconfig.rs`, `keyconfig_tests.rs` | `Shortcut` 추가(`ALL`, 그룹, `config_key`, 라벨, `default_key`, `takes_a_pad`) |
| `apps/rbms-player/src/stage/select/guide.rs` | 단축키 안내 문구(전수 `match`, `guide.rs:151-169`) |
| `apps/rbms-player/src/borrowed_layout.rs` | `plays_by_hand` 를 `pub(crate)` 로, 지정 배치용 알림 갈래(선택) |
| `apps/rbms-player/src/text/menus/layout_pick.rs`(새), `text/menus.rs`, `text/notices.rs` | 세 언어 문구 |
| `apps/rbms-player/src/discord_presence.rs`, `trace_backfill.rs` | 새 StageId 나열(필요한 곳만) |
| `docs/play-options.md`(새 절, 10.15 옆), `docs/acknowledge/`(새 ADR), `docs/quality-assurance/`(실기 점검·테스트 부채) | 문서 |

## 3. 영향받는 기존 기능·시험·골든

- 배치 골든(`shuffle.rs:832-893`): 기존 함수 불변이면 그대로. 새 함수 시험은 "찾은 시드의 `lane_permutation` 이 목표와 같다", "5K·7K 전 순열이 상한 안에서 나온다"(7K 46,287번은 디버그에서도 짧다), "10K 의 알려진 불가능 쌍은 `None`", "골든 시드의 7키 문자열 `5347216` 을 찾으면 그 시드 이하의 시드가 같은 순열을 낸다" 등. 14K 전수와 9K 전수는 디버그 시험에 넣지 않는다(시간). 14K 는 작은 시드(예: 시드 37 의 쌍)를 목표로 한 시험만.
- 빌린 배치 시험(`borrowed_layout.rs` tests, `app_result.rs:1192-1215`, `stage/result/tests.rs:811-863`, `stage/play/screen_tests.rs:664`): 재사용만 하면 영향 없음. 알림 갈래를 바꾸면 `text/notices.rs:878` 의 문구 시험과 `a_refusal_is_reported...` 류.
- 단축키: `Shortcut::ALL` 개수(32→33), `keyconfig_tests.rs:1041`, 충돌 검사, 키 설정 화면 행, 안내(H) 화면 행 수와 렌더 시험(`stage/render_tests_select.rs` 등, 미확인 범위).
- Stage 나열: `stage/mod.rs` 의 이름 표와 목록 시험(`:218-237`).
- 계약 골든 `contract/client-requests/*.json`(`app_result/record_tests/contract_requests.rs`): 제출 형식 불변이라 영향 없음.
- 웹·서버: 변경 없음(결정 (a)).
- 문서: `docs/play-options.md` 7.4 의 "RANDOM 은 미리보기에 보이지 않는다"는 PLAY 메뉴 이야기로 그대로 맞다.

## 4. 위험·엣지 케이스

| 경우 | 내용 | 제안 |
| --- | --- | --- |
| 10K 불가능 쌍 | 1,800쌍은 RANDOM 시드가 없다(1.4) | 거절 + 이유. P1 고정 시 가능한 P2 목록을 보여 주거나 가장 가까운 가능 쌍 제시(선택) |
| 14K 탐색 시간 | 릴리스 최악 수 초, 디버그 수십 초 | 작업자·취소·진행률. 상한 상수. 한쪽만 지정 허용 |
| 24K | 대부분 불가능, BMS 로는 안 나오는 모드 | 화면에서 "지원하지 않는 모드" |
| 9K | 9자리, 스크래치 없음. 레인 키 9개(`Z S X D C F V G B`) | 그대로 지원 |
| 5K·10K 건반 | 키 인덱스 5·6 에 닿는 키가 없다(`docs/play-options.md` 12.6)는 선곡 패널 이야기다. 이 화면은 레인 번호로 받으므로 영향 없음 | — |
| 항등·MIRROR 모양 | `1234567` 도 RANDOM + 시드 24088 로 기록된다(결정 (a) 그대로). 사용자가 OFF·MIRROR 로 기록되길 원할 수 있다 | 결정 필요(아래 5) |
| AUTOPLAY 켬(출하 기본) | `borrow_layout` 이 `NotFreePlay` 로 거절 | 화면을 열 때 미리 알림 |
| 코스·연습 | 거절 대상(`plays_by_hand`) | 코스 막대에서는 열지 않음 |
| 차트 파일이 라이브러리 스캔 뒤 바뀜 | 요청 md5(라이브러리)와 로드 md5(파일)가 다르면 `lay_out_as_borrowed` 가 조용히 버리고 내 RANDOM 으로 친다(`borrowed_layout.rs:321-326`) | 미리보기 파싱의 `decoded.md5` 와 라이브러리 md5 를 비교해 다르면 시작 전에 알림 |
| LOADING 에서 Esc | 기존 위험(테스트 부채 3.119): 다음 load 까지 설정 화면에 빌린 RANDOM 이 보일 수 있다 | 기존과 같음 |
| LN | 레인 순열이라 LN 머리·꼬리가 같은 레인. LN MODE 는 종류(LN/CN/HCN)만 바꾼다. 정의 안 된 LN 경고(`warn_of_a_borrowed_layout_the_ln_mode_decides`)는 RANDOM 에 해당 없음 | 미리보기 LN 색은 플레이어 LN MODE 로 해석 |
| 숨은 노트·지뢰 | `apply_perm` 이 `hidden` 도 옮기고 지뢰도 노트 칸이라 같이 옮긴다(`shuffle.rs:171-180`) | 미리보기도 같은 함수로 |
| SCRATCH AUTO | 스크래치는 RANDOM 이 고정 | 미리보기에서 자동 레인 색 |
| 모듈로 편향 | `next() % n` 편향은 배치 가능 여부와 무관(측정상 SP 전부 도달) | — |
| 같은 배치 = 같은 시드 | 결정적 탐색이면 지정 배치 판들이 같은 시드를 쓴다 | 문제 없음(2.1) |
| 시드 크기 | 최대 5억 미만, 2^53 미만, 0 아님 | — |
| 프레임 정지 | 프레임 스레드에서 파싱·탐색하면 멈춘다 | 작업자(2.5) |
| `render_playfield_view` 의 `clear` | 오버레이로 그리면 그 앞의 선곡 그림이 사라진다 | 독립 화면에서 첫 그리기로(2.3 A) |

## 5. 미확인

- 실제 `lane_permutation` 반복 호출의 시드당 비용(할당 포함)과 디버그 빌드에서의 14K 탐색 시간. 위 수치는 scratch 복제 코드의 측정이다.
- 대형 차트에서 `decode_chart` + 순열 적용(모델 복제) 한 번의 시간.
- 새 Stage 를 선곡 위에 열 때 선곡 BGM·곡 미리듣기의 동작(`end_select_bgm` 이 `on_exit` 에서 불리는지, PLAY 메뉴와 같은지).
- 새 단축키 기본 키의 충돌 여부(시험에서 빈 키로 쓰는 키, ControlAction 기본 키 `DEFAULT_COVER_TOGGLE_KEY`·`DEFAULT_OFFSET_UP_KEY`·`DEFAULT_OFFSET_DOWN_KEY` 의 값은 읽지 않았다, `keyconfig.rs:573-579`).
- 내장 `Skin` 을 미리보기 영역에 맞춰 만들 때 `SkinConfig::default()` 의 `top_y`·`judge_y` 단위(픽셀인지 비율인지)와 `bga` 영역 처리(`skin.rs:60-110`, 일부만 읽음).
- 10K 불가능 쌍의 수학적 원인(연속 시드 s, s+1 의 xorshift 저위 비트 관계로 추정, 증명하지 않음). 세 시드 영역에서 같은 집합인 것만 확인했다.
- 기본 스킨(Lua) 선곡에서 이 화면을 여는 스킨 단추나 확장 이벤트(21000~21099)를 둘지는 정하지 않았다. 내장 화면 방식이면 필수는 아니다.

## 6. 결정이 필요한 것(구현 전)

1. 화면 형태: **A. 선곡 위 독립 화면(추천)** / B. 선곡 오버레이 모달 / C. 연습처럼 로드 후 화면.
2. 여는 키: **새 단축키 하나(추천 후보 `Y` 또는 `F7`)** / 기록 모달 단추만 / 둘 다.
3. 항등·MIRROR 모양 문자열: **결정 (a) 그대로 RANDOM + 시드(추천: 일관성)** / OFF·MIRROR 로 기록.
4. DP 한쪽만 지정 허용: **허용(추천: 14K 탐색 비용을 SP 수준으로)** / 양쪽 필수.
5. 10K 불가능 쌍: **거절 + 이유(추천)** / 가장 가까운 가능 쌍 제안.
6. 지정 배치 문자열 기억: **세션 동안 모드별로 기억해 다음에 채움(추천, 설정 파일 변경 없음)** / 설정 파일에 저장(`#[serde(default)]` 필드 추가) / 기억 안 함.
