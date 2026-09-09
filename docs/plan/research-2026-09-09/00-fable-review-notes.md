# Fable 검토 노트 (관점별)

## Fable 직접 검증 결과 (코드 확인 완료)
- [확정] 見逃し POOR → `matcher.rs:306-311` `events.push(Judge::Miss)` → `gauge.update(Miss)` = deltas[5](MS: NORMAL -2, HARD -5). 레퍼런스 구현 `JudgeManager.java:595` `updateMicro(...,4,...)` → value[4](POOR: NORMAL -6, HARD -10). 空POOR 는 `matcher.rs:231-235` Miss(-2) 로 레퍼런스 구현 MS 와 일치. → 자동미스 게이지 페널티가 절반~1/3. **critical, S 효력**(sweep 1줄 + 카운트 라벨 + 테스트). PROCESS.md §6 "게이지 대조 검증됨" 은 이 점에서 오류.
- [확정] `windows.rs`: 5K → SEVENKEY_NOTE(`note_for_mode` `_ =>`), POPN 복제(주석에 pending 명시), SEVENKEY_LN_END=(120,150,200,±250)+MS 는 레퍼런스 구현 FIVEKEYS LN end 값(SEVENKEYS 는 120,160,200,(-280,220), MS 없음). `LN_MARGIN=200_000` 전모드(레퍼런스 구현 7K/5K/KB=0, PMS=200000). `scaled()` 가 bd 까지 스케일(레퍼런스 구현 judgeWindowRate 는 i<3 만 + 클램프). 스크래치 전용 윈도우 없음.
- [확정] 오디오 클럭: `engine.rs:133` 콜백 말미에만 clock store → 콜백 사이 정지값. `play/lib.rs:174` `at <= now_us` 만 방출 + `mixer.rs:93` `delay = at_frame.saturating_sub(clock)` → delay 항상 0. 키음 온셋·입력 타임스탬프 모두 버퍼 주기 양자화. 클럭 = 믹싱 완료 프레임(가청보다 앞섬). PROCESS.md §1 "vsync 양자화 구조적 해결" 주장은 부정확(vsync 대신 오디오 버퍼로 양자화).
- [부분] FLOATING HI-SPEED: rbms `constant_speed` = 그린넘버 고정(2000/hispeed) + 곡 내 BPM 변화 무시(레퍼런스 구현 CONSTANT). IIDX FLOATING/레퍼런스 구현 MAIN/MAX/MIN/START BPM 기준 hispeed 자동조정(곡 내 BPM 변화는 반영)은 없음. 라벨 "FLOATING" 이 IIDX 의미와 반대(rbms FLOATING = 일반 hi-speed).
- [확정] 키빔은 존재(playfield.rs BEAM_*, SkinConfig beam_*). iidx 보고서 "미확인" 은 미조사일 뿐.

## iidx-feature-gap.md (research:iidx-features)
- 근거 있음: FLIP/BATTLE/SYNC-RAN 미구현(shuffle.rs:95-97 주석), LEGACY NOTE·5KEYS·HID+·target·코스/DAN·연습·즐겨찾기·필터·판정분포그래프·게이지추이·MAX- 미구현.
- IIDX 31~34 신규는 웹 미확보 → 계획서에 '미확인' 유지.

## skin-render-inventory.md (research:skin-current)
- 결론 신뢰: Renderer trait = size/clear/fill_rect 3개, 텍스처 슬롯 1개(256x256, BGA·커버 공유), 곡선택/결과/설정/키설정/로딩/디버그 전부 하드코딩, 3종 RON 차이는 스칼라 6~7개.
- 저해 지점 A~J + 규모(XL) 타당. 단계 순서 A→B→C→D→E 합리적.
- 결과화면이 skin.judge_colors 무시(J) — S 규모 즉시 수정 후보.

## skin-system.md (research:skin-the reference implementation)
- 핵심: 모델은 하나(JsonSkin.Skin), Lua 로더는 JSON 로더 서브클래스. default 스킨의 7K play/decide/result/keyconfig/skinselect 는 .luaskin. JSON 안에도 Lua 식("draw":"gauge() >= 75").
- → 결정 필요: (a) 자체 RON 오브젝트 스킨(레퍼런스 구현 모델 미러, Lua 없음) (b) JSON 호환 + 소형 Lua 식 평가기(mlua) (c) LR2 CSV 까지. 규모 5.6~8.1k LOC(+LR2 1.5~2.5k).
- 프로퍼티 968개 중 play 100~150, select/result 포함 250~350.

