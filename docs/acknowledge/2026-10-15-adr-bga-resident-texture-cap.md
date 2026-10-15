# ADR: BGA 상주 텍스처 상한은 256 MiB 로 둔다(메모리보다 플레이 프레임)

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(추천안 A, "둘다 A" 의 1) · 커밋 `4cc72d0`(상주·미리 칠하기), `1676c57`(사본 중복 제거·계획), `c81eda1`(연습 구간)

## 배경

S19 2차 감사 R2-PERF-2: BGA 그림이 바뀌는 프레임이 그림을 복사·검정 키아웃·전체 업로드해 1 ms 가까이 썼다. 수리로 곡이 시작되기 전에 차트가 보이는 그림을 텍스처로 올려 두고(상주), 그 밖의 그림은 디코드 작업자가 미리 칠해 둔다. 그 대가로 메모리가 늘었다. 그림이 많은 차트(YATA)에서 phys_footprint 가 1,142 MB(`a13f6ce`, 상주 전)에서 1,503 MB 가 됐고, 그중 graphics +271 MB 가 상주 텍스처다(측정: `docs/reference/frame-cost.md`). 메인이 상한을 낮출지 사용자에게 물었다.

## 결정

1. 상주 텍스처 상한 `RESIDENT_BYTES` 는 256 MiB 로 둔다(1024 x 1024 그림 64장, 256 x 256 그림 천 장쯤). 미리 칠하기 예산 `PREPARED_BGA_BYTES` 도 256 MiB 다.
2. 상한 밖의 그림만 보일 때 그 프레임이 올린다(미리 칠한 픽셀이 있으면 칠하지 않는다).
3. 어느 모양을 상주로 올리고 어느 모양을 미리 칠할지는 디코드 전에 그림 크기로 한 번 정한다(`plan_uploads_ahead`). 연습 구간은 구간 자신의 순서로 올린다(`BgaPreloadOrder::Part`).

## 이유

- 플레이 프레임의 p99 를 메모리보다 먼저 지킨다([렉 기준 ADR](2026-10-14-adr-play-hitch-criteria.md)). YATA 의 플레이 프레임 p99 는 상주로 약 0.14~0.16 ms 에 머문다(`frame-cost.md` 의 표).
- 상한을 낮추면 그림이 많은 차트일수록 플레이 중에 올리는 그림이 다시 생긴다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 상주 상한을 낮춘다(B) | 메모리는 줄지만 그림이 많은 차트에서 플레이 프레임의 업로드가 돌아온다. 사용자가 A 를 택함 |
| 상주한 그림의 CPU 원본을 버리고 연습 재진입·BGA 다시 켬 때 다시 디코드한다 | 다시 디코드하는 경로가 새로 필요하다. 이번 범위 밖(`frame-cost.md`) |

## 영향과 관련 파일

- `crates/rbms-render/src/skin_render/bga/pictures.rs`(`RESIDENT_BYTES`, `BgaTextures::preload`, `BgaPreloadOrder`, 모듈 문서), `apps/rbms-player/src/assets.rs`(`PREPARED_BGA_BYTES`, `plan_uploads_ahead`), `apps/rbms-player/src/stage/play/mod.rs`(연습의 preload 순서).
- 측정과 설계의 정본: `docs/reference/frame-cost.md` 의 BGA 절, 테스트 부채 3.40.

## 날짜와 사용자 원문

2026-10-15. "둘다 A"(1: BGA 상주 텍스처 상한 256 MiB 유지. 2 는 아날로그 턴테이블 디바운스로 [키보드 시각 ADR](2026-10-15-adr-keyboard-input-timing.md) 의 "확인받은 것").
