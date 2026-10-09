# beatoraja 형식 Lua 스킨 완전 호환과 기본 스킨 교체 — 사양

> 최종 갱신 2026-10-09 · 상태: 확정(구현 전) · 기준 커밋 `9ce92bb` · 체크리스트 `docs/PROCESS.md` L1~L7
> 조사 근거: `docs/reference/skin-compat/`(색인 `README.md`). 이 문서의 `[b3 §5]` 같은 표기는 그 폴더의 보고서 절이다.
> 결정 기록: `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`

## 1. 목표와 완료 조건

1. **호환**: beatoraja 용 풀 Lua 스킨 ModernChic(`.luaskin` 10개)을 파일 수정 없이 읽어 beatoraja 와 같은 배치·타이밍으로 그린다. 스킨 폴더는 저장소 밖 사용자 경로를 가리킨다(저장소에 복사하지 않는다).
2. **구조 교체**: 내장 화면과 스킨 문서를 섞는 혼합 합성(`composition`/`layer`/`replace`/`hotspot`/사설 id)과 `steel-neon` 3세대를 삭제하고, 스킨이 있는 화면은 스킨이 단독으로 그린다.
3. **기본 스킨**: ModernChic 의 배치를 참조해 단순화한 기본 스킨을 **자작**한다(Lua·이미지·사운드 전부 새로 만든다. 원본 파일을 복사·개변하지 않는다). 첫 실행부터 전 화면이 이 스킨으로 그려진다.
4. **UI 흐름**: 선곡 → 결정 → 플레이(PRELOAD → READY → PLAY → FAILED/FINISHED → 페이드) → 결과로 바뀐다.

완료 조건
- 스킨 덤프 도구가 ModernChic 10개를 읽어 keyconfig 본체(원본 결함)를 뺀 나머지에서 pcall 실패 0 을 출력한다.
- 결정·결과·코스 결과·선곡·플레이(5/7/10/14키) 화면의 헤드리스 캡처가 `m2`~`m5` 의 레이아웃 좌표와 맞는다(메인이 캡처를 직접 열어 판정).
- 기본 스킨으로 선곡 → 결정 → 플레이 → 결과가 끊김 없이 돈다(5/7/9/10/14키).
- 게이트(§8) 통과, 조사 문서와 `docs/skin.md` 가 코드와 일치.

한계(사용자 결정 D6): beatoraja 실행 캡처가 없으므로 글자 위치의 픽셀 단위 일치와 색감은 좌표 대조와 코드 근거까지만 보장한다. GPU 창 확인은 사용자 절차다.

## 2. 사용자 결정 (2026-10-09)

| # | 결정 | 내용 |
| --- | --- | --- |
| D1 | 기본 스킨은 자작 | ModernChic 을 참조해 스킨 파일을 새로 만든다. 원본의 Lua·이미지·폰트·사운드를 저장소에 넣지 않는다. 원작자 표기 의무가 생기지 않도록 원본 코드를 복사·개변하지 않고, 배치(좌표·구성)만 참고한다 |
| D2 | 이름 금지 규칙 폐지 | 저장소에 beatoraja·ModernChic 이름을 쓸 수 있다. 조사 문서를 그대로 커밋한다. 게이트의 "금지 명칭 grep" 항목을 없앤다 |
| D3 | 구 스킨 삭제 | `steel-neon` 3세대, 혼합 합성 코드, 관련 테스트, 설정의 구 선택값을 삭제한다 |
| D4 | 단순화 범위 | 문구는 영어. 선곡 사이드메뉴는 설정·볼륨 2패널만. 뺄 것: BPM 연동 캐릭터, 음성, IR 메뉴, 이력 기록, 인프레션, 자산 변종(노트·판정·폭발·배경 다종), 로테이션, 버전 확인 |
| D5 | mp4 동영상 포함 | 스킨의 동영상 source 를 이번 범위에서 지원한다. 디코더 의존성 추가를 승인받았다(구체 크레이트는 웨이브 7 에서 공식 문서로 선정해 결정 문서에 기록) |
| D6 | 비교 기준 | beatoraja 캡처 없이 좌표 표 대조로 검증한다 |
| D7 | 커밋 | 웨이브가 게이트를 통과할 때마다 논리 단위로 커밋·푸시한다. 이 작업에 한해 `docs/PROCESS.md` §9 의 "커밋은 사용자 요청 시에만"을 대체한다 |

메인이 정한 기본값(사용자에게 고지, 이의 없음)

| # | 기본값 |
| --- | --- |
| E1 | Lua 격리: `require`·`dofile`·io 읽기는 스킨 루트 안만. io 쓰기는 `<설정 폴더>/skin-data/<스킨 식별자>/` 오버레이로 보낸다. `main_state.http_*` 와 luajava `URL` 은 항상 실패를 돌려준다 |
| E2 | R-BMS 고유 기능 유지: Settings·Tables·Folders·Practice·스캔 대기 화면은 내장. IR 랭킹 패널, 기록 모달, 필터 패널, 옵션 오버레이(F1), 첫 실행·빈 목록 안내, 토스트, 디버그 패널, 리플레이 분석 띠는 스킨 위 시스템 오버레이 |
| E3 | 조작: 기존 키보드 단축키 유지 + START/SELECT 바인딩 신설. 패널 1~3 은 START/SELECT 로 연다. 키보드 기본값은 START = A, SELECT = W(beatoraja 는 Q/W 지만 Q 는 IR 랭킹 패널이 쓰고 있다), 패드는 표준 Start/Select 버튼 |
| E4 | 폴백: 내장 play/select/result 화면은 이번 작업에서 지우지 않는다. 스킨 로드 실패와 24키가 이 폴백을 쓴다 |
| E5 | 글자: 스킨 폰트에 없는 글자는 앱의 다국어 폰트로 그린다(beatoraja 는 사각형). "똑같이"의 의도적 예외 |
| E6 | 창: 기본 창 1280x720 유지, 해상도·전체 화면 설정 추가. 16:9 가 아니면 레터박스. 구현 상태(웨이브 1B): BORDERLESS 는 항상 비율을 맞추고, 일반 창은 기존 `display.letterbox` 설정(기본 꺼짐)을 따른다. 일반 창의 기본값을 켤지는 웨이브 8 에서 정한다 |
| E7 | "똑같이"의 기준: 그리기·타이밍은 이 저장소 옆 beatoraja 체크아웃(HEAD `8320241d`) 그대로(특이 동작 18개 [b2 §18] 포함). 명백한 입력 결함은 원작 의도로 교정(luajava `Input.Keys` 는 UP 19 / DOWN 20 / LEFT 21 / RIGHT 22 를 돌려준다) |
| E8 | IR: 스킨의 IR 타이머 172~174 와 관련 옵션은 실제 `ir_status` 에서 공급한다. 랭킹 값 380~399 등 목록 데이터의 스킨 연결은 후속(웨이브 9) |
| E9 | 9키: 기본 스킨에 9키 문서를 포함한다(자작이므로 가능). ModernChic 을 고른 사용자의 9키는 내장 폴백 |
| E10 | 색 공간: **바이트 통과로 확정(웨이브 1B)**. sRGB 표면에서는 같은 그리기 목록의 GPU 출력이 CpuCanvas 보다 최대 75/255 밝았다(128 → 188, 64 → 137). 표면을 비 sRGB 포맷으로 고른 뒤 차이는 최대 2/255. 부작용: 실제 창의 내장 화면이 전보다 어두워지고 대비가 강해진다(헤드리스 골든과는 일치). 실창 확인은 사용자 절차이며, 테마 색 재조정이 필요하면 별도 작업으로 한다 |

## 3. 조사로 확정된 전제

- ModernChic 은 실행되는 Lua 프로그램이다. 화면 진입마다 새 Lua 상태에서 진입 파일을 헤더 패스(`skin_config == nil`)와 본체 패스로 두 번 실행하고, 본체가 만든 함수 값(draw 약 330, value 약 130, act 35, timer 17, `timer_observe_boolean` 148)을 매 프레임 부른다 [b1 §3, m1 §2].
- beatoraja 의 Lua 는 LuaJ 3.0.2(Lua 5.2 계열, 수가 double 하나). mlua 0.12.2 의 `lua52` 피처로 새 의존성 없이 맞춘다 [00 §2.2].
- 재사용: `SkinDef` 모델(최상위 46필드 일치), `dst.rs` 보간, 속성·타이머 id 표 968개, `Renderer` 프리미티브, `SkinViewport` [r1 §3·§6, r2 §1, b5 §1].
- 요구 기능 전수는 [00 §1] 매트릭스(C1~C32, P, D, S, E, T, G, K, N)와 [99 §3] 의 추가 9건(M1~M9)이다. 구현 단위는 해당 행 번호를 지시에 적는다.
- R-BMS 고유 기능의 처리 규칙은 [99 §5] H1~H15 를 따른다(§6 에 반영).

