# 웹에서 서버 리플레이를 플레이 화면으로 재생하기 조사 (web-bms 로직 재사용)

> 작성 2026-11-25 · 대응 커밋 R-BMS `9409cc5`, web-bms 작업 트리(커밋 없음) · 상태: 조사만 했고 코드는 바꾸지 않았다.
> 요청: "`/Users/hyunseokbyun/development/web-bms` 의 로직을 가져오면 웹에서도 리플레이를 그대로 재생할 수 있지 않을까, 검토만".

## 0. 결론

1. 가능하다. web-bms 는 R-BMS 의 파서·`to_model`·판정·게이지·`PlaySession`·기본 스킨(Lua)·WebGL2 렌더·WebAudio 를 원본 대조 테스트와 함께 이미 옮겼다. 리플레이 재생에 빠진 것은 세 가지뿐이다: 세션의 리플레이 재생부(`feed_replay`), 레인 배치(`shuffle.rs`), 서버 리플레이를 세션 입력으로 바꾸는 변환.
2. 가장 큰 제약은 로직이 아니라 차트 파일이다. 서버는 리플레이(입력 로그+옵션)와 차트 해시만 갖고 BMS·키음·BGA 를 갖지 않는다(`2026-11-04-ai-training-dashboard/02-training-chart-delivery.md` 0절 4항: 서버는 파일을 서빙한 적이 없다). 그래서 보는 사람이 그 곡 폴더를 브라우저에 넣어야 하고, 넣은 파일의 sha256 이 리플레이의 `chart.sha256` 과 맞을 때만 재생한다. web-bms 에 폴더·zip 넣기(`shared/lib/vfs`)와 `sha256Hex`(`entities/chart/parser/hash.ts:90`)가 이미 있다.
3. "그대로"의 기준은 네이티브 클라이언트가 같은 서버 리플레이를 받아 재생한 결과다. 서버 리플레이는 판정 설정을 `judge_rate` 한 값만 싣고 스크래치 방향(`backward`)이 없어서 네이티브도 `from_ir_replay`(`apps/rbms-player/src/ir_replay.rs:110-127`)로 기본값을 채워 재생한다. 웹도 같은 변환을 쓰면 네이티브와 같은 결과가 나온다. 로컬 `.ron` 리플레이의 원본 판정과 완전히 같다는 뜻은 아니다.
4. 추천 구조: web-bms 에 리플레이 모드를 넣고, R-BMS 웹의 리플레이 상세(`/replays/[replayId]`)에서 그 화면으로 링크한다. web-bms 의 약 250개 파일과 Lua·스킨 자산을 Next.js 앱(`web/`)으로 옮기는 것은 3절 이유로 뒤로 미룬다.

## 1. 지금 있는 것

| 영역 | R-BMS | web-bms |
| --- | --- | --- |
| 서버 리플레이 다운로드 | `GET /api/replays/{id}`, 인증 없음. 본문은 `web/src/server/service/domain/replay/replay.service.ts` `toReplayData`: `chart{md5,sha256}`, `mode`, `random`, `random_p2`, `seed`, `lntype`, `offset_ms`, `judge_rate`, `scratch_auto`, `constant`, `gauge`, `events[{t_us,lane,press}]` | 없음 |
| 웹 리플레이 화면 | `web/src/widgets/replay/replay-waterfall.tsx`: 레인별 입력 시각을 SVG 로 찍는 그림(차트·판정 없음, 최대 4,000개) | 없음 |
| 세션 리플레이 재생 | `crates/rbms-play/src/session.rs`: `ReplayTrack`(:283-300), `tick` 이 `feed_replay(clock.audible_us)` 로 입력을 재생(:666-), 스크래치 방향(:1180-1201) | `PlaySession` 은 옮겼지만 리플레이·분석·셔플 시드 필드는 뺐다(`src/entities/play/play.type.ts:54`). 아키텍처 문서가 "리플레이 저장·재생, 랜덤 배치(shuffle)는 옮기지 않음"으로 명시(`docs/memory/architecture.md` 4절) |
| 레인 배치 | `crates/rbms-chart/src/shuffle.rs`(1,260줄, 테스트 포함). 자체 u64 RNG(:80-105), `lane_permutation`(:126), `apply`(:159), S-RANDOM(:385), 시간 기반 H-RANDOM·ALL-SCRATCH(:435) | 없음 |
| 판정·게이지·LN | 원본 | 옮김, 원본 대조 불일치 0(web-bms `docs/PROCESS.md` 3절). `LnMode` 3종(`src/entities/judge/ln.type.ts:4`), 게이지 9종 |
| 차트 해시 | 원본 | `md5Hex`, `sha256Hex` 있음 |

