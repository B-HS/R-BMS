# G-02 — rbms-ir 크레이트 ↔ beatoraja ir/ 대조

조사 범위: `crates/rbms-ir/src/{lib.rs,dto.rs,http.rs,null.rs}` 전량(1,549줄), `examples/ir_probe.rs`, `docs/reference/ir-api.md`, `docs/backend/{api-spec.md,compatibility.md,contract-freeze.md}`, beatoraja `src/bms/player/beatoraja/ir/` 16 클래스(2,323줄) + 호출부(`MainController`, `select/MusicSelector`, `play/TargetProperty`, `PlayerResource`), rbms 소비측(`apps/rbms-player/src/{main.rs,app_play.rs,app_select.rs,app_input.rs,settings.rs,ir_map.rs}`, `crates/rbms-render/src/{hud.rs,select.rs,result.rs}`).

---

## Q1. rbms `ScoreServer`/DTO 가 전제하는 프로토콜과 런타임 성질

### 1-1. 프로토콜 정체 = **rbms 자체 REST/JSON "IR 슈퍼셋"** (LR2IR도 beatoraja IR도 아님)

| 항목 | 근거 |
|---|---|
| 트레이트 선언 | `crates/rbms-ir/src/lib.rs:46` `pub trait ScoreServer: Send + Sync` — 8 필수 + 6 default(=`Unsupported`) 메서드 |
| 자기 규정 | `lib.rs:38-45` doc: "intentionally a *superset* of what BMS IRs (LR2IR, Mocha, Cinnamon, …) expose" |
| 와이어 | `http.rs:1-64` reqwest **blocking** + JSON, `Cargo.toml:9` `features=["blocking","json","rustls-tls"]` |
| 경로 | `http.rs:69-125` `/health`·`/scores`·`/charts/{md5}/ranking?limit=`·`/charts/{md5}/best?player=`·`/players/{id}`·`/players/{id}/rivals`·`/courses`·`/charts/{md5}/replays`·`/courses/{hash}/ranking`·`/replays/{id}`·`/players/{id}/settings/{name}`(GET/PUT)·`/auth/register`·`/auth/login` |
| 서버 부재 | `docs/backend/contract-freeze.md:1-10` — 백엔드는 **미구현**(별 MIT 레포 예정, 상태 "대기"). 즉 현재 이 트레이트를 만족하는 실서버는 존재하지 않음(미확인 서버 제외) |
| LR2IR/beatoraja 관계 | 직접 호환 아님. `docs/backend/compatibility.md:1-119` 가 **매핑표**로만 존재(§2 램프 다운매핑, §3 early/late 12필드, §6 `IRConnection` 메서드↔rbms 엔드포인트, §7 LR2IR `getrankingxml.cgi`는 "어댑터 대상", `gateway.cgi` 제출은 "재현 비범위") |

DTO 슈퍼셋 요소: `ChartId{md5,sha256}`(`dto.rs:7-11`), `JudgeBreakdown`의 beatoraja 12필드 `epg..lms`+`avgjudge`+rbms 고유 `empty_poor`(`dto.rs:64-92`), `PlayOptions`의 `option`(beatoraja 원시 비트마스크)·`seed`·`judge_algorithm`·`rule`·`skin`·`client_build_sha256`(`dto.rs:104-129,150-170`), 모든 DTO의 `extra: HashMap<String,Value>`, `ServerCapabilities`(`dto.rs:196-204`).

### 1-2. 인증·재시도·큐·캐시·타임아웃·blocking

