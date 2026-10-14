# ADR: Vercel 은 Git 연결, 운영 브랜치 dev, 루트 web

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(U5) + 메인(M10, 연결 실행)

## 배경

웹(https://bms.hyuns.uk)은 Git 연동이 아니라 CLI 업로드로 배포된 상태였다(`/api/version` 의 `commit` 이 null). 사용자 지시: Vercel 과 GitHub 을 연결한다. 이 저장소는 Rust 플레이어와 웹이 함께 있고, `prod` 브랜치는 앱 릴리스 워크플로(`release.yml`)의 버전 올림 브랜치다.

## 결정

1. Vercel 프로젝트 `rbms-web`(팀 b-hs-projects)를 `B-HS/R-BMS` 에 Git 연결한다(`vercel git connect`).
2. Production Branch 는 `dev`, Root Directory 는 `web`(API PATCH 로 설정).
3. Preview 배포는 만들지 않는다. `web/vercel.json` 의 `ignoreCommand` 가 프로덕션이 아닌 빌드를 건너뛰고, 프로덕션이어도 `web/` 이 바뀌지 않은 푸시(Rust·docs 만 바뀐 푸시)를 건너뛴다. 리전 `icn1`.
4. 웹 CI 는 따로 둔다(`.github/workflows/web.yml`). `ci.yml` 은 `web/**` 만 바뀐 푸시에서 Rust 빌드를 건너뛴다(단 `web/data/tables/**` 는 플레이어가 내장하므로 Rust CI 도 돈다).
5. 앱 릴리스 태그·`prod` 푸시는 웹을 배포하지 않는다.

## 이유

- `dev`: 작업 브랜치이자 웹이 바뀌는 곳이다. `prod` 는 앱 릴리스용으로 이미 쓰인다.
- Root `web`: 저장소 루트는 Cargo 워크스페이스라 Vercel 의 기본 감지가 맞지 않는다. 내장 "Skip deployment" 토글은 JS workspace 모노레포용이라 쓸 수 없어 `ignoreCommand` 로 대신한다(`docs/web/architecture.md` 6-2).
- Preview 없음: 운영 DB 하나뿐이고 Preview 용 DB·비밀이 없다(M10).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `prod` 를 웹 운영 브랜치로 | `prod` 는 앱 릴리스 버전 올림 브랜치라 겹친다 |
| CLI 업로드 배포 유지 | 배포한 커밋을 알 수 없다(`/api/version` 의 `commit` null) |
| Preview 배포 사용 | Preview 용 DB·비밀이 없다(M10) |

## 영향과 관련 파일

- `web/vercel.json`, `.github/workflows/web.yml`, `.github/workflows/ci.yml`(`paths-ignore`).
- 문서: `docs/web/architecture.md` 6-2(설정값·환경 변수 정본), `docs/ci-release.md` 6절.
- 연결 과정의 사고: `vercel link` 가 `web/.env.local` 을 자동으로 만들었고 사용자가 지웠다(내용은 읽지 않음). `web/.vercel` 링크는 무시 파일이다. 같은 일을 다시 할 때 `vercel link` 뒤 `web/.env.local` 이 생기는지 확인하고 사용자에게 알린다.
- 환경 변수 6개는 Vercel 에만 있다(`DATABASE_URL`·`BETTER_AUTH_SECRET` 은 Secret 형식). 운영 DB 작업은 [2026-10-14-adr-production-db-operations.md](2026-10-14-adr-production-db-operations.md).
- 첫 Git 배포: 커밋 `d81eafc` Ready(`/api/version` commit `d81eafc`). 커밋 `6b10947`(웹 CI 분리), `d38b6fb`(`vercel.json`).

## 날짜와 사용자 원문

2026-10-14. 사용자 결정 U5: "Vercel 프로덕션 브랜치는 `dev`, 루트 `web/`, `web/` 이 바뀐 푸시에서만 배포(`prod` 는 앱 릴리스 워크플로의 브랜치)"(`docs/PROCESS.md` 현재 작업의 사용자 결정 기록. 원문 그대로는 보존되지 않았다).
