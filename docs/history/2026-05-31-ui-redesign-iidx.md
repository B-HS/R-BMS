# UI 재설계 (IIDX/LR2 지향) + 폴더 영속·백그라운드 스캔 (2026-05-31)

> 사용자 피드백(플레이/결과 화면이 MVP 같음, 노트 팝인, 그래프 없음, 폴더 미저장/프리즈) 대응. 레퍼런스는 `docs/reference/ui/`(제공 4장 + 수집 18장), 디자인 스펙은 `docs/reference/ui-design.md`(IIDX/LR2/beatoraja 레이아웃 리서치).

## 버그 수정
- **#1 선택 폴더 영속** — `PlaySettings.songs_folder` 추가. `App::new`의 폴더 시작·`O` picker가 폴더를 저장하고, `main()`이 **인자 > 저장폴더 > `$RBMS_SONGS`** 순으로 복원. `start.sh` 무인자 시 positional 제거(저장폴더 사용, RBMS_SONGS는 첫 실행 폴백). 검증: 폴더 실행→settings.ron 저장→무인자 재실행 복원.
- **#2 폴더 스캔 프리즈** — `O` picker의 재귀 스캔+테이블 fetch를 `std::thread`로 분리, 메인은 `scan_rx` 채널을 매 프레임 폴링하며 **애니메이션 LOADING**(점멸 점 + 핑퐁 스윕 바) 렌더. `ScanOutcome`(songs/names/levels, Send) 전달. Esc 취소 시 `scan_rx` 드롭.

## 화면 재설계 (1280×720, rect+text)
- **#3 노트 팝인 → 프레임 상단 클리핑(근본 수정)** — 원인은 노트가 필드 상단 경계에서 클리핑되지 않고 통째로 그려져 위로 삐져나오던 것. `playfield.rs`에서 **노트 사각형을 `[top_y, judge_y]`에 클리핑**(상단 초과분은 보이는 만큼만 그림)해 프레임 상단에서 자연스럽게 흘러나오게 함. `top_y` 28→**60**(필드를 프레임화). *(초기엔 불투명 베젤로 가렸으나 hack이라 제거하고 클리핑으로 근본 수정 — CLAUDE.md 원칙.)*
- **#4 플레이 IIDX 레이아웃 + 라이브 그래프** — 좌측 정보(EX/BEST/GREEN), 중앙 필드(베젤), **중앙 라이브 스코어 그래프**(`draw_score_graph`: A/AA/AAA 9분 임계 가로선 + NOW(cyan)/BEST(green) 막대 + `vs BEST ±Δ`), 우측 판정카운트(BGA 위), 하단 가로 게이지. `HudView`에 `max_ex`/`best_ex`. 스킨 레이아웃을 필드 중앙좌·BGA 우(905,360)로 이동.
- **#5 결과 IIDX 레이아웃** — 좌측 거대 **DJ LEVEL**(s7.0, 등급색) + EX% + CLEAR 램프 바 + 랭크바 + ΔBEST/ΔPREV/NEW RECORD, 우측 **스코어 리포트**(EX/MAX COMBO/TOTAL NOTES + PGREAT **핫핑크**/GREAT/GOOD/BAD/POOR/MISS 카운트+막대 + FAST/SLOW + GAUGE). `ResultView`에 title/fast/slow. 상단 곡 타이틀 헤더.

## 검증
- `cargo test --workspace` 123 통과·0 경고.
- 렌더 PNG: 플레이(베젤 emerge + 중앙 그래프 + 좌우 HUD), 결과(AAA + 리포트). 폴더 영속 스모크, 플레이/리플레이 GUI 구동 무패닉.
- 적대적 리뷰로 신규 로직(백그라운드 스레드·레이아웃 math·DP 충돌) 검증.

## 곡선택 상세 패널 고도화 (bms-rs 파싱 참조)
`/Users/hyunseokbyun/bms-rs`(BMS 파서 1.0)의 `MusicInfo`(genre/title/subtitle/artist/sub_artist/**maker**/comment/preview)·`Metadata`를 참조해 노출 메타를 확장.
- 파서: `#MAKER` 추가(`Headers.maker`).
- `SongEntry`: 스캔 시 subtitle/artist/genre/maker/difficulty/init_bpm/rank/total 캐싱(전부 lexical, 저비용).
- 포커스 곡 한정 **lazy 계산**(`compute_chart_detail`→`to_model`): playable NOTES·LONG NOTE 수·길이(m:ss)·BPM min–max. `focused_detail` 캐시는 포커스가 다른 곡으로 이동할 때만 1곡 파싱(스크롤 비용 최소).
- 상세 패널: 타이틀/부제/아티스트/(장르·제작자) + 2열 스탯 그리드(BPM 범위·DIFFICULTY명·NOTES(+LN)·JUDGE(#RANK명+%)·LENGTH·TOTAL) + RECORDS(베스트램프·랭크바·이력)로 재구성.

## 후속 / 주의
- DP(14K) 듀얼필드는 새 IIDX 레이아웃과 폭이 겹칠 수 있어 별도 좌측 앵커 처리(아래 적대적 리뷰 반영).
- 곡선택(#21) 세부 폴리시(행별 클리어램프 LED 등)는 점진 진행. BGA는 정적/이미지 프레임(동영상 미지원).
