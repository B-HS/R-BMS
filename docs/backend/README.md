# rbms 백엔드 — IR 슈퍼셋 서버 (docs/backend)

> rbms 클라이언트(`rbms-ir`)가 이미 호출하는 [`docs/reference/ir-api.md`](../reference/ir-api.md) 계약을 **구현 가능한 수준으로 확장·확정**한 백엔드 설계 묶음이다.
> 설계 원칙: **LR2IR + beatoraja IR의 슈퍼셋** + rbms 고유 기능(설정 동기화·리플레이 서버 저장·캐퍼빌리티 탐색·전방호환 `extra`).
> 스택: **Hono.js (Bun) + Drizzle(MySQL) + Zod + better-auth** — 사용자 백엔드 컨벤션(`~/.claude/convention/backend.md`)을 그대로 따른다.

## 문서

| 파일 | 내용 |
|---|---|
| [PRD.md](./PRD.md) | 제품 요구사항 — 목표·사용자·범위/비범위·호환 매트릭스·성공지표·보안·로드맵 |
| [api-spec.md](./api-spec.md) | **전체 HTTP API 규격** — 도메인별 모든 엔드포인트·메서드·경로·요청/응답 DTO(JSON)·인증·상태코드·에러 |
| [data-model.md](./data-model.md) | DB 스키마(Drizzle/MySQL) — user·session·chart·score(early/late)·course·table·rival·setting·replay·인덱스 |
| [endpoint-tasks.md](./endpoint-tasks.md) | **구현 태스크 체크리스트** — 도메인·엔드포인트별 Route→Service→ServiceDb→Drizzle 분해 |
| [compatibility.md](./compatibility.md) | LR2IR / beatoraja IR ↔ rbms 슈퍼셋 필드 매핑 + 출처(원본 소스/프로토콜) |

## 한눈 요약 (도메인)

```
auth        register / login / token / me (Bearer+세션쿠키)  (beatoraja IRAccount 슈퍼셋)
players     profile / rivals / recent                       (IRPlayerData)
charts      ranking(md5·sha256) / best / meta / search       (LR2IR getrankingxml 슈퍼셋)
scores      submit(전체옵션+빌드해시) / get(by player·chart)  (IRScoreData early·late 슈퍼셋)
courses     submit / ranking / course meta                  (IRCourseData)
tables      list / folders / courses                        (IRTableData)
replays     upload / download (µs 정밀, score 연결)          (rbms 추가 — 핵분석)
settings    get / put (settings·keyconfig·tables·임의 blob)  (rbms 추가)
integrity   client build allowlist · ranked 판정 · 감사로그   (rbms 추가 — 변조/공정)
fe(web)     search · leaderboards · activity · 리플레이뷰어   (rbms 추가 — 향후 FE)
system      health(capability) / version                    (rbms 추가)
compat      LR2IR getrankingxml.cgi 등 레거시 어댑터(선택)    (LR2 클라이언트 호환)
```

## 핵심 결정 (요지)
- **스택 = Bun + Hono.js + Drizzle(MySQL) + Zod + better-auth**. 사용자 백엔드 컨벤션(`~/.claude/convention/backend.md`) **준수 필수**: Route(DTO검증)→Service(로직)→ServiceDb(compose Drizzle)→Drizzle, Factory DI, HOF(`withErrorHandling`/`withAuth`), 에러 3파일 중앙화, 응답 헬퍼, `getEnv` Zod, snake_case 컬럼.
- **차트 키 = sha256 우선 + md5 동시 보유**. LR2IR은 md5만, beatoraja는 sha256. 둘 다 인덱싱해 양쪽 클라이언트 호환.
- **스코어 모델 = beatoraja IRScoreData 슈퍼셋**: 판정 PG/GR/GD/BD/PR/MS × {early, late} 분리(EX·BP·avgjudge 유도) + `option`·`seed`·`gauge`·`assist`·`rule`·`skin` 보존.
- **공평 평가 = 플레이 전체정보 보존**: 모든 옵션(judge_rate·total·autoplay·assist·hispeed·lift·cover…) + `client_build_sha256`(변조 탐지) + µs 리플레이. 서버가 `ranked` 산출.
- **µs 리플레이**: 입력 타임스탬프를 µs로 무손실 저장 → 핵분석·재시뮬.
- **인증 = better-auth**: 네이티브=Bearer 토큰, 웹 FE=세션 쿠키. guest 허용(설정).
- **FE 대비**: 검색·리더보드·피드·리플레이뷰어 + envelope·CORS.
- **전방호환 = 모든 DTO `extra` + `/health` capabilities + `api_version`**. 레거시 LR2IR은 선택적 어댑터.

> 출처·매핑 근거는 [compatibility.md](./compatibility.md) 참조.
