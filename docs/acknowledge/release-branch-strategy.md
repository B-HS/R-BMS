# 릴리스: prod 브랜치 · 보호 전략 · 첫 버전 (결정 대기)

> 로드맵 Phase 2. **사용자 결정 필요** 항목이라 자동 실행하지 않고 절차·옵션·권장을 고정한다.
> 근거: `prod` push는 `release.yml`의 `version` 잡을 트리거해 **즉시 공개 릴리스를 자동 생성**한다(아래). 되돌리기 어려운 공개 액션이라 무분별 실행 금지(ai-process §3).

## 사실 (검증됨)
- 원격 `origin = github.com/B-HS/R-BMS`, `dev` 푸시됨(`origin/dev` == 로컬 dev, `a9ff66e`). `prod` 브랜치·`v*` 태그 **없음**.
- `release.yml` 트리거: ① `prod` push → `version` 잡(태그 없으면 `v0.0.0`→patch=**`v0.0.1`**으로 자동 bump·커밋·태그·push·빌드·릴리스), ② 수동 dispatch(bump 선택), ③ `v*.*.*` 태그 push(→ `version` skip, 그 태그로 빌드·릴리스).
- 산출물: macOS 유니버설 tar.gz·Windows zip·각 sha256(백엔드 client_build allowlist 입력).
- `Cargo.toml [workspace.package] version = "0.1.0"`.

## 결정 필요 (open decision F)
1. **첫 버전**: prod push 자동 bump는 `v0.0.1`을 만든다. Cargo.toml과 일치하는 **`v0.1.0`을 원하면 수동 태그**(트리거 ③)로 내야 한다.
   - 권장: `git tag v0.1.0 && git push origin v0.1.0` (prod 브랜치는 별도로 만들어 두되, 첫 릴리스는 태그로).
2. **prod 브랜치 보호**: `version` 잡이 `prod`로 커밋+태그를 push한다. prod를 보호하면 이 자동 push가 막힐 수 있다. 선택:
   - (a) prod **미보호**(가장 단순, 자동 bump 동작) — 권장(소규모/단독).
   - (b) 보호 + `github-actions[bot]` 예외 허용.
   - (c) 보호 + **자동 bump 미사용, 수동 태그만**(트리거 ③) — 위 1의 권장과 결합 시 가장 안전.

## 권장 실행 절차 (greenlight 시 1회)
```bash
git branch prod && git push -u origin prod        # prod 생성(보호 안 하면 release.yml가 v0.0.1 자동 릴리스 → 원치 않으면 아래만)
# 또는 첫 버전을 v0.1.0으로 고정하려면 (prod 자동 bump 회피):
git tag v0.1.0 && git push origin v0.1.0           # version 잡 skip, build가 v0.1.0으로 릴리스
```
- 권장 조합: **prod 미보호 + 첫 릴리스는 `v0.1.0` 수동 태그**. 이후 patch는 prod push 자동 bump, minor/major는 수동 dispatch.

## 사전 조건 (충족됨)
- 루트 `LICENSE`(GPL-3.0) 추가됨. CI(`ci.yml`) dev에서 3-OS green 확인(직전 커밋 success).
- 선택(P3): macOS notarize($99/yr)·Windows Authenticode — 미설정(소프트 경고만, 기능 릴리스엔 불필요).

## 상태
**대기** — 위 1·2 결정 후 절차 실행. 코드/워크플로/LICENSE는 준비 완료.
