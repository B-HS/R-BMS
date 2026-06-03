# 2026-05-31 — 키 빔 · 모드별 키맵 · 720p 랜드스케이프 · 재귀 곡선택 · 난이도표 · 로딩/속도조절

사용자 요청 7건을 한 세션에서 구현·검증. beatoraja 원본 메커닉 리서치(병렬 워크플로) + 적대적 멀티에이전트 리뷰(7결함 확정·전부 수정).

## 대상 파일
- `crates/rbms-play/src/lib.rs` — 키 빔 상태머신, 짝없는 LN 처리.
- `crates/rbms-render/src/{skin,playfield,hud,result}.rs` — 빔 렌더, 1280×720 랜드스케이프.
- `crates/rbms-table/` (신규) — BMS 난이도표 데이터레이어.
- `apps/rbms-player/src/main.rs` — 해상도, 모드별 키맵, 폴더/곡 네비게이션, 난이도표 연동, 로딩 화면, 속도 조절.
- `assets/skins/default.ron`, `Cargo.toml`(×2) — 랜드스케이프 스킨, 크레이트 배선.

## 리포트 (무엇을·왜)

### a. 키 빔
beatoraja는 키 빔을 스킨 객체로 두고 엔진은 per-lane keyon/keyoff timer(µs 타임스탬프)만 토글한다(KeyInputProccessor/SkinObject). autoplay는 탭=80ms 플래시(`auto_minduration`), LN=hold. 이를 그대로 포팅: `Player`에 `beam_on/beam_off/ln_active`(`i64::MIN`=off). press=on·release=fade, autoplay 액션 루프에서 set, 탭은 80ms 후 auto-clear(`!ln_active` 게이트, interactive 레인은 미적용). 렌더는 `render_playfield`가 레인배경↔노트 사이에 판정선→위로 6슬라이스 그라데이션 빔(데이터주도: `SkinConfig.beam_color/alpha/height_frac`). 프레임레이트 독립(µs 기반).

### b. 모드별 키맵
기존 7K 단일 하드코딩 → `default_keys_for_mode(Mode)`로 5K/7K/9K(PMS)/10K/14K 프리셋(사용자 결정: **beatoraja Z열**). 차트 mode 자동감지로 `App.active_keys` 선택, `--keys` 오버라이드 유지. 14K는 rbms 실제 모델(BEAT7 table) 기준 P1 키=lane0-6/스크7, **P2 키=lane8-14/스크15**(좌손/우손). `key_from_name`에 QUOTE/CONTROL 추가.

### c. 1280×720 16:9 랜드스케이프
레퍼런스 공간을 480×640 포트레이트 → 1280×720으로(GPU 유니폼이 `(CW,CH)`를 NDC로 매핑하므로 상수+좌표만 변경). `SkinConfig`/`default.ron` 랜드스케이프 기본값: 필드 center-left(`field_x` 0.05/`field_width` 0.28 = 폭 비율), `judge_y` 672(하단), BGA 우측 512² 정사각(텍스처 정사각이라 비정사각 시 스트레치). HUD 카운트패널을 BGA 우측으로, Select(리스트+우측 정보패널)/Settings(중앙 컬럼)/Result(중앙 컬럼) 재배치.

### e. 재귀 곡선택 + 네비게이션
`scan_folder`는 이미 재귀(727곡 확인). 곡당 `md5` 저장. 선택화면을 폴더/곡 모델로: `SelectView`(Root→ALL SONGS / TABLE→레벨→차트) + `SelectItem`(Folder/Song). Esc=상위/종료, Enter=열기/플레이.

### f. 난이도표(커스텀 폴더)
신규 `rbms-table` 크레이트(rbms-ir 패턴, reqwest blocking). `DifficultyTable{name,symbol,level_order,entries}` + `fetch(url)`(응답이 배열=body / 객체=header 자동판별, header면 `data_url` 추적, body면 sibling `header.json` best-effort) + `fetch_or_cache`(엔벨로프 디스크 캐시) + `by_level()`. `--table <url>`로 활성, **md5로 로컬 차트 매칭**해 level별 폴더(`LV n`). 폰트가 ASCII뿐이라 표명/★은 회피 라벨.

### g. 로딩 대기 + 타이밍 버그 수정
`Stage::Loading`: Select Enter→LOADING 화면 1프레임 표시→다음 프레임 블로킹 로드→Play(2프레임 지연 로드). **핵심 버그**: 기존 `anchor_us`(song_us 기준선)를 키음 로드 직후·BGA 로드 **전**에 캡처해, BGA 로딩 시간만큼 곡이 앞서 시작(초반 노트/키음 스킵)했다. → load() 끝에서 `self.audio.clock_us()`로 캡처하도록 이동.

### h. 게임 중 속도 조절
Play 중 ↑/↓로 `config.hispeed` ±0.25(clamp 0.5~10.0). `render_playfield`가 매 프레임 그 값을 쓰므로 즉시 반영. autoplay·interactive 공통, 레인키(문자/Shift)와 무충돌.

