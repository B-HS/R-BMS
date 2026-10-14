# ADR: 릴리스 빌드의 Rust 툴체인을 rust-toolchain.toml 과 같은 1.95.0 으로 고정

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-12 · 결정자 메인(첫 릴리스 실패의 수정, 사용자에게 보고)

## 배경

첫 릴리스 실행(2026-10-12, 태그 `v0.1.0` = `ef75bbc`, run 38222704409)이 macOS 의 `Build universal binary` 에서 `error[E0463]: can't find crate for core`(`x86_64-apple-darwin`)로 실패했다. 워크플로는 `dtolnay/rust-toolchain@stable` 에 두 타깃을 설치했지만 cargo 는 저장소의 `rust-toolchain.toml`(`channel = "1.95.0"`)을 따라 1.95.0 으로 빌드했고, 그 툴체인에는 `x86_64-apple-darwin` 타깃이 없었다.

## 결정

- `release.yml` 의 세 잡(version, macOS, Windows)이 CI 와 같은 `dtolnay/rust-toolchain@master` + `toolchain: "1.95.0"` 을 쓴다. 타깃은 빌드에 실제로 쓰이는 툴체인에 설치된다.
- 툴체인을 올릴 때는 `rust-toolchain.toml`, `ci.yml`, `release.yml` 을 함께 바꾼다(워크플로 주석 "Pinned to the same version as rust-toolchain.toml; bump both together").
- 태그 경로는 태그가 가리키는 커밋의 워크플로 파일을 쓰므로, 고친 워크플로로 다시 돌리려고 `v0.1.0` 태그를 웨이브 15 커밋으로 옮겼다(사용자 승인, [2026-10-14-adr-release-policy.md](2026-10-14-adr-release-policy.md)).

## 이유

`rust-toolchain.toml` 이 있으면 설치된 기본 툴체인과 상관없이 그 판으로 빌드한다. 설치와 빌드의 판을 하나로 맞추는 것이 근본 수정이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `@stable` 유지 | `rust-toolchain.toml` 의 1.95.0 과 달라 타깃이 없다 |
| 워크플로에서 `rustup target add` 를 1.95.0 에 따로 더하기 | 같은 판 지정을 두 곳에 흩는다. CI 와 같은 액션 꼴로 맞추는 편이 단순하다 |

## 영향과 관련 파일

- `.github/workflows/release.yml`, `rust-toolchain.toml`, `.github/workflows/ci.yml`.
- 문서: `docs/ci-release.md` 1절(툴체인 1.95.0), 2절.
- 커밋: `9dbb8ff`.
- 로컬 부수 사항: 조사 중 로컬에 `x86_64-apple-darwin` 타깃을 설치했다(사용자에게 제거 방법을 안내함).

## 날짜와 사용자 원문

2026-10-12. 사용자 원문 없음(메인이 실패 원인을 찾아 고침). 다시 돌리는 방법(태그 옮김)은 사용자 결정: "웨이브 15 가 끝난 커밋으로 `v0.1.0` 태그를 옮겨 다시 돌린다(A)"(`docs/history/2026-10-14-process-archive.md` 113행).
