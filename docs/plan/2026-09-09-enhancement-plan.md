# R-BMS 전면 고도화 계획 (2026-09-09)

> 근거: 9관점 리서치(Opus) + 관점별 회의적 검증(Opus) + 완전성 비평 + IIDX 30~34 웹 조사 + Fable 직접 코드 확인 6건. 원 보고서·검증 보고서·후속 조사·비평 24편과 Fable 검토 노트는 `docs/plan/research-2026-09-09/`에 보존(파일명 `*-verify.md`가 검증, `followup-*.md`가 후속 조사, `critic.md`가 비평, `00-fable-review-notes.md`가 검토 노트). 이 문서는 그 종합이며, 각 주장은 검증자가 확인(confirmed/partially)한 것만 담고 반박된 항목은 제외했다.
> 대상 커밋 `c6f0885`(dev). 효력 표기: S<1일 · M 1~3일 · L≈1주 · XL 수주.

---

## 0. 한 페이지 요약

| 관점 | 총평 | 가장 중요한 사실 |
|---|---|---|
| 판정·게이지 패리티 | **7K 게이지 6종 수치·modifier·guts는 beatoraja와 완전 일치.** 그러나 판정 경로에 발산 18건, 그중 1건 critical | **見逃し POOR가 게이지 MS 슬롯으로 들어간다**(NORMAL -2, 정답 -6 / HARD -5, 정답 -10). 5K·PMS·24K 윈도우 부재, 7K LN 종단 값이 실은 5K 값, LN 마진 200ms 전모드, JUDGE WIDTH가 BD까지 확대, 스크래치 윈도우·지뢰 데미지·JudgeAlgorithm 선택 없음 |
| 판정 설정 노출 | 오프셋·JUDGE WIDTH·AUTO CAL만 노출 | judgealgorithm·키/스크래치 분리 폭·LN 마진·lnmode 강제·GAS·bottom shiftable 미노출. `JudgeWindows`가 serde 없는 const라 외부화 자체가 불가 |
| 오디오·입력 | 리샘플(선형보간)은 beatoraja(최근접)보다 낫다. 클럭 설계가 문제 | **게임 클럭이 오디오 콜백 주기(실측 512프레임=10.667ms)로 계단화**되고, BGM/키음 스케줄이 항상 `delay=0`이라 온셋도 버퍼 경계로 반올림. **키 입력 키음에 판정 오프셋이 섞여 offset>0이면 키음이 늦게 남**(AUTO CAL 켜면 상시). 볼륨 체계 없음(gain 1.0 하드클리핑), 컨트롤러/MIDI 없음, 스크래치 정/역 2키 없음 |
| 스킨 커스터마이징 | 데이터화된 건 플레이필드 40필드 + chrome 21색뿐 | `Renderer` trait이 `size/clear/fill_rect` 3개. 텍스처 슬롯 1개(256×256, BGA·커버 공유). 곡선택/결과/설정/로딩/디버그 좌표·색은 전부 코드 상수. beatoraja급(오브젝트 트리·키프레임·타이머·968 프로퍼티)은 **4개 층 신설 = XL** |
| 크레이트·Rust 관용성 | **매우 깨끗함**: unsafe 0, allow 0, Mutex 0, 빌드 경고 0, clippy 51(전부 style), DAG 정상 | 진짜 긴 파일은 `main.rs`(1277)·`app_select.rs`(967)·`app_play.rs`(796) 셋(나머지는 인라인 테스트). 구조 결함: `App` 93필드 god object, `PlayerConfig`↔`PlaySettings` 이중화, 설정 UI 정수 인덱스 결합. CI lint 잡이 `continue-on-error: true` |
| 메모리·성능 | 누수 확정 1건, 핫패스 2건 | 폰트 레이아웃 캐시 무상한(매 프레임 `format!` 문자열이 영구 적재). 곡선택 커서 이동마다 전곡 재구축 + 행×기록 O(N×M) 선형 스캔. 로딩 Esc 취소 시 키음 디코드 워커 cancel 플래그를 아무도 안 세움 |
| UX 흐름 | 백그라운드 패턴은 이미 있음(스캔·키음·미리듣기). 누락이 문제 | 표 추가 fetch(최대 15초)·포커스 상세 파싱·BGA 전량 디코드·rfd가 메인 스레드. **실패 알림 수단 0**(전부 eprintln). Root/Play에서 Esc가 무확인 종료·폐기. 플레이 중 바꾼 hispeed 미저장 |
| beatoraja 비판정 기능 | select/result 커버리지 절반 이하 | 곡DB 없음(매 실행 전량 파싱), 코스/단위·연습·필터·즐겨찾기·IR 랭킹 패널·라이벌·볼륨 3분리·비디오 BGA·시스템 사운드 없음 |
| IIDX(~34) 기능 | 옵션 축은 절반, 표시·타깃 축은 부족 | FLIP/BATTLE/SYNC-RAN·LEGACY NOTE·HID+·target/PACEMAKER·MAX-·판정 분포·게이지 추이 없음. 31에서 H-RANDOM·EXPAND-JUDGE 폐지, 33 CN/HCN 별도 스킨, 34 BPM 변화 대응(LANE COVER/SPEED ADJUST) |
| 시작 전 옵션 UX | 전체화면 설정 1종 | 권고: **곡선택 오버레이 옵션 패널(beatoraja START 홀드형)을 주경로**로, 전체화면 설정은 환경설정으로 축소. decide 화면은 도입하지 않음 |

**결론.** 코드 품질 자체는 손댈 게 적다. 고도화의 순서는 (A) 판정·오디오 **정확성 핫픽스** → (B) 오디오 클럭 재설계 → (C) 구조 개편(상태기계·설정 통합·판정 데이터화) → (D) 판정 패리티 완성+설정 노출 → (E) 스킨 완전 커스터마이징 → (F) UX/기능 → (G) 데이터 스케일/롱테일. C가 D·E·F의 전제이므로 A·B와 병렬로 먼저 끝낸다.

---

## 1. 관점별 현황과 확정 갭

### 1.1 판정·게이지 (`crates/rbms-judge`, `crates/rbms-play`)

**일치(발산 아님):** 7K NOTE 윈도우, 7K 게이지 6종 전 수치·TOTAL/LIMIT_INCREMENT 공식·guts·사망 후 동결·클리어 판정·EX스코어, 空POOR 노트 미소실·콤보 유지, 후보 게이트, 見逃し 스윕 경계, autoplay PG, CN/HCN 머리 판정.

**발산(검증 확정):**

