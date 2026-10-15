# ADR: 기록 동기화는 같은 빌드끼리 이미 거절된 판을 저절로 다시 보내지 않는다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효(사용자 미확인) · 결정일 2026-10-15 · 결정자 메인(S19 지시 "잠재적 버그도 다 수정" 아래의 판단, 감사 테스트 부채 3.14) · 커밋 `f81d499`

## 배경

[기록 동기화 ADR](2026-10-14-adr-record-sync.md) 결정 2 는 "거절된 판도 다시 보낸다(서버 수정으로 받아질 수 있다)"였다. 그런데 클라이언트와 서버가 그대로면 같은 본문은 같은 거절을 받는다. 시작할 때마다 모든 거절 판을 다시 보내 요청 한도를 쓰고, 413 으로 거절된 리플레이는 시작마다 리플레이 업로드 몫(분당 10)을 차지했다(테스트 부채 3.14).

## 결정

1. 거절을 기록할 때 그 거절을 받은 클라이언트 빌드와 서버 빌드를 함께 적는다(`ir_sync.refused_under`·`replay_refused_under`, `course_sync.refused_under`. 값은 `refusal_key` = `<클라이언트 빌드>|<서버 빌드>`).
2. 클라이언트 빌드는 이 클라이언트의 버전과 실행 파일 해시다. 서버 빌드는 서버가 `/version` 으로 알리는 commit, commit 이 없으면 `<서버 이름>@<API 세대>`, 답이 없으면 빈 문자열이다(`server_build_of`).
3. 저절로 시작한 동기화(시작할 때, 로그인·가입 직후, Esc 로 포기한 판 하나를 보낼 때)는 두 빌드가 그대로인 거절 판을 건너뛴다(`SyncWalk::refused_under`). 빌드 하나라도 바뀌면 다시 보낸다.
4. 설정 NETWORK 의 `SYNC RECORDS NOW`(수동)는 거절 판을 빌드와 상관없이 모두 다시 보낸다(`plan.by_hand` 이면 `refused_under` 를 넘기지 않는다).
5. 일시 실패(네트워크·5xx·429)와 아직 보내지 않은 판은 이 규칙과 상관없이 다음 동기화에 보낸다.

## 이유

- 같은 본문을 같은 서버에 보내면 같은 거절이 온다. 다시 보내도 받아질 수 있는 때는 서버나 클라이언트가 바뀐 때뿐이다.
- 판을 보내지 않기로 하는 것이 아니다. 거절 판은 기록 DB 에 남고, 빌드가 바뀌면 저절로, 사용자가 원하면 언제든 다시 간다. 사용자 원칙 "실패하면 실패한거만 잘 처리해놔"([기록 동기화 ADR](2026-10-14-adr-record-sync.md))와 "기록 제외 조건을 만들지 않는다"([피드백](../feedback/2026-10-15-do-not-exclude-runs.md))의 범위 안이라고 메인이 판단했다. 사용자에게는 아직 확인받지 않았다(색인의 "확인받지 않은 메인의 결정").

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 지금처럼 시작마다 모든 거절 판을 다시 보낸다(결정 2 원안) | 같은 거절이 반복되고 413 리플레이가 업로드 몫을 매번 쓴다(부채 3.14) |
| 영구 거절 코드(VALIDATION_ERROR 등)는 다시 보내지 않는다 | 서버 수정(예: 空POOR 판정 수 검사, [판정 수 ADR](2026-10-14-adr-judge-count-consumed-notes.md))으로 받아질 판을 영영 보내지 않게 된다 |
| 시간 간격(하루 한 번 등)으로 다시 보낸다 | 빌드가 같으면 몇 번을 보내도 같은 거절이다 |

## 영향과 관련 파일

- `crates/rbms-store/src/scoredb/sync.rs`(모듈 문서, `IrSyncRow::refused_under`, `SyncWalk::refused_under`, `SyncState::Rejected` 문서), `crates/rbms-store/src/scoredb/schema.sql`(`ir_sync.refused_under`·`replay_refused_under`, `course_sync.refused_under`), `apps/rbms-player/src/record_sync.rs`(`refusal_key`, `REFUSAL_KEY_SEPARATOR`, `server_build_of`, `sync_row`, `course_sync_row`, `plan.by_hand`).
- 빌드 열이 생기기 전에 기록된 거절은 `refused_under` 가 없으므로 다음 동기화에 한 번 다시 간다.
- 동작 설명의 정본: `docs/play-options.md` 10.9절.

## 날짜와 사용자 원문

2026-10-15. 사용자 원문 없음(메인 결정). 근거가 된 사용자 지시(S19): "가끔씩 이런건 없어야해 그거포함해서 잠재적 버그도 다 수정하고 기록저장 유저관련 기능 그리고 플레이관련 기능 그리고 플레이시에 렉걸릴 가능성 이런것들을 포함해서 고도화 및 최적화도 제대로 잘 해놔."
