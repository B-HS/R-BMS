# 지적(스스로 발견): 마이그레이션만 담으려던 커밋에 웨이브 전체가 들어감

> 최종 갱신 2026-10-23 · 대응 커밋 9e39262

## 무엇이 일어났나

- 2026-10-23 웨이브 1 병합에서 worktree 패치를 `git apply --3way` 로 메인 작업 트리에 적용했다. `--3way` 는 적용한 변경을 인덱스에도 올린다.
- 그 뒤 "마이그레이션 0012 파일 세 개만" 커밋하려고 세 파일을 `git add` 하고 `git diff --cached --stat` 과 `git commit`·`git push` 를 한 명령 줄에 이어 붙였다. 출력에 64개 파일이 보였지만 이미 커밋·푸시된 뒤였다.
- 결과: `9e39262` 에 웨이브 1 의 Rust 변경(59파일)과 `web/src/server/db/schema.ts`·`web/tests/db/migration-0012.test.ts` 가 함께 들어가 푸시됐다. 교차 검토서 5.1-1 이 금지한 "migrate 전 `schema.ts` 배포" 가 됐고, 재생성한 `contract/client-requests/settings-blob.json` 은 빠져 ci 의 `a_settings_upload` 가 깨질 상태였다.

## 왜 틀렸나

- 스테이징 확인을 커밋과 같은 줄에 두어 확인이 확인 역할을 하지 못했다(git.md 6절 "커밋 전에 staged diff 를 확인").
- `git apply --3way` 의 인덱스 부작용을 고려하지 않았다.

## 어떻게 고치나

- 병합은 `git apply`(인덱스를 건드리지 않음)로 하고, 충돌이 나면 그때만 `--3way` 를 쓴 뒤 `git reset -q` 로 인덱스를 비운다.
- 커밋 전 `git diff --cached --name-only` 를 별도 명령으로 실행해 파일 목록을 읽고, 의도한 목록과 다르면 커밋하지 않는다. `commit` 과 `push` 를 확인 명령과 같은 줄에 잇지 않는다.

## 언제 적용하나

- worktree 패치를 메인 트리에 합치는 모든 웨이브 병합, 그리고 "일부 파일만" 커밋해야 하는 순서 제약(마이그레이션 먼저 등)이 있는 커밋.

## 사후 처리

- 0012 는 추가만 하는 마이그레이션이라 사용자가 승인한 운영 적용으로 맞춘다(자동 모드가 메인의 migrate 를 거부해 사용자 `!` 실행 요청).
- 웨이브 1 통합 게이트(같은 코드): fmt·clippy 통과, cargo test 6,382 통과·0 실패. `settings-blob.json` 은 바로 다음 커밋으로 보냈다.
