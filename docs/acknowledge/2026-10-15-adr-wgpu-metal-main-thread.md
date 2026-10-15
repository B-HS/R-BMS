# ADR: wgpu Metal 의 메인 스레드 위반은 구조를 유지하고 알려진 위험으로 둔다(THR2-1)

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(THR2-1 의 A) · 문서 커밋 `afa1e2f`(구조는 `794361b`)

## 배경

[키보드 시각 ADR](2026-10-15-adr-keyboard-input-timing.md) 결정 1 은 화면 대기(`Surface::get_current_texture`)를 전용 스레드 `rbms-present` 로 뗐다. S19 2차 감사(THR2-1)가 macOS 에서 Xcode Main Thread Checker 로 위반을 재현했다. wgpu-hal 29.0.4 의 Metal 표면 획득이 occlusion 검사로 메인 스레드 전용 AppKit API `NSView.window`·`NSWindow.occlusionState` 를 그 스레드에서 읽는다(`metal/surface.rs:121-156`, 30.0.1 에도 같은 코드). 호출 지점마다 한 번 보고되고 코드 경로는 이미지마다 실행된다. 크래시는 재현되지 않았다.

## 결정

1. present 전용 스레드 구조를 그대로 둔다.
2. 이 위반을 알려진 위험으로 기록한다(읽기만 하는 정의되지 않은 동작. 메인이 같은 순간 창을 바꾸는 드문 경우, 곧 전체 화면 전환·창 닫힘에만 의미가 있다). 실기 확인 항목 K8(Main Thread Checker 로 이 두 건만 나오는지)을 둔다.
3. 상류 수정을 기다린다: https://github.com/gfx-rs/wgpu/issues/10627 (`bug` 라벨, objc2 `MainThreadMarker` 관련 답변 1건). wgpu 를 올릴 때 이 이슈가 고쳐졌는지 확인하고 K8 을 다시 돈다.

## 이유

- 위반은 읽기 두 건이고 크래시가 재현되지 않았다. 되돌리면 키보드 판정 시각의 개선(하네스 p95 8.0 ms → 0.04 ms)을 잃는다.
- 프로젝트 안에서 고칠 수 있는 길은 모두 의존성 사본이나 구조 후퇴다(아래).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| macOS 에서는 표면 획득을 메인 스레드로 되돌린다(B) | macOS 에서 키보드 판정 시각 개선이 사라진다(present 대기 동안 키를 받지 못함) |
| wgpu 를 패치한 사본을 저장소에 둔다(C) | 의존성 사본 유지 부담, 업그레이드마다 다시 맞춰야 한다 |

## 영향과 관련 파일

- `apps/rbms-player/src/gpu/present.rs`(`Presenter`, 스레드 이름 `rbms-present`), `Cargo.lock`(wgpu·wgpu-hal 29.0.4).
- 위험 서술의 정본: [키보드 시각 ADR](2026-10-15-adr-keyboard-input-timing.md) 남은 한계 6. 확인 절차: `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 의 K8.
- 상류 이슈는 메인이 사용자에게 초안을 보이지 않고 올렸다(지적받음, [피드백](../feedback/2026-10-15-confirm-before-external-posts.md)). 사용자 결정으로 이슈는 그대로 둔다.

## 날짜와 사용자 원문

2026-10-15. THR2-1 에 "A"(구조 유지, ADR·QA 에 알려진 위험으로 기록, 상류 이슈). 이슈 게시에 대한 원문: "뭐야 이슈를 왜 멋대로 올린거야". 이슈를 그대로 두는 것도 사용자 결정(원문은 저장소에 남아 있지 않다).
