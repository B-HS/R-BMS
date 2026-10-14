# ADR: SCRATCH AUTO 규칙 — 초록, 없는 노트, 똑같이 저장·전송

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(램프 LIGHT ASSIST 유지는 메인이 정함, 사용자 미확인)
> 대체하는 결정: 같은 날의 "SC 점수 빼고 저장" 설계, `2026-09-09-enhancement-decisions.md` 결정 12 의 SCRATCH AUTO 부분

## 배경

- 사용자 보고(2026-10-14): 로그인하지 않으면 기록이 램프만 저장되는 것 같다. 웨이브 16 이 사용자 실제 DB 의 읽기 전용 사본으로 원인을 확정했다. 로그인이 아니라 SCRATCH AUTO 였다. `scorelog` 37판 전부 assist=1, 최고 기록 29행 전부 램프만. 결정 12(원본 `ScoreData.update` 이식)대로 어시스트 판은 램프·횟수·이력만 남겼다.
- 원본(beatoraja)의 SCRATCH AUTO 는 턴테이블 노트를 대신 쳐 주고(그 노트도 PGREAT 로 센다) 판을 어시스트로 강등한다.
- 사용자는 처음에 "당연히 갱신. 다만 SC auto 인경우 SC의 점수는 빼고 저장"으로 답했고, 메인은 스크래치 몫을 뺀 별도 최고 기록 행을 두는 설계로 웨이브 18 을 띄웠다(`wf_f22147c8-bb6`, 파일 변경 전에 멈춤). 사용자가 같은 날 규칙을 다시 정리했다(아래 원문). 이 ADR 은 그 재정리가 정본이다.

## 결정

1. SCRATCH AUTO 를 켜면 턴테이블 노트는 초록 단색으로 그려지고 키음만 울린다. 색은 `AUTO_LANE_NOTE`(`#30E050`, `crates/rbms-render/src/skin_render/notes.rs`).
2. 그 노트는 판정할 차트에서 빠진다(원본 `AutoplayModifier` 가 배경으로 옮기는 것과 같다). 판정·콤보·게이지·EX·노트 수 어디에도 들어가지 않는다. 틀린 노트(POOR)가 아니라 존재하지 않는 노트로 처리한다. 턴테이블 입력은 무시한다. 구현: `crates/rbms-play/src/lib.rs` 의 `take_auto_lanes`, `Player::with_auto_lanes`. 플레이어는 `apps/rbms-player/src/app_play.rs` 에서 `config.play.scratch_auto` 일 때 턴테이블 레인을 `auto_lanes` 로 넘긴다.
3. 그 판도 일반 판과 똑같이 로컬 최고 기록을 갱신하고, 이력·리플레이를 남기고, 점수 서버로 보낸다. 서버로 보내는 본문에는 `options.scratch_auto` 와 `assist: ["AUTO_SCRATCH"]` 가 실린다(`crates/rbms-ir/src/mapping.rs` 의 `assist_flags`).
4. 램프는 LIGHT ASSIST(`ClearType::LightAssistEasy`, id 3)로 강등한다. 원본 `BMSPlayer.java:866` 의 `assist == 1 ? LightAssistEasy : AssistEasy` 그대로이며 FC 이상 램프는 나오지 않는다(`apps/rbms-player/src/lib.rs` 의 `assisted_lamp`).
5. SCRATCH AUTO 는 그 판의 턴테이블 레인에 노트가 있을 때만 어시스트로 센다(원본 규칙, `apps/rbms-player/src/app_result.rs` 317행 부근). 턴테이블 노트가 없는 차트에서 켜 두면 일반 판이다.
6. 판정 규칙 세대를 3 으로 올린다(`crates/rbms-store/src/score.rs` 의 `SCORE_RULE_VERSION = 3`, `SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION = 3`). 세대 2 이하의 SCRATCH AUTO 기록(사용자 DB 의 옛 37판)은 턴테이블을 PGREAT 로 센 점수라 비교할 수 없으므로 램프와 이력으로만 읽는다(`sets_best(assist, rule_version)`, `apps/rbms-player/src/scoredb_store.rs` 215행). 점수는 버리고 램프는 남긴다.