| # | 항목 | rbms | beatoraja | 근거 | 효력 |
|---|---|---|---|---|---|
| J1 | **見逃し POOR 게이지 슬롯** | 스윕 미스를 `Judge::Miss`(idx5=MS)로 push → deltas[5] | 코드 4(PR) → value[4] | `matcher.rs:306-312` / `JudgeManager.java:595`, `GaugeProperty.java:89,133` | S |
| J2 | 空POOR 카운트 | `empty_poor` 별도 카운터, `counts[]` 미집계 → IR `ems=0` | `addJudgeCount(5)` → ems/lms | `matcher.rs:231-236`, `app_play.rs:264-266` / `ScoreData.java:262` | S |
| J3 | combocond 테이블 | Poor·Miss 모두 콤보 리셋(空POOR는 apply를 안 타서 마스킹) | 7K `{T,T,T,F,F,T}`(空POOR 콤보 유지), 5K/PMS `{T,T,T,F,F,F}` | `matcher.rs:355-362` / `JudgeProperty.java:29,18,40` | S |
| J4 | 5K NOTE 윈도우 | 7K 표 사용 | ±20/50/100/150, MS(-150,500) | `windows.rs:60-64` / `JudgeProperty.java:12` | S |
| J5 | PMS NOTE 윈도우 | 7K 복제(주석에 pending 명시) | ±20/50/117/183, MS(-175,500) | `windows.rs:36-44` / `JudgeProperty.java:34` | S |
| J6 | 24K(KEYBOARD) | 모드 자체 없음 | ±30/90/200, (-320,240), MS(-200,650) | `JudgeProperty.java:45` | M(모드 추가 포함) |
| J7 | **7K LN 종단 윈도우** | (120,150,200,±250)+MS — 실은 FIVEKEYS 값 | (120,160,200,(-280,220)), MS 없음(밖=코드4) | `windows.rs:29-35` / `JudgeProperty.java:25,14` | S |
| J8 | PMS LN 종단 | 7K(=5K) 복제 | 120/150/217/283 | `JudgeProperty.java:36` | S |
| J9 | 스크래치 NOTE/LN 윈도우 | 없음(키와 동일) | 7K scr ±30/70/160,(-290,230),MS(-160,500); longscratch 130/170/210,(-290,230). 5K 별도 | `JudgeProperty.java:24,27,13` | M |
| J10 | LN 릴리스 마진 | `LN_MARGIN=200_000` 전모드 | 7K/5K/KB 0, PMS 200000, `longnoteMarginRate` 설정 | `matcher.rs:7` / `JudgeProperty.java:26,15,37,48` | S |
| J11 | plain LN 미릴리스 확정 | 마진 후 종단 윈도우 재판정 | `lnstartJudge`(머리 판정)로 확정 | `matcher.rs:295-299` / `JudgeManager.java:572-577` | S |
| J12 | JUDGE WIDTH | judgerank×rate를 PG~BD 일괄, 클램프 없음, 키/스크 공통 | PG/GR/GD 3개만 각각, MS 상한·단조 클램프, 키 3+스크 3 분리 | `play/lib.rs:128-134`, `windows.rs:74-81` / `JudgeProperty.java:262-273`, `JudgeManager.java:168-173` | M |
| J13 | `#RANK` 범위 밖 | clamp(0,4) → 25/125 | NORMAL(75) 폴백 | `windows.rs:106` / `BMSPlayerRule.java:62` | S |
| J14 | `#DEFEXRANK` | 미파싱 | judgerank×75/100 | `parser/lib.rs:245` / `BMSPlayerRule.java:63` | S |
| J15 | 기본 TOTAL | 헤더 없으면 200 | `max(260, 7.605n/(0.01n+6.5))`, KB `max(300, 7.605(n+100)/(0.01n+6.5))` | `gauge.rs:77` / `BMSPlayerRule.java:84-89` | S |
| J16 | 지뢰 데미지 | 판정 대상 제외, 데미지 없음 | 눌린 채 통과 시 `gauge.addValue(-damage)` | `matcher.rs:145`, `play/lib.rs:66` / `JudgeManager.java:244-247` | S |
| J17 | JudgeAlgorithm | Duration(|dm| 최소) 고정 | Combo(기본)/Duration/Lowest/Score 선택 | `matcher.rs:210-220` / `JudgeAlgorithm.java:17-42`, `PlayConfig.java:103` | M |
| J18 | 판정 완료 노트 재타격 | 후보 제외(무판정) | MS 범위면 空POOR | `matcher.rs:212` / `JudgeManager.java:390-391` | S |
| J19 | fast/slow dm==0 | LATE(집계 누락) | EARLY | `matcher.rs:366-382` / `JudgeManager.java:648` | S |
| J20 | 게이지 병렬 갱신 | 단일 `Gauge` | 9종 전부 갱신, 선택만 표시 → GAS·결과 "다른 게이지였다면" 전제 | `matcher.rs:60` / `GrooveGauge.java:36-61` | M |
| J21 | 게이지 세트 | 7K 6종 | 7K 9종(CLASS/EXCLASS/EXHARDCLASS 추가) + 5K·PMS·KB·LR2 각 9종(PMS는 min2/max120/init30/border85 등 스펙 상이) | `gauge.rs:4-11` / `GaugeProperty.java:12-59,97-125` | M |
| J22 | MODIFY_DAMAGE | 없음 | EXHARD_5/HARD_LR2/EXHARD_LR2 | `GrooveGauge.java:274-288` | S(J21 후) |
| J23 | PMS 판정 규칙 | 없음 | `MissCondition.ONE`, judgeVanish `{T,T,T,F,T,F}`, `JudgeWindowRule.PMS`(fixjudge per-index + fixmin/fixmax) | `JudgeProperty.java:40-42,211,224-260` | M |
| J24 | CN deferral / HCN 연속 게이지 / BSS·MSS | 없음(문서상 Phase 7) | `JudgeManager.java:498-506, 313-330, 352-372` | L |
| J25 | lnmode 강제(LN→CN/HCN) | 차트 `#LNMODE`만 | `PlayerConfig.lnmode` 0~3 | `PlayerConfig.java:109` | S |
| J26 | GAS / bottom shiftable gauge | 없음 | `gaugeAutoShift` 5모드 + `bottomShiftableGauge` | `PlayerConfig.java:157-167` | M(J20 후) |

**파급:** J1+J2로 IR 제출의 `pr/ms` 4필드(epr/lpr/ems/lms)가 전부 어긋난다(`app_play.rs:251-266`). J1을 슬롯 교체만으로 고치면 J3 미구현이 **空POOR가 콤보를 끊는 회귀**로 즉시 표면화하므로 J1·J2·J3은 한 묶음이다.

**문서 stale:** `docs/acknowledge/beatoraja-divergences.md`에 판정 윈도우·게이지 섹션 없음(누락형), `docs/PROCESS.md` §6 "게이지 대조 검증됨"은 J1 때문에 부정확, `docs/reference/cn-hcn-judgment.md:8` 원본 경로 stale, `windows.rs:28` 독스트링이 5K 값을 "7K"로 표기.

### 1.2 오디오·입력 (`crates/rbms-audio`, `apps/rbms-player`)