## 적대적 리뷰 — 확정 7결함 (전부 수정)
1. (HIGH) `join_url`이 루트상대(`/abs`)·`..` 오처리 → `reqwest::Url::parse(base).join(rel)` 리졸버로 교체(+테스트 수정).
2. (MED) 짝없는 LongStart가 빔/`ln_active` 영구 고착 → `collect_actions`가 LongEnd 매칭 시에만 head Press+Release 커밋(`from_model`과 일치), 미종료 LN은 드롭.
3. (MED) 인터랙티브 키 안내문이 실제 키맵과 불일치 → `key_name` 헬퍼로 `active_keys`에서 생성.
4. (MED) 높은 lift에서 HUD 텍스트가 화면 위로 이탈 → `(jy - off).max(top_y)` 클램프.
5. (LOW) `derive_level_order` 중복 레벨 → `dedup_by(level_key)`.
6. (LOW) 오프라인 캐시가 헤더 메타 유실 → `CachedTable` 엔벨로프(name/symbol/level_order/entries).
7. (LOW) 빈 배열 캐시가 장애 은폐 → 비어있으면 캐시 미기록.

## 검증
- `cargo test --workspace --exclude rbms-player` = **75 통과**(빔 +4, table +4).
- release/debug 빌드 OK. render_frame(7K·9K)·render_result 예제 1280×720 갱신, `table_probe` 예제 추가.
- 시각: render_frame PNG로 7K·9K 레이아웃+빔 확인.
- 실측: 발광1 표 fetch(1035개, header 자동발견 symbol=★) → 로컬 727곡 중 **86곡/18레벨 매칭**. 단곡 autoplay 라이브 구동 패닉 없음.

### i. 통합 키 설정(파일 + 인앱 에디터) + 풍부한 조작 키
사용자 요청: "속도조절 등 모든 (레인 외) 조작 키가 사용자 지정이어야 한다", 방식=파일+에디터 둘 다, 조작=beatoraja식 풍부하게. 신규 `apps/rbms-player/src/keyconfig.rs`: `KeyConfig{lanes(모드별 Vec<String>), controls: ControlBinds}` RON 직렬화 + `key_from_name`/`key_name`(라운드트립, 화살표·브래킷 추가) + `default_keys_for_mode`/`mode_config_key`(모드 **name** 기준 디스패치) + `lane_keys`(누락/오타 레인을 기본값으로 백필) + `control_key`/`set_*`/`collisions`. 
- **파일**: `~/.config/rbms/keyconfig.ron` (없으면 자동생성, `--keyconfig`로 경로 지정). CLI `--keys`는 레인 오버라이드.
- **인앱 에디터**: `Stage::KeyConfig`(설정 Tab→`KEY CONFIG`). EDIT MODE 전환(←/→) + 컨트롤/레인 행, Enter=키 캡처 재바인딩, 충돌 시 거부+빨강 DUP 표시, Esc=저장 후 복귀.
- **조작 액션**(설정키, 기본): hi-speed ±(↑/↓), lane cover/sudden ±(→/←, `render_lane_cover` 상단 차폐), lift ±(]/[, skin 재빌드). `control_for`→`apply_control`, control이 lane보다 우선(early-return).

## 적대적 리뷰 2차 — 키설정/조작 확정 9결함 (전부 수정)
1. (HIGH) ControlBinds에 `#[serde(default)]` 누락 → 부분/오타 controls 블록이 전체 파싱 실패→전 설정 유실. 구조체에 `#[serde(default)]` 추가.
2. (HIGH) load() 파싱 실패 시 손상 파일 미보존 → 다음 save가 덮어씀(유실). Err 분기에서 `.ron.bak`로 백업 후 기본값.
3. (MED) `lane_keys`가 짧은/오타 행을 폴백 안 함 → 죽은 레인. 누락·무효 레인을 기본값으로 백필 + 경고.
4·5·6. (MED) 에디터가 충돌 바인딩 허용(control↔lane, lane↔lane) → control 우선이라 죽은 레인. `binding_collides` 캡처 시 거부 + `collisions` 빨강 표시 + 경고 힌트.
7. (LOW) `default_keys_for_mode`/`mode_config_key`가 `mode.key`(레인수)로 디스패치 → 향후 키수 충돌 위험. `mode.name`(식별자) 기준으로.
8. (LOW) `save()`가 쓰기 오류 무시 → 저장 실패 무신호. write/create_dir_all 오류 보고.
9. (LOW) Esc-저장 시 active_keys를 잘못된 모드로 갱신(죽은 코드). 제거(load()가 권위).
10. (LOW) cover/lift 클램프 불일치(apply 0.8/0.9 vs render 0.95/skin 0.9, `--lift` 미클램프). 0.9로 통일 + `--lift` 클램프.
검증: 81 테스트(키설정 회귀 +3: 부분파싱 생존·짧은행 백필·충돌). 손상 keyconfig → `.bak` 백업 실측. release/debug 빌드.

## 후속 후보
CJK 폰트, 14K 듀얼필드 렌더, 윈도우 리사이즈 리플로우, 다중 난이도표, lane cover green-number 재계산. (PROCESS §6)
