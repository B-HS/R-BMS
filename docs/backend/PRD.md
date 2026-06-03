# rbms 백엔드 — PRD (Product Requirements)

> 대상: rbms(beatoraja 코어 PLAY 포팅) 전용 IR-슈퍼셋 백엔드. 본 문서는 "무엇을·왜"를, [api-spec.md](./api-spec.md)는 "어떻게(계약)"를 규정한다.

## 1. 배경 / 문제
- rbms는 오프라인으로 완전 동작(로컬 `scores.ron`·`replays/`·`settings.ron`). 그러나 **기기 간 동기화·랭킹·라이벌·발광 표 연동**이 없다.
- BMS 생태계의 표준은 **IR(Internet Ranking)**: 원조 **LR2IR**(md5 기반), 현대 **beatoraja IR**(sha256 기반, Mocha-Repository·LR2oraja·MinIR 등). rbms는 이미 `rbms-ir`로 IR 슈퍼셋 클라이언트를 갖췄으나 **서버가 없다**.
- 요구: **LR2의 슈퍼셋**이면서 beatoraja IR도 포괄하고, 추가로 **rbms의 모든 설정·리플레이를 서버에 저장**할 수 있는 백엔드.

## 2. 목표 (Goals)
1. **스코어 IR**: 차트(md5·sha256)별 랭킹 제출/조회, 개인 베스트, 라이벌. beatoraja IRScoreData의 풀 데이터(판정 early/late, 옵션, 시드, 게이지, 룰) 보존.
2. **코스(단위인정) IR**: 코스 결과 제출/랭킹, 코스/표 메타 제공(IRTableData/IRCourseData).
3. **설정 동기화**: rbms의 `settings.ron`·`keyconfig.ron`·`tables.ron`(+임의 named blob)을 계정에 저장/복원.
4. **리플레이 서버 저장**: 스코어에 연결된 리플레이 업로드/다운로드(다른 기기·관전·검증·고스트).
5. **호환/탐색**: `/health`로 capability 광고, `api_version`·`extra`로 전방호환, (선택) LR2IR 레거시 어댑터.
6. **인증**: 계정 register/login(beatoraja IRAccount 슈퍼셋) → 토큰(네이티브) + 세션 쿠키(웹 FE). 익명 `guest` 옵션.
7. **공평 평가(무결성)**: 제출은 **플레이에 사용된 모든 정보**(전체 옵션·시드·판정 early/late)를 보존 → 동일 조건 비교. **클라 빌드 SHA-256**을 모든 제출에 동봉·로깅 → 변조 탐지/allowlist. autoplay·assist·judge폭 등은 `ranked=false` 처리.
8. **µs 리플레이**: 리플레이를 **마이크로초(µs) 정밀**로 저장 → 재시뮬 검증·핵분석(매크로·오토 탐지) 가능.
9. **웹 FE 대비**: 향후 FE 프로젝트가 바로 쓸 검색/탐색/리더보드/리플레이 뷰어 엔드포인트 + envelope·CORS·세션.

## 3. 비목표 (Non-Goals, v1)
- 곡 파일(BMS/음원) 호스팅·배포. (URL 메타만 — `getSongURL` 대응)
- 부정행위 자동탐지(리플레이 재검증 엔진). v1은 리플레이 저장만, 검증은 capability 플래그로 후속.
- 소셜(팔로우 피드·코멘트)·실시간(WebSocket 대전). 후속 후보.
- LR2IR `gateway.cgi`의 난독화된 바이너리 제출 프로토콜 100% 재현. (랭킹 조회 호환만 우선; 제출은 REST.)

## 4. 사용자 / 시나리오
- **플레이어**: 기기 A에서 플레이 → 서버 제출 → 기기 B에서 같은 계정으로 랭킹·내 기록·리플레이·설정 복원.
- **표 운영자/커뮤니티**: 발광/단위 표를 IR 표로 제공(`getTableDatas`) → 클라이언트가 표 폴더·코스 노출.
- **beatoraja 사용자**: (호환 모드) beatoraja 클라이언트가 본 서버를 IR로 등록해 sha256 스코어 제출/조회.
- **LR2 사용자**: (레거시 어댑터) md5 랭킹 XML 조회.