| # | 항목 | 확정 사실 | 근거 | 효력 |
|---|---|---|---|---|
| A1 | **클럭 계단화** | `clock`은 콜백 말미에만 갱신 → 콜백 사이 정지값. **실측(이 기기, CoreAudio, `BufferSize::Default`): 512프레임/콜백 = 10.667ms 고정.** 판정 타임스탬프 양자화 오차 평균 5.33ms·최대 10.67ms·σ 3.08ms(PG 창 ±20ms의 26.7%). 보고 클럭은 가청 위치보다 16.6~27.3ms(평균 22ms) 앞섬 — 상수분은 offset/AUTO CAL이 흡수하지만 지터는 잔존. `Fixed(128/256)` 요청 시 실제 적용됨(2.67/5.33ms). 근본 해법은 버퍼 축소가 아니라 마지막 콜백 `Instant` 기준 연속 보간. `PROCESS.md` §1의 "vsync 양자화 구조적 해결"은 정확하지 않다(vsync 대신 오디오 버퍼로 양자화) | `engine.rs:122-133`, `app_play.rs:191-195`, `main.rs:1130`; 실측 `followup-cpal-measure.md` | M |
| A2 | 스케줄 `delay` 미사용 | BGM/autoplay/프리뷰는 `at <= now`만 방출 → `delay=0` → 온셋이 다음 콜백 경계로 반올림 | `play/lib.rs:174-178`, `mixer.rs:93` | (A1과 함께) |
| A3 | **키음 온셋에 판정 오프셋 혼입** | press 경로는 `at_us = judge_t = song + offset` → offset>0이면 키음이 그만큼 지연(여기서만 delay가 살아 있음). AUTO CAL 켜면 2회차부터 상시 | `main.rs:1131-1138`, `play/lib.rs:229`, `app_input.rs:237` | S |
| A4 | 시청각 스큐 | 클럭=믹싱 완료 프레임이라 가청보다 콜백 버퍼 1개분 앞섬(디바이스 레이턴시분은 beatoraja도 동일) | `mixer.rs:183` | (A1) |
| A5 | 볼륨 체계 없음 | `set_master_gain` 호출 0, Play gain 1.0 리터럴(프리뷰 0.85), `#VOLWAV` 미반영, 최종단 하드 클리핑 | `engine.rs:103`, `mixer.rs:181`, `app_input.rs:237` | S(기본값)/M(3분리+UI) |
| A6 | 보이스 스틸·페이드 | 커서 위치 활성 보이스 무조건 강탈, start/stop 페이드 없음(클릭 노이즈) | `mixer.rs:86-120` | S |
| A7 | 스트림 사망 무복구 | 에러 콜백은 eprintln만. 엔진 객체가 살아 있어 벽시계 폴백(`audio=None`일 때만)이 안 걸림 → 게임 정지. beatoraja도 런타임 복구는 없고 오픈 시 폴백만 | `engine.rs:135`, `app_play.rs:191-195` | S |
| A8 | 프리뷰 이중 엔진 | 포커스 정착마다 두 번째 cpal 스트림 오픈(512 보이스·독립 클럭), Play 진입 시 파기 | `app_select.rs:422,457`, `app_play.rs:10-13` | M |
| A9 | 오디오 설정 미노출 | 디바이스·버퍼·샘플레이트·동시발음(512 고정) | `engine.rs:25-33` / `AudioConfig.java:15-32` | M |
| A10 | 스크래치 2키 | 레인당 1키. beatoraja 7K는 9키/8레인(정/역), 컨트롤러 아날로그 알고리즘 2종 | `keyconfig.rs:193-212` / `PlayModeConfig.java:300`, `BMControllerInputProcessor.java:98-114` | M |
| A11 | 컨트롤러/MIDI/마우스 스크래치 | 의존성 0 | `apps/rbms-player/Cargo.toml` | L |
| A12 | 시스템 사운드·판정음 | 없음 | `SystemSoundManager.java`, `AbstractAudioDriver.java:493-502` | M |
| A13 | 채널 키 | `key = wav id`라 피치 다른 동일 wav도 서로 끊음(beatoraja `id*256+pitch+128`) | `engine.rs:95` | S |
| A14 | 다운샘플 AA 부재 | 48k 소스→44.1k 디바이스만 해당, 저빈도 | `mixer.rs:85` | M(후순위) |

### 1.3 스킨·렌더 (`crates/rbms-render`, `apps/rbms-player/src/gpu.rs`)

**현 상태:** `SkinConfig` 40필드(기하 7·색 15·DP 3·HUD 15) + `ThemeConfig` 21색. 임베드 스킨은 NORMAL/WIDE 2종(`default.ron`은 `--skin`으로만), 세 RON의 차이는 스칼라 6~7개, HUD 15필드와 `dual_field/dual_gap`은 어느 RON에도 없음. 곡선택·결과·설정·키설정·표·폴더·로딩·디버그 오버레이의 좌표·크기·폰트 스케일·문자열은 전부 코드 상수(`select.rs:143-150,217-256,495-537`, `result.rs:25-43,96-152`, `app_play.rs:465-779`). 결과 화면은 스킨의 `judge_colors/labels`를 무시하고 자체 상수를 씀(팔레트 이원화).

**저해 지점(전부 검증 확정):**

| # | 지점 | 상세 | 효력 |
|---|---|---|---|
| K1 | `Renderer` 프리미티브 3개 | `size/clear/fill_rect`뿐. 텍스처 쿼드·UV·회전·블렌드·클립·z 없음. wgpu 백엔드의 BGA 파이프라인(`gpu.rs:33-51,178-190,312-316`)은 단일 슬롯·256² 고정·항상 최하단·틴트/클립/회전 없음이라 "일반화" = 그 90여 줄 재작성. **실제 diff 산정: +400~500 / -70~85 LOC, 파일 4개(lib/cpu/gpu/app_play)**, 신규 메서드에 default impl을 주면 기존 호출부·테스트 106개 무수정. **PNG 골든 비교 하네스가 없어** 회귀 안전망부터 필요 | M(아틀라스·회전 제외) |
| K2 | 텍스처 자원 관리 부재 | 텍스처 1장(256×256) 고정, BGA와 커버가 같은 슬롯. 핸들·아틀라스·수명 없음 | L |
| K3 | 타이머/이벤트 부재 | beatoraja `TIMER_*`(151종)·키프레임 `dst`(time/x/y/w/h/clip/a/r/g/b/angle, acc 4종, loop)에 대응물 없음. 애니메이션은 함수 안 상수(`BEAM_RELEASE_US`, 봄 곡선, 로딩 점) | M |
| K4 | 게임 상태→프로퍼티 바인딩 부재 | 968 ID(OPTION 287/NUMBER 273/TIMER 151/…) 중 play 100~150, select/result 포함 250~350 필요 | L |
| K5 | 화면 렌더 함수가 고정 레이아웃 전제 | `render_select/result/hud/playfield` 4개가 "레이아웃+데이터" 결합. 오브젝트 리스트 순회 구조 아님 | L |
| K6 | 논리 해상도 1280×720 컴파일 고정 + 스트레치 | `Renderer::size()`가 상수 반환, 리사이즈는 surface만 재구성(비16:9 왜곡) | M |
| K7 | 폰트가 1px `fill_rect` 런 방출 | 글리프 아틀라스 없음. 매 프레임 인스턴스 버퍼로 밀어 넣음(K1의 정량 근거) | M(K1 후) |
| K8 | 테마/폰트 thread_local 전역 | 스킨별 팔레트·핫리로드·워커 렌더에 장애 | S |
| K9 | 결과 팔레트 이원화 | `result.rs:25-26` 자체 상수 | S |

