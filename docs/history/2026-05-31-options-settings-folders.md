# 2026-05-31 — 노트옵션 · 설정영속 · 게이지 · BGA토글 · 하이스피드고정 · 오프셋 · 폴더선택

3차 배치(사용자 요청 7+세부). 초기 git 커밋(`d9dcb9d`, co-author 없이) 후 진행. 9개 중 7개 완료, r(난이도표 인앱관리)·p(리플레이)는 다음 증분.

## 완료
- **j. 노트 옵션** — `crates/rbms-chart/src/shuffle.rs`: `NoteOption`(Off/Mirror/Random/SRandom/RRandom/Rotate) + xorshift64 시드 RNG. 사이드별 레인 퍼뮤테이션(DP는 P1/P2 분리, 교차 금지), 스크래치 제외, LN 헤드/테일 동일 레인 유지, hidden 레이어 동반 매핑. 로드 시 `apply(model, random, seed)`. 시드는 SystemTime, 리플레이용으로 보관(`App.seed`).
- **k. 설정 영속** — `apps/rbms-player/src/settings.rs` + `~/.config/rbms/settings.ron`. 우선순위 defaults→settings→CLI(`main()`이 apply_settings 후 CLI 파싱). Settings 화면 나갈 때(Esc/Tab/Enter) 저장. gauge는 `gauge_token`(canonical)으로 round-trip.
- **l. 게이지 표시** — `HudView.gauge` + `render_hud` 좌측 세로 게이지 바(값 색상: ≥80 green/≥20 yellow/else red, 80% 클리어 틱).
- **m. BGA on/off** — `config.bga`, 로드(이미지 디코드)·렌더 양쪽 게이트.
- **n. 하이스피드 고정** — `scroll::constant_offsets`(시간선형, BPM 독립) + `render_playfield(.., constant)`. 설정 `SPEED FIX` FLOATING/CONSTANT. scroll/stop 미반영 단순 버전.
- **o. 판정 오프셋** — `config.offset_ms`(±200, 설정 ±5ms 스텝) → 인터랙티브 판정 시각(`song+offset_us`)에만 적용(렌더/오토플레이 불변).
- **q. 폴더 선택** — `rfd` 의존성. Select에서 `O`키 → 네이티브 폴더 다이얼로그 → 재스캔 + 난이도표 재매칭 + Root 리셋.

설정 UI 12항목(AUTOPLAY/HI-SPEED/SPEED FIX/RANDOM/GAUGE/LIFT/LANE COVER/SCRATCH SIDE/SCRATCH AUTO/JUDGE OFFSET/BGA/KEY CONFIG). CLI 추가: `--auto`.

## 적대적 리뷰 — 확정 5결함 (전부 수정)
1. (HIGH) 선택한 RANDOM이 스코어 제출 시 항상 Off로 보고 → `ir_random(NoteOption)→rbms_ir::RandomOption` 매핑(Rotate→Spiral).
2. (MED) DP(10K/14K) 셔플이 P1↔P2 교차 → `side_key_lanes`로 사이드별 퍼뮤테이션(beatoraja getKeys 모델). 회귀 테스트.
3·5. (MED/LOW) 잘못된/누락 `--hispeed`·`--lift`가 저장값을 하드코딩 기본으로 덮고 `--hispeed` 미클램프 → 파싱 성공 시에만 적용 + clamp(0.5~10.0 / 0~0.9).
4. (LOW) hidden 레이어(채널 3x)가 셔플에 미반영 → `remap_lanes`로 notes와 동일 매핑(현재 잠재적, play가 hidden 미사용).

## 검증
- 88 테스트(노트옵션 +6, constant +1, DP교차 +1). release/debug 빌드, 무경고.
- render_frame PNG로 게이지 바·레이아웃 확인. rfd 폴더 다이얼로그 빌드 통과.

## 완료 (이어서, 2차 증분)
- **p. 리플레이 저장·재생** — `apps/rbms-player/src/replay.rs`. 인터랙티브 입력(레인·raw시각) 기록 → 결과 시 `~/.config/rbms/replays/<md5>-<ts>.ron` 저장(chart_path·md5·mode·random·seed·offset·scratch_auto·gauge·events). `--replay file`로 재생: 시드/옵션/게이지/스크래치오토 복원해 동일 셔플·판정, `feed_replay`로 입력 주입, 키보드 무시. 차트 누락 시 우아한 처리(패닉 X), md5 불일치 경고.
- **r. 난이도표 인앱 관리** — `apps/rbms-player/src/tables.rs` + `~/.config/rbms/tables.ron`. 단일표→**다중표**(SelectView Root→TableLevels(ti)→TableLevel(ti,li)→차트, sources/names/levels 평행 정렬). `T`키 `Stage::Tables`(목록 + ADD URL 텍스트입력 + ADD FILE rfd + `D` 제거), 추가/제거는 **증분**(변경된 표만 fetch). URL/로컬 data.json 모두 지원.

## 적대적 리뷰 2회차 — 확정 11결함 (5+6, 전부 수정)
1차(옵션/설정): RANDOM 스코어제출 누락(ir_random), DP교차셔플(사이드분리), --hispeed/--lift 저장값 보존+클램프, hidden 미리매핑.
2차(리플레이/표): (HIGH) 이동된 차트 리플레이 패닉→load() 폴백(Result)+md5경고, (MED) scratch_auto·gauge 리플레이 미저장→저장·복원, (MED) 표 추가/제거 시 전체 재fetch UI멈춤→증분, (LOW) 리플레이 시 config.random 미동기화→IR 오보고, (LOW) self.replay 미해제→to_select_or_exit에서 해제.

## 검증
- **88 테스트**, release/debug 빌드 무경고. 없는 리플레이·이동 차트 우아한 종료(패닉 X) 실측. 다중표 라이브(--table 86곡/18레벨).

## 후속 후보
노트옵션 ALL-SCRATCH/H-RANDOM, lane cover green-number 재계산, 표 fetch 비동기(현재 추가 시 1개만 동기 fetch).
