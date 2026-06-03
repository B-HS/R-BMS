# rbms 백엔드 — 구현 태스크 (endpoint tasks)

> 컨벤션 `backend.md`: 도메인별로 **route / service/domain / dto / compose** 4곳에 같은 이름으로 흩어진다. 각 엔드포인트는 **Route(DTO검증·인증·에러throw) → Service(로직) → ServiceDb(compose의 Drizzle 구현)**. 핸들러는 `withErrorHandling`(+`withAuth`/`withAdmin`) 합성. 응답은 헬퍼.
> 체크는 M1→M4(PRD §9) 순. 각 항목 = (route 등록 · service 메서드 · ServiceDb 쿼리 · dto Zod · 테스트).

## 0. 부트스트랩 / 공통 (P0)
- [ ] `index.ts` 부트스트랩: `compose() → createRouter() → createMiddleware() → mount`(`/api`, `/lr2ir`). 비프로덕션 `/docs`·`/swagger`.
- [ ] `lib/error-*.ts` 3파일: `IR_CHART_NOT_FOUND`·`IR_PLAYER_NOT_FOUND`·`IR_SCORE_NOT_FOUND`·`IR_REPLAY_NOT_FOUND`·`IR_SETTING_NOT_FOUND`·`IR_ACCOUNT_EXISTS`·`IR_PAYLOAD_TOO_LARGE`·`RATE_LIMITED`·공통(`UNAUTHORIZED`·`FORBIDDEN`·`VALIDATION_ERROR`·`SERVICE_NOT_CONFIGURED`) + STATUS_MAP.
- [ ] `lib/api-response.ts`: `successResponse`/`paginatedResponse`/`errorResponse`(+OpenAPI 스키마). raw 모드(IR 호환) 직렬화 헬퍼 `irRaw(data)`.
- [ ] HOF: `withErrorHandling`·`withAuth({getSession})`·`withAdmin`·`withApiToken`(Bearer→user)·`withRateLimit(key, n, window)`.
- [ ] `lib/env.ts`(getEnv+Zod): `DATABASE_URL`·`BASE_URL`·`JWT_SECRET`·`ALLOW_GUEST`·`REPLAY_MAX_BYTES`·`STORAGE_*`·`NODE_ENV`.
- [ ] `db/index.ts`(싱글톤 풀) + `db/schema.ts`([data-model.md]).
- [ ] `compose/index.ts`: `core={db,env}` → `composeShared`(auth·storage·ratelimit) → 도메인 compose 스프레드 병합.
- [ ] `middleware/`: CORS·보안헤더·경로단위 `requireAuth`(/api/auth/me 등).

## 1. system (P0)
- [ ] `GET /api/health` → ServerInfo(capabilities 계산: 구성된 서비스 기준). route만, 인증X.
- [ ] `GET /api/version` → 빌드 버전/커밋(env).

## 2. auth (P0) — beatoraja IRAccount
- [ ] dto: `accountSchema`(id·password·name·email?)·`loginSchema`.
- [ ] `POST /api/auth/register` → better-auth signUp + user row + 토큰. 중복 시 `IR_ACCOUNT_EXISTS`.
- [ ] `POST /api/auth/login` → 검증 → AuthResult(token+player). 실패 `UNAUTHORIZED`. rate-limit.
- [ ] `GET /api/auth/me`(withAuth) → 본인 PlayerProfile.
- [ ] `POST /api/auth/token`(withAuth) → api_token 발급/회전.
- [ ] service: `createAuthService`(register/login/me/issueToken). ServiceDb: user upsert·token insert·token_hash 조회.

## 3. players (P1)
- [ ] `GET /api/players/{id}` → PlayerProfile. 없으면 `IR_PLAYER_NOT_FOUND`.
- [ ] `GET /api/players/{id}/rivals` → PlayerProfile[].
- [ ] `PUT /api/players/{id}/rivals`(withAuth 본인) → rival 테이블 치환.
- [ ] `GET /api/players/{id}/scores`(query: since·limit·mode) → ScoreRecord[](beatoraja getPlayData(player,null)).