**beatoraja 스킨 모델(재구현 스펙 요지):** 모델은 하나(`JsonSkin.Skin`), Lua 로더는 JSON 로더의 서브클래스. default 스킨의 7K play/decide/result/keyconfig/skinselect는 `.luaskin`이고 JSON 안에도 `"draw":"gauge() >= 75"` 같은 Lua 식이 있어 **JSON 파서만으로는 default 스킨도 못 읽는다.** 또 default JSON은 trailing comma를 허용하는 관대 JSON이라 `serde_json`이 실패한다(json5 계열 필요). Lua는 스킨 디렉터리 루트 샌드박스. 규모 추정 JSON+Lua식 5,600~8,100 LOC, LR2 CSV 추가 시 +1,500~2,500(원본 lr2 패키지 3,195 LOC, 106 명령). 최소 객체: play = Image/Value/Text/NoteSet/Gauge/Judge/Hidden·LiftCover/BGA/Graph, select = +ImageSet/SongList/BPMGraph/JudgeGraph/Slider, result = +GaugeGraph. 최상위 타이밍 필드(`input/scene/fadeout/playstart/close`), 커스텀 `property/filepath/offset`(사용자 설정 저장 연동)까지 포함해야 한다.

### 1.4 크레이트·Rust 관용성

**실측:** 9 crate + 2 app, 20,879 LOC, `cargo build` 경고 0, clippy 51(collapsible_if 24·unnecessary_get_then_check 13·needless_range_loop 13 등 전부 style/complexity), `#[allow]` 0, unsafe 0, Mutex/RwLock 0(오디오는 rtrb SPSC+AtomicU64), 테스트 889. 의존 DAG 정상, `docs/crates.md`와 일치.

| # | 구조 결함 | 근거 | 효력 |
|---|---|---|---|
| C1 | `App` 93필드 god object, `Stage`는 데이터 없는 enum | `main.rs:479-492,609-733` | L |
| C2 | `PlayerConfig`(30)↔`PlaySettings`(24) 이중화 + 수동 양방향 매핑(필드 추가 시 5곳) | `main.rs:71-167`, `app_input.rs:107-135` | M |
| C3 | 설정 UI가 정수 인덱스(`SETTING_KEYCONFIG=11` 등 + 6탭 배열)로 결합 | `main.rs:592-607`, `app_select.rs:888-967` | M |
| C4 | `JudgeWindows`가 serde 없는 const 4개, 판정 알고리즘 추상화 없음 | `windows.rs:19-52` | M |
| C5 | 앱에 있으나 크레이트로 내려야 할 것: `ir_map.rs`·`clear_type_id`(→rbms-ir/judge), `ScoreBook`·`Replay`(→rbms-store), 설정 스키마(→rbms-config), 폴더 스캔(→rbms-library), `compute_table_levels`(→rbms-table). `apps/rbms-player`에 lib 타깃이 없어 통합 테스트가 앱 모듈에 접근 불가 | `apps/rbms-player/Cargo.toml` | M |
| C6 | 긴 파일 3개: `main.rs` 1277 / `app_select.rs` 967(`build_select_view` 168줄) / `app_play.rs` 796(`frame()` 410줄, `load()` 6책임, `enter_result()` 174줄) | — | M |
| C7 | 런타임 패닉 후보: `chart/lib.rs:156` `partial_cmp().unwrap()`(`#xxx02:nan`으로 재현 가능, 본질은 파서 단계 거부), 기동 실패 unwrap(사용자 안내 없음). `matcher.rs` LN unwrap은 가드로 도달 불가(위생 항목) | — | S |
| C8 | `Result<_, String>` 7곳, thiserror/anyhow 없음, GUI 오류 표시 경로 없음 | table 4·render 1·tablesrc 1·replay 1 | M |
| C9 | CI: lint 잡 `continue-on-error: true` + `--all-targets` 누락(테스트 타깃 경고 미노출). `[workspace.lints]`·`#![forbid(unsafe_code)]` 없음. serde/serde_json/ron/reqwest 워크스페이스 미등록(버전 표기 혼재), reqwest 클라이언트 ir/table 중복 | `.github/workflows/ci.yml:61-67` | S |
| C10 | `theme`/`font` thread_local 전역, `Player::judge` pub 필드, `pub use dto::*` 글롭 | — | S |
| C11 | `rbms-render → rbms-chart` 의존은 `playfield.rs:1`이 scroll을 실사용하므로 단순 제거 불가(저우선) | — | M |

### 1.5 메모리·성능

| # | 항목 | 판정 | 근거 | 효력 |
|---|---|---|---|---|
| P1 | 폰트 레이아웃 캐시(`cache`) 무한 성장 + `runs` 캐시 (글리프,RGB) 축 성장. HUD가 매 프레임 새 문자열(`EX {}`, combo 등)을 만들어 영구 적재 | 확정 | `font.rs:50,54,89`, `hud.rs:81-128` | M |
| P2 | 곡선택 커서 이동 = 전곡 재구축. `select_key`에 `sel` 포함 → 전 행 `title/level clone` + 행마다 `best_clear_for_md5` O(records) 선형 스캔 → O(N×M) | 확정 | `app_select.rs:78-128`, `scores.rs:85-86` | M |
| P3 | 포커스 상세 파싱+커버 디코드가 디바운스 없이 렌더 스레드(행 이동당 1회) | 확정 | `app_play.rs:408`, `app_select.rs:266-278` | S |
| P4 | `#PREVIEW` 파일 경로가 렌더 스레드(read+스트림 오픈+디코드). autoplay 경로만 백그라운드 | 확정 | `app_select.rs:412-433` | S |
| P5 | 프리뷰 코디네이터 detach·join 없음, cancel 확인 2곳뿐, 호버만으로 곡 전체 wavmap 디코드(수백 MB 가능) | 확정 | `app_select.rs:474-498` | M |
| P6 | **로딩 Esc 취소 시 플레이 키음 디코드 워커 cancel 플래그가 항상 false** → 8스레드 완주 | 확정 | `app_play.rs:78`, `main.rs:341-343,1095` | S |
| P7 | 오디오 콜백 내 `scratch.resize`(버퍼 크기 변동 시), `producer.push` 실패 3곳 무음 유실·카운터 없음 | 확정 | `engine.rs:95-104,126-128` | S |
| P8 | `Matcher::release`가 레인 처음부터 `position(holding)` O(n) | 확정 | `matcher.rs:262` | S |
| P9 | Settings 화면 매 프레임 `setting_line` Vec 재구축 | 확정 | `app_play.rs:451-453` | S |
| P10 | 실측 부재: 커밋된 오디오 픽스처가 8kHz 짧은 클립뿐, GUI 프레임타임/RSS 추세 미측정 | 한계 | — | 하네스 필요 |

### 1.6 UX 흐름

