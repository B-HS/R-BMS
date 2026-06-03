# 2026-05-31 — 스킨 추상화 / BGA / IR-슈퍼셋 인터페이스

## 9. 스킨 추상화 (rbms-render::skin)
- 데이터 주도 `SkinConfig`(serde, RON 로드) → 컴파일된 `Skin`(레인 x/w, 판정선, 노트/레인/판정선 색, BGA 영역). `LaneLayout` 대체.
- render_playfield가 Skin 소비. 앱 `--skin file.ron` + scratch_left/lift override. 샘플 `assets/skins/default.ron`(BGA 영역 포함).
- 검증: 5 render 테스트(RON 부분파싱·scratch 좌측 재배치 포함), `--skin` 실차트 로드.

## 10. BGA (이미지)
- `image`(png/bmp/jpeg)로 #BMP 디코드→256×256 RGBA. BGA 채널(04) 타임라인으로 현재 프레임 추적.
- GPU에 **텍스처드 쿼드 파이프라인**(BGA 전용 셰이더+텍스처+uniform) 추가, 노트 앞에 스킨 BGA 영역으로 렌더. set_bga/clear_bga.
- 검증: FELYS 255 BGA 이미지 로드·렌더, 패닉 없음. (비디오 mpg는 후속.)

## 11. IR-슈퍼셋 인터페이스 (rbms-ir) — 백엔드는 추후
- **인터페이스만**(사용자 방침): IR 슈퍼셋 DTO(serde) + `ScoreServer` 트레이트 + `HttpScoreServer`(reqwest blocking) + `NullScoreServer`. MD5+SHA-256, fast/slow, 리플레이, 코스, capability, 모든 DTO `extra` 확장점.
- 앱: `--server <url>` `--player <id>`. 백그라운드 5초 `/health` → **초록/빨강 연결 표시**(우상단). 결과 진입 시 비차단 `/scores` 제출.
- API 계약 문서: docs/reference/ir-api.md (백엔드 구현용 엔드포인트·스키마).
- 검증: JSON 왕복 테스트, **파이썬 목 서버로 health+submit 왕복**(EX 1488 수신, accepted/rank 반환), 죽은 URL→graceful red.

## 누적
워크스페이스 **66 테스트, 0 실패**. 크레이트 9개(+rbms-ir). 통합 CLI:
`rbms-player <chart|folder> [--interactive] [--sc-left] [--sc-auto] [--lift F] [--hispeed F] [--gauge X] [--keys ...] [--skin file.ron] [--server url] [--player id]`
