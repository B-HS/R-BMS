# memory-perf 감사 보고서 검증 (회의적 재검토)

대상: `scratchpad/research/memory-perf.md` findings 15건. 각 항목을 코드 원문으로 재확인했다.
검증 범위는 rbms 측 코드 전수 확인이며, 레퍼런스 구현 대조는 원 보고서가 전 항목 "미조사"로 남긴 부분이라 이번에도 열지 않았다(미조사 범위).

## 요약

| id | 판정 | 한 줄 |
|----|------|-------|
| 01 | confirmed | 근거 정확. 다만 실제 비용은 String clone이 아니라 행마다 도는 O(records) 스코어 선형탐색이 지배적(원 보고서 누락) |
| 02 | confirmed | cache/runs 둘 다 evict 없음, HUD가 매 프레임 새 문자열 생성 — 그대로 성립 |
| 03 | confirmed | 정착마다 `AudioEngine::new()`. 다만 20프레임 디바운스로 open은 정착 시만, close는 커서 이동마다 |
| 04 | confirmed | 파일 프리뷰 경로가 read+stream open+decode 전부 렌더 스레드. 라인 근거 정확 |
| 05 | confirmed | detach·join 없음, cancel 확인 지점 476/498 둘뿐, 그 사이 parse→to_model→autoplay 완주 |
| 06 | partially | 무제한 채널은 사실이나 "PCM 통째 적체"는 과장. 매 프레임 드레인 + 포기 시 receiver 드롭(531)으로 producer 종료. 진짜 비용은 wavmap 전체 디코드 자체 |
| 07 | confirmed | 콜백 내 resize 존재. 실제 발생은 버퍼 크기 변동 시뿐이라 low 등급이 적절 |
| 08 | confirmed | push 실패 3곳 전부 무시, 카운터 없음 |
| 09 | confirmed | `position(|n| n.holding)`이 0부터 스캔. cursor 시작 권고도 타당 |
| 10 | confirmed | "현재는 안전, 뱅크 제거 API 추가 시 위험"이라는 조건부 서술 그대로 정확 |
| 11 | partially | 제목의 "프레임마다 힙 할당"은 틀림 — `Vec::new()`는 무할당, push가 있을 때만 할당. 본문은 맞음. 실효 영향 거의 없음 |
| 12 | confirmed | `if v.delay > 0 { v.delay -= 1; continue; }` 존재. 다만 호출부가 at_us≈now라 대기 보이스가 드물어 실효 미미 |
| 13 | confirmed | 사실이나 01과 동일 할당 지점의 다른 관점(중복 항목) |
| 14 | partially | PROCESS.md:132는 2026-05-31 **세션 이력 문단**이고, 같은 문서 40/137/176행이 하이브리드 프리뷰 완료를 기록. 문서 현재상태 서술은 stale 아님 |
| 15 | confirmed | font 캐시 크기 노출 API 없음, 오디오 드롭 카운터 없음, 테스트는 스모크뿐 |

---

## 항목별 검증 상세

### 01 — confirmed (단, 원인 진단이 절반만 맞음)
`apps/rbms-player/src/app_select.rs:79`가 `(self.select_gen, self.sel, ...)`로 `sel`을 포함하고, `:86-92`가 키 변경 시 `build_select_view()` 전체를 재실행한다. `:99-128`은 `select_items` 전부를 `SelectRow`로 map한다. 렌더는 `crates/rbms-render/src/select.rs:288`에서 `start..(start+visible).min(m)`만 그린다 — 전부 확인.

**보완**: 지배적 비용은 title/level clone이 아니라 `app_select.rs:113`의 `self.scores.best_clear_for_md5(&e.md5)`다. 구현이 `apps/rbms-player/src/scores.rs:85-86`의 `records.iter().filter(|r| r.md5.eq_ignore_ascii_case(md5))` 전수 선형탐색이라, 행 수 x 레코드 수 만큼의 대소문자 무시 문자열 비교가 커서 1칸 이동마다 돈다. 곡 5,000 x 기록 5,000이면 2,500만 회. 권고에 "md5 -> best_clear 해시맵을 select_gen 시점에 1회 구축"을 추가해야 한다.