## 4. 목표 구조

### 4.1 크레이트 책임

| 크레이트 | 맡는 것 | 버리는 것 |
| --- | --- | --- |
| `rbms-skin` | 스킨 모델(`SkinDef`), Lua 5.2 런타임, 2패스 로더, Lua 값 변환, 경로·설정 병합, destination 보간, 타이머 저장소(µs), 호스트 계약(`SkinHost`) | 혼합 합성 확장 필드, `skin.*` 6함수, 한 줄 식 전용 예산 |
| `rbms-render` | `Renderer` 프리미티브, TTF·비트맵 폰트, 스킨 객체 빌드와 prepare/draw 2단계, 클릭·드래그 판정, 텍스처 관리, 동영상 프레임 업로드 통로 | 화면별 상태 어댑터 `skin_render/state.rs`, 타이머 드라이버 `screen.rs`, `content.rs`, 내장 화면의 `*_with_content*` 진입점 |
| `rbms-player` | 장면 상태기계와 장면 시계, `SkinHost` 구현(`src/skin_host/` 군집별 파일), 타이머 드라이버, 이벤트 실행, 입력 분배, 스킨 팩 선택·설치, 디코드 워커, 사운드 | 대체 요건 표, 프리셋 결합, 구 번들 `include_bytes!` 표 |
| `rbms-config` | 스킨 팩 선택, 문서별 커스터마이즈, 해상도·창 모드 | `display.skin` 프리셋 결합, 번들 공유 스코프 |
| `rbms-audio` | 루프·정지, 스킨 사운드 버스 | - |
| `rbms-cli` | 스킨 덤프(객체 수·경고·pcall 실패·프레임 비용) | - |

### 4.2 Lua 런타임

| 항목 | 규칙 | 근거 |
| --- | --- | --- |
| 버전 | mlua `lua52` + `vendored` | [00 §2.2] |
| 상태 수명 | 스킨 로드 1회 = 상태 1개. 헤더 패스와 본체 패스가 같은 상태를 쓴다. 화면 진입마다 새로 만든다. 단 `Transition::Open`/`Back`(설정·폴더·표 등을 선곡 위에 여닫기)은 스킨·Lua 상태·장면 시계를 보존한다 | [b1 §3.2, 99 P8-3] |
| 헤더 전용 읽기 | 스킨 목록·SKIN 탭용. 별도 상태에서 `main_state`/`timer_util`/`event_util` 을 빈 테이블로 두고 1회 실행, 파일 mtime 키로 캐시 | [b1 §14] |
| 표준 라이브러리 | base 전부(`print` 는 로그), package(자체 검색기), table, string(패턴 포함, `dump` 제외), math, bit32, coroutine. `load` 는 텍스트 청크만. 패턴 함수 4종(find/match/gmatch/gsub)은 예산에 청구되는 자체 구현(`lua/pattern.rs`, lstrlib.c 5.2.4 이식, C 구현과 대조 테스트)이고, `setmetatable` 은 `__gc` 종료자가 돌지 않게 감싼다(LuaJ 와 같은 관찰 결과) | [b1 §2.1] |
| `require` | 점을 구분자로 바꿔 스킨 루트에서만 찾고 캐시한다 | [b1 §2.4] |
| `dofile`/`loadfile` | 절대 경로와 루트 상대 경로 허용, 정규화 뒤 루트 밖이면 오류. 캐시 없음, 같은 전역 | [m1 §2.5] |
| io | 읽기는 오버레이 → 스킨 루트 순. 없는 파일은 `nil, 메시지`. 쓰기는 오버레이(E1). 줄 읽기는 `\r` 을 버린다. 한도: 동시에 열린 파일 64개, 한 번의 읽기 64 MiB, 오버레이 폴더 총 크기 상한(웨이브 2B 에서 추가) | [b1 §2.2, 99 M8] |
| os | `clock`, `date`(현지), `difftime`, `time`, `setlocale` 만 | [b1 §2.1] |
| luajava | `bindClass` 는 `java.io.File`, `com.badlogic.gdx.Gdx`, `com.badlogic.gdx.Input`, Controllers 계열만. `File:mkdir/listFiles` 는 오버레이 규칙. `Gdx.input:isKeyPressed(code)` 는 호스트 키 상태. 키 코드는 E7. `URL` 은 `connect()` 실패 | [b1 §2.3] |
| 난수 | 운영은 호스트 엔트로피 시드, 테스트·캡처는 고정 시드 | [b1 §12] |
| 예산 | 로드 단계와 프레임 단계를 분리. 수치는 덤프 도구의 실측(선곡 본체 1000 프레임)으로 정한다. 프레임 초과 시 그 프레임 나머지는 직전 값. 프레임 벽시계는 Lua 호출 구간의 누적 시간만 잰다. Rust 로 쓴 라이브러리(패턴, 파일 헬퍼)는 `Meter::charge` 로 작업량을 청구한다 | [r1 §5.4, 99 P6] |
| 오류 | beatoraja 처럼 기본값(false/0/""/OFF)으로 대체하고 매 프레임 재호출. 로그는 함수별 첫 1회 + 누적 횟수. pcall 로 삼켜진 오류도 기록 | [b1 §5.1] |
| 바인딩 | `main_state.*` 는 영구 트램펄린, 호스트는 프레임당 1회 `scope` 로 교체(`#![forbid(unsafe_code)]` 유지) | [r1 §5.5] |
| 스레드 | 장면 전환 시 동기 로드. 이미지 디코드는 기존 워커 | [00 §2.2] |

### 4.3 로드 결과와 값 참조

- `SkinDef` 를 재사용한다. Lua 테이블은 전용 변환기 `loader/from_lua.rs` 가 옮긴다: 필드명 일치 키만 대입, int 는 0 방향 절삭, boolean 은 Lua 진리값(숫자 0 도 참), 문자열 필드의 숫자는 문자열화, 배열 구멍 메움, 속성 자리의 Lua boolean 은 조건 없음 [b1 §4].
- `PropertyRef` 에 `Func(LuaFnId)` 와 `Name(String)` 을 추가한다. 해석 순서: 함수 → 숫자 id → 이름 조회 → `return <문자열>` 컴파일 [b2 §17].
- `DestinationTrack.timer` 는 `Option<TimerRef>`(`Id`/`Lua`). `act`/`event`/`click` 은 `EventRef`, 슬라이더·텍스트 쓰기는 `FloatWriterRef`/`StringWriterRef`.
- 함수 값은 Lua 레지스트리에 두고 `LoadedSkin` 이 Lua 상태를 소유한다.
- 프레임당 호출 순서: 조건 통과 객체에 한해 선언 순서로 `op`/`draw` → `timer` → 값 함수. 첫 false 에서 중단. 부수효과 훅이 이 순서에 의존한다 [b1 §5.3].
- JSON/JSON5 로더는 유지한다(beatoraja JSON 스킨도 같은 모델).

### 4.4 호스트 계약 `SkinHost` (`rbms-skin` 선언, 앱 구현)

구현 상태(웨이브 2A, `crates/rbms-skin/src/property/host.rs`): 아래 표의 의미는 그대로이고 배치만 다르다. `boolean(id) -> Option<bool>` 과 `offset(id) -> Option<SkinOffset>`(None = 설정 없음, Lua 에는 0)은 상위 트레이트 `DrawStateSource: OffsetSource`(`dst.rs`)에 있고 `SkinHost: DrawStateSource` 다. `rate()`·`exscore()` 계열은 `score(ScoreSlot) -> ScoreSnapshot`, 볼륨은 `volume(VolumeBus)`/`set_volume` 으로 묶였다. 모든 메서드가 `&self` 이므로 부수효과(이벤트, 오디오, 쓰기)는 호스트가 내부 가변성으로 쌓았다가 프레임 뒤에 적용한다. 테스트·덤프용 `MapHost`(id → 값 맵, serde, 호출 기록)가 같은 파일에 있다.

