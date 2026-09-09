# 2026-05-31 — 확장 기능 8종 (모드/게이지/LN/결과/GPU/폴더/키/SC)

플레이 모듈 위에 사용자 로드맵 8개를 구현·검증. 워크스페이스 62 테스트.

## 1. 모드 자동 감지 (rbms-chart::detect_mode)
- 사용 채널(P2 21-29, keys6/7 '18'/'19') + 확장자(.pms)로 BEAT_5K/7K/14K/POPN_9K 판별. 앱·cli·예제 연결.
- 검증: 5a→5K, 7a→7K, 14keys→14K(1222노트), .pms→POPN_9K.

## 2. 게이지/클리어 (rbms-judge::gauge)
- 레퍼런스 구현 GrooveGauge 포팅: 6종(ASSIST_EASY/EASY/NORMAL/HARD/EXHARD/HAZARD), init/border/min/max, TOTAL·LIMIT_INCREMENT 모디파이어, HARD guts. 델타[6]=판정 인덱스. `clear_lamp`(Failed/Easy/Normal/Hard/ExHard/FullCombo/Perfect/Max).
- JudgeEngine에 gauge 통합(apply/miss마다 update). 검증: autoplay→100% NORMAL→Max, hard 미스 드레인, 미클리어 Failed.

## 3. LN 풀 판정 (rbms-judge::matcher)
- LN을 (head, end) 쌍으로. press(head)→hold(head_judge 저장, 미카운트), release(end)→LN-end 윈도우 판정→worse(head,end) 1회 카운트. 패시브: margin 경과 finalize / 미입력 miss. SEVENKEY_LN_END 윈도우.
- rbms-play: autoplay 액션(press+release), interactive press/release. 검증: LN 1판정, 실차트 25곡 autoplay PERFECT(LN 포함).

## 4. 결과 화면 (rbms-render::result + 앱 Stage)
- ResultView(판정6·EX·콤보·게이지·클리어램프). 바 기반 렌더(폰트 후속). 앱 Play→Result 전환(곡 종료+2s). stdout에 정확 수치.

## 5. 네이티브 GPU (apps/rbms-player Gpu)
- CPU캔버스 블릿 → wgpu 인스턴스드 쿼드(fill_rect=1 인스턴스, 노트필드 1 draw call). `Renderer` trait를 GPU가 직접 구현. 알파블렌딩. 검증: 라이브 실행.

## 6. 폴더/곡 선택 (앱 Stage::Select)
- 폴더 인자→재귀 스캔(차트+타이틀+모드)→정렬 목록. ↑/↓ 네비, Enter 플레이, Esc 복귀, 곡 종료→Select. 검증: FELYS 폴더 26차트 스캔.

## 7. 키 설정 (PlayerConfig.keys + --keys)
- 레인↔키 바인딩 config(기본 S D F Space J K L A). CLI `--keys S,D,F,SPACE,J,K,L,A`. key_from_name(A-Z/숫자/SPACE/SHIFT/기호).

## 8. SC 위치/auto/판정선 (PlayerConfig + LaneLayout)
- `--sc-left`(스크래치 좌측 레이아웃 재배치), `--sc-auto`(스크래치 레인 자동판정·입력무시, Player.set_auto_lanes), `--lift F`(판정선 상승, LaneLayout.with_options). `--hispeed F`, `--gauge`. 검증: SC-좌+lift 프레임 PNG, SC-auto 헤드리스 테스트.

## CLI
`rbms-player <chart|folder> [--interactive] [--sc-left] [--sc-auto] [--lift F] [--hispeed F] [--gauge normal|easy|hard|exhard|assist|hazard] [--keys ...]`

## 후속(미구현)
- 비트맵/CJK 폰트(타이틀·수치 화면표시), CN/HCN 구분·HCN 게이지틱(차트 LnKind 구분 필요), 스크래치 회전(2키) 입력, 키 설정 파일, 리플레이/IR, BGA.