## 5. 범위 — 기능 요구 (FR)
| ID | 기능 | 우선순위 |
|---|---|---|
| FR-1 | 계정 register/login/me, Bearer 토큰, guest 허용 | P0 |
| FR-2 | 스코어 제출(`/scores`) — early/late 판정·옵션·시드·게이지 전부 | P0 |
| FR-3 | 차트 랭킹(`/charts/{hash}/ranking`) md5·sha256 양쪽 키 | P0 |
| FR-4 | 개인 베스트(`/charts/{hash}/best?player=`) | P0 |
| FR-5 | `/health` capability·`/version` | P0 |
| FR-6 | 리플레이 업로드/다운로드, 스코어 연결 | P1 |
| FR-7 | 설정 동기화 get/put(settings·keyconfig·tables·named blob) | P1 |
| FR-8 | 라이벌 등록/조회, 라이벌 랭킹 필터 | P1 |
| FR-9 | 코스 제출/랭킹, 표(IRTableData) 제공 | P1 |
| FR-10 | 차트 메타 업서트/조회(title/artist/bpm/notes/url) | P2 |
| FR-11 | LR2IR 레거시 어댑터(`getrankingxml.cgi` 등) | P2 |
| FR-12 | 리플레이 기반 스코어 검증(anti-cheat) | P3(후속) |
| FR-13 | **클라 빌드 SHA-256 동봉·로깅·allowlist**(변조 탐지) | P0 |
| FR-14 | **ranked 적격성 산출**(autoplay·assist·judge폭·total·미상빌드 → unranked+flags) | P0 |
| FR-15 | **전체 플레이옵션 보존**(공평 평가 단일 출처) | P0 |
| FR-16 | **µs 리플레이**(핵분석·재시뮬 입력) | P1 |
| FR-17 | **FE 전용 조회**(검색·리더보드·최근피드·리플레이뷰어·통계) + envelope·CORS·세션 | P1 |
| FR-18 | 제출 감사 로그(`submission_audit`: build·ip·ua·flags) | P1 |

## 6. 비기능 요구 (NFR)
- **호환**: 단일 클라이언트가 md5만(LR2 차트) / sha256만(beatoraja) 보내도 동작. 미지원 기능은 `capabilities`로 광고 → 클라가 UI 토글.
- **성능**: 랭킹 조회 P95 < 150ms(상위 N 캐시), 제출 < 200ms. 차트당 best는 (chart, player) 유니크 인덱스로 O(1).
- **무결성**: 제출 멱등(같은 (player, sha256, played_at) 재전송 무중복). best는 항상 "더 좋은 결과만" 갱신(램프>EX>BP 순).
- **보안**: 토큰 해시 저장, 비번 better-auth, rate-limit(제출/로그인), 제출 본문 크기 제한, 리플레이 크기 상한.
- **전방호환**: `api_version`(현재 1) + 모든 DTO `extra`(free-form) 보존. 새 엔드포인트는 추가만, 기존 불변.
- **관측**: 구조적 로그·에러 중앙화(`createAppError`)·`/health` 헬스.

## 7. 호환 매트릭스 (요지 — 상세 [compatibility.md](./compatibility.md))
| 기능 | LR2IR | beatoraja IR | rbms 슈퍼셋 |
|---|---|---|---|
| 차트 키 | md5(32) | sha256(64) | **둘 다** |
| 클리어 램프 | 1-5(FAILED~FC) | ClearType 0-10 | **0-10(LightAssist 포함)** |
| 판정 | pg/gr/minbp | PG~MS × early/late | **early/late 전부 + 空POOR** |
| 옵션 | (제한) | option 비트필드·seed·gauge·assist·rule | **전부 + random_p2·judge_rate·offset 등 rbms 확장** |
| 코스 | courseid | IRCourseData(constraint·trophy) | **지원** |
| 표 | (외부 표 json) | IRTableData(folders·courses) | **지원** |
| 리플레이 | ✗ | (고스트 일부) | **업로드/다운로드** |
| 설정 동기화 | ✗ | ✗ | **지원(rbms 고유)** |
| 인증 | per-user post | id/password(IRAccount) | **register/login→token** |

## 8. 성공 지표
- rbms 클라이언트가 `--server <url>`로 제출·랭킹·베스트·리플레이·설정 동기화를 모두 수행(로컬과 일치).
- beatoraja 클라이언트가 (호환 모드) sha256 스코어 제출/조회 성공.
- `/health` capability가 클라 UI 토글과 일치.

## 9. 로드맵 (마일스톤)
- **M1(P0)**: auth·scores·charts ranking/best·health. (코어 IR)
- **M2(P1)**: replays·settings sync·rivals·courses·tables.
- **M3(P2)**: chart meta·LR2IR 레거시 어댑터·관리(모더레이션).
- **M4(P3)**: 리플레이 검증·anti-cheat·소셜.

## 10. 리스크/오픈이슈
- **부정 스코어**: v1은 신뢰 기반. 리플레이 저장으로 후속 검증 가능하게 설계(FR-12).
- **LR2IR `gateway.cgi`**: 난독 바이너리 제출은 재현 비용 큼 → REST 제출을 정본으로, 레거시는 *조회*만 어댑팅.
- **차트 동일성**: md5/sha256 미스매치(차트 재인코딩) — 둘 다 보관·교차 인덱스로 완화.
- **라이선스**: rbms 코드 GPL-3.0(beatoraja 포팅). 백엔드는 별 저장소/별 라이선스 가능(서버는 파생 아님). 사용자 확인 권장.