| 메서드 | 의미 |
| --- | --- |
| `boolean(id) -> Option<bool>` | `None` = 구현 없는 id. 로더는 스킨 옵션 맵으로, Lua `option()` 은 false 로 |
| `is_static(id) -> bool` | 정적 불리언(로드 시 1회 평가 후 제거). 생성 도구가 표로 내보낸다 |
| `integer(id) -> i32` | 값 없음 = `i32::MIN` |
| `image_index(id) -> i32` | 숫자와 별개인 이미지 인덱스 공간. 음수는 미표시 |
| `rate(id) -> Option<f32>`, `float(id) -> f32` | 슬라이더·그래프는 RATE, `float_number` 는 FLOAT 우선 |
| `text(id) -> Cow<str>` | 없으면 "" |
| `offset(id) -> SkinOffset` | 0..199. 값 단위는 출력 픽셀 |
| `timer_us(id) -> i64`, `now_us()` | OFF = `i64::MIN`. Lua 에는 double |
| `exec_event(id, a1, a2)` | 미정의 id 무시 |
| `write_rate(id, v)`, `write_text(id, s)` | 슬라이더 드래그, 검색 입력 |
| `audio(cmd)`, `key_pressed(code)`, `gauge()`, `gauge_type()`, `judge(n)`, `rate()`, `exscore()`, `volume_*`, `set_volume_*` | `main_state` 전용 |

구현 배치(`apps/rbms-player/src/skin_host/`, 군집은 [b5 §17]): A 곡 메타 `chart.rs`, B 라이브 점수 `score.rs`, C·D 판정·입력 `play.rs`, E 플레이어 설정 `options.rs`, F 선곡 바·패널 `select.rs`, G 결과 `result.rs`, H IR `ir.rs`, I 시간·통계 `system.rs`, J 스킨 설정 `skin_config.rs`, K 키 설정 `keyconfig.rs`, M 로딩 `loading.rs`, 그 외 `play_timers.rs`, `audio.rs`, `writers.rs`. `mod.rs` 가 id → 군집 분배 표를 갖는다.

프레임 데이터는 화면당 한 종류였던 `FrameExtra` 를 능력별 선택 필드(노트 필드, 곡 바 목록, 시계열: 게이지 이력·타이밍·BPM·노트 분포, 참조 이미지 세트)로 바꾼다.

### 4.5 장면 수명과 화면 흐름

공통 전환 규칙: 이전 장면 종료 → 이전 스킨 해제 → 새 장면 상태 확정 → 스킨 동기 로드(헤더 → 본체) → 정적 조건 제거 → 타이머 전체 OFF + 장면 시계 0 → 장면 시작 [b4 §1]. 스킨 구성 시점에 곡·결과 상태가 확정돼 있어야 한다 [m1 §10]. 타이머는 µs, 0..2999 내장.

| beatoraja 화면 (type) | 목표 |
| --- | --- |
| MUSICSELECT (5) | `Select` 가 스킨 단독. `input` 뒤 STARTINPUT. 곡 결정 시 즉시 Decide |
| DECIDE (6) | `Decide` Stage 신설. `input` → STARTINPUT, `scene` → FADEOUT, FADEOUT 경과 > `fadeout` → Play. 건너뛰기: STARTINPUT 뒤 키 인덱스 0/2/4/6 또는 Enter. 취소: Esc 또는 START+SELECT. 차트·키음 로드는 이 동안 백그라운드로 시작 [b4 §4] |
| PLAY (0~3) | PRELOAD(로딩) → READY(타이머 40) → PLAY(41, 140) → FAILED(3, `close` 뒤 종료) / FINISHED(908 → `finishmargin` → 2 → `fadeout`). 헤더 시간 필드는 note 객체가 있을 때만 적용 [b3 §2·§10] |
| RESULT (7) | 타이머 150/151/152 첫 프레임, `input` 동안 입력 잠금, FADEOUT 뒤 전환. 키 0~3 OK, 5 게이지 종류 전환, 리플레이 슬롯 [b4 §5] |
| COURSERESULT (15) | 스킨 화면. 결과 헬퍼 재사용 [b4 §6] |
| CONFIG (8) | 내장 키 설정 유지. 스킨은 헤더만 인식 |
| SKINCONFIG (9) | SKIN 탭 유지. type 9 Stage 는 후속 |
| 직접 실행(명령줄 차트·리플레이) | Play(PRELOAD) → Result → 종료 또는 선곡 |

스킨이 없을 때(내장 폴백) Stage 는 헤더 시간값을 0 으로 본 같은 상태기계를 탄다(즉시 전환). 뷰 모델은 새 호스트 모델에서 내장 뷰(`SelectView`/`HudView`/`ResultView`)로 내려 변환한다 [99 P4].

R-BMS 고유 규칙
- 연습 구간 반복: PRELOAD 를 생략하고 READY 부터 다시. 타이머 41/140 은 구간 시작 기준으로 재설정 [99 H6].
- 리플레이 분석: 장면 시계를 가상 시계에 묶고, 탐색 시 레인별 타이머를 전부 끈다 [99 H7].
- 차트 미리보기(타이머 141): PRELOAD 중 START/SELECT 를 누르면 켠다 [99 M1]. 폐점 즉시 재시작(START xor SELECT)도 구현한다 [99 M7].
- 플레이 탈출은 R-BMS 방식(Esc 홀드/두 번) 유지. FAILED/FINISHED 중 Esc 는 연출 건너뛰기 [99 H13].
- 결과: R = REPLAY_SAME, N(다음 곡)은 고유 유지. 입력 잠금은 둘 다에 적용 [99 H8].
- 선곡: 곡 목록은 원형 인덱스. 막대 0개면 songlist 를 그리지 않고 시스템 안내를 띄운다 [99 M2·H1]. 미리듣기 재생 중에는 선택 BGM 을 멈춘다 [99 H14].
- 스킨 버튼 대응: 13 → 키 설정, 14 → SKIN 탭, PRACTICE → Practice Stage, AUTOPLAY → 오토플레이 시작, REPLAY1~4 → 최근 리플레이 4개(없으면 무동작), 90 → 즐겨찾기 토글 [99 H3·H4].
- 스크래치 방향 등 스킨 옵션과 앱 설정이 겹치면 스킨 옵션이 우선하고 앱 설정 행은 숨긴다. 스킨 화면에서 의미 없는 설정 행(`five_key_layout`, `judge_text_y`, `show_white_number`, `score_graph`, `result_graphs`)은 "내장 폴백 전용"으로 표시한다 [99 H10·H11].

### 4.6 렌더

- 해상도: 스킨 화면의 논리 크기 = 레터박스 뒤 물리 뷰포트. 스킨 w/h 가 Resolution 열거값이 아니면 1280x720 으로 본다. 내장 화면과 시스템 오버레이는 1280x720 좌표 + 배율 래퍼 [r2 §4·§10].
- 스케일되지 않는 값은 beatoraja 대로 둔다: 오프셋 값, TTF size, shadowOffset, 자릿수 offset, 비주얼라이저 width [b2, b3 §15].
- 프레임 순서: 시각 확정 → 화면 로직 → 전 객체 prepare(조건 → 타이머 → 값) → 그리기 → 클릭 [b1 §8.2]. customTimers/customEvents 는 ModernChic 에서 빈 테이블이므로 웨이브 9.
- 조건: 미지 op 는 부호와 무관하게 제거, 정적 불리언은 로드 시 1회 평가 [b1 §5.3].
- 기본 객체: 이미지 인덱스 공간, 음수 ref 미표시, 값 없음 센티널, 음수 w/h 뒤집기, 참조 이미지(-100 스테이지파일, -101 BACKBMP, -102 배너, -110 검정, -111 흰색), stretch 11종, acc 는 선언 순서 첫 비영 값, 오프셋 `r` 부호 교정. graph/slider 값은 클램프하지 않고 NaN 은 0 [99 M4].
- 텍스트: 1차 TTF(크기 = dst h, 정렬 기준점, 상단 기준 y, overflow 0/1/2, wrapping, 그림자, `ref` 유효 시 `constantText` 무시, 직전 객체 blend 상속, 누락 글리프는 E5). 2차 `.fnt` type 0, 3차 distance field(type 1/2) [b2 §12, r2 §6].
- 텍스처: 참조된 source 만 지연 로드, 화면 이탈 시 해제, 어댑터 한도 초과는 경고 후 생략 [r2 §3].
- 동영상 source(D5): source 를 종류로 분류하고 디코더 트레이트 뒤에서 프레임을 같은 key 로 제자리 업로드한다. 프레임이 없으면 그리지 않는다 [b2 §9, r2 §3.5].
- 입력: press 는 z 역순으로 clickevent 가 있고 draw 인 첫 객체, click 0~3 규칙, 드래그는 슬라이더만, mouseRect 는 호버 조건 [b2 §15].
- songlist: 막대 종류 0~6 인덱스, 텍스트 11종, 스크롤 보간식, 그리기 순서는 막대별이 아니라 패스별(전 막대 이미지 → 그래프 → 제목 → 트로피 → 램프 …) [b4 §2, 99 M6].
- 플레이 객체: note y 누적식과 구식 LN 명명, gauge 36칸과 결과 화면 차오름, judge 영역 콤보와 shift, bga 레이어, 그래프 4종 [b3 §3~§8, 99 M5].
- 금지: beatoraja 특이 동작([b2 §18], [b3 §15])을 "정상화"하지 않는다.