## judge-gauge.md (research:judge-parity)
- 위 직접 검증 항목 외: JudgeAlgorithm Duration 고정(레퍼런스 구현 기본 Combo), 지뢰 데미지 없음, MODIFY_DAMAGE 없음, CLASS 게이지 없음, 5K/PMS/KB/LR2 게이지 없음, 9게이지 병렬 갱신 없음(GAS 불가), lnmode 강제 없음, #DEFEXRANK 미파싱, TOTAL fallback 200 vs validate 공식 max(260, 7.605n/(0.01n+6.5)), CN deferral 없음, LN 미릴리스 확정 규칙 다름(레퍼런스 구현 7K 는 마진 0 + 머리판정 확정), fast/slow dm==0 반대, 판정된 노트 재타격 空POOR 없음.
- 문서 stale: divergences.md 가 위 발산을 기록 안 함(누락형). cn-hcn-judgment.md 경로 stale.

## ux-flow.md (research:ux-flow)
- 블로킹 B1(표 fetch 15s)·B2(포커스 파싱+커버)·B3(차트 로드)·B4(BGA 전량 디코드)·B5(rfd)·B6(결과 저장). GUI 에러 피드백 0. 디바운스 프레임 기준. 플레이 중 hispeed 변경 미저장. 빈 라이브러리 Esc=종료. Tab 과부하.
- 옵션 UX 권고: B(곡선택 오버레이 패널) 주경로 + A(환경설정) 축소 + PAUSE/RETRY + decide 미도입. 타당.
- target: 서버 불필요(MAX/RATE/RANK_NEXT/LOCAL_BEST) 먼저, chart_ranking/player_best 호출부 0.
- PROCESS.md §4 "#PREVIEW 실재생 미동작" stale(같은 문서 내 모순).

## feature-inventory.md (research:the reference implementation-parity)
- 곡DB 없음(매 실행 전량 파싱), 곡선택 O(N×M), scores.ron 비원자 write, 컨트롤러/MIDI 0, 코스/연습/일시정지/리트라이/즐겨찾기/필터/볼륨3분리/비디오 BGA 미구현.
- 우선순위 16개 목록 타당. #4 원자적 저장 S 즉시.

## memory-perf.md (research:memory-perf)
- 확정 누수: 폰트 레이아웃 캐시 무상한(font.rs:50,89) + 매 프레임 format! 문자열이 캐시에 영구 적재. 의심: 글리프 색 캐시, 프리뷰 스레드 9개/정착·무제한 채널·cpal 스트림 churn.
- RT: 콜백 내 scratch.resize 할당, producer.push 실패 무음 유실.
- 핫패스: select_key 에 sel 포함 → 커서 이동마다 전곡 String 재할당(feature-inventory 와 일치). release() 레인 전수 스캔.
- 실측은 대표성 없음(픽스처 8kHz).