| # | 항목 | 근거 | 효력 |
|---|---|---|---|
| U1 | **실패 알림 수단 없음**(차트 없음·표 로드 실패·폰트·미리듣기·스코어 제출·파일 저장·리플레이 로드 전부 eprintln, 일부는 debug 게이트). 서버 인디케이터·온보딩 힌트만 존재 | `app_play.rs:17,58,314`, `tablesrc.rs:84`, `app_select.rs:580` | M |
| U2 | 메인 스레드 블로킹: 표 추가 fetch(reqwest blocking 15s)·포커스 상세·BGA 전량 디코드(진행바 100% 후 정지처럼 보임)·rfd 3종·결과 저장 3연속 | `app_select.rs:740-748`, `app_play.rs:105-118,334-367`, `app_input.rs:147` | M |
| U3 | **Root에서 Esc = 곡 유무 무관 무확인 종료**, **Play 중 Esc = 무확인 폐기**(단 Play-Esc 즉시 이탈은 PROCESS §7의 의도 사양 → 변경은 결정 필요) | `app_select.rs:824-827`, `main.rs:1108-1117` | S |
| U4 | 플레이 중 hispeed/cover/lift 변경 미저장 | `app_input.rs:89-105` | S |
| U5 | 미리듣기 디바운스가 프레임 수(20) 기준, 페이드 없음, 볼륨 설정 없음 | `main.rs:59-60` | S |
| U6 | 텍스트 입력: 커서/붙여넣기/스크롤 없음. 표 URL 중복 미검사. 표 로드 실패가 빈 레벨로 흡수 | `app_input.rs:180-203`, `app_select.rs:740-745`, `tablesrc.rs:84-87` | S |
| U7 | 검색 Esc가 뷰(AllSongs) 미복원, 결과 개수 미표시, `SortMode::Level` 비교자 내 parse | `app_select.rs:25-45`, `app_input.rs:68` | S |
| U8 | 결과 화면에 리트라이/다음곡 동선 없음, 로딩 단계(BGA/스캔) 표시 없음 | `main.rs:1085-1089` | S |
| U9 | Tab 과부하(화면 이동 vs 탭 이동), 뒤로가기 키 의미가 화면마다 상이, Loading 취소 시 선택 위치 유실 | `main.rs:999-1106` | S |
| U10 | 창 리사이즈 스트레치(K6) | — | (K6) |
| U11 | 디버그 오버레이는 **이미 있음**(FPS/RAM/QUADS/오디오 클럭/앵커). 남는 갭은 판정 ms-off 스캐터·활성 보이스 수·드롭 카운터 | `app_play.rs:747-778` | S |

**시작 전 옵션 UX 3방식 비교와 권고:**

| 방식 | 장점 | 단점 |
|---|---|---|
| A 전체화면 설정(현행) | 항목 많아도 다 보임, 환경설정과 같은 자리 | 곡 컨텍스트(램프·베스트·밀도) 상실, 곡마다 바꾸는 루프와 상충 |
| B 곡선택 오버레이 패널(beatoraja START 홀드, IIDX START 홀드) | 곡 정보 보면서 RANDOM/GAUGE/HI-SPEED 조정, 홀드-조작-릴리스 1동작, 플레이 직전 옵션 = 제출 옵션 | 홀드 키 개념 추가, 항목 수 제한 |
| C decide/플레이 중 전환 | 로딩 대기 활용, 노트 보며 hispeed 확정 | 랜덤/게이지는 로드 후 변경 불가(셔플이 `load()`에서 적용), 제출 옵션의 진실 출처가 갈림 |

**권고:** B를 주경로로 도입(곡마다 바꾸는 값 = RANDOM/GAUGE/HI-SPEED+SPEED FIX/LANE COVER/LIFT/SCRATCH SIDE·AUTO/AUTOPLAY/TARGET), A는 환경설정(키·폰트·서버·스킨·오디오·PREVIEW·SCORE GRAPH)으로 축소, 플레이 중 조작(hispeed/cover/lift)은 유지하되 저장을 붙임. decide 화면은 스킨 시스템(E) 완성 후 "스킨이 정의하는 장면"으로만 도입.

### 1.7 beatoraja 비판정 기능 / IIDX 기능 갭 (요약)

| 영역 | 갭 | 효력 |
|---|---|---|
| 데이터 | 곡DB·증분 스캔 없음(매 실행 전량 read+parse+MD5+SHA256, 단일 스레드, 추정 1만 차트 10~50초·미실측), `scores.ron` 전체 재직렬화·비원자 write·락 없음(`replay/folders/tables.ron` 동일), 리플레이 무제한 누적·GC 없음, ScoreLog·프로필/통계 없음 | L/S |
| 곡선택 | 정렬 5/12, 난이도·모드 필터·즐겨찾기·랜덤 선택·IR 랭킹 패널·라이벌·커스텀 폴더·코스 없음. IIDX 34 DJ LEVEL 표시, 32 BGA 썸네일 프리뷰 없음 | M |
| 플레이 옵션 | FLIP/BATTLE/SYNC·SYMM-RAN, LEGACY NOTE, 5KEYS, HID+, ExtraNote/Mine/Scroll 모디파이어, lnmode, assist 플래그 기록(항상 빈 배열), hispeed 0.25 step·0.5~10(beatoraja 0.01~20), 레인커버 5% 계단, floating hi-speed(MAIN/MAX/MIN/START BPM 기준 그린넘버 고정 — 현 `SPEED FIX`는 BPM 변화까지 무시하는 CONSTANT이고 라벨 "FLOATING"이 IIDX 의미와 반대), `2000.0` 매직상수 2곳(Phase A 통합). **정정(Phase A, 2026-09-09)**: "그린넘버 LIFT 미반영"은 발산이 아님 — beatoraja `LaneRenderer.java:321-330`도 duration에 `1 − lanecover`만 곱하고 lift는 기하(판정선)만 바꾼다. rbms 동일 | S~M |
| 표시 | target/PACEMAKER(MAX/RATE_x/RANK_NEXT/RIVAL/IR 순위) 없음(로컬 베스트 1개), 라이브·결과 그래프 3밴드(27분위 아님), MAX- 표기, 게이지 추이·판정 분포·타이밍 히스토그램, 화이트넘버, 키/스크 FAST-SLOW 분리(IIDX 30), CN/HCN 별도 스킨(33), 판정문자 위치(33), BPM 변화 예고·LANE COVER/SPEED ADJUST(34) | M |
| 모드 | 코스/단위(IR DTO만), 연습, 일시정지/리트라이(beatoraja에도 일시정지는 없음 → 신규 제안), ARENA/BATTLE(축 자체 없음), STEP UP/DJ TRAINING | L |
| 입력/오디오 | 컨트롤러·MIDI·아날로그 스크래치, 볼륨 3분리·오디오 설정, 시스템 사운드, BGA 비디오 | L |
| IR | `rbms-ir`(1,549 LOC)은 LR2IR/beatoraja IR 호환이 아니라 **rbms 자체 REST+JSON 슈퍼셋** 클라이언트. 전 메서드 blocking(reqwest), 재시도·오프라인 큐·캐시 없음, **타임아웃 5초가 빌더 실패 시 무제한으로 조용히 강등**(`http.rs:19`), Bearer 인증 DTO는 있으나 앱이 항상 토큰 None(`main.rs:173`, login/register 호출 0). 조회용 `ScoreRecord` 9필드(판정·옵션·lntype 없음, beatoraja `IRScoreData` 32필드). `chart_ranking/player_best/rivals` 호출부 0. HTTP 계층 단위 테스트 0. beatoraja 격차 4축: 워커 격리(IsolatedIRConnection)·`RankingDataCache`(sha256+lnmode 키, TTL 없음)·클라 랭킹 후처리(YOU/RIVAL·램프 분포·localrank)·`TargetProperty` IR 목표. 랭킹 패널+target 최소 변경 12항목(S5/M6/L1): 패널 M, target 포함 M+, 인증·큐 포함 L → `followup-ir-crate.md` | M~L |
| IIDX 31 변경 | H-RANDOM·EXPAND-JUDGE 폐지 — rbms의 H-RANDOM·JUDGE WIDTH는 beatoraja 계열 옵션이므로 **유지**(결정 4) | — |

---

## 2. 실행 계획 (Phase)

