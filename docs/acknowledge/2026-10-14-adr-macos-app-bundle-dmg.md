# ADR: macOS 배포는 공증 티켓을 붙인 앱 번들 + dmg

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자("다 추천대로")

## 배경

`v0.1.1` 까지 macOS 배포물은 유니버설 단독 실행 파일을 담은 `rbms-macos-universal.tar.gz` 였다. 서명·공증(`status: Accepted`)이 통과했는데도 사용자가 받은 실행 파일이 "Apple could not verify ... free of malware" 로 막혔다. 같은 파일은 `spctl` 온라인 판정이 `accepted, source=Notarized Developer ID` 였지만 `syspolicy_check distribution` 은 "Notary Ticket Missing" 을 냈다. 단독 실행 파일에는 공증 티켓을 붙일(staple) 수 없어 Gatekeeper 가 온라인 조회에 기대고, 내려받은 환경에서는 그 조회가 통과를 보장하지 않는다.

## 결정

1. `scripts/macos-app.sh` 로 앱 번들 `R-BMS.app` 을 만든다: 번들 ID `uk.hyuns.rbms`, 실행 파일 `Contents/MacOS/rbms-player`, 최소 macOS 11.0, `CFBundleShortVersionString` 은 `Cargo.toml` 의 버전.
2. 앱을 Developer ID(하드닝 런타임, 보안 타임스탬프)로 서명하고 공증해 티켓을 붙인다(`scripts/macos-notarize.sh`, 앱은 zip 으로 제출). 그 다음 앱, `Applications` 링크, `LICENSE`·`README.md`·`THIRD-PARTY-NOTICES.md` 를 담은 `R-BMS-macos-universal.dmg` 를 만들어 dmg 도 서명·공증·티켓 부착한다. 마지막에 `spctl` 로 앱과 dmg 의 Gatekeeper 판정을 확인한다. 고지 파일이 없으면 잡이 실패한다.
3. 서명 시크릿이 없으면 ad-hoc 서명(Apple Silicon 이 서명 없는 코드를 거부하므로)이다.
4. Finder 로 앱을 처음 열 때 macOS 가 붙일 수 있는 `-psn_…` 인자는 곡 경로로 읽지 않는다(`apps/rbms-player/src/lib.rs` 의 `is_system_launch_arg`, `PROCESS_SERIAL_ARG_PREFIX`).
5. `v0.1.1` draft 를 지우고 `v0.1.2` 로 다시 낸다.

## 이유

앱과 dmg 에는 티켓을 붙일 수 있어 오프라인에서도 Gatekeeper 를 통과한다. 확인(v0.1.2, run 38559305840): dmg 와 안의 앱 모두 `stapler validate` 통과, `accepted, source=Notarized Developer ID`, `syspolicy_check distribution` "passed all pre-distribution checks", `x86_64 arm64`.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 맨 실행 파일 tar.gz 유지 | 티켓을 붙일 수 없어 내려받은 사본이 Gatekeeper 에 막힌다(v0.1.1 에서 실제로 막힘) |
| 앱 번들만 zip 으로 | 앱에는 티켓이 붙지만 사용자에게 `Applications` 로 옮기는 안내와 고지 문서를 함께 주기 어렵다. dmg 가 macOS 의 표준 배포 꼴이다(메인 추천, 사용자 "다 추천대로") |
| 사용자에게 격리 속성 지우기(`xattr`)를 안내 | 증상만 가리는 우회 |

## 영향과 관련 파일

- `scripts/macos-app.sh`, `scripts/macos-notarize.sh`, `.github/workflows/release.yml`(macOS 잡), `apps/rbms-player/src/lib.rs`, `apps/rbms-player/src/main_tests.rs`, `README.md`.
- 문서 정본: `docs/ci-release.md` 2절.
- 커밋: `f34e889`(워크플로·스크립트), `bfa0cca`(`-psn_` 인자), 릴리스 `c3937f4`(v0.1.2, 봇), 기록 `8ab318d`.
- 남은 일: 앱 아이콘(`.icns`) 없음. Windows 코드 서명은 나중에(사용자).
- 문서 어긋남(이 문서 소유 밖, 고치지 않음): `docs/ci-release.md` 4절 "macOS 아카이브는 압축 해제 후 `./rbms-player <곡폴더>`로 실행"은 tar.gz 시절의 안내다. 지금은 dmg 를 열어 `R-BMS.app` 을 `Applications` 로 옮긴다.

## 날짜와 사용자 원문

2026-10-14. 사용자 보고: 받은 v0.1.1 실행 파일이 "Apple could not verify ... free of malware" 로 막힘. 결정 답: "다 추천대로"(앱 번들 + dmg, 앱과 dmg 모두 공증·티켓, v0.1.1 draft 를 지우고 v0.1.2).