## 이유

- 사용자 원문 1~3: 초록, 카운트되지 않음, 없는 노트로 처리.
- 사용자 원문 4~5: "서버로 저장되지 않는 기록따윈 없어 무조건 저장될거야". 턴테이블을 판정에서 빼면 그 판의 EX 는 사람이 친 노트만으로 계산되므로 일반 판과 같은 표에 둘 근거가 생긴다(옛 규칙은 턴테이블을 PGREAT 로 넣어 점수가 부풀었다).
- 37판은 "그냥 없어져도 괜찮으니 (램프만있어도됨)" — 옛 세대의 점수를 새 세대의 최고 기록과 섞지 않는다.
- 램프 LIGHT ASSIST: 원본의 assist=1 강등과 같고, 서버와 웹의 램프 표기(`LightAssistEasy`)가 이미 있다. 사용자에게 따로 확인받지 않았다(미해결 질문).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 결정 12 유지(어시스트 판은 램프·횟수·이력만) | 사용자 "당연히 갱신" |
| 스크래치 노트의 몫을 뺀 별도 최고 기록 행(첫 해석) | 사용자가 "없는 노트로 처리, 서버로 똑같이"로 재정리. 노트가 판정 차트에서 빠지면 뺄 몫이 처음부터 없다 |
| 원본처럼 턴테이블을 PGREAT 로 대신 치기 | 점수가 부풀어 일반 판과 같은 최고 기록에 둘 수 없다. 사용자 원문 2·3 과 반대 |
| 옛 37판을 새 규칙으로 다시 계산 | 저장된 판정 수에서 턴테이블 몫을 분리할 수 없다. 사용자가 램프만 남겨도 된다고 함 |

## 영향과 관련 파일

- 클라이언트: `crates/rbms-play/src/lib.rs`, `crates/rbms-render/src/playfield.rs`, `crates/rbms-render/src/skin_render/notes.rs`, `crates/rbms-store/src/score.rs`, `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/app_result.rs`, `apps/rbms-player/src/scoredb_store.rs`. 테스트 `the_turntable_scratch_auto_plays_is_drawn_green_and_counts_for_nothing`(`apps/rbms-player/src/stage/play/screen_tests/default_pack_tests.rs`).
- 서버 쪽 순위 처리: [2026-10-14-adr-scratch-auto-ranking-toggle.md](2026-10-14-adr-scratch-auto-ranking-toggle.md).
- 저장 규칙 전반: [2026-10-14-adr-save-and-send-every-played-run.md](2026-10-14-adr-save-and-send-every-played-run.md).
- 커밋: `8b11a17`(규칙·세대 3), 웨이브 18(`wf_0dc71355-847`). 사용자 문서: `docs/play-options.md`.
- 남은 일(미착수): 초록 노트를 스킨 그림 모양 그대로 칠하기(GPU 셰이더 기능 필요, `apps/rbms-player/src/gpu`), 턴테이블 노트만 있는 차트의 자동 종료.

## 날짜와 사용자 원문

2026-10-14. 첫 답(대체됨): "당연히 갱신. 다만 SC auto 인경우 SC의 점수는 빼고 저장".

재정리(정본): "1. scratch auto를 하면 scratch 가 초록색이된다 2. scratch auto 초록색은 플레이중에 카운트되지않는다. 그냥 치지 않은 노트처리이다. 3. 다만 틀린 노트는아니고 진짜로 존재하지 않는 노트처럼 처리된다 4. 플레이가 끝나면 서버로 똑같이 보낸다 5. 37판의 내용은 그냥 없어져도 괜찮으니 (램프만있어도됨) 이것도 잘 처리해둔다. 서버로 저장되지 않는 기록따윈 없어 무조건 저장될거야. 또한 리플레이도 잘 저장되어야하고"