### 4.7 스킨 팩과 기본 스킨

- 스킨 팩 = `.luaskin`(또는 `.json`) 여러 개가 든 폴더 하나. 설정은 팩 폴더 하나와 화면 타입별 커스터마이즈(`{option, file, offset}` 이름 기준)를 저장한다. 화면 타입 → 문서 매핑은 헤더 `type` 으로 한다.
- 웨이브 2 부터 팩 폴더를 설정 한 줄과 환경 변수 `RBMS_SKIN_PACK` 으로 지정할 수 있어야 한다(실화면 확인용). SKIN 탭 UI 는 웨이브 8.
- 기본 스킨(자작, D1): 폴더 `assets/skins/rbms-default/`. beatoraja 형식 `.luaskin` 문서(선곡, 결정, 플레이 5/7/9/10/14키, 결과, 코스 결과) + 이미지 생성 스크립트(PEP 723, `uv run`) + 사운드 생성 스크립트. 1920x1080 저작. 배치는 `m2`~`m5` 의 레이아웃 절을 참고하되 D4 범위로 줄인다. luajava·파일 쓰기·`os.date` 를 쓰지 않는다(읽기 전용 위치에서 동작). 폰트는 앱 번들 폰트를 쓴다. 구 번들의 자작 사운드 22개와 생성 스크립트는 삭제하지 말고 이 폴더로 옮겨 재사용한다.
- 배포: 기본 스킨은 바이너리에 내장하고 첫 실행 때 `<설정 폴더>/skin/rbms-default/` 로 설치한다(이미 있는 파일은 덮어쓰지 않는다). 내장 표는 손으로 쓰지 않고 스크립트로 생성한다. 용량이 10 MiB 를 넘으면 폴더 동봉 방식으로 바꿀지 그때 묻는다.

## 5. 삭제 대상 (D3)

| 대상 | 위치 |
| --- | --- |
| 번들 3세대와 내장 표·세대 이동·설치 로직 | `assets/skins/steel-neon*`, `apps/rbms-player/src/assets.rs`(`BUNDLE_GENERATIONS` 등) |
| 혼합 합성 모델 | `crates/rbms-skin/src/{model.rs, model/objects.rs, loader.rs, resolve.rs}` 의 `composition`, `destination.layer`, `replace`, `hotspot`, `scope: 'bundle'`, `densitygraph`, `judge.images` 의 text 허용 |
| 혼합 합성 소비자 | `apps/rbms-player/src/{skin_screen.rs, app_options.rs, app_input.rs, skin_select.rs}`, `stage/{play/mod.rs, select/mod.rs, result.rs, loading.rs}`, `crates/rbms-render/src/{content.rs, lib.rs, hud.rs, select.rs, result.rs}`, `skin_render/*` 의 `SkinLayer`·`Body::Density`·사설 id 20001~20316 |
| 설정 | `crates/rbms-config/src/{schema.rs, settings.rs, lib.rs, tests.rs}` 의 `STEEL_NEON_SKIN`, `shared`, `default_skin_installed`, 프리셋 결합. 스키마 버전을 올려 구 선택값을 버리고 마이그레이션 테스트를 둔다 |
| 테스트 | `apps/rbms-player/src/stage/render_tests_skin*.rs` 5개와 `stage/mod.rs` 의 선언, `skin_select/{tests,fixtures}.rs`·`skin_screen/tests.rs`·`stage/settings/skin_tests.rs`·`main_tests.rs` 의 번들 참조, `crates/rbms-skin/tests/{skin_nested.rs, fixtures/nested}` 중 확장 전용분 |
| 문서 | `docs/skin.md` 는 웨이브 8 에서 새로 쓴다. 구 사양·결정(`docs/plan/2026-09-17-*`, `docs/acknowledge/2026-09-17-skin-system-decisions.md`)은 이력으로 두고 상단에 "2026-10-09 폐기"를 적는다 |

유지: 경로 가두기, 시드 난수, 문서 크기 한도, `nested` 선조립, NORMAL/WIDE 필드 RON(내장 폴백용), 자작 사운드와 생성 스크립트(기본 스킨으로 이동).

## 6. 구현 웨이브

공통 규칙
- 웨이브 하나 = Workflow 하나(필요하면 둘). 골격 단위(모듈 선언·빈 파일·열거형 변형)를 먼저 직렬로 깔고, 그 뒤 단위는 자기 파일만 채운다. 같은 파일을 같은 시점에 두 단위가 고치지 않는다.
- 시그니처를 바꾸는 단위는 호출부 전부를 소유하고 단독 직렬로 돈다.
- 병렬 단위는 끝에 `cargo check -p <크레이트>` 와 자기 테스트만 돌린다. 다른 단위 파일 때문에 생긴 일시적 컴파일 오류는 60초 간격으로 3회까지 다시 시도한 뒤 보고한다. 전체 게이트는 메인이 통합 뒤 한 번 돌린다.
- 각 단위 지시에는 목표·완료 조건, 근거(조사 보고서 절), 소유 파일과 비목표, 순서, 금지 사항, 검증 명령, 보고 형식(변경 파일, 판단 근거, 검증 출력, 남은 위험, **고쳐야 할 조사 문서 절**), 통합 관계를 적는다.
- 웨이브 끝: 메인이 게이트 → 캡처 확인 → 조사 문서(`r1`~`r3`, `00-synthesis` 매트릭스, 색인 갱신 이력) 갱신 → `docs/PROCESS.md` 체크 → 커밋·푸시.
- 모델: 구조 판단이 큰 단위는 Opus high, 사양이 표로 고정된 단위는 Sonnet high 또는 xhigh. 상한은 Opus high, Sonnet xhigh.

### 웨이브 1 — 철거와 기반

끝 상태: 구 번들과 혼합 합성이 없고 앱은 내장 화면으로 동작한다. 창 해상도로 직접 그린다. 새 캡처 하니스가 PNG 를 낸다.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W1-1a | 구 번들 자산·내장 표·설치·세대 이동 삭제(사운드와 생성 스크립트는 `assets/skins/rbms-default/` 로 이동), 설정 프리셋·`shared`·`default_skin_installed` 삭제와 스키마 버전 상승·마이그레이션 테스트 | `assets/skins/steel-neon*`, `apps/rbms-player/src/{assets.rs, syssound.rs, lib.rs, main_tests.rs, settings_ui.rs}`, `stage/{settings.rs, settings/skin_tests.rs}`, `crates/rbms-config/src/{schema.rs, settings.rs, lib.rs, tests.rs}` | 없음 | opus high | [r3 §3·§8], §5 |
| W1-1b | 혼합 합성 소비자 철거(layer/replace/hotspot/content/사설 id). 내장 화면 골든 불변 확인 | §5 "혼합 합성 소비자" 행 전부, `skin_render/{mod, screen, state, songlist, object, graphs, judge, draw}.rs`, `skin_render/tests*.rs`, `stage/{select,play}/tests.rs`, `skin_screen/tests.rs`, `skin_select/{tests,fixtures}.rs` | W1-1a | opus high | [r3 §8.2·§8.6, r2 §8, 99 P1] |
| W1-1c | 구 스킨 캡처 테스트 5개와 선언 삭제 | `stage/render_tests_skin*.rs`, `stage/mod.rs` | W1-1b | sonnet high | [99 P7] |
| W1-2 | 모델·로더 확장 필드 삭제 | `crates/rbms-skin/src/{model.rs, model/objects.rs, loader.rs, resolve.rs}`, `tests/{skin_nested.rs, skin_model.rs, skin_loader.rs, fixtures/nested/*}` | W1-1b | sonnet high | [r1 §7] |
| W1-3 | mlua 피처 `lua54` → `lua52`. 기존 `tests/skin_lua.rs` 통과 확인만(샌드박스 재작성은 W2) | `crates/rbms-skin/Cargo.toml`, 필요한 최소의 `src/lua.rs` | W1-2 | sonnet high | [r1 §5.3] |
| W1-4 | 타이머 µs(OFF = `i64::MIN`), 타이머 참조형 자리, 오프셋 `r` 부호, acc 선언 순서, stretch 11종((목적지, 소스) 쌍 반환). **호출부 전부 소유** | `crates/rbms-skin/src/{timer.rs, timer/tests.rs, dst.rs, dst/tests.rs, loader/track.rs, loader/stretch.rs, property/mod.rs, lua.rs}`, `tests/*.rs`, `crates/rbms-render/src/{lib.rs, skin_render/*.rs}`, `crates/rbms-render/tests/skin_render.rs`, 앱의 `skin_screen.rs`·`stage/{play/mod.rs, play/tests.rs, result.rs, select/mod.rs}`·`lib.rs` 의 타이머 호출부 | W1-3 | opus high | [r1 §6.2·§6.3, 99 W1] |
| W1-5 | 장면 시계, 전환 시 타이머 리셋(Open/Back 은 보존), 마우스 버튼·release·drag·휠 이벤트 배선(`StageHandler` 기본 no-op) | `apps/rbms-player/src/{lib.rs, stage/mod.rs, skin_screen.rs, app_play.rs}` | W1-4 | sonnet xhigh | [r3 §7.5·§12, 99 P8-3] |
| W1-6 | 텍스트 캐시 패밀리 분리, `load_font` 성공 판정 | `crates/rbms-render/src/font.rs` | W1-1b | sonnet high | [r2 §6.2·§6.3] |
| W1-7 | GPU 논리 크기 런타임화, 내장 화면용 배율 래퍼, 텍스처 한도 확인, 색 공간 비교 캡처와 결정(E10) | `apps/rbms-player/src/gpu/{mod.rs, batch.rs, background.rs}`, `stage/canvas.rs`, `crates/rbms-render/src/lib.rs`, `apps/rbms-player/src/lib.rs` | W1-5 | opus high | [r2 §10 A1~A5] |
| W1-8 | 해상도·창 모드 설정 행 | `crates/rbms-config/src/{schema.rs, settings.rs}`, `apps/rbms-player/src/{settings_ui.rs, stage/settings.rs}` | W1-7 | sonnet high | [r3 §9.4] |
| W1-9 | 캡처 하니스: 임의 해상도 헤드리스 캔버스 → PNG, `RBMS_SKIN_CAPTURE_DIR` 출력, 외부 스킨 경로 `RBMS_SKIN_PACK` 을 받을 때만 도는 선택 테스트. 가능하면 wgpu 오프스크린 읽기 | `apps/rbms-player/src/stage/{capture.rs(신규), mod.rs}` | W1-7 | opus high | [99 P7] |
| W1-10 | START/SELECT 바인딩, "키 인덱스 0~8 눌림" 질의(키보드·패드), 키 설정 UI 행, `SelectState::handle_pad` | `apps/rbms-player/src/{keyconfig.rs, keyconfig_tests.rs, gamepad.rs, app_input.rs}`, `stage/keyconfig.rs` | W1-5 | sonnet xhigh | [99 W3·P6] |

