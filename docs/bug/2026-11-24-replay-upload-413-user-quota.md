# 리플레이 업로드가 413 "요청 본문이 허용 크기를 초과했습니다" 로 전부 거절됨 (사용자별 개수 한도 도달)

최종 갱신 2026-11-24 · 상태: 수리함(한도 상향, [ADR](../acknowledge/2026-11-24-adr-replay-user-quota-raised.md))

## 대상 파일

- `web/src/server/dto/replay.ts` (`REPLAY_USER_QUOTA`. 진단 때 값은 `{ COUNT_MAX: 2000, TOTAL_BYTES_MAX: 512MiB }`)
- `web/src/server/service/domain/replay/replay.service.ts` (`findReplayQuotaExceeded`, `upload`)
- `web/src/server/route/replay.ts` (`result.quotaExceeded` 를 `IR_PAYLOAD_TOO_LARGE` 로 던지는 줄)
- `web/src/server/lib/error-message.ts` (`IR_PAYLOAD_TOO_LARGE` 문구)
- `apps/rbms-player/src/record_sync.rs` (리플레이 단계, `UploadVerdict::Rejected`)

## 증상

2026-11-23 02:19 부터 친 모든 판의 리플레이 업로드가 `payload too large (413): 요청 본문이 허용 크기를 초과했습니다. (IR_PAYLOAD_TOO_LARGE)` 로 거절된다. 점수 제출은 정상으로 수락된다. 로컬 리플레이 파일(`~/.config/rbms/replays/*.ron`)도 정상으로 써진다(거절 62건 모두 파일이 있다). 안 되는 것은 서버 업로드뿐이다.

## 원인

서버의 사용자별 리플레이 개수 한도 2,000개에 닿았다. 본문 크기와는 관계없다.

`POST /api/charts/{hash}/replays` 가 413 을 내는 경로는 둘이다.

| 경로 | 조건 | 이번 경우 |
| --- | --- | --- |
| 본문 크기 | 요청 본문이 `REPLAY_MAX_BYTES`(기본 4MiB) 초과 | 아님 |
| 사용자별 저장 한도 | 새 리플레이를 만들 때 개수 2,000개 또는 합계 512MiB 초과 | 개수 한도 |

두 경로가 같은 오류 코드와 같은 문구를 쓰고, 운영 응답은 `details.scope`(`replay_count` / `replay_bytes`)를 싣지 않는다. 그래서 클라이언트에는 본문이 크다는 문구만 보인다.

### 근거 (로컬 `scoredb.sqlite` 의 `ir_sync`, 2026-11-24 조회)

- `replay_state`: uploaded 2,114, rejected 62, none 8, skipped 5. 거절 62건의 `replay_error` 는 모두 위 413 문구다.
- 경계가 한 점이다. `log_id` 2164(2026-11-23 02:17:16)까지는 전부 uploaded, 2165(02:19:09)부터는 전부 rejected 다. 크기 문제라면 큰 판만 골라서 거절된다.
- 거절된 판의 로컬 파일 크기는 122,888~717,522바이트(중앙값 481,713)로, 정상 업로드된 판(중앙값 455,945, 최대 870,993)과 같은 범위다.
- 전송 본문은 로컬 RON 의 약 3분의 1 이다. 이벤트 하나가 RON 으로 약 122바이트, 전송 JSON 으로 약 41바이트다. 로컬에 남은 업로드 완료 판 1,092개로 계산하면 이벤트 3,976,435개, 전송 JSON 약 161.9MB, 판당 평균 약 148KB 다. 가장 큰 판도 약 300KB 라 4MiB 한도의 10분의 1 이하다.
- 합계 바이트는 한도 아래다. 판당 148KB 로 2,000개면 약 283MiB 다(한도 512MiB).
- 개수가 정확히 맞는다. 로컬의 uploaded 2,114개 가운데 서버에서 지워진 것이 114개다: 架空ユリカゴ [SABOTHER] 57개([ADR](../acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md), `replay` 57행 삭제)와 PopCoRian -EX- 57개([ADR](../acknowledge/2026-11-02-adr-popcorian-ex-removed.md), `replay` 57행 삭제). 2,114 − 114 = 2,000 = `COUNT_MAX`. `findReplayQuotaExceeded` 는 `count + 1 > 2000` 이면 거절하므로 2,001번째부터 전부 413 이 된다.

운영 DB 의 행 수를 직접 세지는 않았다(접속 정보는 읽지 않는다). 위 수치는 로컬 기록과 purge 실행 기록에서 나온 것이다.

### 왜 이렇게 빨리 닿았나

한도 값은 2026-10-15 S19 에서 메인이 정했고 사용자 확인을 받지 않았다([테스트 부채 3.34](../quality-assurance/2026-10-14-test-debt.md): "값이 현실과 안 맞으면 정상 사용자의 업로드가 413 이 된다"). 그때의 가정은 7K 한 판 약 60KB 였는데 실제 평균은 약 148KB 다. 모든 판을 저장하고 보내는 원칙([피드백](../feedback/2026-10-15-do-not-exclude-runs.md)) 아래에서 11월 업로드는 23일 동안 598개(하루 약 26개, 약 3.8MiB)였다. 2026-10-15 부터 39일 만에 2,000개가 찼다.

## 해결

사용자가 한도 상향을 골랐다(2026-11-24, "1"). `REPLAY_USER_QUOTA` 를 20,000개·4GiB 로 올렸다. 마이그레이션은 없다. 결정 기록: [ADR](../acknowledge/2026-11-24-adr-replay-user-quota-raised.md).

진단 때 낸 세 안:

| 안 | 내용 | 결과 |
| --- | --- | --- |
| 한도 상향 | `REPLAY_USER_QUOTA` 를 예: 20,000개·4GiB 로 | 상수 한 줄과 웹 배포. 현재 속도로 약 2년. 운영 DB 여유 용량 확인이 필요하다 |
| 서버 순환 삭제 | 한도에 닿으면 차트별 최고 기록·최근 N개 밖의 오래된 리플레이를 지움(클라이언트 `ReplayPolicy` 와 같은 방식) | 다시 막히지 않는다. 서버의 옛 리플레이가 지워진다 |
| 개수 한도만 제거 | 512MiB 는 유지 | 약 3,600개(현재 속도로 약 2개월 뒤)에서 `replay_bytes` 로 다시 막힌다 |

고치지 않고 남긴 것: 저장 한도 거절이 본문 크기 초과와 같은 문구를 쓴다(원인을 가린다. 요청 범위 밖이라 다음 작업 후보로만 둔다).

## 수리 뒤 거절된 62건

다시 올라간다. 거절 판은 `rejected` 로 남아 있고 로컬 파일도 있다. 웹을 배포하면 서버 빌드(commit)가 바뀌어 저절로 시작하는 동기화가 다시 보낸다([거절 재전송 ADR](../acknowledge/2026-10-15-adr-record-sync-refusal-resend.md) 결정 3). 설정 NETWORK 의 `SYNC RECORDS NOW` 는 빌드와 상관없이 다시 보낸다(결정 4). 시작할 때의 로컬 리플레이 정리는 아직 올라가지 않은 판을 남긴다([play-options.md](../play-options.md) 리플레이 보존).

## 날짜와 사용자 원문

2026-11-24. "지금 리플레이저장이 payload가 너무크다면서 저장이안되고있어 확인해봐 왜그런지"