| 축 | 현재 상태 | 근거 |
|---|---|---|
| 인증 방식 | Bearer 토큰 1종. `AuthRequest{id,password,email?,name?}`→`AuthResponse{token,player,name}` | `dto.rs:255-283`, `http.rs:29-32,41-44,55-58` `bearer_auth` |
| 인증 **배선** | **없음**. 앱이 항상 `HttpScoreServer::new(url, None)` → 토큰 영구 `None`, `login`/`register` 호출부 0곳 | `apps/rbms-player/src/main.rs:173`; `settings.rs:35-38`에 `server_url`/`player_id`만 존재(토큰·비밀번호 필드 없음) |
| 재시도 | **없음**. 1회 전송, 실패 즉시 `IrError::Network/Server` | `http.rs:33-37,45-49` (백오프·재시도 코드 부재) |
| 오프라인 큐 | **없음**. 제출 실패는 `eprintln!` 후 소실 | `apps/rbms-player/src/app_play.rs:311-315` `spawn(... Err(e)=>eprintln!("score submit: {e}"))` |
| 캐시 | **없음**(크레이트·앱 양쪽). 랭킹/베스트/프로필 결과를 보관하는 구조체 부재 | `rbms-ir` 전량에 cache 심볼 없음; 앱 상태 구조체(`main.rs:659-733`)에도 IR 캐시 필드 없음 |
| 타임아웃 | 5초 — 단 **빌더 실패 시 무제한으로 조용히 강등**(`build().unwrap_or_default()`, 기본 클라이언트는 timeout 없음) | `http.rs:19` |
| blocking 여부 | 트레이트 전체가 **동기 blocking**. 비동기화는 호출부의 `std::thread::spawn` 책임 | `Cargo.toml:9`; `main.rs:181-186`(health 5초 폴링 루프), `app_play.rs:311`(결과 진입 시 제출) |
| 연결 표시 | `health()` 결과를 `AtomicBool`에 저장, 5초 주기 | `main.rs:177-187`, `app_play.rs:743` |
| 차트 키 | URL 은 **md5**(DTO 는 md5+sha256 동시 보유) — beatoraja 는 sha256 키 | `http.rs:76,80,101`; `beatoraja IRScoreData.java:19`, `RankingDataCache.java:41` |
| 미구현 표면 | `tables` capability 플래그는 있으나 **트레이트에 tables 메서드 없음** | `dto.rs:202` vs `lib.rs:46-95` |
| 사소 결함 | `player.id`/`name`을 URL 에 **미인코딩 삽입**(퍼센트 인코딩 의존성 없음) | `http.rs:80,84,88,113,117` |
| 테스트 | serde 왕복/`Display`/dyn-safe 위주. **HTTP 계층 테스트 0** (서버 목 없음) | `lib.rs:98-355`, `null.rs:34-227`, `http.rs` 내 `#[cfg(test)]` 없음 |

---

## Q2. beatoraja `ir/` 아키텍처 대비 격차