### 02 — confirmed
`crates/rbms-render/src/font.rs:50`(`cache`)·`:54`(`runs`) 모두 insert만 있고(`:89`) 삭제 경로는 `set_ui_family`/`reset_ui_family`의 `clear()`뿐(`:225-226`, `:237-238`). HUD 측 가변 문자열도 `crates/rbms-render/src/hud.rs:85`(`format!("EX {}")`), `:94`(`combo.to_string()`), `:81`, `:90`, `:122`, `:126`, `:128` 전부 실재. 무한 성장 성립.

### 03 — confirmed (범위 한정)
`app_select.rs:422`·`:457` 두 경로 모두 `AudioEngine::new()`, `:339`·`:522`에서 `preview_audio = None`. `crates/rbms-audio/src/engine.rs:28-47`에서 스트림 빌드 + `play()` + `RingBuffer::new(8192)`. 뱅크 제거 API 부재도 `engine.rs:20,64-97` 확인(insert/조회만).
**정정 뉘앙스**: `PREVIEW_DEBOUNCE_FRAMES = 20`(`main.rs:58`) 때문에 스트림 *생성*은 정착 시에만 일어난다. 다만 `update_preview`가 커서 이동마다 `reset_preview_playback()`을 불러 엔진을 즉시 드롭하므로(`app_select.rs:296-299`, `:522`) *닫기*는 이동마다 발생한다. "스크롤 중 반복 churn"은 close 측면에서 성립.

### 04 — confirmed
`app_select.rs:412`(`std::fs::read`) → `:422`(`AudioEngine::new`) → `:433`(`eng.load` → `engine.rs:64-67` 동기 심포니아 디코드)이 전부 `frame()` 호출 경로(`apps/rbms-player/src/app_play.rs:409`) 안에 있다. autoplay 경로가 `:474`에서 백그라운드인 것과의 대조도 사실.

### 05 — confirmed
`app_select.rs:474` detach, JoinHandle 미보관. cancel 확인은 `:476`(진입)과 `:498`(sched 완성 후)뿐이고, 그 사이 `:479-496`의 `fs::read` → `parse_with` → `detect_mode` → `to_model` → `Player::new` + 전곡 `update()`가 취소 무관하게 완주한다. 디코드 워커는 `apps/rbms-player/src/main.rs:328-330`에서 최대 8스레드, `:334-346`에서 job 경계 cancel 확인 — 원 보고서 서술 그대로.

### 06 — partially
사실 관계: `app_select.rs:469`의 `mpsc::channel::<PreviewMsg>()`는 무제한이 맞고, `main.rs:68`의 `Keysound(u32, DecodedAudio)`가 f32 PCM을 소유하는 것도 맞다.
**과장 부분**: (a) 메인 스레드는 `update_preview`가 매 프레임 `try_recv` 루프로 드레인하므로(`app_select.rs:311-331`) 적체 상한은 대략 "1프레임 동안 8스레드가 디코드한 양"이다. (b) 프리뷰를 포기하면 `reset_preview_playback`이 `preview_prep_rx = None`(`app_select.rs:531`)으로 receiver를 드롭 → 코디네이터의 `tx.send`가 Err → `break`(`:510`)로 종료하므로 무한 적체가 아니다.
**진짜 비용**: `:485`의 `keysound_jobs(&model.wavmap, &dir)`가 곡의 **전체 키음**을 디코드해 프리뷰 뱅크에 넣는 것. 키음 650개급 차트를 호버만 해도 곡 전체 PCM이 RSS에 올라온다. 원 보고서의 "sched 참조 wav만 필터" 권고는 이 쪽 근거로 재작성해야 하고, bounded 채널은 부수적이다.

### 07 — confirmed
`engine.rs:118` `Vec::new()`, `:126-128` 콜백 내 `resize`. 다만 `data.len()`이 바뀔 때만 실행되므로 정상 상태에서는 첫 콜백 1회. low/S 등급이 과대평가는 아님.