## 4. charts (P0 ranking/best, P2 meta)
- [ ] dto: `rankingQuerySchema`(limit·page·rival_of?·lnmode?)·`chartUpsertSchema`(ChartMeta).
- [ ] `GET /api/charts/{hash}` → ChartMeta. 키 길이로 md5/sha256 판별.
- [ ] `POST /api/charts`(withAuth) → chart upsert + md5↔sha256 backfill.
- [ ] `GET /api/charts/{hash}/ranking` → chart_best 정렬(clear,ex desc / bp asc) + user join(player_name) → ScoreRecord[]. rival_of 필터.
- [ ] `GET /api/charts/{hash}/best?player=` → chart_best 단건 또는 null.
- [ ] service: `createChartService`(resolveKey·meta·upsert·ranking·best). ServiceDb: chart select/upsert·chart_best join·rival 필터.

## 5. scores (P0)
- [ ] dto: `scoreSubmissionSchema`(early/late judge·options·seed·avgjudge·empty_poor·합계 미러). `judgeBreakdownSchema`.
- [ ] `POST /api/scores`(withApiToken 또는 guest) → 멱등 upsert(history) + best 갱신(트랜잭션) + chart upsert(없으면) → SubmitResponse(rank·previous_best·is_new_best·score_id).
- [ ] `GET /api/scores/{score_id}` → ScoreRecord 상세.
- [ ] service: `createScoreService`(submit·get). 규칙: EX/BP 유도·멱등·best 비교(램프>EX>BP)·rank 계산. ServiceDb: score insert(onDuplicate)·chart_best upsert·rank count.
- [ ] 검증: 토큰 player == body player(guest 제외), played_at 합리범위, judge 합 = passnotes 일관성.

## 5.5 integrity / 공평 평가 (P0) — scores에 통합
- [ ] dto `playOptionsSchema`: 플레이 전체옵션(gauge·random·random_p2·option·seed·hispeed·constant·green_number·lift·lane_cover·judge_rate·offset_ms·auto_offset·total_override·autoplay·scratch_left·scratch_auto·assist·input_device·judge_algorithm·rule·skin·lntype) — 누락 시 거부/유도.
- [ ] dto `client_build_sha256`(length 64)·`client_platform` 필수(env `REQUIRE_BUILD_HASH`).
- [ ] service `rankedPolicy(options, build)` → `{ranked, flags[]}`: AUTOPLAY·SCRATCH_AUTO·ASSIST·JUDGE_WIDTH(rate≠100)·TOTAL_OVERRIDE·UNKNOWN_BUILD·GUEST.
- [ ] `submit`에서 ranked 산출·flags 저장, 랭킹/best는 ranked=true만.
- [ ] `submission_audit` insert(모든 제출, 거부 포함): build_sha256·platform·ip·ua·accepted·ranked·flags.
- [ ] admin: `GET/POST /api/admin/builds`(withAdmin) — client_build allowlist 등록/조회/trusted. **릴리스 CI가 산출물 SHA-256 자동 POST**(토큰).
- [ ] (후속 FR-12) 리플레이 재시뮬 워커 → `verified`·`SUSPECT_TIMING`.

## 6. replays (P1, µs 정밀)
- [ ] dto: `replayUploadSchema`(format·**events µs**(`{t_us,lane,press}[]`)·event_count·duration_us·재현옵션·client_build_sha256·size). 크기 상한(env `REPLAY_MAX_BYTES`) 초과 `IR_PAYLOAD_TOO_LARGE`(413). **µs 라운딩 금지 검증**.
- [ ] `POST /api/charts/{hash}/replays`(withAuth+rate-limit) → storage put(events µs 무손실) + replay row(event_count·duration_us) + (score_id 있으면 score.replay_id 연결) → {id,url,event_count}.
- [ ] `GET /api/replays/{id}` → ReplayData(events µs 포함, storage get) — 고스트·관전·뷰어·핵분석.
- [ ] `GET /api/charts/{hash}/replays?player=` → ReplayMeta[](events 제외).
- [ ] service: `createReplayService`(upload·get·list) + `storageService`(shared, 파일/S3 추상화).