검증: 게이트, 내장 select/play/result 골든 불변, 새 하니스로 내장 화면 PNG 1장.

진행 기록(2026-10-09): **웨이브 1A 완료** — W1-1b·W1-1c → W1-1a ∥ W1-6 → W1-2 → W1-3 순으로 수행했다(중간 상태가 컴파일되도록 표의 선행 열과 순서를 바꿨다). Workflow `wf_575fa3b4-9ac`, 게이트 통과(테스트 3,111 통과·0 실패·ignored 3), 내장 골든 불변. 표와 달라진 점: 구 스킨 테스트 5개는 지웠지만 문서 단독 그리기·화면 간 격리 검증은 `stage/render_tests_document.rs` 로 남겼다. 계정 설정 블롭(`apps/rbms-player/src/ir_sync.rs`)도 구 번들 정리 마이그레이션을 탄다. `ResultExtras` 는 내장 결과 화면 입력이라 남겼다. 설정 스키마는 3 이다. 남은 것은 웨이브 1B(W1-4, W1-5, W1-7~W1-10).

**웨이브 1B 완료(2026-10-10)** — W1-4 → W1-5 → W1-7 ∥ W1-10 → W1-8 ∥ W1-9 → 리뷰 → 수정. Workflow `wf_e03c0a9e-2fd`(8 에이전트), 리뷰 major 2·minor 5 처리. 게이트 통과(테스트 3,262 통과·0 실패·ignored 3), 내장 골든 불변, GPU 대 CpuCanvas 픽셀 비교 테스트가 이 기기(Metal)에서 실제로 통과. 캡처 하니스 `apps/rbms-player/src/stage/capture.rs`(`Shot`, `take`, `take_on_gpu`, `RBMS_SKIN_CAPTURE_DIR`, `RBMS_SKIN_PACK`, `RBMS_REQUIRE_GPU`)로 내장 선곡 1280x720·1920x1080 PNG 를 CPU·GPU 양쪽으로 확인했다.

웨이브 1 끝 상태의 예외와 뒤 웨이브로 넘긴 것
- 선곡·결정·결과·키 설정 문서는 창 픽셀에 직접 그린다(`canvas.native()`). **플레이 문서만 1280x720 배율 래퍼에 남아 있다**(note·cover 가 내장 `Skin` 의 세로 지오메트리를 읽기 때문). W6-1 이 note 를 문서 기반 지오메트리로 바꾸면서 `draw_play_skin` 을 `canvas.native()` 로 전환한다.
- 경과 시간은 beatoraja 원본대로 `now_us/1000 - timer_us/1000`(각각 ms 절삭) 이다. `TimerRef` 는 `crates/rbms-skin/src/dst.rs` 에 있고 `TimerRef::value_us` 한 곳에서 해소한다. 렌더의 `Sprite.timer`(이미지 셀 애니메이션)는 아직 `Option<TimerId>` 라 W2-1 이 함께 넓힌다.
- 음수 타이머 id 를 "타이머 없음"으로 보는 beatoraja 규칙과 graph 소스 폭의 `(int)` 절삭은 아직 옮기지 않았다(W3-1b, W3-2).
- 캡처 하니스는 무작위 파일 선택 시드를 고정하지 못한다(`skin_select::reload_for` 가 시드를 받지 않음). W2-8 이 시드 인자를 넣는다.
- 휠 한 줄 환산값 `PIXELS_PER_SCROLL_LINE = 40` 은 실측하지 않은 값이다(웨이브 5 실화면에서 확인).
- `KeyConfig.held`(눌림 상태)는 설정 구조체 안의 런타임 상태로 임시 배치돼 있다. 입력 상태를 따로 떼는 정리는 웨이브 5 의 입력 단위에서 한다.

웨이브 8 로 넘기는 결정 사항: 시스템 사운드는 `<설정 폴더>/skin/rbms-default/sound/` 에 고정 설치되고 `skin.folder` 설정을 따르지 않는다. 기본 스킨 문서를 같은 폴더에 설치할 때 SKIN 탭 스캔 루트와 맞춘다. 계정 동기화가 `skin.folder`·`skin.selected`·`skin.custom`(기기 절대 경로)을 원격 값 그대로 받는 기존 동작도 스킨 팩 선택을 넣을 때 기기 로컬로 바꿀지 정한다. 단독 문서의 note 객체는 W6 재작성 전까지 세로 지오메트리를 내장 `Skin` 에서 읽는다.

### 웨이브 2 — Lua 런타임과 로더