## audio-input.md (research:audio-input)
- 클럭 계단(직접 확정). 볼륨 체계 없음(set_master_gain 호출 0, gain 1.0 리터럴, #VOLWAV 미반영, 하드 클리핑). 보이스 스틸 = 커서 위치 강탈. 스트림 에러 무복구(장치 변경 시 게임 정지). 프리뷰가 두 번째 cpal 스트림. 스크래치 1키(정/역 2키 없음). 컨트롤러/MIDI 없음. 시스템 사운드 없음.
- 리샘플 선형보간은 레퍼런스 구현(최근접)보다 나음.

## workspace-quality.md (research:crate-arch)
- 실측: clippy 51(전부 style), unsafe 0, allow 0, Mutex 0, 빌드 경고 0. DAG 정상.
- 진짜 긴 파일은 main.rs 1277 / app_select.rs 967 / app_play.rs 796 (나머지는 인라인 테스트). 분할안 구체적.
- 구조 결함: App god object(~100 필드), PlayerConfig↔PlaySettings 이중화(5곳 동시 수정), 설정 UI 정수 인덱스 결합, JudgeWindows 비데이터(serde 없음), 판정 알고리즘 추상화 없음, theme/font thread_local 전역, Result<_,String> 8곳, 런타임 패닉 6곳(partial_cmp unwrap, end_us unwrap, 믹서 콜백 unwrap).
- 앱→크레이트 이관 후보: ir_map, clear_type_id, ScoreBook, Replay, settings, 폴더 스캔, compute_table_levels.

## 검증 보고서 반영 (verify)
- iidx-verify: 14건 중 confirmed 11·partially 3(04 시작전패널 severity high→medium, 07 LEGACY NOTE 는 S, 11 게이지 폴백은 GAS 포팅으로 교체). missed: M1 그린넘버 LIFT 미반영(Fable 확정), M2 assist 항상 빈 배열(Fable 확정), M3 GAS, M4 레인커버 5% 계단, M5 hispeed 0.25 step/0.5~10(레퍼런스 구현 0.01~20), M6 HUD 그래프도 3밴드, M7 2000.0 매직상수 2곳, M8 ARENA 축 없음.
- skin-current-verify: 14건 전부 confirmed/partially(반박 0). 정정: gpu.rs 에 이미 UV 텍스처 파이프라인(BGA) 존재 → 텍스처 프리미티브 effort M. 임베드 스킨은 NORMAL/WIDE 2종(default.ron 은 --skin 경로만). 미기재 필드 17개(dual_field/dual_gap 포함).
- skin-reference-verify: 17건 confirmed 12·partially 5(반박 0), 968 카운트 전수 재현. missed: M1 **default JSON 이 trailing comma 포함 관대 JSON → serde_json 파싱 실패**(json5 계열 필요), M2 Lua 샌드박스(스킨=임의 코드 실행, mlua 채택 시 경계 설계), M5 scene/input/fadeout/playstart/close 타이밍 필드, M6 커스텀 property(op 920/921 등) + 설정 저장, M7 filepath 선언, M8 기존 RON SkinConfig 호환 유지 제약. LR2 패키지 3,195 LOC(9파일, 106 명령).
- ux-flow-verify: 21건 confirmed 13·partially 7·**refuted 1(17 디버그 오버레이 부재 → 이미 app_play.rs:747-778 에 FPS/RAM/QUADS/오디오클럭 표시)**. 09 레퍼런스 구현 에 일시정지 없음 → PAUSE/RETRY 는 신규 제안(low). 12 정정: Root 에서 Esc 는 곡 유무 무관 무조건 exit(select_back). missed: M1 Root Esc 무확인 종료, M2 Play Esc 무확인 폐기, M3 표 로드 실패 빈 레벨로 흡수, M4 replay load failed 무성, M5 검색 Esc 가 뷰 미복원, M6 결과화면 리트라이/다음곡 동선 없음, M7 로딩 단계 표시 없음, M8 표 URL 중복 미검사. 01 severity critical→high(서버 인디케이터·온보딩 힌트 존재). 20 리사이즈는 레터박스 아닌 스트레치(main.rs:942 주석).
- reference-parity-verify: 19건 confirmed 12·partially 6·uncertain 1(외부연동)·refuted 0. 정정: 01 부팅 스캔은 이미 백그라운드(프리징 아님, 지연+낭비 → high), 06 텍스트 검색 필터는 있음(난이도/모드 필터·즐겨찾기만 없음), 10 Play-Esc 즉시 이탈은 PROCESS §7 의도 사양(되돌리려면 사용자 확인), 11 리플레이는 플레이마다 파일 저장+모달 재생 가능(갭 = 슬롯/보존 정책·무제한 누적), 13 표 rescan 은 비동기·추가 경로만 동기, 16 KeyCommand 13종(COPY_MD5/SHA256 누락). missed: M2 시스템 사운드, M3 라이벌 미배선, M4 replay.ron 비원자, M5 리플레이 무제한 누적 GC 없음, M6 플레이어 프로필/통계 계층, M7 런처 계층, M8 랜덤 코스.
- memory-perf-verify: 15건 confirmed 11·partially 4(06 채널 적체 과장→진짜 비용은 호버 시 전곡 wavmap 디코드, 11 Vec::new 무할당, 14 PROCESS:132 는 이력 문단이라 stale 과함, 13 은 01 중복). missed: 1 refresh_focused_detail 디바운스 없는 동기 파싱+디코드(04 보다 큼), 2 best_clear_for_md5 O(rows×records) 가 01 지배 비용 → md5→best 해시맵, 3 플레이 키음 채널도 무제한, 4 **로딩 Esc 취소 시 플레이 디코드 워커 cancel 플래그가 항상 false(아무도 안 세움) → 8스레드 완주**, 5 프리뷰가 전체 wavmap 디코드(수백 MB 가능), 6 runs 캐시 축 = (글리프,RGB), 7 Settings 화면 매 프레임 setting_line Vec 재구축, 8 build_select_view 헤더/기록행 String.
- crate-arch-verify: 20건 confirmed 15·partially 4·**refuted 1(11 end_us unwrap 은 holding 가드로 도달 불가 → 설계 위생 low)**. 정정: 03 App 93필드, 07 PlayerConfig 30, 10 믹서 unwrap 현재 도달 불가(medium/S 위생), 14 Result<_,String> 7곳, 19 render→chart 는 playfield.rs:1 이 scroll 실사용 → 제거 M·저우선, 06 모드 축은 이미 있고 사용자 설정 축만 없음. 09 NaN 정렬 패닉은 confirmed(`#xxx02:nan`). missed: 1 **CI lint 잡이 continue-on-error:true 로 무력화 + --all-targets 누락**, 2 apps/rbms-player 에 lib 타깃 없음(통합테스트가 앱 모듈 접근 불가 → [lib] 추가), 3 텍스트가 1px fill_rect 런으로 매 프레임 인스턴스 버퍼 → 텍스처 쿼드 정량 근거, 4 [workspace.lints] 없음, 5 forbid(unsafe_code) 없음, 6 reqwest 클라이언트 ir/table 중복, 7 rbms-cli 테스트 0, 8 NaN 은 파서 단계 거부가 본질(measure.rate).
- iidx-30-34-web: rbms 관련 신규 사양 — 30 스크래치/건반 FAST-SLOW 분리 표시·흰수 SUB-LENGTH/녹수 NOTES TIME 명칭·"FAILED" 문구 / 31 **H-RANDOM·EXPAND-JUDGE 폐지**, LIFT 를 SUD+ 와 독립(rbms 이미 독립), 키빔 길이 4단·노트 두께 4단, 레인커버 디자인·LIFT 이미지, 애니메이션 페이스메이커 배경, 커스텀 폴더 5개 필터 / 32 곡선택 BGA 썸네일 프리뷰, STEP UP HARD/EX-HARD 강제 폴더, 카테고리 필터(난이도 범위·레이더·라이벌), 서브라이벌 10명, FREE/FREE+/HAZARD 모드 폐지→PREMIUM FREE / 33 판정문자 위치 -2~+10, 레인 표시물 밝기 90~100%, **CN/HCN 스킨 별도 설정+두께**, 집중 모드 배경 어둡게, DJ TRAINING / 34(로케테) **녹색수 리셋(START 2회)·LANE COVER ADJUST(BPM 변화 시 SUD+ 슬라이드)·SPEED ADJUST(BPM 변화 시 hispeed 조정)**, BPM 변화 예고 표시(상승 파랑/하강 빨강), 레인커버 수치 상시 표시, 곡선택 DJ LEVEL DISPLAY, 譜面情報(시간대별 노트수·클리어율), 스코어 클래스(상위 10/100/500위 점수), CLEAR DATA 에 HARD/EX-HARD 클리어율. 타깃 종류 신규는 미확인. remywiki 본문 403(스니펫만).
- audio-input-verify: 13건 confirmed 8·partially 5·refuted 0. 정정: 02 delay 는 press 경로에서 살아 있음 — `at_us = judge_t = song + offset` 이라 **offset>0 이면 키음이 offset 만큼 늦게 발음(버그, auto_offset 켜면 상시)** → lookahead 도입 시 press 는 at_us=즉시 예외 필수. 03 시청각 스큐는 레퍼런스 구현 도 디바이스 레이턴시만큼 있고 rbms 추가분은 콜백 버퍼 1개분. 04 프리뷰는 0.85 gain, master_gain 기본값 하향 한 줄이 즉시 완화책. 06 레퍼런스 구현 도 런타임 복구 없음(오픈 시 폴백만). 07 autoplay 프리뷰 경로(457)도 새 스트림. missed: AutoVsync present 블로킹 입력 지연(별개 원인), #WAV slice(bmson) 미대응, 보이스 start/stop 페이드 없어 클릭 노이즈, 엔진 None 시 벽시계 폴백은 이미 존재(스트림 사망을 None 취급만 필요), 프리뷰 엔진도 512 보이스+독립 클럭(05·07 통합 수정 필요).
- judge-parity-verify: 18건 confirmed 16·partially 2·refuted 0. 정정: 03 LN 종단 윈도우 밖은 코드 4(見逃し POOR)라 rbms unwrap_or(Poor) 는 우연히 정확 → LN_END 의 ms 필드 제거만, 14 MODIFY_DAMAGE 는 EXHARD_5/HARD_LR2/EXHARD_LR2 3종(HARD_5 아님, 현재 적용 대상 0 → low). 라인 드리프트 다수(실제 라인표 보고서에 있음). missed: **M1 空POOR 가 counts 에 미집계 → IR ems=0**, **M2 combocond 테이블 없음 — 01 을 슬롯 교체만으로 고치면 空POOR 가 콤보를 끊는 회귀 발생(7K combo[5]=true, combo[4]=false 테이블 필요)**, M3 모드별 게이지 스펙 상이(NORMAL_PMS min2 max120 init30 border85 deltas{1,1,0.5,-2,-6,-6}), M4 fixjudge per-index 라 scaled() 시그니처 변경 필요, M5 키/스크래치 JUDGE WIDTH 분리(각 PG/GR/GD 3개), M6 bmson TOTAL 미확인, M7 7K 게이지 6종 수치·modifier·guts 는 완전 일치(발산은 슬롯 1건 집중), M8 후보 게이트·見逃し 경계는 동일(발산 아님). IR 파급: app_play.rs:251-266 poor=c[4]/miss=c[5] 라 見逃し 전부 ms 필드로 전송.
