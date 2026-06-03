# 2026-05-31 — 폰트 / HUD / GUI (곡선택·설정) + 레이아웃 수정

사용자 피드백("콤보·Fast/Slow·판정·카운트 안 보임", "설정·곡선택 GUI로", "BGA가 레인 뒤에 가려짐", "저 빨강은 뭐냐") 반영.

## 폰트 (rbms-render::font)
- 5x7 비트맵 글리프(0-9, A-Z, 기호) + `draw_text`/`draw_text_centered`/`draw_text_right`/`text_width`. 이진 리터럴로 가독성. 화면 텍스트의 전제.
- CpuCanvas에 **알파 블렌딩**(src-over) 추가 → GPU 표시와 레퍼런스 일치(반투명 레인).

## 플레이 HUD (rbms-render::hud)
- JudgeEngine에 `last_judge`/`last_fast`/`fast`/`slow` 추가(press/release에서 기록).
- `HudView` + `render_hud`: EX스코어, **콤보(대형)**, **직전 판정(PERFECT/GREAT/...)**, **FAST/SLOW**, **판정 누적 카운트(PG/GR/GD/BD/PR/MS)**. 스킨 기하 기반 배치.

## 결과 화면 텍스트
- render_result에 폰트 적용: 클리어램프 배너 큰 글자 + 판정별 라벨·카운트 + EX/콤보/게이지 수치.

## GUI 곡 선택
- 선택 화면에 **제목·모드·레벨** 텍스트(현재 선택 하이라이트, 스크롤). SongEntry에 level 추가. 헤더 "SELECT", "TAB SETTINGS" 힌트.

## GUI 설정 메뉴 (Stage::Settings, Tab 진입)
- AUTOPLAY / HI-SPEED / LIFT / GAUGE / SCRATCH SIDE / SCRATCH AUTO 를 ↑↓ 이동, ←→ 변경. HI-SPEED는 라이브, 나머지는 다음 곡부터.

## 레이아웃/표시 수정
- 기본 스킨을 **좌측 좁은 필드 + 우측 BGA 박스(레인 밖)** + 반투명 레인(alpha 160)으로 변경(내장 default + default.ron 동일). BGA가 더 이상 노트에 가려지지 않음.
- 서버 표시등(초록/빨강)은 **`--server` 줄 때만** 표시(평소엔 숨김).

## 누적
워크스페이스 **66 테스트, 0 실패**. PNG 검증: HUD(콤보 123·PERFECT·카운트), 결과화면(텍스트).
