# rbms 백엔드 — IR 슈퍼셋 서버 (docs/backend)

> rbms 클라이언트(`rbms-ir`)가 호출하는 [`docs/reference/ir-api.md`](../reference/ir-api.md) 계약을 `web/`의 Next.js 서버로 구현한 문서 묶음이다. 실제 배포 키를 넣는 Phase R 전까지는 로컬 코드·테스트 정합성만 이 문서의 범위입니다.
> 설계 원칙: **LR2IR + 레퍼런스 구현 IR의 슈퍼셋** + rbms 고유 기능(설정 동기화·리플레이 서버 저장·캐퍼빌리티 탐색·전방호환 `extra`).
> 스택: **Next.js Route Handler (Bun) + Drizzle(MySQL) + Zod + better-auth**. Hono 별도 서버 전제는 구현 전에 폐기됐습니다.

## 문서

| 파일 | 내용 |
|---|---|
| [PRD.md](./PRD.md) | 제품 요구사항 — 목표·사용자·범위/비범위·호환 매트릭스·성공지표·보안·로드맵 |
| [api-spec.md](./api-spec.md) | **전체 HTTP API 규격** — 도메인별 모든 엔드포인트·메서드·경로·요청/응답 DTO(JSON)·인증·상태코드·에러 |
| [data-model.md](./data-model.md) | DB 스키마(Drizzle/MySQL) — user·session·chart·score(early/late)·course·table·rival·setting·replay·인덱스 |
| [endpoint-tasks.md](./endpoint-tasks.md) | **구현 상태와 남은 갭** — 실제 Route→Service→compose/Drizzle 경계와 미구현 범위 |
| [compatibility.md](./compatibility.md) | LR2IR / 레퍼런스 구현 IR ↔ rbms 슈퍼셋 필드 매핑 + 출처(원본 소스/프로토콜) |
| [contract-freeze.md](./contract-freeze.md) | 초기 계약 동결 기록 — 현재 구현으로 확정된 결정과 폐기된 별도 레포 전제 |

## 한눈 요약 (도메인)

```
auth        register / login / token / me (Bearer+세션쿠키)  (레퍼런스 구현 IRAccount 슈퍼셋)
players     profile / rivals / recent                       (IRPlayerData)
charts      ranking(md5·sha256) / best / meta / search       (LR2IR getrankingxml 슈퍼셋)
scores      submit(전체옵션+빌드해시) / get(by player·chart)  (IRScoreData early·late 슈퍼셋)
courses     submit / ranking / course meta                  (IRCourseData)
tables      list / folders / courses                        (IRTableData)
replays     upload / download (µs 정밀, score 연결)          (rbms 추가 — 핵분석)
settings    get / put (settings·keyconfig·tables·임의 blob)  (rbms 추가)
integrity   client build allowlist · ranked 판정 · 감사로그   (rbms 추가 — 변조/공정)
fe(web)     search · leaderboards · activity · 리플레이뷰어   (rbms 추가 — 구현됨)
system      health(capability) / version                    (rbms 추가)
compat      LR2IR getrankingxml.cgi 등 레거시 어댑터(선택)    (LR2 클라이언트 호환)
```

## 핵심 결정 (요지)
- **스택 = Next.js Route Handler + Drizzle(MySQL) + Zod + better-auth**. 구현은 `src/server/route → service → compose(Drizzle)` 경계, `getEnv()` Zod, snake_case 컬럼을 사용합니다.
- **차트 키 = sha256 우선 + md5 동시 보유**. LR2IR은 md5만, 레퍼런스 구현는 sha256. 둘 다 인덱싱해 양쪽 클라이언트 호환.
- **스코어 모델 = 레퍼런스 구현 IRScoreData 슈퍼셋**: 판정 PG/GR/GD/BD/PR/MS × {early, late} 분리(EX·BP·avgjudge 유도) + `option`·`seed`·`gauge`·`assist`·`rule`·`skin` 보존.
- **공평 평가 = 플레이 전체정보 보존**: 모든 옵션(judge_rate·total·autoplay·assist·hispeed·lift·cover…) + `client_build_sha256`(변조 탐지) + µs 리플레이. 서버가 `ranked` 산출.
- **µs 리플레이**: 입력 타임스탬프를 µs로 무손실 저장 → 핵분석·재시뮬.
- **인증 = better-auth**: 네이티브=Bearer 토큰, 웹 FE=세션 쿠키. guest 허용(설정).
- **FE 구현**: 검색·리더보드·피드·리플레이뷰어와 `/api/fe/*` envelope 경로가 있습니다. 단일 오리진이므로 CORS는 구현하지 않았습니다.
- **전방호환 = DTO `extra` + `/health` capabilities + `api_version`**. 레거시 LR2IR 어댑터는 아직 구현하지 않았습니다.

> 출처·매핑 근거는 [compatibility.md](./compatibility.md) 참조.
