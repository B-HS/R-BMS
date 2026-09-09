# Phase A — 정확성 핫픽스 (2026-09-09)

> 근거 계획: `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase A. 산출물 원본(에이전트 작업 로그): `scratchpad/phase-a/{core,audio,ir,render,app}.md`(구현) + `review-*.md`(적대적 리뷰) + `fix-*.md`(리뷰 반영). 대상 커밋 `c6f0885` 기준 작업.

## 진행 방식

파일 소유권을 분리해 5갈래(core/audio/ir/render/app)를 병렬 진행한 뒤, 갈래별로 독립된 적대적 리뷰 1라운드 → 리뷰 반영을 거쳤다. `apps/rbms-player`는 다른 4개 크레이트 갈래가 끝난 뒤 2차로 통합했다.

- **A-core** — `crates/rbms-{model,parser,chart,judge,play}/**`
- **A-audio** — `crates/rbms-audio/**`
- **A-ir** — `crates/rbms-ir/**`
- **A-render** — `crates/rbms-render/**`
- **A-app** — `apps/rbms-player/**` + `crates/rbms-chart/src/scroll.rs`(그린넘버 한정)
- **A-parser(#SWITCH)** — `crates/rbms-parser/src/control.rs`(결정 10에 따라 A-core와 별도 갈래로 진행)

## 갈래별 변경 요약

### A-core (판정·게이지)

계획 §1.1 J1~J19 중 이번 과제 범위(J1·J2·J3·J4·J5·J7·J8·J10·J11·J12·J13·J15·J16·J18·J19)와 J14(`#DEFEXRANK`)를 전부 수정했다. 상세 대조표는 `docs/acknowledge/reference-divergences.md` "판정 윈도우·게이지 발산(2026-09-09 감사)" 섹션.

리뷰에서 계획 표에 없던 결함 3건이 추가로 드러나 함께 수정했다.
- 재타격(空POOR) 후보 스캔이 `update()`가 커서를 전진시킨 뒤에는 죽어 있던 결함(레인 스캔 시작점을 과거로 되돌리는 로직 추가) + 재타격 판정이 판정 사다리(PG→GR→GD→BD→MS)를 타 BD 안이면 空POOR가 안 나던 결함(MS 밴드 단독 검사로 정정).
- `JudgeEngine::from_model(model, windows)`이 `apply_mode`로 유도한 모드별 표를 키 note 표로 덮어써 5K/PMS가 실제로는 7K 윈도우를 쓰던 결함 — 공개 API는 유지하고 `from_model_for_mode(model)`을 신설, `rbms-play::Player::new`를 이쪽으로 전환.
- `scaled()`의 하한 클램프가 `max(1)`이라 `#DEFEXRANK`가 0에 가까울 때 원본처럼 판정폭이 (0,0)으로 붕괴하지 않던 결함 — `max(0)`(음수 방어만)으로 정정.

**보류(사유 명시)**: 지뢰 데미지 스케일(레퍼런스 구현 `BMSDecoder`가 바이너리라 1차 출처 미확인, 현재 값 유지), over-hold 조건의 `+ margin` 제거(지연 커밋 경로 자체가 rbms에 없어 근사 유지, 독스트링에 명시), PMS/KEYBOARD scratch 표 복제(두 모드 모두 스크래치 레인이 없어 현재 도달 불가 — 레퍼런스 구현 원본을 그대로 옮기면 "스크래치=전부 PGREAT"라는 더 위험한 축퇴를 이식하게 됨).

### A-audio (오디오 엔진)

A5(기본 마스터 게인)·A7(스트림 사망 감지)·P7(드롭 카운터·scratch 선할당)·A13(채널 키에 피치 반영) 전부 완료. 리뷰에서 마스터 게인 초기값을 0.7에서 **0.5**로 재조정했다(레퍼런스 구현 `keyvolume`/`bgvolume` 원본과 정확히 일치하도록 원본 라인을 재확인한 결과). 클럭 페어(프레임 수·나노초)를 짝/홀 시퀀스 카운터(seqlock)로 재구현해 콜백-리더 간 데이터 레이스로 찢어진 쌍이 반환되던 결함을 제거했다.

**보류**: A6(보이스 스틸 start/stop 램프)는 계획대로 Phase B 범위. `#VOLWAV` 반영·오디오 divergence 3건의 `reference-divergences.md` 등재는 A-audio 소유 파일 밖이라 문안만 인계(본 문서 및 §문서 정정 참조).

### A-ir (IR 클라이언트)

`http.rs:19`의 타임아웃 빌더 실패 시 무제한으로 조용히 강등되던 결함을 명시적 에러로 전환. HTTP 계층 mock 테스트 신규 11건(degraded 경로, 8개 엔드포인트 URL 검증, 요청 바디 필드). 워커(`spawn_query`) 테스트를 실제 회귀 신호를 내도록 재설계(초판 테스트는 단언 0개였다).

**보류**: URL percent-encoding(신규 의존성 추가+와이어 계약 변경이 필요해 서버 계약 동결 전으로 보류, 전제만 독스트링에 명시), `spawn_query`의 최종 위치(Phase F `ir_ranking.rs` 확정 시 재결정).

### A-render (렌더/폰트)

P1(폰트 레이아웃/글리프 런 캐시 무한 성장)에 LRU 상한 도입, K9(결과 화면 팔레트 스킨화) 완료. 리뷰에서 골든 PNG 하네스의 검출 감도가 너무 거칠다는 지적을 받아 격자를 16×9→64×36, 양자화 비트를 4→3으로 강화하고 CJK 부제를 ASCII로 교체(CI 3-OS 폰트 폴백 플래키 방지)했다. 글리프 런 캐시의 퇴출 시점을 문자열 처리 도중이 아니라 완료 후로 옮겨 "방금 그린 글리프가 조용히 실종"되는 결함을 수정.

**보류**: 계획 문구의 "10,000 문자열 후 RSS 상한" 검증과 골든 하네스 확장을 계획 문서에 반영하는 일 — 둘 다 `docs/plan/**`가 A-render 소유 파일 밖이라 코드 쪽 이행(엔트리 수 상한 + LRU 의미 고정, 검출 회귀 테스트)만 완료하고 문서 반영은 인계.

### A-app (앱 통합)

A3(키음/판정 시각 분리), IR 제출 게이트(autoplay/replay/judge rate>100% 차단), assist 플래그 실값 채우기, U3(Root Esc 확인), U4(플레이 중 조작 저장), P6(로딩 취소 플래그 배선), 원자적 저장(temp+rename) + `ScoreRecord.rule_version`, U6(표 URL 중복 검사), P3 디바운스, 오디오 스트림 사망 시 벽시계 폴백, 그린넘버 매직상수(`2000.0`) 통합을 전부 완료. 리뷰에서 `keyconfig.ron` 비원자 저장, ArrowLeft 무확인 종료, IR `combobreak`가 5K/PMS에서 空POOR를 과소 집계하던 결함(모드별 combo 테이블에서 유도하도록 수정)을 추가로 반영.

리뷰 과정에서 레퍼런스 구현 원본을 재확인해 두 가지를 정정했다: (1) "어시스트 런은 로컬 스코어를 갱신하지 않는다"는 절반만 맞다 — 원본은 clear(램프)는 무조건 갱신하고 exscore/avgjudge/minbp/combo만 게이트한다. rbms는 아직 램프 강등(`LightAssistEasy`)이 없어, 강등 없이 clear만 무조건 갱신하면 어시스트 풀콤보가 정규 풀콤보로 보이는 위험이 있어 `best_clear_for_md5`가 어시스트 기록을 보수적으로 제외하는 방식을 택했다. (2) 그린넘버 LIFT 미반영은 발산이 아니다 — 레퍼런스 구현도 duration에는 `1 − lanecover`만 곱하고 lift는 기하(판정선 위치)만 바꾼다.

**보류**: 어시스트 램프 강등은 `rbms_judge::ClearType`에 해당 변형이 없어 크레이트 변경이 필요 — Phase D 이월.

### A-parser (`#SWITCH/#CASE/#SKIP/#DEF`, 결정 10)

`#SKIP`이 바깥 프레임 게이트를 확인하지 않아 비활성 중첩 분기 안의 `#SKIP`이 활성 케이스 전체를 끊던 high 등급 결함을 수정. `#CASE` 토큰 파싱을 `#SWITCH`/`#SETSWITCH`와 동일 규칙으로 통일하고, 파싱 실패한 라벨이 `#SETSWITCH 0`과 우연히 매칭되던 결함도 제거. `#SWITCH`+`#CASE`와 `#RANDOM`+`#IF`가 동일 시드에서 같은 값을 뽑는지 대조하는 패리티 테스트 추가.

**보류(문서화, 동작 유지)**: `#DEF`가 C `default:`와 달리 "그 지점까지의 matched"만 보는 단일 패스 구조 — C 의미로 바꾸려면 2패스 선버퍼링이 필요해 아키텍처 변경(소유 범위 밖)이 따른다. 독스트링에 이탈을 명시하고 동작은 유지, 사용자 판단이 필요한 항목으로 남겼다.

## 검증

전 갈래 공통: `cargo test --workspace` 전부 통과(실패 0), 소유 크레이트 `cargo clippy --all-targets` 신규 경고 0, 신규 `unwrap`/`expect`/`#[allow]` 미도입, 새 매직넘버는 상수화. 리뷰·수정 라운드는 갈래당 1회(적대적 리뷰 → 처리표 작성 → 재검증).

최종 시점 크레이트별 테스트 수(리뷰 반영 후): `rbms-model` 99 · `rbms-parser` 130 · `rbms-chart` 150 · `rbms-judge` 131 · `rbms-play` 49 · `rbms-audio` 99(+1 ignored) · `rbms-ir` 114(+1 ignored) · `rbms-render` 121(+골든 8) · `rbms-player`(앱) 158. 갈래가 동시에 작업 중이던 시점에는 크로스 갈래 in-flight 컴파일로 일시적 실패가 관측된 적이 있으나(예: judge 갈래 편집 중 render/audio 검증), 각 갈래 최종 실행 시점에는 전부 통과했다.

## 실기 가청 확인 (사용자 몫, 미실시)

Phase A는 헤드리스 환경에서 진행되어 실제 오디오 디바이스로 확인하지 못했다. `scratchpad/phase-a/app.md` §5 절차를 따른다.

1. `cargo run -p rbms-player -- <songs 폴더>`로 기동.
2. SETTINGS → JUDGE → `JUDGE OFFSET`을 `+100 MS`로 설정.
3. 곡을 인터랙티브(`AUTOPLAY OFF`)로 플레이하며 건반을 누른다 — **키음이 손가락과 동시에** 나야 한다(판정만 늦게 잡힘). 이전 빌드는 소리가 offset만큼(예 0.1초) 늦었다.
4. 결과 화면에서 콘솔 확인 — `score not submitted:`가 없어야 한다(offset은 assist가 아님).
5. `JUDGE WIDTH`를 `105%`로 올리고 다시 플레이 — 결과에서 `score not submitted: judge window widened`가 찍혀야 한다.
6. 플레이 중 hi-speed를 바꾸고 Esc로 이탈 → 재진입 시 값이 유지돼야 한다.
7. 로딩 중 Esc → CPU 사용률이 즉시 떨어져야 한다(디코드 중단).
8. 곡선택 루트에서 Esc 1회 → `PRESS ESC AGAIN TO QUIT`, 2초 뒤 Esc → 종료되지 않아야 한다(확인창 만료).

## 앱 통합 시 동작이 바뀌는 지점 (사용자 체감)

1. 판정 오프셋을 걸어도 키음이 밀리지 않는다(이전엔 offset만큼 지연/선행).
2. autoplay·리플레이·JUDGE WIDTH>100%·SCRATCH AUTO는 IR 제출이 막힌다(콘솔에 사유 출력). 로컬 기록은 종전대로 autoplay/replay만 제외.
3. 곡선택 루트 Esc가 1회로 종료되지 않는다(2초 확인창).
4. 플레이 중 바꾼 HI-SPEED/LANE COVER/LIFT가 저장된다.
5. 로딩 화면 Esc가 실제로 디코드를 중단시킨다.
6. 설정/기록/리플레이/폴더/표 파일이 temp+rename으로 저장된다.
7. 구 기록(`rule_version` 이전)에 표식이 붙는다(목록 ` *`, 모달 `OLD RULE`).
8. 오디오 스트림이 죽어도 곡이 멈추지 않고 벽시계로 이어 붙는다.
9. 마스터 게인 기본값이 0.7→0.5로 낮아져 체감 음량이 약 3dB 조용해진다(레퍼런스 구현과 동일 수준).

## 남은 항목 (Phase D 이월)

J6·J17·J20·J21·J22·J23(엔진 배선)·J24·J25·J26 — 상세는 `docs/acknowledge/reference-divergences.md` 신설 섹션과 계획 §2 Phase D. 어시스트 램프 강등, 지뢰 데미지 스케일(1차 출처 확인 필요), URL percent-encoding, 오디오 볼륨 3분리도 함께 잔존한다.

## 코드에서 옮긴 설계 노트

> `git diff HEAD` 에 새로 추가됐던 `//` 한 줄 주석 중 근거·레퍼런스 구현 라인·설계 의도를 담은 것을 코드에서 제거하며 이곳으로 옮긴다. 단순 구분선 주석(`// --- ... ---`)은 옮기지 않고 삭제만 했다.

### apps/rbms-player/src/app_input.rs
- 키음 재생 오프셋 처리 (입력 이벤트 처리 경로): 키음은 실제 입력이 눌린 순간(RECORDED input instant)에 울리고, 판정만 오프셋을 적용해 이동한다 — 그래서 오프셋을 건 채로 리플레이해도 오디오는 절대 밀리지 않는다.

### apps/rbms-player/src/app_play.rs
- 플레이 옵션 영속화 로직: 플레이 중 컨트롤로 바꿀 수 있는 hi-speed / lane cover / lift 를 저장해, 곡 중간의 조정이 다음 플레이·다음 실행까지 이어지도록 한다.

### apps/rbms-player/src/app_select.rs
- 곡선택 복귀 처리: 곡 중간에 이탈해도 플레이 중 바꾼 hi-speed / lane cover / lift 값은 유지된다.

### apps/rbms-player/src/format.rs
- `#RANK` 파싱: 이름 항목이 없으면(`"?"`), 범위를 벗어난 `#RANK` 값은 가장 가까운 밴드로 클램프되지 않고 NORMAL(75%)로 폴백한다 — 레퍼런스 구현의 기본 judgerank 동작과 일치.

### apps/rbms-player/src/ir_map.rs
- 콤보 리셋 판정 (`combo` 테이블 매핑): BEAT_7K 의 `JudgeProperty.combo`는 `[t, t, t, f, f, t]` — BAD(인덱스 3)와 스윕된 POOR(인덱스 4)만 콤보를 끊는다. `counts = [PG 9, GR 8, GD 7, BD 6, PR 5, MS 4]` 이면 `6 + 5`가 이어진 콤보.
- FIVEKEYS/PMS 는 `combo` 테이블이 `[t, t, t, f, f, f]`라 빈 POOR(MS)도 콤보를 끊는다 — 따라서 `6 + 5 + 4 = 15`.

### apps/rbms-player/src/keyconfig.rs
- `keyconfig.ron` 저장: settings/scores/replay 와 동일한 temp+rename 원자적 쓰기 경로를 탄다.

### apps/rbms-player/src/main.rs
- 취소 로직(뒤로가기 확인): 두 back 키 외의 다른 키를 누르면 대기 중인 "다시 누르면 종료" 상태가 취소된다.
- `A3` 테스트(키음 스케줄링은 판정 오프셋과 무관함을 검증): 키음은 키가 눌린 순간을 오디오 클록에 리베이스해 울려야 한다. 노트가 120BPM 바 1(1_000_000us)에 있을 때 raw 1_000_000에 입력하고 오디오 클록이 7_500_000us 앞서 있으면, 판정 오프셋과 무관하게 항상 8_500_000에 스케줄되어야 한다. +30ms 오프셋은 같은 입력의 판정만 30ms 늦출 뿐 스케줄은 그대로다 — 만약 press 호출이 `judge_t`를 오디오 경로에 넘겼다면 두 번째 assert 는 8_530_000을 읽게 된다.
- 오디오 죽음(dead-audio) 시 벽시계 폴백: 12.0초에 스트림이 죽고 벽시계로 2.5초가 더 지났다면 14.5초여야 하며, 절대 0으로 되돌아가면 안 된다.
- 그린 넘버(green number) 계산: CONSTANT 모드는 `2000 / hispeed`를 보이는 창 비율로 스케일한다(BPM·SCROLL 무시). `green_number` 인자 순서는 `(constant, bpm, hispeed, scroll, cover)`. FLOATING 모드는 `(240000 / bpm / hispeed) / scroll * (1 - cover)`. cover는 f32라 f32 정밀도 허용 오차로 비교한다: `2000 * (1 - 0.4) = 1200`.
- IR 제출 정책 (레퍼런스 구현 `MusicResult.java:82` + `BMSPlayer.java:200-213/233`): 로컬 점수책과 IR은 매 실행마다 반드시 같은 결과를 내야 한다(레퍼런스 구현의 단일 `score` 플래그와 동일).
- `U3` 테스트: 곡선택 루트에서 Esc를 다시 누르면 종료.
- 원자적 config 쓰기: 두 번째 쓰기가 첫 번째보다 짧으면(예: 축약된 상태 저장) 첫 쓰기의 꼬리가 남아선 안 된다 — 일반 `fs::write`가 도중 실패할 때 발생하는 절단 위험을 막기 위함.

### apps/rbms-player/src/scores.rs
- 기록 저장 정책: 넓혀진 판정 창 / auto scratch 로 플레이해도 `for_md5` 조회에는 잡히지만(점수책엔 남음) best EX나 램프 LED 는 갱신하지 않는다 — 어시스트 없는 40 / 램프 5가 그대로 유지.
- `rule_version` 필드가 없던 시절 기록은 로드 시 `rule_version 0`으로 취급되어 구버전 표식이 붙는다.
- `scores.ron`은 atomic temp+rename 경로로 쓰여서, 임시 파일이 결과물로 남아있으면 안 된다.

### crates/rbms-audio/src/engine.rs
- 버퍼 크기 계산: `4096 frames * 2 channels`, `8192 frames * 2 channels`.
- 384-frame 디바이스 버퍼(레퍼런스 구현의 `deviceBufferSize` 기본값)도 4096-frame 하한선을 적용받아, 이후 더 큰 콜백이 와도 재할당하지 않는다.
- writer 는 `nanos = frames * 7`을 유지하므로, 서로 다른 두 write 가 섞이면 비율이 깨진 것으로 바로 드러난다.

### crates/rbms-audio/src/mixer.rs
- 팬/게인 계산 예시: `L=0.5, R=0.25`, 중앙 팬, unity 게인에 0.7071 스케일 후 0.5 master headroom 적용. `l=1.0*g, r=0.0`이면 mono out = `(l+0)*0.5` 후 0.5 headroom.
- 기본 헤드룸: full-scale 근처 보이스도 기본으로 헤드룸을 남긴다 — `0.5 * DEFAULT_MASTER_GAIN(0.5) = 0.25`, 리미터의 선형 구간 안쪽. 0.5는 레퍼런스 구현의 keyvolume/bgvolume 기본값.
- 소프트 리미터(A5) 계산 근거: `value 0.9 * master 4.0 = 3.6` → `soft_limit(3.6) = 0.8 + 0.2*tanh(2.8/0.2) = 0.8 + 0.2*tanh(14)`, f32에서 `tanh(14)`는 1.0으로 반올림되어 결과가 정확히 풀스케일에 닿고 절대 넘지 않는다. `value 0.5 * master 2.0 = 1.0` → `soft_limit(1.0) = 0.8 + 0.2*tanh(1) = 0.95231883119115`(하드 클램프였다면 1.0). 소프트 니 상수: `0.8 + 0.2*tanh(1) = 0.95231883119115`, `0.8 + 0.2*tanh(2) = 0.96402758007581...`.
- 보이스 수 증가에 따른 클리핑 방지: 보이스 하나당 좌채널에 0.5씩 기여(모노, full-left 팬)하므로 리미터 이전 버스는 `0.5 * n * DEFAULT_MASTER_GAIN(0.5) = 0.25n`. 하드 클리핑이면 n>=4부터 전부 1.0으로 뭉개지지만 소프트 니는 이를 구분해 풀스케일 아래로 유지한다. n=8(버스 2.0, tanh 인자 6.0)을 넘으면 니가 f32 정밀도 안에서 포화되므로, 표현 가능한 범위에서만 엄격한 증가를 검증한다. `64 * 0.5 * 0.5 = 16.0` → `tanh(76)`이 f32에서 1.0으로 반올림되어 니가 정확히 풀스케일에 닿는다 — 현실적인 보이스 수에서의 니의 부드러움은 `simultaneous_voices_increase_monotonically_without_clipping` 테스트로 고정. `4 * 0.5 * 0.5 = 1.0` → `0.8 + 0.2*tanh(1) = 0.95231883119115`(하드 클램프면 1.0). `3 * 0.5 * 0.5 = 0.75 <= 0.8` 임계값이라 합은 그대로.
- 피치 인지 채널 키(A13): `2^(1/12) = 1.0594630943592953`(반음 위), `2^(-3/12) = 0.8408964152537145`(3반음 아래). `AbstractAudioDriver.java:507-509` 의 `channel(id, pitch) = id * 256 + pitch + 128` 공식을 따른다. 둘 다 semitone 0으로 양자화되면 두 번째 재생이 첫 번째를 끊는다.

### crates/rbms-chart/src/scroll.rs
- 그린 넘버 계산 근거: 레퍼런스 구현 스킨의 GREEN 값은 `0.6 x currentduration`(`IntegerPropertyFactory.java:555-556`)이고, `green_number`는 `currentduration`(`LaneRenderer.java:330`)을 반환하므로 120BPM/hs 1.0/커버 없음에서 2000ms이지만 레퍼런스 구현 스킨은 `240000/120/1 * 0.6 = 1200`을 표시한다.
- CONSTANT 스크롤은 hi-speed 1.0이 레인을 `240000/120 = 2000ms`에 통과하도록 캘리브레이션됨. 25% 커버는 레인의 75%만 보이게 하므로 `2000 / 2.0 * 0.75 = 750ms`. 120BPM/SCROLL 1.0에서는 FLOATING과 CONSTANT 그린 넘버가 설계상 일치한다. 픽셀 이동은 보고된 그린 넘버와 일치해야 한다 — 정확히 그 ms가 지나면 노트가 레인 상단에 있어야 함.

### crates/rbms-chart/src/tests.rs
- `NoteKind::Mine { damage }` 파싱 (파서→모델 스케일 고정 테스트): D1-E9 오브젝트 id 가 차트의 `#BASE`(기본 36, `#BASE 62` 이면 62) 기준 데미지 값으로 읽힌다 — 상세 표는 `docs/reference/_appendix-raw.md` "Mine damage encoding". 레퍼런스 구현 자체 제너레이터는 10을 사용(`MineNoteModifier.java:11`, 여기서는 `0A`)하며 게이지는 이 값을 그대로 뺀다(`JudgeManager.java:246`).
- `calculateDefaultTotal` 관련 테스트: 레퍼런스 구현 공식은 `max(260, 7.605n / (0.01n + 6.5))`. n=1000 이면 `7605 / 16.5 = 460.909...`. n=100 이면 `760.5 / 7.5 = 101.4` → 바닥값 미만. `#TOTAL` 이 있는 차트는 `max(300, 7.605(n + 100) / (0.01n + 6.5))`이며 n=1000 이면 `7.605*1100 / 16.5 = 507.0`.

### crates/rbms-judge/src/gauge.rs
- `#TOTAL` 미기재 차트의 게이지 초기화: 표준 BMS 공식으로 폴백하며 200 고정값이 아니다.
- 空POOR(`counts[5]`)은 노트를 소모하지 않아 풀콤보를 깨지 못한다 — 오직 BAD 와 見逃し POOR(놓친 POOR)만 콤보를 끊는다.
- 풀콤보 시 게이지 상한: 見逃し POOR만(`counts[4]`) 램프에 영향, 空POOR(`counts[5]`)은 노트를 소모하지 않으므로 풀콤보면 램프가 그대로 상한을 찍는다.
- `GrooveGauge` TOTAL 보정 (레퍼런스 구현): 양수 델타는 `total / notes` 로 스케일된다 — NORMAL PG 델타 `1.0 * 300 / 1000 = 0.3`.
- LIMIT_INCREMENT: `pg = clamp((2*total - 320) / notes, 0, 0.15)`, 이후 모든 양수 델타가 `pg / 0.15`로 스케일된다. total 300, notes 1000 → `(600-320)/1000 = 0.28` → 0.15로 클램프되어 HARD 는 자신의 0.15 PGREAT 게인을 그대로 유지. total 200, notes 1000 → `(400-320)/1000 = 0.08` → PGREAT 게인 `0.15 * (0.08/0.15) = 0.08`.
- `#TOTAL` 미기재 차트는 과거엔 200 고정이었으나 이제 `max(260, 7.605n/(0.01n+6.5))`를 쓴다 — n=1000 이면 460.909..., 그래서 PGREAT 게인도 0.2가 아니다.

### crates/rbms-judge/src/lib.rs
- 스윕된 미판정 노트: 레퍼런스 구현 `JudgeManager.java:598` 는 스윕 시 judge code 4(見逃し POOR)를 부여하며 code 5가 아니다 — 그래서 `counts[4]`에 쌓이고 PR 게이지 델타를 받는다. NORMAL 게이지 초기값 20.0, PR 델타는 -6.0(`GaugeProperty`)이며 과거 스윕 노트가 쓰던 MS -2.0이 아니다. 300ms 이른 프레스는 空POOR(MS 델타 -2.0).
- 이미 판정된 노트에 대한 재입력: 레퍼런스 구현 `JudgeManager.java:400-401` — 이미 상태가 설정된 노트에 대한 press 는 window index 4 안이면 judge code 5(空POOR), 밖이면 code 6(무판정)이며 노트 자체는 재소모되지 않는다.
- 7K ln_end: gd는 `±200_000`, bd는 `(-280_000, 220_000)`이며 210ms 이른 릴리즈는 `dm +210_000` → 종료 BD.
- 오버-홀드된 일반 LN: 레퍼런스 구현 `JudgeManager.java:572-577` 는 여전히 홀드 중인 LN 이 끝을 지나면 `lnstartJudge`(헤드 판정)로 확정하며, 7K longnoteMargin이 0이라 끝을 지나는 순간 바로 확정된다(과거엔 하드코딩된 200ms를 기다렸다가 끝 창으로 재판정해 스퓨리어스 BAD 를 냈었음). `JudgeManager.java:573`은 `lnstartDuration`(헤드 델타)을 `mfast`로 넘기므로 커밋은 헤드의 방향을 물려받는다 — 헤드가 정확히 dm=0 이면 EARLY. `JudgeProperty.java:34`가 PMS longnoteMargin=200ms 를 주고 `PlayerConfig.longnoteMarginRate`(`JudgeManager.java:186`)로 스케일된다 — 100%면 오버-홀드된 일반 LN은 끝난 뒤 200ms에 확정, 0%면 즉시 확정. 30ms 늦은 헤드(dm=-30000): 7K GREAT 창은 ±60ms 라 헤드는 GREAT, `mfast=-30000<0`이라 LATE.
- `JudgeManager.java:652`: `mfast >= 0`을 "fast" 플래그로 사용하므로 dm 이 정확히 0이면 EARLY(과거엔 fast/slow 어디에도 속하지 못하고 드롭됐음).
- 모드별 테이블(콤보/스크래치 창/지뢰 데미지): `JudgeProperty combo[5]`는 SEVENKEYS true(콤보 유지), FIVEKEYS false(콤보 리셋). `JudgeProperty.java:12` FIVEKEYS GOOD은 ±100ms, BAD ±150ms; `:23` SEVENKEYS GOOD은 ±150ms — #RANK 3(100%)에서 120ms 이른 프레스는 5K에서 BAD, 7K에서 GOOD이어야 각 모드 테이블이 실제로 키 레인에 반영된 것. 7K 키 PGREAT은 ±20ms, 스크래치 PGREAT은 ±30ms — #RANK 3(100%)에서 25ms 이른 프레스는 키 레인 GREAT, 스크래치 레인(7번) PGREAT. `JudgeManager.java:244-247`: 레인 키가 눌린 채 지뢰를 지나면 게이지에서 데미지를 뺀다(NORMAL 초기 20.0). `GrooveGauge.setValue`는 value>0일 때만 쓰므로 죽은 게이지는 0에 머문다.
- `JudgeManager.java:400-401`: 이미 판정된 노트에 대한 press가 MS 창에 들어가면 judge code 5. 레퍼런스 구현 는 매 프레임 `prevmtime + mjudgestart - 100000` 지점에서 레인을 다시 마킹(`JudgeManager.java:220`)하므로 해결된 노트도 프레임을 넘어 재입력 후보로 남는다 — 이 scan-back 이 없으면 `update`가 커서를 이미 진행시켜 press 가 아무것도 못 찾는다. scan-back 하한은 `press + mjudgestart - 100000`이며, 그보다 과거는 레퍼런스 구현의 마킹이 이미 노트를 버린 뒤라 MS 밴드(-150ms)도 더 좁다.
- CN(charge note) 은 release 로만 해소된다 — 끝이 BAD-late 경계를 지나도록 미해소면 見逃し POOR(레퍼런스 구현 `JudgeManager.java:615-626`)이지 재판정된 끝이 아니다.

### crates/rbms-judge/src/matcher.rs
- 노트 창(window)은 이미 스케일된 채로 전달되고, 나머지 테이블도 같은 모드 + `#RANK`/`#DEFEXRANK` 정책을 따라 스크래치 레인·롱노트 릴리즈가 보조를 맞춘다.
- 후보 게이트: 레퍼런스 구현 는 NOTE/SCRATCH 테이블의 가장 넓은 경계를 하나의 차트-전역 창(`mjudgestart`/`mjudgeend`)으로 접어서, 레인별로 좁히지 않는다.
- 레퍼런스 구현는 각 레인을 스캔 전에 `prevmtime + mjudgestart - 100000`에서 재마킹(`JudgeManager.java:220`)하므로, 이미 해소된 노트도 게이트가 닿는 한 계속 후보로 남는다 — 이것이 커서가 진행된 뒤에도 空POOR 재입력 경로(`:400-401`)를 살아있게 한다. 커서에서 시작하지 않고 같은 하한까지 되짚어간다.
- 이미 해소된 노트에 대한 재입력은 MS 밴드에 들어가면 空POOR(레퍼런스 구현 `JudgeManager.java:400-401`: 이미 상태가 설정된 노트는 window index 4를 직접 검사해 안이면 code 5, 밖이면 code 6 — PG/GR/GD/BD 사다리를 타지 않는다).
- 空POOR: press 가 BAD 너머 넓은 MS 밴드에만 들어간 경우. 레퍼런스 구현 `JudgeProperty.SEVENKEYS`는 `judgeVanish[5]=false`, `combo[5]=true`로 설정하므로 7K에서 노트를 소모하지도 콤보를 끊지도 않는다 — MS 게이지 페널티와 `ems`/`lms` 집계만 발생하고, 노트는 실제 도착 시점에 여전히 히트 가능하다.
- 롱노트 끝 테이블에는 MS 페어가 없어 BAD 밖은 전부 judge code 4(見逃し POOR).
- 홀드된 CN/HCN이 release 없이 끝이 BAD-late 경계를 지나면 見逃し POOR(레퍼런스 구현 `JudgeManager.java:615-626`). 홀드된 일반 LN이 끝까지 유지되면 헤드 판정을 취한다(레퍼런스 구현 `JudgeManager.java:572-577`, `lnstartJudge`) — 레퍼런스 구현는 끝이 지나는 순간 확정하며, 여기서의 `margin`은 rbms가 아직 구현하지 않은 `releasetime + releasemargin` 지연 커밋(`:558-565`)을 대신하므로 모든 BEAT 모드에서 0, PMS에서만 200ms 늦다.
- 스윕된 노트는 항상 LATE(시간이 지났으니 `dm < 0`)이지만, 오버-홀드된 일반 LN은 헤드 판정을 커밋하며 레퍼런스 구현는 헤드 델타를 `mfast`로 넘긴다(`JudgeManager.java:573`이 `lnstartDuration`을 `updateMicro`에 전달) — 그래서 이른-히트 헤드는 EARLY로 남는다.
- `JudgeManager.updateMicro:660-669`: 모드의 콤보 테이블이 판정이 콤보를 늘리는지(인덱스 5 미만만) 리셋하는지 결정한다.

### crates/rbms-judge/src/windows.rs
- 옛 상수는 FIVEKEYS 값(GR ±150ms, BD ±250ms)을 쓰고 잘못된 MS 페어를 갖고 있었다.
- BAD 밖에는 아무것도 남지 않으므로 호출자의 `unwrap_or(Judge::Poor)`가 judge code 4를 준다.
- 7K: 가장 넓은 late 경계는 스크래치 BAD(-290ms), 가장 넓은 early는 MS(+500ms) 중 하나.
- `JudgeProperty.java:234`는 하한 클램프가 없어 `org * 0 / 100 == 0`이 PG/GR/GD/BD 에 그대로 적용되고 고정 MS 페어만 살아남는다 — `#DEFEXRANK 1`(`1 * 75 / 100 == 0`)로 도달 가능.
- 7K GOOD은 ±150ms; 300%면 ±450ms 지만 BAD는 `(-280ms, +220ms)`. GREAT은 0%면 0으로 붕괴하지만 단조 클램프가 PGREAT 창까지 끌어올린다. GOOD 10%(±15ms)는 GREAT(±60ms)보다 좁아지므로 GREAT으로 끌어올려진다.
- 레퍼런스 구현 `BMSPlayerRule.java:62`는 0..4 범위 밖이면 `judgerank[2][1]`(=75)을 쓰며 테이블 끝으로 클램프하지 않는다. `BMSPlayerRule.java:63`: `judgerank * 75 / 100` — `#DEFEXRANK 100 == NORMAL`. `BMSPlayerRule.java:63`의 BMS_DEFEXRANK 분기 else 암은 `judgerank[2][1] = 75` — `#DEFEXRANK`가 있는 차트는 `#RANK` 값에 상관없이 그 테이블을 다시 읽지 않는다.

### crates/rbms-parser/src/lib.rs, crates/rbms-parser/src/tests.rs
- 측정 배율(measure rate) 검증: non-finite 이거나 0 이하인 값은 이후 모든 섹션 위치를 오염시키고(NaN 은 이벤트 정렬을 비결정적으로 만듦) 거부되어 마디는 기본값 1.0을 유지한다.

### crates/rbms-play/src/lib.rs
- 레퍼런스 구현 `JudgeWindowRule.create:262-273`: 사용자 JUDGE WIDTH는 PG/GR/GD 에만 적용되고 BAD 경계는 `#RANK`로 고정된다. `#RANK 2`(75%)에서 late BAD 경계는 `-280ms*0.75 = -210ms`이며, 250ms 늦은 프레스는 200% 폭에서도 닿지 않는다.
- `JudgeManager.java:169-174`: `keyJudgeWindowRate`와 `scratchJudgeWindowRate`는 별개다. `#RANK 3`에서 7K 키 PGREAT은 ±20ms, 스크래치 PGREAT은 ±30ms — 키 비율 50%면 키 레인이 ±10ms로 좁아지지만 스크래치 레인은 자기 ±30ms 를 유지한다.
- 空POOR: 300ms 이른 프레스는 MS 창(인덱스 5)에만 들어가 폭탄(bomb)이 되지 않는다.
- `#RANK 3`(100%) BAD late 경계는 -280ms — 레퍼런스 구현의 JUDGE WIDTH는 PG/GR/GD만 스케일하고 BAD로 클램프하므로 어떤 폭 설정으로도 300ms 늦은 프레스는 닿지 않는다.
- 일반 LN 헤드를 누르고 release 하지 않는 경우: 레퍼런스 구현의 7K longnoteMargin은 0이고 `JudgeManager.java:572-577`이 `lnstartJudge`를 커밋하므로 끝이 지나는 순간 LN은 헤드 판정으로 해소된다 — 재판정된(far-negative) 끝이 아니다.

### crates/rbms-render/src/cpu.rs
- 좌우 절반 흑백 테스트 이미지: 좌측 흰색·우측 검은색으로 명확히 다른 평균값을 갖는 두 블록을 만든다.

### crates/rbms-render/src/font.rs
- 폰트 캐시 LRU 테스트: 가장 최근에 그린 문자열은 축출(eviction)에서 살아남으므로 재드로우가 히트가 되어 엔트리 수가 변하지 않는다. 아주 오래전에 그려진 첫 문자열은 이미 축출되어 재드로우가 미스가 되고 엔트리 하나가 추가된다.