### 08 — confirmed
`engine.rs:95`, `:100`, `:104` 전부 `let _ = self.producer.push(...)`. 링 `:37` 8192. 소비는 콜백(`:123-125`)에서만. 관측 수단 부재도 사실.

### 09 — confirmed
`crates/rbms-judge/src/matcher.rs:262` `let idx = l.notes.iter().position(|n| n.holding)?;` — 0부터 스캔. press는 `:205-220`에서 `l.cursor` 시작 + `gate_early` break로 유계인 것도 사실. 권고의 "cursor부터 스캔해도 안전" 근거(`update`가 holding 노트에서 `break`해 cursor를 그 자리에 남김, `:294-306`)도 코드와 일치한다.

### 10 — confirmed
`crates/rbms-audio/src/mixer.rs:102-103`, `:145-146`에서 `v.sample = None`이 콜백 경로(`engine.rs:123-129`)에서 실행되는 것 맞고, 현재는 `engine.rs:20`의 bank가 Arc를 계속 붙들어 refcount 감소로만 끝나는 것도 맞다. "조건부 위험" 서술이 정확하다.

### 11 — partially
`matcher.rs:281-284`, push 지점 `:301,:309,:312`, 소비 `:322-325` 전부 확인. 다만 `Vec::new()`는 **할당하지 않는다** — 첫 push가 있어야 할당한다. 제목의 "프레임마다 판정 이벤트 Vec을 새로 만든다(=매 프레임 힙 할당)"은 틀렸고, 본문의 "판정이 발생하는 프레임마다"만 맞다. 게다가 이 Vec에 push되는 것은 miss 스윕/LN 만료뿐이라 밀도가 낮다. 권고(필드 승격 + clear)는 여전히 무해하나 우선순위는 info에 가깝다.

### 12 — confirmed (실효 낮음)
`mixer.rs:122`부터가 `mix`, `:139`에 `if v.delay > 0 { v.delay -= 1; continue; }`. 보이스 풀 512(`engine.rs:25`)도 맞다. 다만 호출부가 `at_us`를 현재 시각 근처로 주므로(`app_play.rs:434`, `app_select.rs:353`) 대기 보이스는 드물다. 상수 시간 스킵 권고 자체는 옳다.

### 13 — confirmed (01과 중복)
`app_select.rs:99-128`이 전 항목 clone, `:90`이 상주. 원본은 `main.rs:358-366`. 사실이지만 01과 같은 코드 지점이라 별도 finding으로 계상하면 심각도가 이중 계산된다.

### 14 — partially
`docs/PROCESS.md:132`의 "실재생 미동작" 문구는 실재한다. 그러나 같은 문서 `:40`("2026-06-07 곡선택 하이브리드 미리듣기 — 완료"), `:137`(구현 상세), `:176`("~~#PREVIEW 파일만~~ 하이브리드 미리듣기 완료")이 현재 상태를 정확히 기록하고 있고, `:132`는 **2026-05-31 세션 이력 문단**이다. 즉 "문서가 코드와 불일치"가 아니라 "이력 문단이 당시 상태를 기록"한 것. 문서 stale 판정은 과함. 손댄다면 `:132`에 "(2026-06-07 해소)" 한 줄 추가 정도.

### 15 — confirmed
계측 현존: `app_play.rs:379`(fps EMA), `:382-385`(debug 게이트 + 15프레임마다 `memory_stats`), `:748-752`(오버레이). 부재: `font.rs`에 `pub fn`이 `text_width/draw_text*/fit_text/load_font/set_ui_family/reset_ui_family`뿐이라 캐시 크기 노출 없음(`:164-232`), `engine.rs`에 드롭/언더런 카운터 없음, `apps/rbms-player/tests/autoplay_preview.rs:1-39`는 parse→sched→decode 스모크뿐(RSS·스레드 assert 없음). 전부 사실.

---