| 축 | beatoraja | rbms 현재 | 격차 |
|---|---|---|---|
| 구현 탐색 | `IRConnectionManager` 가 jar/디렉터리를 스캔해 `IRConnection` 구현 + `public static NAME` 필드를 런타임 발견 (`IRConnectionManager.java:82-107,150-178`) | 컴파일타임 `impl ScoreServer` 2종(`HttpScoreServer`/`NullScoreServer`), URL 1개 | 플러그인 로딩 없음(설계 선택) |
| **프로세스 격리** | `IsolatedIRConnection` 이 별도 JVM(`IRWorkerMain`)을 띄우고 stdin/stdout **1행 JSON RPC**로 통신, 요청 타임아웃 30초, 리더 전용 데몬 스레드 (`IsolatedIRConnection.java:24-40,45-115`, `IRWorkerMain.java:33-49`, `IRWorkerProtocol.java` 551줄) | 인프로세스 reqwest 호출 | **큼**. rbms 는 IR 코드가 서드파티가 아니라 자체 HTTP 클라이언트라 격리 필요성 낮음. 다만 30초 상한/응답 스트림 보호(`System.setOut(System.err)`) 같은 방어는 없음 |
| 복수 IR | `IRStatus[]` 배열(`MainController.java:92,166-182`) — 플레이어 설정의 `IRConfig` 마다 로그인 시도. **단 랭킹 조회는 `ir[0]` 고정**(`RankingData.java:73-79`) | 단일 `Arc<dyn ScoreServer>`(`main.rs:169-176`) | 중간. 실사용상 beatoraja 도 사실상 1개 |
| 기동 시 로그인 | `ir.login(new IRAccount(userid,password,""))` 성공분만 `IRStatus` 등록(`MainController.java:167-181`) | 없음(토큰 `None`) | **큼** |
| **랭킹 캐시** | `RankingDataCache` — `ObjectMap<String,RankingData>[4]`, 인덱스 = `song.hasUndefinedLongNote() ? lnmode : 3`, 키 = **sha256**(코스는 구성곡 sha256+제약명 결합의 SHA-256). 곡 2000/코스 100 초기용량, **TTL·eviction 없음** (`RankingDataCache.java:22-32,38-45,68-78,80-104`) | 없음 | **큼** |
| 갱신 주기 | 커서가 멈춘 뒤 `currentRankingDuration` 경과 시 1회 트리거(`MusicSelector.java:223-245`). 트리거마다 `irc.load()` 재호출 → 캐시는 객체 재사용(랭크 diff 유지)이고 **네트워크는 매번 재조회** | 없음 | — |
| 조회 스레드 모델 | `RankingData.load()` 가 요청마다 `new Thread`, 상태를 `NONE/ACCESS/FINISH/FAIL` + `lastUpdateTime` 로 노출(`RankingData.java:56-95,213-229`) | 앱이 health/제출용 스레드만 생성 | 패턴은 그대로 이식 가능 |
| 랭킹 후처리 | 클라이언트에서 EX 내림차순 정렬, 동점 동순위, `YOU/RIVAL` 태그, 램프 히스토그램 11칸, `localrank`(로컬 베스트의 가상 순위), `prevrank` (`RankingData.java:96-140`) | 없음 | **큼**(서버가 rank 를 주더라도 로컬랭크/이전랭크/램프분포는 클라 계산) |
| 응답 모델 | `IRResponse<T>{isSucceeded,getMessage,getData}`(`IRResponse.java`), `SimpleIRResponse` | `Result<T, IrError>` (`lib.rs:16-36`) | 동등. 단 rbms 는 성공 시 서버 메시지를 못 싣음(`SubmitResponse.message` 예외) |
| 스코어 DTO | `IRScoreData` 32 필드(early/late 12 + `option`/`seed`/`assist`/`gauge`/`deviceType`/`judgeAlgorithm`/`rule`/`skin`) (`IRScoreData.java:19-115`) | 제출용 `ScoreSubmission` 은 전부 커버. **조회용 `ScoreRecord` 는 9필드뿐**(`dto.rs:167-178`: player/name/clear/ex/max_combo/minbp/rank/played_at/extra) | **큼** — 랭킹 행에 판정·옵션·lntype 없음 |
| 부가 기능 | `getTableDatas`(IR 표), `getVersionInfo`, `getIllegalSongs` 기본 메서드(`IRConnection.java:40,116,128`), 라이벌 DB 임포트(`RivalDataAccessor.java:62-135`) | tables/version/illegal 메서드 없음(capability 플래그만), rivals 는 트레이트에 있으나 미호출 | 중간 |
| target(목표) 연동 | `TargetProperty` IR 모드 `NEXT/RANK/RANKRATE` → `ranking.getScore(index).getExscore()` 로 목표 EX 설정, 미완료 시 500ms 폴링 스레드(`play/TargetProperty.java:363-414,417-430`) | 없음. 인게임 그래프는 **로컬 BEST 전용**(`crates/rbms-render/src/hud.rs:22,27-52` `best_ex`) | **큼** |

---

## Q3. "곡선택 랭킹 패널 + 인플레이 target 그래프" 최소 변경 목록

전제: 서버가 아직 없으므로(§contract-freeze) 배선은 **`NullScoreServer`/타임아웃 경로에서 조용히 비활성**되어야 한다.