> 각 Phase는 Workflow 1개로 실행: 구현(Opus, 파일군별 병렬·worktree) → 적대적 리뷰(Opus) → 수정 → `cargo test --workspace` + clippy → 실기 확인(가청·렌더 PNG). 기계적 작업(clippy --fix, 문서 경로 정정, 테스트 파일 분리, 상수화)은 Sonnet. 종합 판단·머지·문서 정본은 Fable.

### Phase A — 정확성 핫픽스 (S 묶음, 병렬 4갈래, 총 M)

| 갈래 | 항목 | 파일군 | 검증 |
|---|---|---|---|
| A-judge | J1+J2+J3(한 묶음), J4·J5·J7·J8(윈도우 표 정정), J10·J11(마진/미릴리스), J12 1단계(PG/GR/GD만 + 클램프), J13, J15, J16, J18, J19, `windows.rs` 독스트링 | `rbms-judge` | beatoraja 수치 테이블을 테스트 상수로 박아 byte 대조, 합성 픽스처(空POOR·見逃し·LN 미릴리스·지뢰), 기존 889 통과 |
| A-parser | J14 `#DEFEXRANK`, C7 NaN 소절 거부(파서 단계) | `rbms-parser`, `rbms-chart` | 픽스처 + corpus 727 MD5 불변 |
| A-audio | A3(press 키음 = raw 시각, 판정 = raw+offset 분리), A5 기본 master_gain 하향, A7 스트림 사망→벽시계 폴백, P7 카운터, A13 채널 키에 피치 포함 | `rbms-audio`, `main.rs` | 단위 테스트 + 실기 가청 |
| A-app | 그린넘버 `2000.0` 상수 통합(LIFT 반영은 beatoraja 대조 결과 불필요로 확정), assist 플래그 기록, **autoplay/리플레이 결과의 IR 제출 차단**(현재 로컬 기록만 제외하고 제출 스레드는 무조건 실행: `app_play.rs:306-311` vs `:340`), U3 Root Esc 확인/차단, U4 플레이 중 조작 저장, P6 cancel 플래그 배선, 원자적 저장(temp+rename, scores/replay/folders/tables), U6 표 URL 중복 검사, P3 포커스 상세 디바운스 | `apps/rbms-player` | 테스트 + 헤드리스 렌더 |
| A-ir | `http.rs:19` 타임아웃 빌더 실패 시 무제한 강등 → 명시 에러, HTTP 계층 단위 테스트(mock) 추가 | `rbms-ir` | 단위 테스트 |
| A-render | P1 폰트 캐시 상한(LRU) + HUD 숫자 자릿수 렌더, K9 결과 팔레트 스킨화 | `rbms-render` | 누수 회귀 테스트(10,000 문자열 후 RSS 상한) |

**주의:** J1 수정은 게이지가 엄해져 기존 로컬 기록·리플레이 재현 결과가 달라진다 → `scores.ron`에 판정 규칙 버전 필드를 넣고 구기록을 구분 표시(결정 2).

### Phase B — 오디오 클럭 재설계 (M)

1. `song_us()`를 "마지막 콜백 시각 기준 프레임 + 벽시계 경과"로 보간(콜백에서 `Instant`+frames를 원자적으로 기록). cpal `OutputCallbackInfo.timestamp().playback`으로 가청 시각 보정.
2. 게임 시계를 믹서 클럭보다 룩어헤드(1버퍼 + 1프레임)만큼 앞서 방출해 `delay` 스케줄을 활성화(BGM/autoplay/프리뷰). **press 키음은 즉시 발음 예외.**
3. 단일 `AudioEngine`을 앱 수명 동안 유지, 프리뷰/플레이가 믹서 공유(키 네임스페이스 분리) → A8 해소, 스트림 반복 오픈 제거.
4. 볼륨 3분리(system/key/bg) + `#VOLWAV` + 소프트 리미터, 보이스 스틸 정책(가장 오래된/조용한) + start/stop 램프.
5. 오디오 설정 노출(디바이스·버퍼 크기·샘플레이트·동시발음), 오픈 시 폴백.
6. 검증: 판정 오차 실측 하네스(입력 이벤트→판정 시각 분포), 오디오 드롭/언더런 카운터 오버레이, 10~15분 소크(RSS·fps·언더런).

### Phase C — 구조 개편 (L, D·E·F의 전제)

1. `Stage`를 데이터 보유 enum으로(`Select(SelectState)/Play(PlaySession)/…`), `PlaySession`(player+audio+bga+replay+analysis+calibration)을 `rbms-play`로 이관. `frame()` 410줄·`window_event` 220줄이 stage별 `update/draw/handle_key`로 해체.
2. `PlayerConfig`/`PlaySettings` 통합 → `rbms-config` 크레이트(serde 단일 스키마, 마이그레이션 버전).
3. 설정 UI를 정수 인덱스 → descriptor 테이블(enum + 라벨/범위/스텝/탭 메타). D·F의 노출 통로.
4. `JudgeWindows`·게이지 테이블을 serde 데이터화(모드별 RON, 프로그램 기본값 = beatoraja 표) + `JudgeAlgorithm` trait.
5. 앱→크레이트 이관(`rbms-store`: ScoreBook/Replay, `rbms-library`: 스캔/상세, ir_map→rbms-ir, clear_type_id→rbms-judge, table levels→rbms-table) + `apps/rbms-player`에 `[lib]` 추가 + `rbms-cli`를 첫 소비자로.
6. 위생: CI lint 게이트(`continue-on-error` 제거, `--all-targets`, `-D warnings`), `[workspace.lints]`, `#![forbid(unsafe_code)]`, 워크스페이스 dep 통합, `Result<_,String>`→thiserror, 인라인 대형 테스트→`tests.rs`, theme/font 전역→인자 주입.

### Phase D — 판정 패리티 완성 + 설정 노출 (M~L)

**커스텀 판정·어시스트 기록 정책(결정 12) 먼저 확정**, J17 알고리즘 4종(기본 Combo), J20 9게이지 병렬 + J26 GAS/bottom shiftable, J21·J22·J23(5K/PMS/KB/LR2 세트·CLASS 계열·PMS 규칙·fixjudge per-index), J9 스크래치 윈도우 + A10 정/역 2키 + BSS/MSS, J24 CN deferral·HCN 연속 게이지, J25 lnmode 강제, J12 2단계 키/스크 분리, J6 24K 모드. JUDGE 탭에 노출: 알고리즘·판정별 폭(PG/GR/GD, 키/스크)·LN 마진 배율·lnmode·GAS·bottom shiftable·TARGET. 검증: beatoraja `JudgeProperty/GaugeProperty` 전 배열을 테스트 상수로 대조, 리플레이 재시뮬 회귀.

### Phase E — 스킨 완전 커스터마이징 (XL, 단계별 출하)