## 2. 해야 할 일 (web-bms 쪽)

1. 셔플 포팅: `shuffle.rs` 를 옮기고 Rust 테스트도 `bun test` 로 옮긴다. RNG 가 u64 `wrapping_mul` 이라 `BigInt` 또는 32비트 쌍 연산이 필요하다. 시드가 같을 때 배치가 원본과 같은지는 기존 differential 하니스(web-bms `docs/utils/2026-11-25-differential/`)에 셔플 출력을 더해 대조한다.
2. 세션 리플레이 재생: `ReplayTrack` 과 `feed_replay`, `judged_off_by_us`·`offset_steps` 처리, 스크래치 방향 변환을 옮긴다. 리플레이 판은 입력 녹화와 키 입력을 받지 않는다(`session.rs:640`).
3. 서버 리플레이 변환: `from_ir_replay` 와 `note_option_from_ir` 를 그대로 옮긴다. `Converge` 는 네이티브가 OFF 로 바꾸므로(`ir_replay.rs:131-136`) 웹도 같게 하거나 거절한다(조사 보고 `2026-10-24-replay-pattern-play-survey.md` 4.3절은 거절 쪽).
4. 재생 옵션 고정: 배속은 100% 로 강제한다(기록된 판은 모두 100%, `2026-10-29-feature-upgrade/play-rate.md:139`). 게이지·SCRATCH AUTO·LN 모드·RANDOM 은 리플레이 값으로 덮는다(네이티브 `play_as_recorded`, `app_play.rs:66-74`). DP 는 `random_p2` 와 `side_rng` 를 함께 옮긴다.
5. 진입: URL 의 리플레이 id 로 R-BMS 서버에서 리플레이를 받고, 사용자에게 곡 폴더를 넣게 한 뒤 sha256 으로 차트를 고른다. 다른 오리진에서 받으므로 `/api/replays/{id}` 에 CORS 허용이 필요하다(지금 설정은 확인하지 않았다).

## 3. 대안: R-BMS 웹(Next.js)에 직접 넣기

- 장점: 리플레이 상세 화면 안에서 바로 재생, CORS 불필요.
- 단점: web-bms 는 Vite 8 + React Compiler + wasmoon(Lua WASM) + WebGL2 이고 파일 약 250개와 `public/skin/rbms-default` 자산을 갖는다. Next.js 의 클라이언트 전용 동적 import, WASM 경로, 스킨 자산 배포(Vercel 정적 자산 용량)를 새로 맞춰야 하고, 같은 로직이 두 저장소에 생겨 원본 대조 테스트도 두 벌이 된다.
- 판단: web-bms 에서 먼저 동작을 확인하고, 웹 통합이 필요해지면 web-bms 를 패키지로 묶어 `web/` 이 가져다 쓰는 방식을 검토한다. 라이선스는 둘 다 GPL-3.0 이라 충돌이 없다.

## 4. 남는 차이와 위험

- 차트 파일 없이는 재생할 수 없다. 곡 미러(저작권 곡 재배포)는 보류 상태라 서버가 차트를 내려주는 방식은 이 작업 범위 밖이다.
- 서버 리플레이는 판정 설정 손실과 스크래치 방향 손실이 있다(1절 3항). 역회전 스크래치가 있던 판은 같은 방향 재입력으로 재생돼 판정이 원 기록과 다를 수 있다. 이건 네이티브도 같다.
- `lntype` 이 정의 없는 LN 의 처리 방식을 바꾸므로 숫자→`LnMode` 매핑을 네이티브와 맞춰야 한다(매핑 위치는 이번에 확인하지 않았다).
- 재생 결과가 기록 점수와 같은지 검증하는 수단이 필요하다. 리플레이의 `score_id` 로 점수 상세를 받아 EX·판정 수를 비교하면 된다. 네이티브의 `ghost.rs` `rejudge` 도 같은 목적의 선례다.
- 확인하지 않은 것: `/api/replays/{id}` 의 CORS 헤더, `lntype` 숫자 정의, `constant` 플래그가 재생에 영향을 주는지.