| # | 변경 | 위치(신규/수정) | 내용 | 효력 |
|---|---|---|---|---|
| 1 | 랭킹 캐시 + 상태 | 신규 `apps/rbms-player/src/ir_ranking.rs` | `RankingState{Idle,Loading,Ready,Failed}` + `HashMap<(String sha256, u8 lnmode), RankingEntry>`. `RankingEntry{rows:Vec<ScoreRecord>, my_rank, local_rank, prev_rank, lamps:[u32;11], fetched_at}` — beatoraja `RankingData.java:96-140` 후처리 이식 | **M** |
| 2 | 비동기 조회 스레드 | 위 파일 + `main.rs`(App 필드) | `mpsc::Sender/Receiver`로 결과 수신(앱이 이미 `preview_prep_rx` 동일 패턴 사용: `main.rs:707-712`). 워커에서 `server.chart_ranking(&chart, limit)` + `player_best` 호출 | **M** |
| 3 | 포커스 디바운스 트리거 | `apps/rbms-player/src/app_select.rs:296-305` | 프리뷰의 `PREVIEW_DEBOUNCE_FRAMES` 디바운스를 그대로 재사용해 커서 정지 후 1회 조회(= beatoraja `currentRankingDuration`) | **S** |
| 4 | 캐시 키 결정 | `crates/rbms-ir/src/http.rs:76,80` | 현재 URL 키가 md5. beatoraja/스펙은 sha256 우선(`compatibility.md:5-11` "길이로 판별"). **캐시 키는 sha256+lnmode 로 고정**하고 URL 키는 서버 계약 확정 시 일치시킴 | **S**(캐시) / **M**(URL 전환 시 계약 변경) |
| 5 | `ScoreRecord` 필드 보강 | `crates/rbms-ir/src/dto.rs:167-178` | `#[serde(default)]`로 `lntype:i32`, `option:i64`, `judge:Option<JudgeBreakdown>`, `total_notes:u32` 추가(모두 default라 하위호환). 램프 히스토그램·판정 표시·목표 옵션 표기에 필요 | **S** |
| 6 | 랭킹 패널 뷰 | `crates/rbms-render/src/select.rs:79-92` `DetailView` | `pub ir: Option<IrRankingView>` 추가 — `{state, rows:Vec<IrRowView{rank,name,ex,lamp,is_you,is_rival}>, my_rank, local_rank, total_players}`. `RecordsView` 아래 섹션으로 렌더 | **M** |
| 7 | 인플레이 target | `crates/rbms-render/src/hud.rs:8-23,27-52` | `HudView`에 `target_ex: Option<u32>` + `target_label: String` 추가, `draw_score_graph`에 3번째 막대(TARGET) + "vs TARGET" 델타. 현재 시그니처가 `(ex,max_ex,best)`뿐이라 확장 필요 | **M** |
| 8 | target 소스 배선 | `apps/rbms-player/src/app_play.rs:232` | 플레이 진입 시 캐시 엔트리에서 목표 EX 산출(`NEXT n / RANK n / TOP n%` — beatoraja `TargetProperty.java:417-430` 이식). 캐시 미스면 target 없이 진행(폴링 스레드 만들지 않음 — beatoraja 의 500ms 폴링은 이식하지 않는 편이 단순) | **M** |
| 9 | 설정 항목 | `apps/rbms-player/src/settings.rs:35-38` + NETWORK 탭(`main.rs:605`, `app_select.rs:913`) | `ir_ranking: bool`, `ir_ranking_limit: u32`(기본 50), `ir_target: String`("OFF"/"NEXT1"/"RANK1"/"TOP10%"), `ir_token: Option<String>`(또는 `ir_password`) | **S** |
| 10 | 인증 배선 | `apps/rbms-player/src/main.rs:169-176` | `HttpScoreServer::new(url, token)` 에 실제 토큰 주입 + 기동 시 `login()` 1회(실패해도 guest 로 진행). 미배선 상태로는 Bearer 필요 서버에서 랭킹이 전부 401 | **M** |
| 11 | 타임아웃/실패 내성 | `crates/rbms-ir/src/http.rs:19` | `unwrap_or_default()` 로 인한 **무제한 타임아웃 강등** 제거(빌더 실패를 `IrError`로 표면화하거나 명시적 타임아웃 재설정). 랭킹 조회는 UI 스레드 밖이지만 스레드 누수 방지에 필요 | **S** |
| 12 | (선택) 오프라인 큐 | 신규 `apps/rbms-player` | 제출 실패분을 `scores` 옆 파일에 적재 후 health 복구 시 재전송. 랭킹 패널과 독립 | **L** |

권장 순서: 11 → 5 → 1·2·3(조회 파이프) → 6(패널) → 9 → 10 → 7·8(target) → 12.
합계 체감: 패널만이면 **M**, target 그래프까지면 **M+**, 인증·큐 포함 시 **L**.

---

## 미확인
- `IRWorkerProtocol.java`(551줄)는 메서드 상수·DTO 변환만 훑었고 전량 통독하지 않음.
- rbms 백엔드 실서버는 존재하지 않아(계약 동결 "대기") **런타임 동작 검증 불가** — 위 결론은 전부 정적 코드 근거.
- `docs/backend/api-spec.md` 는 헤딩·표 단위로만 확인(369줄 중 요지).