| 단계 | 내용 | 효력 |
|---|---|---|
| E1 프리미티브 | 순서: CpuCanvas **PNG 골든 하네스 신설(S)** → trait 확장 `draw_textured_quad(dst, tex, src_uv, tint, blend)`(S) → gpu 텍스처 레지스트리 + per-texture draw call 배치 병합(순서 보존), BGA 특수경로 삭제, `set_bga/clear_bga` 호출 9곳 치환(M) → `push/pop_clip`(시저는 논리 CW/CH↔물리 surface 스케일 변환 필요)(S) → CpuCanvas 샘플 참조 구현(S~M) → 글리프 아틀라스(K7)(L) → 회전(S). 함정: `write_texture` `COPY_BYTES_PER_ROW_ALIGNMENT`(현재 1024로 우연히 통과) | M(아틀라스·회전 제외) / L(아틀라스 포함) |
| E2 타이머·키프레임 | 타이머 레지스트리(`TIMER_*` 필수 집합 ~40) + `dst` 보간기(acc 4종·loop·offset·op) — beatoraja `SkinObject.prepareRegion` 시맨틱 그대로(loop==-1 우회 경로 포함) | M |
| E3 프로퍼티 바인딩 | 정수 ID → Boolean/Integer/Float/String/Timer 조회 레지스트리. play 100~150 → select/result 250~350. C1의 `PlaySession`이 상태 원천 | L |
| E4 스킨 모델·로더 | `JsonSkin.Skin` 미러(serde) + 관대 JSON(json5) + Lua 식 평가(결정 1) + `SkinLoader` 규칙(와일드카드 랜덤·filemap·customfile) + `property/filepath/offset` 설정 저장 | L |
| E5 화면 이식 | play → select → result → decide → keyconfig 순. 기존 `SkinConfig`(RON)는 "기본 스킨 파라미터"로 호환 유지(M8 제약) | L |
| E6 스킨 선택·커스터마이즈 UI | 스킨 목록·커스텀 옵션·파일·오프셋 편집 화면 | M |

### Phase F — UX·기능 고도화 (M 묶음)

옵션 오버레이 패널(1.6 권고 B) + 홀드 키 바인딩 · 에러 토스트/상태줄(U1) · 표 fetch/BGA 디코드/rfd 백그라운드화 + 로딩 단계 표시(U2·U8) · 결과 화면 리트라이/다음곡 · 타깃/PACEMAKER(서버 불필요: MAX/RATE_x/RANK_NEXT/LOCAL_BEST → IR: chart_ranking/player_best/rivals 비동기 캐시) · 그래프 27분위·MAX- · 결과 게이지 추이/판정 분포/타이밍 히스토그램 · 곡선택 정렬 12종·난이도/모드 필터·즐겨찾기·DJ LEVEL 표시·BGA 썸네일 · HID+/SUD+&HID+·화이트넘버·hispeed 0.01~20·레인커버 미세조정·floating hi-speed(MAIN/MAX/MIN/START BPM)·BPM 변화 예고·LANE COVER/SPEED ADJUST(34) · FLIP/BATTLE/SYNC-RAN·LEGACY NOTE·5KEYS · CN/HCN 별도 스킨·판정문자 위치·키/스크 FAST-SLOW 분리 · 미리듣기 시간 기준 디바운스·페이드·볼륨 · 텍스트 입력 커서/붙여넣기 · 검색 뷰 복원 · 리사이즈 레터박스(K6).

### Phase G — 데이터 스케일·롱테일 (L)

곡DB(결정 5) + 증분 스캔 + 병렬 파싱 · 스코어 DB(ScoreLog·프로필/통계) + 리플레이 보존 정책 · 코스/단위(CourseData·CourseResult·랜덤 코스) · 연습 모드 · 컨트롤러(gilrs)·MIDI(midir)·마우스 스크래치 + 디바운스 · 시스템 사운드/판정음 · 라이벌·IR 랭킹 패널·복수 IR · BGA 비디오(선택) · 스크린샷/Discord(선택).

### 의존 관계와 병렬성

```
A(judge/parser/audio/app/render 5갈래 병렬)  B(오디오)  C(구조)   ← 동시 착수 가능
                     C 완료 → D(판정 완성·노출) ∥ E1~E2(프리미티브·타이머) ∥ F(옵션 패널·토스트·백그라운드)
                                  E3~E6(바인딩·로더·화면 이식)  ∥  G(데이터·롱테일)
```
A와 B는 파일이 겹치지 않는다(B는 `engine.rs/mixer.rs/app_play.rs song_us`, A-audio는 `main.rs press`·`engine.rs gain` — 같은 워크플로 안에서 순서만 A-audio→B). C는 `main.rs/app_*.rs` 대규모 이동이라 A-app 완료 후 착수.

---

## 3. 결정 필요 (객관식)

1. **스킨 포맷 전략** — **A안(추천)**: beatoraja JSON 스킨 호환(json5 관대 파서) + Lua 식 평가(`mlua`, 스킨 루트 샌드박스, 노출 API 화이트리스트) → default 스킨·커뮤니티 JSON/Lua 스킨 재사용. 이유: 모델이 하나이고 default 스킨 자체가 Lua를 요구함 / B안: 자체 RON 오브젝트 스킨(beatoraja 모델 미러, Lua 없음, 변환 도구 제공) — 의존성 적고 안전하나 생태계 단절 / C안: A + LR2 CSV 로더(+1,500~2,500 LOC, 레거시 스킨 최다).
2. **판정 규칙 변경 시 기존 기록 처리** — **A안(추천)**: `scores.ron`에 `rule_version` 필드 추가, 구버전 기록은 유지하되 램프 비교에서 별도 표시 / B안: 기록 초기화(백업 후) / C안: 구기록도 리플레이 재시뮬로 재판정.
3. **시작 전 옵션 UX** — **B안(추천, 1.6)**: 곡선택 오버레이 패널 + 환경설정 축소 / A안: 현행 유지 + 인게임 조작 확장 / C안: decide 화면 신설.
4. **IIDX 31 폐지 옵션(H-RANDOM·EXPAND-JUDGE 계열)** — **유지(추천)**: beatoraja 패리티가 목표이므로 H-RANDOM·JUDGE WIDTH 유지 / 제거.
5. **곡DB 구현** — **A안(추천)**: `rusqlite`(bundled)로 beatoraja `songdata.db` 유사 스키마 → 향후 beatoraja DB 임포트 가능 / B안: 자체 바이너리 캐시(경로+mtime+파싱 결과, 의존성 0).
6. **Play 중 Esc 즉시 이탈**(PROCESS §7 의도 사양) — **A안(추천)**: 유지하되 "길게 누르기/2회 누르기" 옵션 추가 / B안: 확인 다이얼로그 / C안: 현행 유지.
7. **입력 장치 범위(G)** — **A안(추천)**: 게임패드(gilrs) + 스크래치 2키/아날로그 먼저, MIDI는 후순위 / B안: 셋 다 한 번에 / C안: 키보드만.
8. **git 운용** — 레포에 `llm-rules.auto-commit/auto-push` 미설정. **수동(추천, 현행)** / 자동 커밋 / 자동 커밋+푸시. Phase A 이후 커밋 단위는 갈래별(judge/parser/audio/app/render).
9. **대상 라이브러리 규모** — 곡DB(결정 5)·곡선택 O(N×M)의 우선순위가 곡 수에 좌우된다. 문서의 테스트 라이브러리 경로(`/Users/hyunseokbyun/Documents/personally/1`, 727차트)는 이 기기에 없고 현재 설정은 다운로드 폴더 1개를 가리킨다. 실제 운용 규모를 알려 주시면(수백 / 수천 / 1만 이상) G의 순서를 확정한다.
10. **bmson·`#SWITCH` 지원** — rbms 스캔 필터는 `bms|bme|bml|pms` 4종, 제어구문은 `#RANDOM/#IF/#ELSE` 계열만(`main.rs:393`, `parser/control.rs:65-107`). beatoraja는 bmson을 스캔·디코드한다. **A안(추천)**: `#SWITCH/#CASE/#SKIP/#DEF`는 Phase A-parser에 포함(S), bmson은 Phase G에 별도 항목(L: 파서+모델 매핑+judgerank/TOTAL 퍼센트 변환) / B안: 둘 다 G / C안: 범위 밖.
11. **별도 런처(실행 전 설정 GUI)** — beatoraja는 JavaFX 런처(오디오/입력/IR/스킨/폴더/코스 에디터)를 두지만 rbms는 단일 바이너리·`.dmg` 첫 실행 온보딩 방향이다. **인게임 환경설정으로 통합(추천)** / 별도 런처 신설.
12. **커스텀 판정·어시스트 기록 정책** — beatoraja 실동작(`BMSPlayer.java:207-214,862-882`, `PlayDataAccessor.java:427-451`, `MusicResult.java:81-97`): 판정폭 rate가 하나라도 >100(또는 LN 마진 rate >100)이면 `assist=2`·`score=false` → IR 제출·리플레이 저장·EX/BP/콤보 갱신 차단, **램프·플레이카운트는 갱신**; `assist>0`이면 FC/PERFECT/MAX 생략 + 램프를 LightAssistEasy(id 3)/AssistEasy(id 2)로 강등; AUTO SCRATCH는 assist=1; autoplay/replay/practice는 기록·제출 대상 아님; 판정을 좁히는 쪽(≤100)은 정상 기록. rbms 현황: `judge_rate` 50~200% 게이트 0, `scratch_auto` 램프 무영향, LightAssistEasy 미생성. **A안(추천)**: beatoraja 정책 그대로 이식(램프 id 3 신설 포함) / B안: 현행 all-or-nothing(로컬 기록 제외)에 IR 제출 차단만 추가.

