# ADR: 릴리스는 태그와 draft 까지 자동, 게시는 사용자 · 자격 증명은 빌드 전에 확인

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-12(draft), 2026-10-13(빌드 전 점검·로컬 스크립트), 2026-10-14(릴리스 실행 주체) · 결정자 사용자
> 대체하는 결정: `release-branch-strategy.md`(결정 대기 상태의 옵션), `2026-09-09-enhancement-decisions.md` "마무리 순서"의 "릴리스는 사용자가 자택 보관 키로 직접. 에이전트는 착수하지 않는다"

## 배경

- 첫 릴리스(R7, 2026-10-12)를 준비하며 `release.yml` 이 공개 릴리스를 바로 만들던 것을 바꿨다.
- 첫 두 실행이 실패했다: 툴체인 타깃 없음([2026-10-14-adr-release-toolchain-pin.md](2026-10-14-adr-release-toolchain-pin.md)), 시크릿 `MACOS_CERTIFICATE_P12` 값의 한 글자. 틀린 시크릿이 두 번의 유니버설 빌드가 끝난 뒤에야 드러나 10분 넘게 걸렸다. 사용자: 빌드 전에 확인하라(짜증 포인트 "오래걸리잖아").
- 사용자의 다른 저장소 TAIDE 가 같은 `error decoding base64 input stream` 을 겪고 로컬 검증 스크립트를 만들어 둔 상태였다.

## 결정

1. 어느 경로(`prod` 푸시, 수동 `workflow_dispatch` 의 bump, `v*.*.*` 태그 푸시)로 돌든 자동은 버전 커밋·주석 태그·draft GitHub Release 까지다(`release.yml` 의 `draft: true`). 게시(publish)는 사용자가 Releases 화면에서 본문을 확인한 뒤 직접 한다.
2. 메인은 사용자의 지시가 있을 때 릴리스를 돌린다. 순서: 릴리스할 커밋의 CI 세 OS 통과 확인 → `gh workflow run release.yml --ref dev -f bump=patch` → 산출물을 내려받아 확인(SHA-256, `stapler validate`, `spctl`, `syspolicy_check distribution`, 버전) → draft 본문을 코드와 문서로 확인한 사실로 쓴 릴리스 노트로 바꾼다(`gh release edit <tag> --notes-file … --draft=true`). 게시하지 않는다.
3. macOS 잡은 체크아웃과 빌드보다 먼저 자격 증명을 확인한다: 인증서를 임시 키체인에 넣어 서명 신원을 정하고(`Prepare the signing keychain`), 공증 계정으로 로그인해 본다(`Check the notarisation account`, `xcrun notarytool history`). 틀리면 첫 몇 초에 멈춘다. 값은 출력하지 않고 꼴만 알린다(base64 밖 바이트 수 등).
4. 빌드 없이 등록된 시크릿만 확인하는 수동 워크플로 `.github/workflows/signing-check.yml` 을 둔다.
5. 로컬 선검증 스크립트(TAIDE 에서 가져옴): `scripts/verify-signing-cert.sh <cert.p12 | base64.txt> [--set-secret]`(검증을 통과한 값을 `gh secret set` 으로 바로 등록, 클립보드·웹 화면을 거치지 않음), `scripts/verify-notary-credentials.sh`. 순서: 로컬 스크립트 → `signing-check` → 릴리스.
6. 이미 만들어진 공개 태그를 옮기는 일은 하지 않는다. 예외로 한 번, 릴리스가 만들어진 적 없는 `v0.1.0` 태그를 웨이브 15 커밋(`00b91b1`)으로 옮겼다(사용자 승인 2026-10-12). 잘못 만든 draft 는 사용자 지시로 지우고 다음 patch 로 간다(v0.1.1 → v0.1.2 → v0.1.3).

## 이유

- 릴리스는 되돌리기 어려운 공개 행동이라 사람이 본문과 파일을 본 뒤 게시한다.
- 틀린 시크릿을 빌드 뒤에 알면 한 번에 10분 넘게 버린다. 확인을 앞에 두면 몇 초다.
- 붙여 넣은 값의 한 글자가 `.` 으로 바뀌어 세 번 실패한 일이 있어, 검증한 값을 그대로 시크릿으로 넣는 경로를 둔다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 릴리스를 바로 게시(이전 `release.yml`) | 공개 행동을 사람 확인 없이 한다 |
| 자격 증명을 서명 단계(빌드 뒤)에서만 확인 | 틀린 값이 빌드 두 번 뒤에 드러남. 사용자 지시 |
| 시크릿을 웹 화면에 붙여 넣기 | 한 글자 변형 사고. `--set-secret` 으로 대체 |
| 에이전트는 릴리스에 손대지 않음(2026-09-10 마무리 순서) | 실제 키가 저장소 시크릿으로 등록됐고 사용자가 실행을 지시함 |

## 영향과 관련 파일

- `.github/workflows/release.yml`, `.github/workflows/signing-check.yml`, `scripts/verify-signing-cert.sh`, `scripts/verify-notary-credentials.sh`.
- 저장소 시크릿: `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`(선택 `APPLE_SIGNING_IDENTITY`).
- 문서 정본: `docs/ci-release.md` 2절, 3.1~3.3절.
- 커밋: `a2289da`(draft), `39afc3a`·`e463057`(signing-check), `287b80d`(빌드 전 확인), `2ad01aa`(로컬 스크립트).
- 릴리스 이력(2026-10-12 ~ 10-15): `v0.1.0` draft(게시 안 됨, 2026-10-14 사용자 결정으로 둔다), `v0.1.1` draft 는 사용자 지시로 지움, `v0.1.2` draft 는 점수 제출 400 버그로 지움, `v0.1.3` draft(버전 커밋 `fb29d24`)는 2026-10-14 사용자 결정으로 지우지 않고 두며, 다음 릴리스는 `v0.1.4`(버전 커밋 `87eacd0`, 2026-10-15T07:27:28Z 에 게시됨). 결정은 [v0.1.4 릴리스 ADR](2026-10-14-adr-v0-1-4-release.md). 결정 6 의 "잘못 만든 draft 는 지우고 다음 patch 로" 는 `v0.1.3` 에는 적용하지 않았다(지우지 않고 다음 patch).
- Windows 코드 서명은 나중에(사용자).

## 날짜와 사용자 원문

- 2026-10-12: draft 까지만 자동(`docs/ci-release.md` 2절 "사용자 결정 2026-10-12"). `v0.1.0` 태그 옮김 승인. 원문은 보존되지 않았다.
- 2026-10-13 지시 요지(`docs/history/2026-10-14-process-archive.md` 131행): "(1) 서명 등 env 확인을 빌드 시작 전에 한다(빌드 뒤에 두어 오래 걸렸다) (2) 로컬에서도 미리 테스트한다, TAIDE 의 스크립트를 가져온다".
- 2026-10-14 원문: "삭제하고 0.1.1 릴리스만들어. 드래프트로 올릴텐데 드래프트 내용도 정확하게 잘 적어놔야해".
