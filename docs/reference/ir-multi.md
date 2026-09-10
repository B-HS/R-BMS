# 다중 IR 프로필 — 제출 팬아웃과 primary 축

> 대상: `apps/rbms-player/src/ir_ext.rs`(`MultiIr`). 와이어 계약 정본은 [`docs/reference/ir-api.md`](./ir-api.md), 트레이트 정본은 `crates/rbms-ir/src/lib.rs` 의 `ScoreServer` 다.
> 스펙: `docs/plan/2026-09-09-phase-g-spec.md` §10. 배선 지시는 `docs/plan/phase-g-wiring/g7.md`.

한 플레이어가 여러 IR 에 계정을 동시에 가질 수 있다. 이 문서는 **그 여러 서버를 하나의 축으로 묶는 규칙**만 다룬다. 새 엔드포인트도, 새 트레이트 메서드도 없다 — `ScoreServer` 구현을 여러 개 들고 있을 뿐이다.

---

## 1. 프로필 목록

`MultiIr.profiles` 는 **표시 순서 = 제출 순서**다. `MultiIr::from_network` 가 설정에서 이 순서를 만든다.

| 순서 | 출처 | 비고 |
|---|---|---|
| 0 | `network.server_url` + `network.ir_token` | 기존 단일 서버. 이름은 `MAIN`(`LEGACY_PROFILE_NAME`). `server_url` 이 `None` 이면 이 행이 없다 |
| 1.. | `network.ir_profiles[..]` | 저장된 순서 그대로 |

- **`ir_profiles` 가 비면 프로필은 레거시 1개뿐**이고, 제출도 랭킹 조회도 단일 서버 시절과 완전히 동일하다(스펙 §10 "빈 벡터면 기존 단일 서버 동작").
- `server_url` 이 없고 `ir_profiles` 도 비면 프로필이 0개다. `submit_all` 은 빈 벡터를, `primary_server()` 는 `None` 을 돌려준다(오프라인).
- 레거시 행이 0번이라는 사실은 `has_legacy` / `legacy_index()` 로 노출된다. 이 인덱스는 **이미 열려 있는 메인 클라이언트를 재사용**하기 위한 것이다(§4).
- 이름이 빈 프로필은 `IR 1`·`IR 2` 처럼 **1-based 위치**로 라벨링된다(`profile_label`). 라벨은 UI 탭과 실패 보고에 그대로 쓰이므로, 이름이 없어도 어느 서버인지 구분된다.

## 2. 제출 — 병렬 팬아웃, 프로필 단위 격리

`MultiIr::submit_all(&servers, &sub)` 의 계약:

1. **`enabled == false` 프로필은 건너뛴다.** 프로필은 남아 있고 제출만 안 한다. 결과 벡터에도 나타나지 않는다.
2. 남은 프로필마다 **스레드 하나**(`std::thread::scope`)로 `submit_score` 를 동시에 호출한다. n개 프로필의 총 대기시간은 합이 아니라 최댓값이다.
3. 결과는 `Vec<(라벨, Result<SubmitResponse, IrError>)>` 이며 **프로필 순서를 보존**한다(완료 순서가 아니다).
4. **실패는 그 프로필에 갇힌다.** 하나가 타임아웃·401·500 을 내도 나머지 결과는 그대로 반환된다. 이것이 §10 의 유일한 필수 요건이다(스펙 §11 순서 12).
5. 서버 구현이 **패닉**해도 그 프로필만 `IrError::Network("profile worker panicked")` 이 되고 팬아웃은 완주한다.
6. `servers` 가 프로필보다 짧아 해당 인덱스에 클라이언트가 없으면 그 프로필은 `IrError::NotConfigured` 다.
7. `ScoreSubmission::clamp_to_server_bounds` 는 **팬아웃 전에 한 번만** 적용한다. 모든 프로필이 `rbms_ir::spawn_submit` 이 보냈을 바이트와 동일한 것을 받는다.

프레임 스레드에서 직접 부르지 않는다. `spawn_submit_all(multi, servers, sub)` 이 워커 스레드로 넘기고 프레임 루프는 `try_recv` 로 폴링한다(`rbms_ir::spawn_submit` 과 같은 모양).