---

## 3.1 완전성 비평 반영 (2026-09-09)

비평(`critic.md`)이 지목한 갭과 처리:

| id | 갭 | 처리 |
|---|---|---|
| G-01 | bmson·`#SWITCH` 미지원 — 어느 보고서도 포맷 커버리지를 다루지 않음 | 결정 10으로 승격. 근거는 비평이 직접 grep으로 확보 |
| G-02 | `rbms-ir` 미통독 → 타깃/랭킹 배선 근거 부족 | **완료**: `followup-ir-crate.md` — §1.7 IR 행·Phase A-ir·Phase F에 반영 |
| G-03 | 성능·메모리 실측 0건(전부 정적) | Phase B 6항(판정 오차·언더런·소크 하네스)과 Phase A-render 누수 회귀 테스트로 baseline 확보. 실측 전까지 P1/P2 순위는 잠정 |
| G-04 | `cargo test` baseline 미실행 | **실행 완료: 889 통과 · 0 실패**(22개 테스트 바이너리, `c6f0885`) |
| G-05 | 크로스플랫폼·배포 축 미조사 | 이번 요청 범위 밖. `docs/reference/windows-compat.md`·`docs/ci-release.md`가 정본, Phase 2(첫 릴리스)에서 다룸 |
| G-06 | 저장 스키마 버저닝 부재 | **확인: `settings/scores/replay/folders/tables.ron` 전부 버전 필드 없음, `#[serde(default)]`+`.bak`만.** Phase C-2(`rbms-config`)에 `schema_version` + 마이그레이션 함수 도입, 결정 2와 연동 |
| G-07 | 커스텀 판정 스코어의 기록·IR 유효성 정책 미확인 | **완료**: `followup-custom-judge-policy.md` — 결정 12로 승격, autoplay/리플레이 IR 무조건 제출 버그는 Phase A-app |
| G-08 | 별도 런처 선택지 누락 | 결정 11 추가 |
| G-09 | 라이브러리 규모 미확인 | 결정 9 추가(사용자 확인) |
| S-01 | IIDX 31~34 1차 출처 | remywiki 403 우회 실패. bemaniwiki·공식·iidx.org 교차 확인분만 채택, 34는 로케테 사양으로 표기(§5) |
| S-02 | 스킨 포맷 3안 비교표 부재 | 결정 1에 반영(A: JSON+Lua식 / B: 자체 RON / C: +LR2). 규모·생태계·안전성 축으로 비교 |
| S-05 | cpal 버퍼 추정치 | **완료(실측)**: `followup-cpal-measure.md` — §1.2 A1에 수치 반영 |
| X-01 | 텍스처 프리미티브 effort 상충(L vs 더 큼) | **완료**: `followup-renderer-diff.md` — E1 = M(아틀라스·회전 제외)/L(아틀라스 포함)/XL(스킨 스프라이트 전체). 골든 하네스 부재가 실제 리스크 |
| X-02 | "rbms는 CPU 캔버스 기반" 전제 오류 | 검증 쪽 채택(wgpu 주력). §1.3 K1에 반영됨 |
| X-04 | PROCESS.md stale 일괄 판정 | 섹션 단위로만 정정(§4 목록) |
| X-05·X-07 | 원보고서 severity 과대 | 이 문서는 검증 조정판만 채택(디버그 오버레이 부재·LN unwrap 패닉 반박, 표 fetch는 추가 경로만 동기 등) |

---

## 4. 문서 정정 목록 (Phase A와 함께, Sonnet)

- `docs/PROCESS.md`: 위치 `/Users/hyunseokbyun/rbms` → `/Users/gkn/R-BMS`; §1 "vsync 양자화 구조적 해결" → 오디오 콜백 양자화 사실 반영; §6 "게이지 대조 검증됨"에 J1 예외 명시; 2026-05-31 이력 문단 `#PREVIEW` TODO에 "(2026-06-07 해소)" 부기; §7 한계에 곡DB·컨트롤러·코스·연습·필터·볼륨 미구현 추가.
- `docs/acknowledge/beatoraja-divergences.md`: 판정 윈도우·게이지 발산 섹션(J1~J26) 신설.
- `docs/reference/cn-hcn-judgment.md:8` 원본 경로 정정. `crates/rbms-judge/src/windows.rs:28` 독스트링(5K 값을 7K로 표기) 정정.
- `docs/reference/mechanics.md`는 이번 조사에서 미통독 → Phase A 판정 수정 시 함께 대조.

---

## 5. 미확인·한계 (추측으로 채우지 않은 것)

- cpal 버퍼·양자화 오차는 **실측 완료**(§1.2 A1). 디스플레이 지연은 미실측(시청각 스큐 계산에 120Hz 1프레임 가정 시 약 +13.6ms). 스캔 시간·프레임타임·RSS는 코드 구조 기반 추정(Phase B 하네스로 측정).
- IIDX 34는 로케테스트 사양(정식 가동일·최종 사양 미확인). remywiki 본문은 403으로 스니펫만 인용.
- beatoraja `Mode`·`PCM` 클래스는 외부 bms-model 의존이라 소스 직독 불가(스크래치 2키 매핑은 키 배열 패턴으로 추론).
- bmson 경로(`#TOTAL` 퍼센트, judgerank, `#WAV` slice)는 rbms가 bmson 미지원이라 범위 밖.
- `clear_lamp` ↔ `ClearType.getClearTypeByGauge`, DJ 랭크 산출식은 미대조(Phase D에서 대조).