끝 상태: 덤프 도구가 ModernChic `.luaskin` 10개의 헤더와 본체 객체 수를 출력한다. 팩 폴더를 지정해 앱이 `.luaskin` 을 고른다. 결정 화면의 이미지 객체만으로 정지 프레임 1장이 나온다.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W2-0 | 골격: 신규 모듈을 빈 파일로 선언 | `crates/rbms-skin/src/{lib.rs, loader.rs, property/mod.rs}` 의 `mod` 선언, `lua/{mod, env, package, io, os, luajava, budget, main_state}.rs`, `lua/prelude.lua`, `loader/{from_lua, lua_skin}.rs`, `property/host.rs`. 구 `src/lua.rs` 는 `lua/legacy.rs` 로 옮겨 컴파일 유지 | 웨이브 1 | opus high | §4.2 |
| W2-1 | 값 참조 타입 확장(함수·이름·이벤트·writer)과 호출부 | `crates/rbms-skin/src/{model.rs, model/objects.rs, dst.rs, loader/track.rs}`, `tests/skin_model.rs`, `crates/rbms-render/src/skin_render/{object.rs, covers.rs}` | W2-0 | opus high | [r1 §3.3, b2 §17] |
| W2-2a | 환경: 표준 라이브러리 구성, `require`/`dofile`/`loadfile`, `print`, 예산, 오류·pcall 실패 로그 | `lua/{mod, env, package, budget}.rs` | W2-0 | opus high | [b1 §2, r1 §5.4·§5.5, m1 §4] |
| W2-2b | io 오버레이와 os | `lua/{io, os}.rs` | W2-2a | opus high | [b1 §2.2, m1 §9] |
| W2-2c | luajava facade | `lua/luajava.rs` | W2-2a | sonnet xhigh | [b1 §2.3, m1 §7] |
| W2-3 | `SkinHost`, `main_state` 전체 API, `timer_util`/`event_util` 프렐류드, 프레임 바인딩, 생성 도구의 정적 분류·이름 역표 | `property/host.rs`, `lua/main_state.rs`, `lua/prelude.lua`, `tools/gen-skin-property.rs`, `property/generated/*` | W2-2a | opus high | [b1 §6·§7, b5 §2] |
| W2-4 | Lua 값 → `SkinDef` 변환기 | `loader/from_lua.rs` | W2-1, W2-2a | opus high | [b1 §4] |
| W2-5 | 2패스 로드, 헤더 병합(`def` 스템 대조, 무작위 -1, 플레이 타입 자동 오프셋 10/30/32/33), type 9·15 허용, 플레이 헤더 필드 적용 조건, `lua/legacy.rs` 제거 | `loader.rs`, `loader/{lua_skin, branch}.rs`, `resolve.rs`, `lua/legacy.rs` | W2-3, W2-4 | opus high | [b1 §3·§9·§10, r1 §4] |
| W2-6 | 스킨 덤프 CLI: 시나리오 파일(JSON, id → 값)을 읽는 호스트, 객체 수·경고·pcall 실패 출력, 본체의 함수 값 전부를 1000 프레임 호출한 평균·최대 시간 | `apps/rbms-cli/**` | W2-5 | sonnet xhigh | [m1 §3, 99 W9·P6] |
| W2-7 | 자작 미니 `.luaskin` 픽스처와 테스트(루트 가두기, 쓰기 오버레이, 패턴 함수, 무한 루프·메모리·바이트코드 차단, 2패스, 변환 규칙) | `crates/rbms-skin/tests/{skin_luaskin.rs, skin_lua.rs, skin_integration.rs}`, `tests/fixtures/luaskin/*` | W2-5 | sonnet xhigh | [r1 §8] |
| W2-8 | 스킨 팩 폴더 지정(설정 한 줄 + `RBMS_SKIN_PACK`), 화면 타입 → 문서 매핑, 헤더 전용 읽기와 mtime 캐시 | `apps/rbms-player/src/{skin_select.rs, skin_select/tests.rs}`, `crates/rbms-config/src/schema.rs` | W2-5 | opus high | [r3 §5, 99 P5] |
| W2-9 | 하니스로 결정 화면 정지 프레임 1장(이미지 객체만) | `apps/rbms-player/src/stage/capture.rs` | W2-8 | sonnet high | [99 P7] |

진행 기록(2026-10-10): **웨이브 2A 완료** — W2-0 → W2-1 ∥ W2-2a → W2-2b ∥ W2-3 ∥ W2-4 → W2-2c → W2-5 → 리뷰 → 수정. Workflow `wf_402f94ef-6d0`(10 에이전트). 게이트 통과(테스트 3,475 통과·0 실패·ignored 3). ModernChic 9/10 로드(keyconfig 는 원본 결함), 9개 모두 pcall 실패 0·경고 0, 스킨 폴더 무변경 확인. 적대 리뷰가 격리 탈출을 실제로 시도했고 파일 가두기·바이트코드·os·debug 는 전부 막혔다. 예산을 뚫는 경로 3건(`__gc` 종료자 루프, 패턴 백트래킹, 조회 실패 이름 캐시)과 파일 읽기·핸들 상한 누락을 수정했다. W2-7 의 미니 픽스처(`tests/fixtures/luaskin/mini/`)와 테스트(`skin_luaskin.rs`, `skin_lua_env.rs`, `skin_lua_io.rs`, `skin_luajava.rs`, `skin_main_state.rs`, `skin_from_lua.rs`, `skin_lua_pattern.rs`)는 이 단계에서 선반영됐다. 남은 것은 웨이브 2B(W2-6 덤프 CLI와 프레임 비용 실측, W2-8 스킨 팩 폴더 지정, W2-9 결정 화면 정지 프레임, 오버레이 총 크기 상한).

**웨이브 2B 완료(2026-10-10)** — W2-6 ∥ W2-8 ∥ W2-10(오버레이 상한) ∥ W2-9 → 리뷰 → 수정. Workflow `wf_c034401f-ce6`(6 에이전트), 리뷰 major 3·minor 8 처리. 게이트 통과(테스트 3,530 통과·0 실패·ignored 3). 결과: `rbms-cli skin-dump`(프레임 비용 실측 포함), 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드(실패 시 알림 1회 + 내장 폴백), 오버레이 총 크기 상한, 외부 스킨 정지 프레임. 메인이 결과·선곡 캡처를 직접 열어 확인했다(상세와 빠진 것 목록은 `docs/reference/skin-compat/v1-first-render.md`). 앱 경로에서는 ModernChic 선곡·결정·결과·플레이 7키가 로드되고 키 설정만 실패한다. 다만 앱의 상태 어댑터가 아는 id 가 적어 앱 경로의 화면은 렌더 단위 캡처보다 빠진 것이 많다(결과의 판정 표 등). 웨이브 3 부터 호스트 군집이 채운다.

웨이브 3 이 먼저 고칠 공통 원인(`v1-first-render.md`): (가) 음수 id 참조 이미지(-100/-101/-102/-110/-111) 미구현, (나) `dst` 가 없는 최상위 destination(`songlist`, `note`)이 키프레임 0개라 그려지지 않음, (다) 음수 폭 객체가 버려짐, (라) 텍스트 크기·overflow 의미론.

덤프 기준 시나리오(화면별): 난이도 option 150~155 중 하나 true(없으면 150), BGA 있음/없음, 곡 메타 문자열 채움 [m5 부록 B, m2 §11]. 기대값: 헤더 표가 [m1 §3] 과 일치, keyconfig 본체만 `Decide/lua/require/textproperty.lua:37` 에서 실패. 객체 수 수기 집계(play7 destination 약 288, play14 약 375, select 1914, result 194, course 175, decide 약 69, skinselect 108)와 다르면 덤프를 믿고 조사 문서를 고친다.

### 웨이브 3 — 공통 렌더 의미론과 결정 화면

끝 상태: ModernChic 결정 화면 캡처(t = 0, 500, 1500, 3000ms). 앱에서 선곡 → 결정 → 플레이로 넘어간다.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W3-0 | 골격: `Body` 변형과 `draw_object` 분기 전부(bga, text_input 포함) 빈 구현, `skin_render/{frame, text, refs, textures, input, bga, text_input}.rs`, `graphs/` 분할, `skin_host/mod.rs` 에 전 군집 모듈 빈 선언과 id 분배 표, 능력별 프레임 데이터 타입 | `crates/rbms-render/src/{lib.rs, skin_render/**}`, `apps/rbms-player/src/{lib.rs, skin_host/**}` | 웨이브 2 | opus high | [99 P1] |
| W3-1a | 프레임 파이프라인: prepare/draw 2단계, 호스트 연결(구 `state.rs` 경로는 남겨 두고 새 경로 추가) | `skin_render/{mod, frame}.rs`, `apps/rbms-player/src/skin_screen.rs` | W3-0 | opus high | [b1 §8.2, r2 §7] |
| W3-1b | 조건 의미론: 미지 op 제거, 정적 1회 평가 | `crates/rbms-skin/src/loader/track.rs`, `dst.rs` | W3-0 | opus high | [b1 §5.3, r1 §6.2] |
| W3-1c | Lua 함수 값 평가와 프레임 바인딩. 타이머 함수는 프레임당 1회 호출 후 재사용(ModernChic 직접 타이머 함수 17개의 부수효과 유무를 확인) | `skin_render/object.rs`, `crates/rbms-skin/src/lua/mod.rs` | W3-1a | opus high | [b1 §5, 99 P8-5] |
| W3-2 | 기본 객체 정합(§4.6 "기본 객체") | `skin_render/{draw, refs}.rs` | W3-1a | opus high | [b2 §9~§14·§18, r2 §5, 99 M4] |
| W3-3 | 텍스트 1차(TTF) | `crates/rbms-render/src/{font.rs, glyph_atlas.rs}`, `skin_render/text.rs` | W3-1a | opus high | [b2 §12, r2 §6, 99 M3] |
| W3-4 | 텍스처 관리자 | `skin_render/textures.rs`, `apps/rbms-player/src/assets.rs` | W3-1a | opus high | [r2 §3] |
| W3-5 | judgegraph(노트 분포), bpmgraph | `skin_render/graphs/{mod, notes_dist, bpm}.rs` | W3-0 | sonnet xhigh | [b3 §8.1·§8.2] |
| W3-6 | 호스트 군집 A·I·M | `skin_host/{chart, system, loading}.rs` | W3-0 | sonnet xhigh | [b5 §3~§6·§17, m5 부록 B] |
| W3-7 | Decide Stage, 장면 수명 헬퍼, 로딩 분리(차트 로드는 백그라운드, 스캔·표 받기 대기는 내장 유지) | `apps/rbms-player/src/stage/{decide.rs, scene_life.rs, loading.rs, mod.rs}`, `stage/select/mod.rs`(곡 결정 전이만), `app_play.rs` | W3-2~W3-6 | opus high | [b4 §4, r3 §6.5] |