### 요약 한 줄

`submit_summary(&results)` 가 결과 벡터를 상태 줄로 접는다.

| 상황 | 문자열 |
|---|---|
| 프로필 0개 | `no IR profiles` |
| 전부 수용 | `IR 2/2` |
| 하나 실패 | `IR 1/2 — B: <서버 메시지>` |
| 수용 거부(`accepted == false`) | `IR 0/1 — A: rejected` |

첫 번째 문제 프로필만 이름으로 지목한다. 서버 메시지는 `ir_outcome::short_error` 로 한 줄에 맞춰 잘린다.

## 3. 조회 — primary 프로필 하나만

랭킹 패널·라이벌 행은 **primary 프로필에만** 질의한다. n개 서버에 동시 질의하면 곡 커서를 움직일 때마다 요청이 n배가 되고, 한 곡의 순위를 서버별로 합칠 의미도 없기 때문이다.

`primary_server() -> Option<usize>`:

1. `primary` 가 범위 안이고 그 프로필이 `enabled` 면 그것.
2. 아니면 **첫 번째 enabled 프로필**. (선택된 탭이 비활성화됐다는 이유로, 살아 있는 서버를 두고 패널을 비우지 않는다)
3. enabled 가 하나도 없으면 `None` → 패널을 숨긴다.

탭 전환은 `set_primary(index)` 다. 프로필이 아닌 인덱스는 거부(`false`)하므로, 프로필이 줄어든 뒤 남아 있던 탭 인덱스가 `primary` 를 범위 밖으로 밀지 못한다.

**코스 랭킹도 primary 전용**이다(스펙 §10). `ScoreServer::course_ranking` 의 기본 구현은 `Err(IrError::Unsupported)` 이고, 이는 오류가 아니라 "이 서버는 코스 랭킹이 없다" 는 뜻이므로 패널을 숨긴다(에러 토스트 금지).

## 4. 클라이언트 생성

`build_servers()` 는 프로필과 **인덱스가 1:1로 정렬된** `Vec<Arc<dyn ScoreServer>>` 를 만든다.

- 정상 프로필 → `HttpScoreServer::try_new(base_url, token)`.
- `enabled == false`, `base_url` 이 빈 문자열, HTTP 클라이언트 빌드 실패 → `NullScoreServer`. 정렬이 깨지지 않고, 그 프로필에 대한 호출은 엉뚱한 서버로 가는 대신 `NotConfigured` 로 즉시 실패한다.

`build_servers_reusing(main)` 은 레거시 인덱스만 이미 만들어 둔 `Arc<dyn ScoreServer>`(= `AppShared::server`, health 프로브와 랭킹 패널이 공유하는 그것)로 바꿔 끼운다. 같은 URL 로 HTTP 클라이언트를 두 벌 열지 않기 위한 것이다.

**토큰은 프로필마다 따로다.** 각 서버는 자기 프로필의 `token` 으로만 인증한다. 한 서버의 베어러 토큰이 다른 서버로 새지 않으며, `token: None` 프로필은 그 서버에서 게스트다(`GUEST_PLAYER_ID` 규칙은 [`ir-api.md`](./ir-api.md) §1 그대로).

## 5. 경계

- **리플레이 업로드는 팬아웃하지 않는다.** `submit_all` 은 `submit_score` 만 부른다. 리플레이는 스코어 id 에 묶이므로(`rbms_ir::spawn_submit`) 서버마다 다른 id 가 생기고, n개 서버에 같은 리플레이를 올리는 것은 대역폭만 쓴다. 리플레이는 primary 프로필 경로(기존 `spawn_submit`)가 그대로 담당한다.
- **설정 동기화·라이벌·계정 로그인도 팬아웃하지 않는다.** 전부 primary(레거시) 프로필의 계정 개념이다.
- 프로필별 `player_id` 는 없다. 제출 id 는 `network.player_id` 하나이고, 프로필은 서버·토큰만 가른다.
- `primary` 는 설정 파일에 저장되지 않는다. 앱을 다시 켜면 첫 번째 enabled 프로필로 돌아간다.
