# ADR: 운영 DB 작업은 web-db 워크플로와 저장소 시크릿 WEB_DATABASE_URL 로

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(실행 주체를 메인으로 넘김, 시험 계정 삭제 승인) + 메인(경로)
> 대체하는 결정: [2026-10-14-web-decisions.md](2026-10-14-web-decisions.md) 의 Q5(사용자가 로컬에서 실행), Q6 의 실행 주체(사용자)

## 배경

- 웹 배포(S10)에는 운영 MySQL 에 마이그레이션 0001(사용자 폴더)과 0002(SCRATCH AUTO 최고 기록 분리)를 적용하고 시험 계정 다섯 개를 지우는 일이 필요했다.
- Q5 의 답은 처음에 A(사용자가 로컬에서 실행, 메인이 상태 확인 스크립트와 명령 순서를 준다)였다. 이후 사용자가 마이그레이션도 메인이 처리하라고 지시했다.
- 운영 `DATABASE_URL` 은 Vercel 에 Secret 형식으로 있어 `vercel env pull`·`vercel env run` 으로 받을 수 없다(확인한 출력: "2 Secret values cannot be pulled"). 에이전트는 `.env` 계열을 읽거나 쓰지 않는다.

## 결정

1. 사용자가 운영 DB 주소를 저장소 시크릿 `WEB_DATABASE_URL` 로 등록한다. 값은 대화에 남기지 않도록 사용자가 별도 터미널에서 `gh secret set` 으로 넣는다.
2. 수동 워크플로 `.github/workflows/web-db.yml`(`workflow_dispatch`, 입력 `action`)이 GitHub 러너에서 운영 DB 작업을 한다. 동작: `status`(적용 상태), `migrate`(상태 → `bun run db:migrate` → 상태), `cleanup-count`, `cleanup-delete`(세기 → 지우기 → 다시 세기). 시크릿이 비면 첫 단계에서 멈추고, 값은 출력하지 않는다. 동시 실행 막음(`concurrency: web-db`).
3. 실행: `gh workflow run web-db.yml --ref dev -f action=<동작>`. SQL 파일은 `web/scripts/run-sql.ts` 로만 돌고, 그 스크립트는 `web/scripts/` 아래의 `.sql` 만 받는다.
4. 실행 주체는 메인이다. 지우기 같은 파괴적 작업은 실행 전에 사용자 승인을 받는다(시험 계정 삭제는 2026-10-14 승인).
5. 운영 DB 가 GitHub 러너의 접속을 받지 않게 바뀌면, 같은 명령을 로컬에서 `DATABASE_URL` 을 환경에만 주고 실행한다(`docs/web/architecture.md` 3-2).

## 이유

- Vercel Secret 은 CLI 로 꺼낼 수 없고, 에이전트는 비밀 파일을 다루지 않는다. 저장소 시크릿은 러너 안에서만 값이 풀리고 로그에 가려진다.
- 사용자 지시로 실행 주체가 메인이 되었으므로, 사용자의 로컬 환경에 기대지 않는 재현 가능한 경로가 필요했다.
- 대화에 값이 남지 않게 `!` 명령으로 넣게 하지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `vercel env pull`·`vercel env run` 으로 운영 주소 받기 | Vercel Secret 이라 불가("2 Secret values cannot be pulled") |
| 사용자에게 시크릿 값을 대화의 `!` 명령으로 넣게 하기 | 대화 기록에 값이 남는다 |
| Q5 의 A(사용자가 로컬에서 실행) 유지 | 사용자가 메인에게 넘김 |
| 빌드 단계에 `drizzle-kit migrate` 넣기 | 빌드는 DB 에 접속하지 않는다는 원칙(`docs/web/architecture.md` 6-2) |

## 영향과 관련 파일

- `.github/workflows/web-db.yml`, `web/scripts/run-sql.ts`, `web/scripts/db-migration-status.ts`(`bun run db:status`), `web/scripts/cleanup-test-accounts.count.sql`, `web/scripts/cleanup-test-accounts.delete.sql`, `web/package.json` 의 `db:migrate`·`db:status`.
- 실행 기록(2026-10-14): 상태 run 38548385742(0000 만 기록) → 마이그레이션 run 38548459519(`__drizzle_migrations` 3행, 0000~0002 적용) → 세기 run 38548532538 → 지우기 run 38549827633(대상 전부 0, 다른 사용자의 score 3건 그대로).
- 커밋: `baff560`, 기록 `d81eafc`.
- 문서: `docs/web/architecture.md` 3-2(운영 DB 작업 워크플로).
- 문서 어긋남(이 문서 소유 밖, 고치지 않음): `docs/ci-release.md` 6절의 "운영 DB 마이그레이션은 배포보다 먼저 사용자가 적용한다"는 이 결정 이전의 절차다. 지금은 메인이 `web-db.yml` 로 적용한다(배포보다 먼저라는 순서는 그대로).

## 날짜와 사용자 원문

2026-10-14. 원문은 보존되지 않았다. 요지(메인의 세션 인벤토리): 처음엔 사용자가 직접 실행(Q5=A) → 이후 "migrate도 너가 처리". 시험 계정 삭제 승인(2026-10-14).