진행 기록(2026-10-10): **웨이브 3A 완료** — W3-0 → W3-1a → W3-1b ∥ W3-2 ∥ W3-5 ∥ W3-6 → W3-1c ∥ W3-3 → 리뷰 → 수정. Workflow `wf_02e2af85-d93`(10 에이전트), 리뷰 major 2·minor 5 처리. 게이트 통과(테스트 3,716 통과·0 실패·ignored 3), 내장 골든 불변. 리뷰어가 캡처를 열어 조사 문서 좌표와 대조한 결과 **ModernChic 결정 화면은 m5 §9.1 의 좌표·색·페이드와 일치**하고(틀린 것 없음), 결과·선곡 화면의 공통 객체도 좌표대로 나온다. 메인도 결정 1500ms·선곡 3000ms 캡처를 직접 열어 확인했다. 선곡은 곡 바가 그려지지만 바 안의 제목·레벨·램프는 웨이브 5, 플레이의 노트 세로 지오메트리·게이지·판정은 웨이브 6 범위다. 상세는 `docs/reference/skin-compat/v1-first-render.md` 상단.

구현 메모(뒤 웨이브가 알아야 할 것)
- 렌더: `SkinScreen::prepare` → `draw` 2단계, 프레임 데이터는 `crates/rbms-render/src/skin_render/frame.rs` 의 `FrameData { field, gauge, bars, series, images, bga }`. 텍스트 조판은 `crates/rbms-render/src/font/block.rs`.
- 앱 호스트: `apps/rbms-player/src/skin_host/`(`ScreenHost`, 군집별 파일, id → 군집 분배 표 `ROUTES`). 군집이 모르는 id 는 구 어댑터(`skin_render/state.rs`)의 답으로 폴백한다. 구 어댑터는 모르는 옵션에 `Some(false)`, 모르는 이미지 인덱스에 0(첫 그림)을 답하므로, 군집이 채워지기 전에는 실제 설정과 무관한 첫 그림이 보일 수 있다.
- 스킨 시계(숫자 21~26)는 Lua `os.date` 와 같은 C 라이브러리 로컬 시각(`rbms_skin::lua::local_time`)이다. 새 의존성은 없다.
- 실수 값 없음은 원본처럼 `f32::from_bits(1)`(`FLOAT_ABSENT`)이다.
- 남은 웨이브 3 단위: W3-4(텍스처 관리자), W3-7(Decide Stage·장면 수명·로딩 분리) = 웨이브 3B.

### 웨이브 4 — 결과와 코스 결과

끝 상태: 결과·코스 결과 캡처(클리어/실패, 메뉴 1/2), Info 클릭 전환. 내장 플레이 → 스킨 결과 실화면 흐름.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W4-1 | 클릭·호버·슬라이더 드래그·휠 디스패치와 앱 연결, 이벤트 실행기(미정의 id 무시) | `skin_render/input.rs`, `apps/rbms-player/src/skin_screen.rs`, `skin_host/mod.rs` | 웨이브 3 | opus high | [b2 §15, b4 §1.3] |
| W4-2 | gauge 칸 선택식과 결과 차오름, gaugegraph, timingdistributiongraph, judgegraph type 1/2 | `skin_render/gauge.rs`, `graphs/{gauge_graph, timing_dist, notes_dist}.rs` | 웨이브 3 | opus high | [b3 §5, b4 §5, 99 M5] |
| W4-3 | 플레이 엔진 기록 확장: 전 게이지 종류 이력(PLAY 중 500ms 간격 동시 기록, 폐점 시 0 채움), 판정별 early/late, 통과 노트 기준 값. 착수 시 `crates/rbms-play` 의 대상 파일을 먼저 확정해 보고 | `crates/rbms-play/**`, `apps/rbms-player/src/app_result.rs` | 웨이브 3 | opus high | [b5 §16, 99 W2] |
| W4-4 | Result Stage 수명·입력·게이지 종류 전환·리플레이 슬롯(§4.5 결과 규칙) | `apps/rbms-player/src/stage/result.rs` | W4-3 | opus high | [b4 §5, m5 §2] |
| W4-5 | 호스트 군집 B·G·H·E 일부(42, 43, 90, 301~307, 370) | `skin_host/{score, result, ir, options}.rs` | W4-3 | sonnet xhigh | [m5 부록 A, b5 §17] |
| W4-6 | CourseResult Stage | `stage/course_result.rs`, `course_ui.rs` | W4-4 | sonnet high | [b4 §6, m5 §14] |
| W4-7 | 오디오: 루프·정지, 스킨 버스 `audio_play`(0..2 클램프 x 시스템 볼륨, 0.0001 은 프리로드), 호스트 키 상태 | `crates/rbms-audio/**`, `apps/rbms-player/src/syssound.rs`, `skin_host/audio.rs` | 웨이브 3 | sonnet xhigh | [m1 §9, b4 §1.6] |

### 웨이브 5 — 선곡

끝 상태: 선곡 캡처 세트(곡/폴더/코스 바 x 점수 유무 x 패널 0~3), 실화면에서 선곡 조작.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W5-1 | songlist 재작성(원형 인덱스, 패스별 그리기, 스크롤 보간, 클릭) | `skin_render/songlist.rs` | 웨이브 4 | opus high | [b4 §2, m4 §5, 99 M2·M6] |
| W5-2 | 선곡 상태 모델(막대 종류, 램프 id, 라벨, 트로피, 리플레이 슬롯, 배너·스테이지파일) + 내장 뷰 변환기 | `apps/rbms-player/src/stage/select/{scene.rs, list.rs}`, `lib.rs` | 웨이브 4 | opus high | [b4 §3.9, r3 §6.3, 99 P4] |
| W5-3a | 선곡 입력 키 표, 장면 수명, 타이머 정합(1, 11, 21~23, 31~33, 172~174) | `stage/select/{mod.rs, filter.rs}` | W5-2 | opus high | [b4 §3.3, b5 §1-8] |
| W5-3b | 패널 1~3 과 옵션 이벤트 표, 스킨 버튼 대응(§4.5) | `apps/rbms-player/src/app_options.rs`, `skin_host/options.rs` | W5-3a | opus high | [b4 §3.4~§3.6, 99 H3·H4·H10] |
| W5-4 | 호스트 군집 F·H(선곡) | `skin_host/{select, ir}.rs` | W5-2 | sonnet xhigh | [b5 §15·§17, m4 §6] |
| W5-5 | 슬라이더 writer(type 1, 8, 17, 18, 19), 편집 텍스트 | `skin_render/text_input.rs`, `skin_host/writers.rs`, `apps/rbms-player/src/textedit.rs` | 웨이브 4 | sonnet xhigh | [b4 §3.7, m4 §9] |
| W5-6 | 선택 BGM 루프, 이동·패널 사운드, 미리듣기와의 규칙 | `apps/rbms-player/src/syssound.rs`, `stage/select/preview.rs` | 웨이브 4 | sonnet high | [b4 §1.6, 99 H14] |
| W5-7 | 시스템 오버레이 재배치: 빈 목록 안내, IR 랭킹 패널, 기록 모달, 필터 패널, 옵션 오버레이, 단축키 안내 | `stage/select/mod.rs` 의 오버레이 그리기부(W5-3a 뒤) | W5-3b | sonnet xhigh | [99 H1·H2·H5] |

### 웨이브 6 — 플레이

끝 상태: 7/5/14/10키 캡처(로딩, READY, 플레이 중, 폐점, 풀콤보)와 오토플레이 실화면.