## 7. settings (P1) — 설정 동기화
- [ ] dto: `settingPutSchema`(format·content·base_updated_at?).
- [ ] `GET /api/settings`(withAuth) → 키 목록.
- [ ] `GET /api/settings/{key}`(withAuth) → blob. 없으면 `IR_SETTING_NOT_FOUND`.
- [ ] `PUT /api/settings/{key}`(withAuth) → 낙관적 잠금 upsert(base_updated_at 불일치 시 409+server 본문).
- [ ] `DELETE /api/settings/{key}`(withAuth) → 204.
- [ ] service: `createSettingsService`(list·get·put·delete). ServiceDb: setting_blob upsert(updated_at 비교).

## 8. courses (P1)
- [ ] dto: `courseSubmissionSchema`·`courseUpsertSchema`.
- [ ] `GET /api/courses/{course_hash}` → CourseMeta.
- [ ] `POST /api/courses`(withAuth) → course+course_chart upsert.
- [ ] `POST /api/courses/{course_hash}/scores`(withApiToken) → course_score 멱등 + course_best 갱신.
- [ ] `GET /api/courses/{course_hash}/ranking` · `/best?player=`.
- [ ] service: `createCourseService`. course_hash 산출 규칙([compatibility.md]).

## 9. tables (P1)
- [ ] `GET /api/tables` · `GET /api/tables/{id}` → TableData(folders·courses).
- [ ] `POST /api/tables`(withAdmin) → table+folder+chart+course upsert.
- [ ] service: `createTableService`. ServiceDb: table 조인 빌드.

## 9.5 FE(web) 전용 조회 (P1) — 향후 FE 프로젝트
- [ ] `GET /api/charts/search`(q·mode·level·sort·page) → ChartMeta[] 공개. dto `chartSearchQuerySchema`.
- [ ] `GET /api/players/{id}/recent` · `GET /api/activity/recent` · `GET /api/leaderboards/players` · `GET /api/stats/summary`(공개, 페이지네이션·envelope).
- [ ] envelope 모드(`?envelope=1` 또는 Accept) — FE는 `{success,data,pagination}`.
- [ ] CORS 미들웨어(env `ALLOWED_ORIGINS`, credentials) + better-auth 세션 쿠키 경로(`/api/auth/*`, OAuth GitHub/Google 선택).
- [ ] service: `createFeQueryService`(search·recent·leaderboard·stats) — 기존 score/chart ServiceDb 재사용.

## 10. (선택) LR2IR 어댑터 (P2)
- [ ] `GET /lr2ir/2/getrankingxml.cgi?id=&songmd5=` → XML(clear 1-5 다운매핑·pg/gr/minbp·notes/combo). chart_best(md5 키) → XML 직렬화.
- [ ] `GET /lr2ir/search.cgi?mode=ranking&bmsmd5=` → 최소 HTML(title). capability `lr2ir_compat`.

## 11. anti-cheat / 검증 (P3, 후속)
- [ ] 리플레이 재생 검증 워커: 제출 score를 seed/options로 재현해 EX/clear 일치 확인 → `verified` 플래그. capability `verified_scores`.

## 12. 테스트 (Bun `bun:test`, 설명 한국어)
- [ ] dto Zod 라운드트립(scoreSubmission early/late·course·setting·replay).
- [ ] service: best 갱신 규칙(램프>EX>BP)·멱등·EX/BP 유도·낙관적 잠금 충돌.
- [ ] route: 인증 게이트(401/403)·404·413·429·랭킹 정렬·rival 필터.
- [ ] 호환: LR2IR XML 다운매핑(슈퍼셋 램프→1-5)·md5↔sha256 backfill.

## 13. 클라이언트(rbms) 측 후속 (별도 — apps/crates)
- [ ] `rbms-ir` DTO 확장: `JudgeBreakdown`에 early/late(epg…lms)·avgjudge·empty_poor, `ScoreSubmission`에 seed·lntype·judge_algorithm·rule·skin·option, `PlayOptions`에 option/judge_rate/offset/constant.
- [ ] `ScoreServer` 트레이트 확장: `get_settings`/`put_settings`·`upload_replay`(이미 있음)·`download_replay`·`register`/`login`/`token`·`course_*`. capability 기반 분기.
- [ ] 플레이어: 로그인 UI, 설정 "서버 동기화" 토글, 리플레이 "서버 업로드/내려받기", 곡선택 랭킹 패널(서버 best/랭킹) — `docs/reference/ir-api.md` 갱신.