## 원 보고서가 놓친 항목 (같은 관점)

1. **`refresh_focused_detail`가 렌더 스레드에서 차트 전체 파싱 + 이미지 디코드를 디바운스 없이 수행** — `app_play.rs:408`이 매 프레임 호출, `app_select.rs:266-278`이 포커스가 바뀐 행마다 `compute_chart_detail`(`main.rs:450-454`: `fs::read` + `parse` + `to_model`)과 `decode_bga_256`(`main.rs:349-355`: PNG 디코드 + 256x256 리사이즈)을 동기 실행한다. finding-04(프리뷰 파일 경로)는 20프레임 디바운스라도 있는데 이 경로는 **디바운스조차 없어** 커서를 누르고 있으면 행마다 풀 파싱이 돈다. 프레임 스톨 관점에서 04보다 크다.
2. **`best_clear_for_md5`의 O(records) 선형탐색이 행마다 반복** — `app_select.rs:113` x `scores.rs:85-86`. 커서 이동당 O(rows x records) 문자열 비교. 01의 실제 지배 비용인데 보고서는 String clone만 지목했다.
3. **플레이 경로 키음 디코드 채널도 무제한** — `main.rs:322` `mpsc::channel()`. finding-06은 프리뷰 채널만 지목했지만 동일 헬퍼를 게임플레이 로드(`app_play.rs:78`)가 공유한다.
4. **로딩 중단 시 플레이용 디코드 워커가 계속 돈다** — `app_play.rs:78`이 `Arc::new(AtomicBool::new(false))`를 **항상 false인 채로** 넘기고(취소 신호를 아무도 세우지 않음), `main.rs:341-343`은 `let _ = tx.send(...)`로 수신자 소멸을 무시한다. `main.rs:1095`에서 Esc로 `ks_rx = None`을 해도 최대 8스레드가 곡 전체 키음을 끝까지 디코드한다. 프리뷰 쪽(finding-05)만 지적되고 플레이 쪽은 빠졌다.
5. **프리뷰가 곡의 전체 wavmap을 디코드해 뱅크에 적재** — `app_select.rs:485`. 호버만으로 곡 전체 키음 PCM(수백 MB 가능)이 RSS에 올라오고, 다음 곡으로 이동해야 해제된다. finding-06이 이 비용을 채널 문제로 잘못 귀속시켰다.
6. **`runs` 캐시의 성장 축은 문자열이 아니라 (글리프, RGB) 조합** — `font.rs:54`. 색상은 스킨/판정 팔레트에서 오므로 상한이 있지만, `hud.rs`의 판정색 6종 x 게이지 색 3종 등과 글리프 수의 곱이라 `cache`와는 다른 축이다. 02 권고의 "같은 상한 정책"만으로는 키 설계가 정리되지 않는다.
7. **Settings 화면에서 매 프레임 `Vec<(&str, String)>` 재구축** — `app_play.rs:451-453`이 `SETTING_TABS[set_tab].1.iter().map(|&i| self.setting_line(i)).collect()`를 프레임마다 실행하고, `setting_line`은 `format!`으로 String을 만든다(`app_select.rs:897` 등). select 캐시(`refresh_select_cache`)와 달리 캐시가 없다.
8. **`build_select_view`의 비-행 부분도 매 재빌드마다 String 생성** — `app_select.rs:129-140`(header `format!`/`to_string`)과 `record_row_view`(`:71-77`)의 `fmt_datetime` per record. 01/13의 rows 최적화만으로는 남는다.

## 미조사 범위

- 레퍼런스 구현 Java 측 대응 구조(`select/`, `audio/`) 대조 — 원 보고서도 전 항목 미조사였고 이번 검증에서도 열지 않았다. "레퍼런스 구현는 이렇게 한다"는 주장은 양쪽 어디에도 근거가 없다.
- 실측(프레임타임 프로파일, RSS 추세) 미실시. 위 판정은 전부 코드 정적 확인 기준이며, 01/02의 실제 ms/MB 수치는 검증하지 않았다.
