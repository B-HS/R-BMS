# #PREVIEW 실재생 — 진단 계측 + 검증 픽스처

## 대상 파일
- `apps/rbms-player/src/app_select.rs` (`update_preview` / `start_preview` / `stop_preview`)
- `apps/rbms-player/src/main.rs` (`SongEntry.preview` — dead_code 주석 제거), `apps/rbms-player/src/app_play.rs` (`load`가 select 진입 전 `stop_preview`)
- 픽스처: `samples/preview-demo/` (`preview-demo.bms` + `preview.wav`)

## 증상
곡 포커스 settle 시 `#PREVIEW`가 **소리가 안 남**(ROADMAP/PROCESS의 미해결 TODO).

## 리포트 (근본 진단)
적대적 분석 결과 **버그가 아니라 진단 불능이 1차 문제**였다. `start_preview`의 6개 실패 분기가 전부 **silent early-return**이라 "소리 없음"이 다음 중 무엇인지 구분 불가였다:
1. 포커스 곡이 `#PREVIEW` 미정의 — **가장 유력**(테스트 라이브러리 차트 대부분 `#PREVIEW` 없음 → 정상 동작, 재생할 파일이 없을 뿐).
2. `AudioEngine::new()`(select 전용 둘째 cpal 출력 스트림) 생성 실패 — macOS CoreAudio 기기 협상 등.
3. `decode_bytes` 실패 — `#PREVIEW` 컨테이너 코덱 미지원.
4. `resolve_file` 경로 불일치(확장자/대소문자/구분자).

→ **각 분기에 `config.debug`(DISPLAY 탭 DEBUG MODE) 게이트 `eprintln!("[preview] …")` 계측을 추가**해 어느 경로인지 즉시 드러나게 했다. `SongEntry.preview`는 이제 `start_preview`가 읽으므로 stale `#[allow(dead_code)]`를 제거했다(빌드 무경고 유지).

## 검증 방법 (수동 — 오디오는 헤드리스 불가)
1. 설정 → DISPLAY → **DEBUG MODE on**, **PREVIEW on**.
2. `./start.sh samples/preview-demo` (또는 `BIN samples/preview-demo`)로 실행 후 데모 곡 포커스.
3. stderr `[preview]` 로그 판독:
   - `playing '…' dur_us=… clock_us=…` 인데 무음 → **가설 2(둘째 cpal 스트림)** — 단일 영속 엔진 재사용 또는 첫 tick 이후 play 스케줄로 수정.
   - `AudioEngine::new failed …` → 스트림 생성 문제(기기/권한).
   - `decode failed …` → 코덱(symphonia feature) 문제.
   - `… not found under …` → `resolve_file` 경로 문제.
   - `defines no #PREVIEW` → 정상(그 곡엔 프리뷰 없음). 데모 곡에선 이 메시지가 **안** 나와야 정상.
4. 디코드 자체는 디바이스 무관 `decode_bytes` 단위테스트(`crates/rbms-audio/src/decode.rs::decodes_in_memory_wav`)로 이미 커버 — 데모 `preview.wav`도 동일 경로.

## 상세 (start_preview 분기 순서)
`song index 범위 → #PREVIEW 빈 문자열 → 부모 디렉토리 → resolve_file → fs::read → AudioEngine::new → load(decode) → sample_duration_us/clock_us/play`. 각 실패 지점에 debug 로그. 성공 시 `preview_loop_us=dur`, `preview_next_us=now+dur`로 `update_preview`가 클립 경계마다 재생(갭리스 루프).

## 상태
계측·픽스처·dead_code 정리 완료(코드 무경고·디코드 테스트 그린). **최종 가청 확인은 위 수동 절차로 사용자 환경에서 1회 필요** — 그 결과(어느 분기/소리 여부)에 따라 후속 수정 여부 결정. 코드 경로는 보존.