| 단위 | 내용 | 소유 파일 | 선행 | 모델 | 근거 |
| --- | --- | --- | --- | --- | --- |
| W6-1 | note(문서 기반 레인 지오메트리, y 누적식, LN/HCN, 마디선류) | `skin_render/notes.rs` | 웨이브 5 | opus high | [b3 §3] |
| W6-2 | 커버·judge | `skin_render/{covers, judge}.rs` | 웨이브 5 | opus high | [b3 §4·§6] |
| W6-3 | 플레이 상태기계(§4.5 의 연습·분석·미리보기·재시작·직접 실행·Esc 규칙 포함) | `apps/rbms-player/src/stage/play/mod.rs`, `stage/play/tests.rs` | 웨이브 5 | opus high | [b3 §10, 99 M1·M7·H6·H7·H12·H13] |
| W6-4 | 타이머 드라이버(봄 50~, 홀드 70~, 키 on 100~/off 120~, READY 유지, RHYTHM, 143, 348~352, 908, 2)와 오프셋 1~5 | `skin_host/play_timers.rs` | 웨이브 5 | opus high | [b3 §11, b5 §16-17~21] |
| W6-5 | 호스트 군집 C·D | `skin_host/play.rs` | 웨이브 5 | sonnet xhigh | [m2 §5, m3 §5, b5 §17] |
| W6-6 | BGA 레이어 | `skin_render/bga.rs`, `apps/rbms-player/src/{stage/canvas.rs, assets.rs}` | 웨이브 5 | sonnet xhigh | [b3 §7] |
| W6-7 | timingvisualizer, hiterrorvisualizer, 슬라이더 type 4/5/6 | `skin_render/graphs/{timing_vis, hit_error}.rs` | 웨이브 5 | sonnet high | [b3 §8.3·§8.4] |
| W6-8 | 스킨 경로의 내장 `Skin`(RON) 의존 제거(내장 렌더러용 `Skin` 은 유지), 구 `state.rs`·`screen.rs` 삭제, `OFFSET_ALL`(플레이 타입에서만, 이동은 스킨 크기의 %, 배율 (w+100)/100) | `crates/rbms-render/src/{skin.rs, lib.rs, skin_render/{mod, state, screen}.rs}`, `apps/rbms-player/src/{app_input.rs, app_play.rs}` | W6-1~W6-5 | opus high | [r3 §11-4, 99 W5·P8-4] |

### 웨이브 7 — 동영상과 비트맵 폰트

끝 상태: ModernChic 의 동영상 배경·범용 BGA 와 "画像フォント" 옵션이 동작한다.

| 단위 | 내용 | 선행 | 모델 |
| --- | --- | --- | --- |
| W7-1 | 디코더 선정 조사(공식 문서: 후보의 라이선스·GPL-3.0 호환·빌드 요구·3-OS CI 영향·H.264/mp4 지원). 결과를 결정 문서에 기록 | 웨이브 6 | opus high |
| W7-2 | 동영상 source: 디코더 트레이트와 구현, 반복 재생, 프레임 업로드 | W7-1 | opus high |
| W7-3 | `.fnt`(BMFont 텍스트) 로더와 type 0 그리기 | 웨이브 6 | opus high |
| W7-4 | distance field(type 1/2) 파이프라인과 검증 방법 | W7-3 | opus high |

### 웨이브 8 — 기본 스킨, 스킨 팩, 정리

끝 상태: 첫 실행부터 자작 기본 스킨으로 전 화면이 그려진다.

| 단위 | 내용 | 선행 | 모델 |
| --- | --- | --- | --- |
| W8-1 | 스킨 팩 선택 모델과 SKIN 탭(팩 폴더, 화면별 옵션·파일·오프셋, 죽은 설정 행 표시 정리) | 웨이브 6 | opus high |
| W8-2 | 기본 스킨 자작: (가) 화면별 배치 사양서 `docs/plan` 추가 → (나) 이미지·사운드 생성 스크립트 → (다) 공통 Lua, 결정, 결과·코스 결과, 선곡, 플레이 SP(5/7/9), 플레이 DP(10/14) 문서. 화면별 하위 단위로 병렬 | 웨이브 6 | opus high(Lua), sonnet high(자산) |
| W8-3 | 내장·설치(내장 표 생성 스크립트 포함), 기본 선택값, 릴리스 워크플로 확인 | W8-2 | sonnet high |
| W8-4 | 내장 폴백 정리: 스킨 로드 실패 안내, 24키 폴백 경로 확인 | W8-3 | sonnet high |
| W8-5 | 테스트·골든 재편 | W8-4 | sonnet xhigh |
| W8-6 | 문서: `docs/skin.md` 재작성, `architecture.md`·`development.md`·`crates.md`·`README.md`·HANDOFF | W8-4 | sonnet high |

### 웨이브 9 — 후속(이번 범위 밖, 별도 지시)

IR 랭킹·라이벌의 스킨 연결, type 9 스킨 설정 Stage, customTimers/customEvents, 24키 문서, `filter != 0` 전용 bilinear 셰이더, skinpreview, 같은 디코더로 BGA 동영상.

## 7. 검증

| 수준 | 수단 |
| --- | --- |
| 단위 | 각 단위의 좁은 테스트(`cargo test -p <크레이트> <이름>`) |
| 로더 | 덤프 도구: pcall 실패 0, 헤더 표, 객체 수, 프레임 비용 |
| 화면 | 캡처 하니스 PNG 를 메인이 직접 열어 `m2`~`m5` 레이아웃 좌표와 대조. 시드 고정 |
| 흐름 | 격리 HOME 으로 실행: `HOME=<임시> ./target/release/rbms-player samples/preview-demo` |
| 저장소 게이트 | ModernChic 은 저장소 밖이라 게이트 테스트에 넣지 않는다. 게이트는 자작 픽스처로, ModernChic 은 `RBMS_SKIN_PACK` 이 있을 때만 도는 선택 테스트로 |

UI 는 실제 렌더를 확인한 뒤에만 됐다고 보고한다(`docs/feedback/2026-09-17-skin-visual-verification.md`). GPU 창 확인은 사용자 절차로 남긴다.

## 8. 게이트와 커밋

- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `git diff --check`.
- Rust 규칙: `//` 주석 금지(`//!`·`///` 영어 문서 주석만), 매직넘버 상수화, `#[allow]` 금지, `unsafe` 금지, rustfmt `max_width 160`.
- 커밋(D7): 웨이브가 게이트를 통과하면 되돌릴 수 있는 논리 단위로 선별 스테이징해 커밋하고 `origin/dev` 에 일반 push. Conventional Commits, 설명은 한국어, author 단독, AI 트레일러 금지, `git add -A` 금지, force push 금지. 하위 에이전트는 커밋하지 않는다.

## 9. 위험

| 위험 | 대응 |
| --- | --- |
| pcall 침묵 실패로 부품이 사라짐 | pcall 실패 로그와 덤프 게이트(W2-2a, W2-6) |
| 조사의 객체 수·좌표가 정적 추정 | 덤프를 기준으로 조사 문서를 고친다 |
| 텍스트 위치(libGDX 소스 부재) | 좌표 대조와 실화면에서 보정. 한계는 §1 에 명시 |
| 프레임 비용(선곡 클로저 약 1380 + 타이머 함수 약 420) | 정적 조건 제거, 타이머 함수 재사용, W2-6 실측으로 예산 확정 |
| GPU 메모리(전부 올리면 2.79 GiB, 4096 초과 8장) | 참조 소스만 로드, 이탈 해제, 한도 초과 생략 |
| 웨이브 1 철거가 테스트 약 50건을 무효화 | W1-1 을 a/b/c 로 나누고 내장 골든 불변 확인 |
| 시그니처 변경이 병렬 단위 컴파일을 깨뜨림 | 골격 단위 선행, 변경 단위는 호출부 전부 소유·직렬 |
| C 라이브러리 한 번의 호출(`table.sort`, `string.rep`, `table.concat`) 안의 시간은 명령 수 훅이 보지 못한다 | 메모리 한도로 한 번의 길이만 묶인다. 영구 정지는 아니고 벽시계 예산을 넘길 수 있다. 실제 스킨에서 문제가 되면 해당 함수를 청구형으로 바꾼다 |
| 사용량 한도 | 웨이브 단위로 PROCESS 에 진행 기록, Workflow 는 `resumeFromRunId` 로 재개 |
| 동영상 디코더가 3-OS CI 빌드를 깨뜨림 | W7-1 에서 CI 영향 확인 후 선정, 기능 플래그 뒤에 둔다 |

## 10. 자동 시작 절차 (예약 실행 또는 새 세션)

1. `docs/PROCESS.md` 의 L 체크리스트에서 첫 미완료 항목을 확인한다.
2. `git status` 로 작업 트리가 깨끗한지 본다. 미커밋 변경이 있으면 내용을 확인해 이전 단위의 잔여인지 판단한다.
3. 해당 웨이브의 표를 읽고, 각 단위의 "근거" 절만 `docs/reference/skin-compat/` 에서 읽어 위임 지시를 쓴다(원본 통독 금지). 소유 파일은 지시를 쓰기 전에 `ls`/`grep` 으로 실재를 확인한다.
4. Workflow 를 실행한다: 골격(직렬) → 구현(의존 순서, 독립 단위 병렬) → 적대 리뷰(Opus high, 조사 문서 절과 대조) → 수정. 모든 `agent()` 에 `model` 과 `effort` 를 명시한다.
5. 메인이 게이트와 캡처를 직접 확인하고, 조사 문서·PROCESS 를 갱신한 뒤 커밋·푸시한다.
6. 다음 웨이브로 넘어간다. 사양에 없는 결정이 필요할 때만 멈춰 묻는다.
