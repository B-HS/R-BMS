# 2026-06-07 — 곡선택 하이브리드 미리듣기(#PREVIEW + autoplay)

## 배경 / 의도 정정
사용자 보고: "곡선택에서 곡 소리 안 나온다". 직전까지 구현/검증한 것은 **`#PREVIEW` 파일 재생**(헤더에 `#PREVIEW xxx.ogg`가 있을 때만)뿐이었다. 사용자가 원한 것은 **곡을 포커스하면 그 곡이 들리는 미리듣기**였고, 사용자 라이브러리(`/Volumes/SSD/bms`) 대부분 차트엔 `#PREVIEW`가 없어 무음이었던 것.

원본 레퍼런스 구현 대조(`select/PreviewMusicProcessor.java`·`select/MusicSelector.java`): 곡선택 미리듣기 = 400ms 디바운스 후 **`#PREVIEW` 파일**(`SongPreview` NONE/ONCE/LOOP) 재생, 없으면 **곡선택 BGM(메뉴 음악)**으로 복귀. **차트 자체를 autoplay로 틀어주는 기능은 원본에 없음**(프리뷰 자동생성 코드도 없음).

사용자 결정(AskUserQuestion): **하이브리드 — `#PREVIEW` 있으면 그 파일, 없으면 곡을 autoplay로 미리듣기** + **전체 키음 로드 후 재생**.

## 구현
- 대상: `apps/rbms-player/src/app_select.rs`(`update_preview`/`start_preview`/`start_autoplay_preview`/`reset_preview_playback`/`stop_preview`), `apps/rbms-player/src/main.rs`(`PreviewMsg` enum, 공통 헬퍼 `keysound_jobs`/`spawn_keysound_decode`, App 프리뷰 필드, 상수 `PREVIEW_LOOP_TAIL_US`), `apps/rbms-player/src/app_play.rs`(`load()`가 공통 헬퍼 사용).
- 동작: 포커스 정착(디바운스 `PREVIEW_DEBOUNCE_FRAMES=20`) 시 `start_preview` →
  - `#PREVIEW` 있으면 기존 **파일 재생**(디코드 후 클립 루프).
  - 없으면 **`start_autoplay_preview`**: **백그라운드 코디네이터 스레드**가 차트 파싱(`parse_with`)→`to_model`→**throwaway `Player::new(model,true).update(end, collect)`로 autoplay 키음 스케줄 `Vec<(at_us, wav)>` 추출**→키음 전체 병렬 디코드. 결과는 `PreviewMsg`(Schedule, Keysound) 채널로 메인 스레드에 전달.
  - 메인(`update_preview`, 매 Select 프레임): 채널 드레인(스케줄 저장·키음 bank 삽입), **채널 disconnect = 로드 완료** → 클럭 앵커(`anchor = clock_us - start_us`) 후 재생 시작. 재생은 **due 이벤트 dispatch 먼저 → `song >= end_us`면 한 주기 전진 재-앵커**(루프). `end_us = 마지막 이벤트 + 2초 tail`.
- 기존 play 파이프라인을 최대 재사용: 키음 디코드 fan-out과 job 빌드는 `load()`와 공통 헬퍼(`keysound_jobs`/`spawn_keysound_decode`)로 통합(2회 룰). 코디네이터는 `spawn_keysound_decode` 결과를 `PreviewMsg::Keysound`로 forward.
- 취소: `spawn_keysound_decode` 워커와 코디네이터가 `Arc<AtomicBool>` cancel을 확인. 포커스 변경 시 `reset_preview_playback`이 cancel을 set(구 로드 중단)·엔진/채널 drop, 새 `start_autoplay_preview`는 fresh cancel 생성.

## 검증
- 컴파일 무경고, 전체 `cargo test --workspace` **889 통과**(기존 887 + 신규 2: `crates/rbms-audio/src/decode.rs::preview_demo_fixture_decodes_and_mixes_to_nonzero_audio`, `apps/rbms-player/tests/autoplay_preview.rs`).
- 헤드리스 통합 테스트: `#PREVIEW` 없는 픽스처(`samples/preview-demo/autoplay-demo.bms`, `#WAV01 preview.wav`+노트)로 파싱→model→autoplay 스케줄(비어있지 않음)→키음 디코드(비-제로 PCM)까지 검증. cpal 디바이스 출력만 헤드리스 불가(게임플레이 키음과 동일 스택이라 가청 보장).
- **적대적 멀티에이전트 리뷰 2라운드**: 1R(4축×검증) 8건 확정 → 전부 반영(루프 이음새 마지막 키음 누락·디코드 스레드 미취소·메인스레드 동기 파싱 잔여·디코드 중복). 2R(코디네이터 동시성 집중) 1건 MEDIUM(재-앵커가 프레임 오버슈트 폐기 → 첫 키음 뭉침·tail 드리프트) → `anchor += period`로 위상연속 수정 + 이벤트0 차트 idle 엔진 정리.

## 잔여(의식적 비채택)
- 원본의 **메뉴 BGM 폴백**(`#PREVIEW`도 autoplay도 없을 때) — 스킨 BGM 자산 필요, 범위 밖.
- 미리듣기 시작점은 첫 이벤트(무음 인트로 스킵), 루프는 전곡(+2초 tail). 짧은 구간만 재생/하이라이트 선택은 후속 튜닝 여지.
- **실기 가청 확인 1회는 사용자 몫**(귀). `./start.sh "<#PREVIEW 없는 폴더>"`로 포커스 시 곡 키음이 들리면 완료.

## 다음(큐)
- `.dmg` 더블클릭 배포용 **첫 실행/폴더 선택/스캔 진행 UI**(터미널 없는 환경) — 사용자 요청, 후속.
