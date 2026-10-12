# rbms — 레퍼런스 구현 core PLAY 모듈 Rust 포팅 (PROCESS / 단일 출처)

> 세션 인수인계 스냅샷은 `docs/HANDOFF.md`(2026-10-11, Lua 스킨 호환 작업의 웨이브 8 까지 완료)다. 2026-10-09 스냅샷은 `docs/history/2026-10-09-handoff-snapshot.md`, 2026-09-18 스냅샷은 `docs/history/2026-09-18-handoff-snapshot.md`. 이전 Phase G 중단 스냅샷은 `docs/history/2026-09-10-handoff-snapshot.md`와 `docs/history/2026-09-10-session-wrap-up.md`에 보존한다. Phase G·H는 완료했고 Phase R은 실제 키와 함께 추후 진행한다.
> 새 세션은 **이 문서부터** 읽는다. 현재 상태·아키텍처·실행법·할 일의 SSOT. (ai-process.md 원칙 1·14)
> 베이스 룰: 프로젝트 작업 환경의 `AGENTS.md`와 llm-rules 전문(`ai-process`·`common`·`comments`·`git`·`security`)을 따른다. Rust 프로젝트에는 TS/JS 전용 규칙을 적용하지 않으며, 공통 원칙(문서는 `docs/`에 기록·정확한 이름·근본 해결·공식 문서 확인·검증 후 진행)은 그대로 적용한다.
> 위치: 저장소 루트(2026-10-09 기준 `/Users/hyunseokbyun/development/R-BMS`). 빌드 `cargo build`, 테스트 `cargo test --workspace`(2026-09-17 스킨 시스템 K5 게이트 실측 3,181 통과·0 실패; 실제 오디오 장치 테스트 등 ignored 3건). lint 는 이제 게이트다 — `cargo fmt --all --check` 와 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 가 CI 필수 통과 조건이고 툴체인은 `rust-toolchain.toml` 로 `1.95.0` 고정. 실행 정본은 `README.md`다.
> git: **dev(작업)/prod(배포) 브랜치 모델**(CI는 dev, 릴리스는 prod → `docs/ci-release.md`). **커밋 메시지에 co-author(Claude) 넣지 않음**(사용자 명시 지시), 작성자 `Hyunseok Byun <gumyoincirno@gmail.com>`. `target`·라이브러리 차트 커밋 금지(.gitignore). `Cargo.lock` 은 2026-10-11 부터 추적한다(D12).
> 다음 할 일(로드맵)은 **`docs/roadmap.md`**, 백엔드 설계는 **`docs/backend/`**, 배포/CI는 **`docs/ci-release.md`**.

---

## 현재 작업 — 플레이 기능 추가: 넓은 레인, 키 사용자 지정, 판정 자동 보정, 녹색 숫자 유지, 플레이 옵션 화면 (2026-10-11)

> 사용자 지시(2026-10-11, SKIN 메뉴 완료 뒤. 원문): "1. wide 스킨모드도 지원 (lane이 지금 좁은데 이걸 1.5배한 스킨도 필요함) 2. key customizing이 가능해야함 3. lane cover 키가 애매한데 이것도 key customizing이 되어야함 4. auto judge adjustmemnt 로 판정을 자동으로 보정해주는 기능도 필요함 5. 흰숫 녹숫으로 현재배율기준으로 잘 계산해주는듯한데 lift, lane cover를 기준으로 녹숫을 한번 설정해두면 다른 배속이 다른 곡에서도 해당 lane cover, lift에 맞게 자동으로 배속을 조절하는 기능도 필요해 6. 5번의 기능을 포함해서 플레이 옵션들도 skin 설정처럼 별도의 화면을 띄우고 실제로 보면서 조절할수있게하면되겠지". 운용은 아래 작업과 같다(Workflow 전용, Opus high 이하·Sonnet xhigh 이하, 웨이브마다 커밋·푸시와 CI 확인).
> 메인이 확인한 전제: 키 설정 화면은 이미 있다(`apps/rbms-player/src/stage/keyconfig.rs`. Settings 의 KEY CONFIG 행과 스킨의 키 설정 버튼으로 열림. 레인·스크래치 역방향·조작 키 10종·패드). 사용자가 없다고 느낀 원인(찾기 어려움, 범위 부족, 레인 커버 조작의 불명확함)을 함께 다룬다. `fix_hispeed`(기준 BPM)도 이미 있다.

- [x] N1. 넓은 레인 — 기본 스킨 플레이 문서(5/7/9/10/14키)에 Lane Width: Normal / Wide(1.5 배) 옵션.
- [x] N2. 판정 자동 보정 — beatoraja 의 notes display timing auto adjust 를 옮긴다(수동 보정 값이 없으면 그것부터).
- [x] N3. 녹색 숫자 유지 — 목표 녹색 숫자를 정해 두면 곡의 BPM, 레인 커버·리프트가 달라도 하이스피드를 맞춰 유지한다. 계산은 화면과 분리된 함수로.
- [x] N4. 키 사용자 지정 — 앱이 읽는 키의 전수 조사, 플레이 중 조작(레인 커버·리프트·히든의 값과 켬·끔 포함)과 선곡 단축키·화면 열기 키의 사용자 지정, 키 설정 화면을 여는 길과 화면 정리.
- [x] N5. 플레이 옵션 화면 — SKIN 메뉴처럼 왼쪽에 플레이 옵션, 오른쪽에 플레이 화면 미리보기를 두고 보면서 조절한다(N2·N3 의 옵션 포함).
- [x] N6. 검증·문서 — 게이트, CI, 캡처, 문서.

웨이브 10A 결과(2026-10-11, Workflow `wf_db42ee48-248`, 5 에이전트, 리뷰 major 1·minor 7 중 7건 수정·1건은 결정 대기). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,157 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 넓은 레인에서 200 BPM 의 녹색 숫자 유지 화면과 키 설정 화면 캡처를 확인. 사용자용 정의는 `docs/play-options.md`.
- N1: 플레이 문서 다섯 개에 Lane Width: Normal(op 909) / Wide(910). 레인 폭이 `lua/play/layout.lua` 의 `LANE_SCALE` 한 값에서 나온다(픽셀 실측 정확히 1.5 배). Normal 은 이전 캡처와 바이트까지 같다.
- N2: 수동 보정(`judge.offset_ms`)과 자동 토글(`judge.auto_offset`)은 이미 있었지만 자동은 한 판의 평균을 결과 화면에서 한 번만 적용했다. 원본(`JudgeManager.java:697-703`)처럼 입력마다 적용한다(GOOD 이상, ±150ms 이내, 오차/30ms). 리플레이에 오프셋 이력(`offset_steps`)을 기록한다. Settings 행 이름은 AUTO ADJUST.
- N3: 이전에는 배율만 저장돼 곡 사이에 녹색 숫자가 달라졌다(같은 설정에서 150 BPM 268, 200 BPM 201). `play.keep_green_number`(기본 꺼짐)와 `play.duration_ms`, 순수 함수 `rbms_config::held_speed`. 곡 시작과 레인 커버 이동·켬끔 때 하이스피드를 다시 계산하고, 하이스피드 키는 목표 녹색 숫자를 바꾼다. 리프트는 스크롤이 리프트된 레인 높이에 맞춰지므로 계산에 넣지 않아도 이동 시간이 유지된다.
- N4: 조작 20종(레인 커버·리프트·HIDDEN+ 의 켬·끔, 녹색 숫자, 판정 오프셋, 미세 단계 포함)과 단축키 29종(선곡, 화면 열기, 결과, 리플레이 분석)을 키보드·패드로 지정할 수 있다. 키 설정은 선곡의 F5, Settings 의 단추로 열린다. 레인 커버 행 이름에 방향을 적었다. 기존 키 설정 파일은 그대로 읽힌다.
- 결정 대기: KEEP GREEN NUMBER 의 기본값(지금 꺼짐). 남은 것: IR 로 올린 자동 보정 판의 리플레이는 서버가 `extra` 를 돌려주지 않아 입력 시각을 옮겨 올리는 방식으로 맞췄다(창 끝의 입력은 다를 수 있음). 내장 화면의 안내 문구는 출하 키 이름을 그대로 적는다.

N5 는 별도 Workflow(웨이브 10B).

웨이브 10B 결과(2026-10-11, Workflow `wf_48863e71-678`, 5 에이전트: 설계·골격 → 플레이 미리보기 ∥ 메뉴 화면 → 리뷰 → 수정·문서. 리뷰 major 1·minor 9 중 9건 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,221 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 유지 모드·PREVIEW BPM 200 의 PLAY 메뉴 캡처를 확인(녹색 300, 하이스피드 1.68 자동, 흰 숫자 300).
- N5: PLAY 메뉴(`stage/play_menu.rs`, `Stage::PlayMenu`, 선곡의 F8 과 Settings 의 단추). 왼쪽에 PREVIEW BPM 과 SPEED·LANE·JUDGE·CHART 묶음의 행(KEEP GREEN NUMBER 가 설정 행의 맨 위), 오른쪽에 고른 모드의 플레이 미리보기, 아래에 BPM·HI-SPEED·TRAVEL·GREEN·WHITE 요약. 리뷰어가 프레임에서 잰 값: 유지 모드에서 PREVIEW BPM 100·150·200 의 노트 이동 시간이 580·580·580ms(표시 584), 꺼짐에서 897·596·446ms. SKIN 메뉴와 공통 부품은 `stage/preview_menu.rs`. 미리보기 대상 지정은 `skin_preview/target.rs` 의 `PlayTarget`·`SpeedReadout`. 설계 `docs/plan/2026-10-11-play-menu.md`.
- Settings 에서 PLAY·GAUGE 탭이 빠졌다(5KEYS LAYOUT 은 DISPLAY 로, TOTAL 은 JUDGE 로, SCRATCH SIDE 는 INPUT 에 남음).
- O1: `rbms_config::JUDGE_OFFSET_STEP_MS` 가 5 에서 1 이 됐다(Shift 로 10). O3: KEEP GREEN NUMBER 는 기본 꺼짐이고 PLAY 메뉴 설정 행의 맨 위다.
- 함께 고친 것: 레인 커버·리프트·HIDDEN+ 를 키로 정하면 흰 숫자가 1 모자라게 나오던 것(30% 가 299). 비율 행을 걸음 수로 옮기고 스킨 number 14·314·315 를 반올림으로.
- 남은 것: RANDOM 과 HI-SPEED FIX 는 미리보기에서 구별되지 않는다(예시 차트가 한 템포). 스킨이 PLAY 메뉴를 여는 표준 이벤트가 없어 스킨 버튼은 없다.

### 추가 요청 (2026-10-11, 웨이브 10B 진행 중에 받음)

> 사용자 지시(요지. 원문은 대화 기록): (1) 판정 오프셋을 1ms 단위로 설정할 수 있어야 한다(지금은 5ms 단위). (2) 스킨 사용자 지정에서 부위별 불투명도를 조절할 수 있게 한다. 예: 레인 선, 레인 배경, 몇몇 UI 를 0 으로 하고 노트, 폭발, 판정선만 100 으로 두면 첨부 사진처럼 나온다. (3) KEEP GREEN NUMBER 는 꺼짐 유지, 다만 잘 보이게 위쪽으로 옮긴다. (4) 그래프(목표)에 따른 +- 표시를 넣는다. 첨부 사진처럼 목표 대비 숫자와 FAST/SLOW 가 뜬다. (5) 판정 글자와 숫자가 튀어나오는 애니메이션을 지운다. 제자리에 고정해 정적으로 콤보만 올라가며 현재 판정만 표시한다(플레이에 매우 거슬린다). (6) 판정 글자와 그 위의 목표 숫자 + FAST/SLOW 의 위치를 지금 위치에서 위로 +, 아래로 - 로 조정하고 숨길 수도 있게 한다(IIDX 에도 있는 기능).
> 첨부 사진 둘: (가) 레인 배경과 선 없이 검은 바탕에 노트, 폭발, 판정선만 보이는 플레이 화면. (나) 판정 글자 "GREAT 18" 바로 위 왼쪽에 목표 대비 점수 차 "+0003", 오른쪽에 "SLOW" 가 뜨는 IIDX 화면.

- [x] O1. 판정 오프셋을 1ms 단위로(지금 5ms 단위인 곳 전부).
- [x] O2. 부위별 불투명도 — 기본 스킨 플레이 화면의 부위(레인 배경, 레인 선, 판정선, 건반 빛, 폭발, 노트, 게이지, 정보 패널 등)마다 불투명도를 0~100% 로 조절한다. 스킨 형식의 오프셋(알파)으로 선언하고 SKIN 메뉴에서 미리보며 바꾼다.
- [x] O3. KEEP GREEN NUMBER 는 기본 꺼짐 유지(결정 D15), PLAY 메뉴의 맨 위에 둔다.
- [x] O4. 판정 글자 위에 목표 대비 점수 차(부호 있는 숫자)와 FAST/SLOW 를 함께 표시(사진 나).
- [x] O5. 판정 글자와 콤보의 튀어나오는 애니메이션 제거 — 제자리에 고정해 정적으로 표시한다(결정 D16).
- [x] O6. 판정 글자와 그 위 줄(점수 차 + FAST/SLOW)의 세로 위치 조절(위로 +, 아래로 -)과 숨김.

웨이브 10C 결과(2026-10-12, Workflow `wf_c7f257e4-595`, 4 에이전트: 스킨 ∥ 메뉴·엔진 → 리뷰 → 수정·문서. 리뷰 major 1·minor 9 중 6건 수정, 4건은 근거를 남기고 보류). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,277 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과, ModernChic 화면은 이전과 픽셀까지 같음. 메인이 부위 불투명도로 레인 배경·선을 끈 7키 화면과 점수 차 + SLOW 가 뜬 판정 줄 캡처를 사용자 사진 두 장과 대조해 확인.
- O5 원인과 처리: 스킨의 키프레임(60ms 동안 1.15 배 확대와 숫자 미끄러짐, 440~500ms 페이드), 시트 프레임(PGREAT 색 돌림, 나머지 깜빡임), `shift`(자릿수가 늘면 글자가 밀림)를 전부 없앴다. 판정 줄은 500ms 동안 같은 자리·크기·색으로 있다가 한 번에 사라진다(0~496ms 의 10프레임이 픽셀까지 같음). PGREAT 는 하늘색, GREAT 는 노랑의 "GREAT". 콤보는 5자리 고정 칸(빈 자리에 옅은 0).
- O4: 판정 글자 위 줄 왼쪽에 부호 + 네 자리 점수 차(옵션 Score Difference: Target / Best / Off), 오른쪽에 FAST/SLOW 또는 ms. 스코어 그래프의 TARGET 과 같은 값.
- O6: 오프셋 "Judge Position"(y 축, + 가 위)이 판정 글자와 위 줄을 함께 옮긴다. 숨김은 옵션 Judge Display: Show / Hide. PLAY 메뉴의 LANE 묶음과 SKIN 메뉴에 나온다.
- O2: 플레이 문서에 알파 축만 가진 "<Part> Opacity" 오프셋 19개(DP 18개). 메뉴는 알파 축만 가진 오프셋을 퍼센트 행(5%, Shift 로 25%)으로 보이고 값이 그 프레임부터 미리보기에 반영된다. 엔진은 보간 중에도 알파 오프셋을 더하고 note·judge·gauge 등 특수 객체에도 적용한다.
- 콤보 빈 자리의 옅은 0 은 사용자 결정(D17)으로 없앴다(아래 웨이브 10D). 남은 것: SKIN 메뉴의 묶음 머리, 불투명도 1% 걸음, 불투명도 범위가 0~200% 로 읽히는 것, LOADING·READY·FULL COMBO·폐점 표시는 불투명도 부위가 아님.

---

웨이브 10D 결과(2026-10-12, Workflow `wf_deae4391-3d1`, 3 에이전트: 구현 → 픽셀 리뷰 → 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,282 통과·0 실패·ignored 5, 골든 불변, ModernChic 캡처 테스트 통과. 메인이 콤보 7·67·648·1234·12345 와 판정 다섯 가지의 캡처를 확인.
- 사용자 결정 D17: 콤보의 옅은 0 을 없애고 숫자를 판정 글자 바로 뒤에 붙인다("GREAT 67"). 판정 글자의 왼쪽 끝은 판정 종류와 콤보에 상관없이 고정이고 BAD·POOR 와 위 줄도 같은 왼쪽 끝에서 시작한다.
- 구현 단위는 스킨만으로는 불가능함을 확인하고 멈췄다: 원본 로더가 judge 숫자를 가운데 정렬로 고정하고(`JsonPlaySkinObjectLoader.java:258-259`) 호스트가 콤보 수와 판정 종류를 속성으로 답하지 않는다. 수정 단위가 엔진에 judge 객체의 `numberAlign`(0 오른쪽, 1 왼쪽, 그 밖 가운데. 쓰지 않으면 원본대로)을 더했다. 원본에 없는 R-BMS 확장이며 메인이 승인했다(기본 스킨은 자작이고, 이 키를 모르는 beatoraja 는 가운데 정렬로 그린다).
- 다섯 자리 콤보가 필드 안에 들도록 좁은 필드의 판정 글자가 조금 작아졌다(7·14키 기본 폭 64 → 60, 5·10키 48 → 46).

### 추가 요청 (2026-10-12)

> 사용자 지시(요지): (1) 표 대조(MATCHING TABLES)가 오래 걸린다. URL 로 한 번 내려받으면 저장해 두고, URL 을 다시 읽는 키를 둔다. (2) 수치 조정이 5 단위 같은 식으로 돼 있다. 슬라이더와 숫자 직접 편집이 가능하게 한다.
> 메인이 확인한 전제: 표는 운영체제 임시 폴더에만 저장되고(`tablesrc.rs` 의 `load_table_source`), 실행할 때마다 먼저 내려받은 뒤 실패했을 때만 저장본을 쓴다.

- [x] Q1. 표 저장과 리로드 — 받은 표를 설정 폴더 아래에 저장하고 다음 실행부터 저장본을 바로 쓴다. Tables 화면과 선곡에서 다시 받는 키.
- [x] Q2. 숫자 행의 슬라이더와 직접 입력 — SKIN 메뉴, PLAY 메뉴, Settings 의 숫자 행.

웨이브 11 결과(2026-10-12, Workflow `wf_16935ed4-201`, 4 에이전트: 표 ∥ 숫자 행 → 리뷰 → 수정·문서. 리뷰 major 2·minor 10 중 10건 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,381 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 PLAY 메뉴에서 LANE COVER 슬라이더를 끄는 중의 캡처를 확인(미리보기가 따라옴).
- Q1 측정: 시간은 전부 네트워크였다(실제 주소의 받기 2.3~5.2초, 저장본 읽기 0.9ms, 11,000 차트의 md5 색인 1.4ms, 대조 0.08ms). 받은 표를 `<설정 폴더>/tables/<해시>.json` 에 저장하고(`crates/rbms-table/src/saved.rs` 의 `SavedTable`) 시작과 재스캔은 저장본이 있으면 네트워크를 타지 않는다. MATCHING TABLES 가 실측 2.43초에서 12ms 로 줄었다. 다시 받기: Tables 화면의 R(커서의 표)·A(전부), 선곡의 F6(`Shortcut::ReloadTables`, 사용자 지정 가능). 백그라운드에서 받고 실패하면 저장본과 폴더를 그대로 둔다. Tables 행에 저장 뒤 지난 시간과 상태, 30일 넘은 저장본은 경고색.
- Q2: 숫자 행의 공통 모델 `rbms_config::NumberSpec`(걸음 수 정수)과 공통 부품 `stage/number_row.rs`. SKIN·PLAY 메뉴와 Settings 의 숫자 행에 슬라이더(끄는 동안 미리보기에 반영, 뗄 때 한 번 저장)와 직접 입력(Enter 나 숫자 클릭, Esc 는 입력만 취소). 좌·우는 가장 잔 단위(불투명도 1%, 오프셋 1, 판정 오프셋 1ms, 하이스피드 0.01, 가림막 0.1%, 볼륨 1%), Shift 로 10 칸.
- 메인이 받아들인 절충: 불투명도는 슬라이더와 화살표가 0~100%, 직접 입력만 200% 까지(ModernChic 의 밝기 행처럼 양수 알파가 뜻이 있는 스킨 때문).
- 사용자 결정(2026-10-12): 저장본이 없는 표는 선곡 뒤에서 받는다(A). 연습 패널에도 슬라이더와 직접 입력을 넣는다(A). 아래 웨이브 12.

웨이브 12 결과(2026-10-12, Workflow `wf_97766c59-324`, 4 에이전트: 표 ∥ 연습 패널 → 리뷰 → 수정·문서. 리뷰 critical·major 0, minor 8 중 7건 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,429 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 연습 패널의 슬라이더 캡처를 확인.
- 표: 로딩의 표 단계는 저장본과 로컬 파일만 읽는다(`tablesrc::read_and_match_all`, 네트워크 0회). 저장본이 없는 주소 표는 선곡의 첫 프레임이 뒤에서 한 번 받기 시작한다(`start_first_fetches`). 실측(실제 주소 하나 + 응답 없는 주소 하나): 선곡 진입 20.34초 → 5.32초, 표 단계 15.01초 → 0.09초. 실패한 주소는 그 실행에서 자동으로 다시 받지 않고 알림은 한 줄. Tables 화면의 새 등록은 로딩으로 가지 않고 그 자리에서 `FETCHING...` 이 된다. 진행 중 목록은 `AppShared.table_fetches`(`TableFetches`). 표 폴더가 뒤늦게 나타나도 커서가 있던 폴더를 따라가고, 검색을 나갈 때도 시작한 폴더로 돌아간다(`tablesrc::ListPlace`).
- 연습 패널: 숫자 행 여섯(START TIME, END TIME, GAUGE VALUE, JUDGERANK, TOTAL, FREQUENCY)이 공통 모델과 공통 부품을 쓴다. 좌·우는 가장 잔 단위(시각 0.1초, 나머지 1), Shift 10 칸. 시각은 초나 분:초로 직접 입력. TOTAL 의 슬라이더와 화살표는 10~1000, 그 위는 직접 입력. 맨 위에 START 행이 생겼고 숫자 행의 Enter 는 직접 입력을 연다(다른 메뉴와 같은 규칙).
- 남은 것: 로딩의 SCANNING 단계에서 Esc 를 누르면 스캔 결과가 버려지는 기존 동작, 스캔 없이 시작하는 실행은 표를 읽지 않음, 선곡 옵션 패널의 걸음은 원본대로.

---

### 추가 요청 (2026-10-12, 두 번째 묶음)

> 사용자 지시(요지): (가) 판정선이 떠 있다. 판정선이 필드 맨 아래에 딱 붙는 스킨으로 한다. (나) 판정 adjust 를 설정하면 곡과 싱크가 안 맞는 느낌이 든다. 디버그한다. 사용자가 덧붙인 증상: -50ms 로 두면 곡도 그에 맞춰 나와야 하는데 곡 자체가 50 밀리는 느낌이다. (다) 플레이 중에 판정 adjustment 를 조정할 수 있게 키를 추가한다. (라) 레인 커버와 리프트를 START 를 누른 채(리프트는 START 와 또 다른 키를 함께 누른 채. 예: A 와 A+W) 스크래치를 돌려 1 씩 조절한다. 녹색 숫자는 고정된 채로 레인 커버가 1 씩 움직인다(지금은 step 단위라 불편). (마) 스킨 배경의 기본 색은 검은색이어야 한다(지금은 약간 파랗다). 전체를 검정·회색·흰색의 모노톤으로 바꾼다. 노트와 폭발은 지금의 흰색·파란색 그대로. 또는 색을 조절할 수 있게 한다. (바) 조절한 스킨을 내보내는(export) 기능을 넣는다.

- [x] R1. 판정선을 필드 맨 아래에(기본 스킨). 웨이브 13.
- [x] R2. 판정 보정과 곡의 싱크 디버그. 웨이브 13.
- [x] R3. 플레이 중 판정 보정을 바꾸는 키(기본 키 배정과 화면 표시).
- [x] R4. START + 스크래치로 레인 커버 1 씩, START + SELECT + 스크래치로 리프트 1 씩(녹색 숫자 유지).
- [x] R5. 기본 스킨의 모노톤 팔레트(바탕 검정)와 색 조절 옵션.
- [x] R6. 조절한 스킨 내보내기.
- [ ] R7. 첫 릴리스 — 사용자 결정(2026-10-12): 웨이브 13 과 웨이브 14 가 끝난 뒤 릴리스 워크플로를 처음 돌린다(A). macOS 서명·공증 시크릿 다섯 개(`MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`)는 사용자가 저장소에 등록했고 메인이 `gh secret list` 로 이름을 확인했다. 워크플로의 서명 단계는 커밋 `8736a20`(이름과 방식은 사용자의 다른 저장소 TAIDE 와 같음, `docs/ci-release.md` 3.1절). 원격에 `prod` 브랜치가 없으므로 실행 경로(prod 생성 뒤 푸시, 태그, 수동 실행)는 그때 사용자에게 확인한다. 릴리스는 공개 게시이므로 실행 직전에 한 번 더 확인받는다. Windows 서명은 나중에.

웨이브 13 결과(2026-10-12, Workflow `wf_0c18e724-696`, 4 에이전트. 세션이 한 번 꺼져 리뷰부터 재개했다. 리뷰 critical 1·minor 4 중 4건 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,448 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 판정선이 필드 맨 아래에 붙은 7키 캡처를 확인.
- R2 원인(측정으로 확정): 원본은 judgetiming 을 노트 필드를 그리는 시각 한 곳에만 더하는데(`LaneRenderer.java:300-301`) R-BMS 는 입력의 판정 시각에 더하고 노트는 그대로 그렸다. 수정 전 +100 에서 PGREAT 창이 박자보다 80~120ms 앞이었다. 수정 뒤 측정(오프셋 0, ±30, ±100): 소리와 PGREAT 창(-20~+20)은 박자 그대로이고 노트가 판정선에 닿는 시각만 오프셋만큼 옮겨진다. 부호는 원본과 같다(+ 는 노트가 먼저 온다). 설정 스키마 5 가 이전 저장값의 부호를 한 번 뒤집어 읽는다. 자동 보정은 원본 식 그대로. BGA 는 오프셋을 타지 않는다. 기존 리플레이는 기록 당시와 같은 판정으로 재생된다(픽스처 3개).
- R1: 판정선 아래의 건반 표시 칸을 없애 필드가 판정선에서 끝난다(판정선 904~911, 그 아래는 화면 바탕, 928 에 게이지). 노트가 내려오는 길이 840 → 904. Key Display 부위와 불투명도 오프셋 105 는 삭제(불투명도 행 18, DP 17).
- 사용자 결정 대기: 폭발 고리가 판정선 아래 변에서 끊겨 위쪽 반원만 보인다(게이지와 겹치지 않게 한 구현 단위의 판단). 선곡 패널 3 의 문구는 "NOTES 1 MS LATER / EARLIER" 로 바꿨다.

웨이브 14 결과(2026-10-12, Workflow `wf_52f9fa93-314`, 5 에이전트: 조작 K ∥ 색 T ∥ 내보내기 X → 리뷰 → 수정·문서. 리뷰 major 2·minor 5 중 6건 수정, 1건은 사용자 결정 대기). 게이트(최종 트리): fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,513 통과·0 실패·ignored 5, 동영상 끈 구성 통과, 골든과 Cargo.lock 불변, 덤프 `--strict` 문서 9개 통과, 생성 이미지 `--check` 일치, 두 팩의 환경 변수 테스트 2,223 통과. 메인이 최종 캡처(7키 플레이, 선곡, 꺼진 리프트를 움직인 발치, SKIN 메뉴의 EXPORT 행)를 확인. 이전 푸시 `103f196` 의 CI 는 세 OS 통과.
- R3: 플레이 중 `9` 가 판정 오프셋 -1 ms, `0` 이 +1 ms. 누르고 있으면 0.3초 뒤 초당 20, ±200 에서 멈춘다. 오토플레이와 리플레이에서는 무시. AUTO ADJUST 켬·끔 키는 미지정으로 둔다. `keyconfig.ron` 에 `generation` 필드(지금 2)를 더해 "그때는 키가 없었다"와 "사용자가 뗐다"를 가른다. 옛 파일은 그 키를 다른 데 주지 않았을 때만 새 출하 키를 읽는다.
- R4: 고치기 전 실측으로 START + 스크래치는 이미 1 단위였으나 누른 첫 프레임부터 50 ms 마다 움직여 보통 길이의 누름이 2 였다(지정 키 RIGHT/LEFT 는 10). 지금은 누름 한 번이 정확히 1, 0.3초 뒤 반복하고 1.5초·3초에 빨라져 끝에서 끝까지 약 7초. START + SELECT + 스크래치는 리프트 1 씩(신규). 녹색 숫자는 유지되고 하이스피드가 따라 바뀐다(HI-SPEED FIX OFF 에서도). 스크래치 역방향 키를 원본처럼 출하한다(왼쪽 Ctrl, 2P 는 오른쪽 Ctrl). 이것이 없으면 한 방향으로만 움직였다(리뷰 major). 가림막을 바꾸는 동안 발치에 COVER·LIFT 가 함께 뜨고 발치는 4열이 됐다. 원본과 다른 곳: SELECT 를 뗀 것을 START 누름으로 세지 않는다. 턴테이블이 묶은 녹색 숫자는 START 를 뗀 뒤에도 그 판 동안 기억한다.
- R5: 기본 스킨의 팔레트가 두 벌(`theme.mono` 기본, `theme.slate` 는 이전 값). Mono 는 바탕 000000 에 면·선·글자가 전부 R=G=B. 노트·폭발·판정 글자·램프·게이지·그래프의 색은 그대로. 옵션: Theme(Mono/Slate), Accent(White/Sky/Amber/Green/Pink)는 9개 문서 전부, Note Colors 5종은 스크래치가 있는 플레이 4개, Bomb Color(Note/White)는 플레이 5개. Theme Slate + Accent Sky 가 이전 화면이다. 폭발은 판정선에 중심을 둔 온전한 고리로 되돌렸다(아래 절반이 게이지 위에 겹친다).
- R6: SKIN 메뉴 맨 아래에 EXPORT 행. 고른 폴더 아래 `<팩 이름>-export`(있으면 -2, -3)에 팩 전체와 조절값 파일 `rbms-adjustments.ron` 을 쓴다. 그 파일이 든 폴더를 팩으로 열면 옵션·파일 슬롯·오프셋·불투명도가 사용자 저장값 아래에 겹쳐 적용된다(설정 파일 형식 불변). 같은 이름의 옵션·오프셋이 다른 화면에도 있으면 `A` 키로 모든 화면에 적용한다(SKIN·PLAY 메뉴). 내보낸 팩의 오프셋이 화면에 그려지지 않던 것(리뷰 major)은 `customisation_for` 로 고쳤고 픽셀 비교 테스트로 고정했다.
- 사용자 결정 대기: 앱이 스킨 위에 덧그리는 UI(SKIN·PLAY 메뉴, H 가이드, 알림, 코스 탭 표지)와 내장 화면은 앱 테마(`crates/rbms-render/src/theme.rs`)의 청색 계열 그대로다. 무채색으로 바꾸면 내장 화면 골든 기대값이 걸린다.
- 남은 것: 아날로그 턴테이블은 주입한 틱으로만 확인(QA 3.14절 N3). 지정 키(RIGHT/LEFT 등)는 누르고 있어도 반복하지 않는다. 내장(스킨 없는) 플레이 화면은 START·SELECT 조합을 읽지 않는다. 파일 슬롯은 "모든 화면에 적용" 대상이 아니다. 이전 빌드가 다시 쓴 `keyconfig.ron` 은 `generation` 이 빠져 사용자가 뗀 출하 키가 되살아날 수 있다. 리뷰가 스크래치에 남긴 임시 사본(wave14-review, 5.4 GB)은 저장소 밖이다.

사용자 결정(2026-10-12, 웨이브 14 보고 뒤): (1) 첫 릴리스는 `v0.1.0` 태그를 `dev` HEAD 에 푸시하는 경로(A). (2) 앱 자체 UI 도 무채색으로 맞추고 내장 화면 골든 기대값을 함께 갱신한다(A, D18). (3) 그 작업은 Workflow 로(A).

- [x] R8. 앱 자체 UI 의 무채색화(웨이브 15) — SKIN·PLAY 메뉴, 키 설정, 설정, Tables·Folders, 로딩, H 가이드, 알림, 코스 탭 표지, 내장(스킨 없는) 선곡·결과·플레이 화면. 바탕 검정, 면·선·글자는 R=G=B, 색은 의미색(판정, 램프, 난이도, 경고)과 노트·폭발에만. 확인된 사실: 앱은 첫 실행에 모든 색을 적은 `~/.config/rbms/theme.ron` 을 쓰므로(`assets.rs` 의 `THEME_TEMPLATE`) 기본값만 바꾸면 기존 사용자의 화면은 바뀌지 않는다. 손대지 않은 옛 템플릿을 알아보고 새 기본을 따르게 하는 것이 범위에 든다.

R7 첫 실행 결과(2026-10-12, 태그 `v0.1.0` = `ef75bbc`, run 38222704409): 실패. Windows 빌드는 통과, macOS 는 `Build universal binary` 에서 `error[E0463]: can't find crate for core`(`x86_64-apple-darwin`). 원인: 워크플로가 `dtolnay/rust-toolchain@stable` 에 두 타깃을 설치했지만 cargo 는 `rust-toolchain.toml` 의 1.95.0 으로 빌드한다. 서명·공증·draft 단계는 돌지 않았고 릴리스는 만들어지지 않았다(태그만 원격에 있다). 수정: 릴리스의 세 잡을 CI 와 같은 `dtolnay/rust-toolchain@master` + `toolchain: "1.95.0"` 으로 맞췄다(`docs/ci-release.md`). 태그 경로는 태그가 가리키는 커밋의 워크플로 파일을 쓰므로 `v0.1.0` 을 다시 돌려도 같은 실패다. 다시 실행하는 방법(태그를 옮길지, 다음 버전으로 갈지)은 사용자 결정.

사용자 결정(2026-10-12, 첫 릴리스 실패 뒤): 웨이브 15 가 끝난 커밋으로 `v0.1.0` 태그를 옮겨 다시 돌린다(A. 릴리스가 만들어진 적이 없다. 원격 태그를 지우고 다시 만드는 작업의 승인).

웨이브 15 결과(2026-10-12, Workflow `wf_3f98dcb0-532`, 3 에이전트: 구현 Opus → 리뷰 Sonnet → 수정·문서 Sonnet. 세션이 한 번 꺼져 리뷰부터 재개했다. 리뷰 major 1·minor 5 중 5건 수정, 1건은 사용자 결정 대기). 게이트(최종 트리): fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,527 통과·0 실패·ignored 5, 동영상 끈 구성 2,235 통과, 덤프 `--strict` 통과, 두 팩의 환경 변수 테스트 2,235 통과. 메인이 최종 캡처(SKIN 메뉴, H 가이드, 내장 선곡)를 확인.
- 팔레트: `Theme::default` 가 기본 팩 Mono 와 같은 단계(바탕 0, 면 22·38, 선 60, 글자 242·184)이고 안내 글자만 144(128 이면 올라온 면 위 대비가 전보다 낮아진다). 고른 행과 탭은 밝은 회색 면과 흰 글자, 슬라이더 손잡이와 키 대기 칸은 흰색. 테마 필드는 더하지도 빼지도 않았다.
- 남긴 색(뜻을 나르는 것): 판정, FAST/SLOW, 클리어 램프와 이름, 난이도, 랭크, 점수 차, 스코어·밀도 그래프, 게이지, GREEN 숫자, 경고·오류 알림, 키 충돌, `warn`(실패·오래됨·한계·되묻기), 내장 필드의 노트·빔·폭발·판정선.
- `theme.ron`: 새 템플릿은 모든 색 줄이 주석이라 값을 못박지 않는다. 지금까지 출하된 템플릿은 이력 전체에서 한 판뿐이고, 그것과 글자까지 같은 파일(CRLF·끝 빈 줄 차이 무시)은 새 템플릿으로 다시 쓰고 기본값을 쓴다. 한 글자라도 고친 파일과 깨진 파일은 읽기만 한다. 쓰기에 실패하면 바꿨다고 알리지 않는다. 사용자의 실제 `~/.config/rbms/theme.ron` 은 옛 템플릿과 같아(읽기 전용 비교) 다음 실행에서 바뀐다.
- 내장 플레이 필드: 번들 `assets/skins/{normal,wide,default}.ron` 의 레인 바탕·테두리·구분선을 회색으로(리뷰 major. `SkinConfig::default` 만 바꿔서는 실제 화면에 닿지 않았다).
- 골든: 서명 8개를 다시 구웠다(SELECT, RESULT 3, HUD, 필드 2, TOASTS). `GOLDEN_BLANK`, `GOLDEN_LONG_TOAST`, PNG 골든 넷은 그대로. 배치는 바꾸지 않았다.
- 스킨 팩 화면: 기본 팩 캡처 741장 중 519장이 바이트까지 같고, 다른 것은 앱이 그린 부분과 시각에 따라 달라지는 프레임(같은 코드 두 실행도 같은 자리가 다름). 메뉴의 미리보기 사각형은 0 픽셀 차이.
- 사용자 결정 대기: 내장 화면의 모드 배지(7K·5K 등)와 결과의 MAX COMBO 초록, 기록 모달의 COMBO 초록·BP 주황을 무채색으로 할지.
- 남은 것: 값을 고친 옛 `theme.ron` 사용자는 전의 색 그대로(의도). 앱 테마는 스킨의 Theme·Accent 옵션을 따르지 않는다. 실제 창 확인은 QA 3.15절 O1~O5. 구현 단위가 바꾸기 전 캡처를 얻으려다 저장소 `target` 의 워크스페이스 빌드 캐시를 지웠다(소스 영향 없음, 다음 빌드가 다시 컴파일).

진행: R7 재실행은 이 웨이브의 푸시와 CI 확인 뒤 태그를 옮겨서.

---

## 현재 작업 — 첫 실행 피드백 처리와 SKIN 메뉴 (2026-10-11)

> 사용자 지시(2026-10-11, 웨이브 8 완료 뒤 실제로 실행해 보고): 결정 4건(D11~D14, `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`)과 문제 4건, 요청 1건. 운용은 아래 스킨 작업과 같다: 위임은 Workflow 전용, 모델은 Opus(high 이하)와 Sonnet(xhigh 이하)만, 웨이브가 게이트를 통과하면 커밋·푸시하고 `gh run list` 로 CI 를 확인한다.

- [x] M1. 검색할 때 설정·보조 옵션이 먹지 않는 문제 — 검색 결과 목록은 일반 목록과 똑같이 조작되고, 검색어를 치는 중에는 글자 키만 검색어가 갖는다. 스킨에 검색 칸이 있으면 단축키도 그 칸에 포커스를 준다(앱 검색 상자와의 이원화 제거).
- [x] M2. 등록한 난이도표 URL 이 폴더에 나오지 않는 문제 — 실제 주소로 재현해 받기·해석·목록의 어느 단계에서 끊기는지 찾아 고친다. 실패하면 이유를 알린다.
- [x] M3. 곡 이름 글자 깨짐 — 파서의 인코딩 판정(Shift_JIS 고정 → UTF-8·Shift_JIS·EUC-KR 판정)과 실제 앱의 글꼴 폴백을 함께 조사해 고친다. 파서 판을 올려 다시 읽게 한다.
- [x] M4. FAST/SLOW 가 보이지 않는 문제 — 원인 확정 뒤 기본 스킨에서 기본으로 보이게 한다.
- [x] M5. 결정 반영 — 레터박스 마이그레이션(D11), `Cargo.lock` 추적과 `--locked`(D12), 기본 스킨의 판정을 IIDX 대로·램프 색 등은 ModernChic 값대로(D13).
- [x] M6. SKIN 메뉴 — SKIN 을 설정 화면에서 빼 별도 화면으로 만든다. 왼쪽에 스킨 설정값(팩, 화면, 옵션·파일·오프셋), 오른쪽에 그 화면의 스킨 미리보기를 두고 값을 바꾸면 미리보기에 바로 반영한다.
- [x] M7. 검증·문서 — 게이트, CI, 캡처 확인, `docs/skin.md` 와 조사 문서 갱신.

웨이브 9A 결과(2026-10-11, Workflow `wf_8f0628bd-66a`, 7 에이전트, 리뷰 major 5·minor 5 전부 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 4,964 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트 통과. 메인이 검색 중 선곡과 SLOW 가 뜬 플레이 캡처를 확인.
- M1 원인: `searching` 플래그 하나가 "치는 중"과 "결과 표시 중"을 겸해 검색이 걸린 내내 입력이 막혔다. 세 상태로 나누고 검색어를 선곡 화면이 갖게 했다(스킨 검색 칸은 표시만). IME 조합 지원.
- M2 원인: 표 받기가 등록 주소를 JSON 으로만 읽어 표준 형식의 HTML 페이지 주소가 실패했고, 실패한 표가 숨겨진 채 "table added" 가 떴다. 실제 주소로 확인: 이름 "発狂BMS難易度表", 1,035 항목, 26 레벨. 사용자 설정의 `library.tables` 는 비어 있어 업데이트 뒤 주소를 다시 등록해야 한다.
- M3 원인: 파서가 UTF-8 이 아니면 전부 Shift_JIS 로 읽었다. 사용자 라이브러리에서 문제 파일 15개(EUC-KR 13, BOM + Shift_JIS 2). 한자가 간체 글꼴로 그려지던 것도 고쳤다. 다음 실행에서 전체 재스캔.
- M4 원인: 기본 스킨의 Judge Detail 기본값이 Off 였다(엔진은 정상). 기본값을 Fast-Slow 로.
- M5: 스키마 4(레터박스), `Cargo.lock` 추적과 `--locked`(릴리스의 version 잡이 `cargo update --workspace` 로 잠금 파일도 올린다), 기본 스킨의 판정을 IIDX 식으로(PGREAT 는 "GREAT" 가 색을 돌며 번쩍임, 판정 수 표 5행 + 결과에 COMBO BREAK·EMPTY POOR)·램프와 난이도 색, 클리어 표기를 ModernChic 값으로.
- 확인 못 한 것: 사용자 라이브러리 볼륨(`/Volumes/SSD`)이 수정 단계에서 분리돼 있어 BOM 규칙 변경 뒤의 전수 비교는 다시 돌리지 못했다. 실제 창의 IME 동작.

웨이브 9B 결과(2026-10-11, Workflow `wf_56291723-64d`, 5 에이전트: 설계·골격 → 미리보기 엔진 ∥ 메뉴 화면 → 리뷰 → 수정·문서. 리뷰 major 2·minor 7 전부 수정). 게이트: fmt·clippy·`cargo test --workspace --no-fail-fast --locked` 5,054 통과·0 실패·ignored 5, 골든 불변, 두 팩의 환경 변수 테스트와 GPU 사각형 비교 통과. 메인이 기본 스킨 7키(옵션 변경 뒤)와 ModernChic 선곡의 SKIN 메뉴 캡처를 확인.
- M6: SKIN 은 Settings 에서 빠져 독립 화면(`stage/skin_menu.rs`, `Stage::SkinMenu`)이 됐다. 왼쪽에 팩·화면·문서·로드 상태·옵션·파일·오프셋 행, 오른쪽에 그 화면의 스킨을 예시 데이터로 그린 미리보기(`skin_preview.rs`, 사각형 그리기 어댑터 `rbms_render::RegionRenderer`). 값을 바꾸면 미리보기에 바로 반영된다. 들어가는 길: 선곡의 F12, 스킨의 SKIN 버튼(이벤트 14), Settings 탭 줄의 "SKIN MENU >" 단추. 설계 `docs/plan/2026-10-11-skin-menu.md`.
- CI: 9B 푸시(`4d637fd`)에서 Windows 테스트 2건 실패 → 원인은 실패 문구에서 팩 폴더를 뺄 때 verbatim 접두사(`\\?\`) 철자를 찾지 못한 것(`skin_select.rs` 의 `reason_beside`). `569a995`·`c9e4269` 로 고쳐 run `c9e4269` 에서 세 OS 통과.
- 남은 것: 키를 누르고 있어도 값이 반복되지 않는다(앱 전체 동작. Shift+좌우로 오프셋 열 칸). 옵션 변경과 장면 반복 때 2~3프레임 검게 보인다. 키 설정 화면은 미리보기가 없다. 실제 창 확인 항목은 `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` §3.7.

---

## 현재 작업 — 레퍼런스 형식 Lua 스킨 완전 호환과 기본 스킨 교체 (2026-10-09)

> 사용자 지시(2026-10-09): beatoraja 용 풀 Lua 스킨 ModernChic 을 그대로 읽어 똑같이 그릴 수 있도록 스킨 체제를 전면 검토·수정하고, 그 스킨을 참조해 조금 단순화한 기본 스킨을 만든다. UI·레이아웃도 크게 바뀐다. 이전 세션의 스킨 작업(`steel-neon` 3세대와 혼합 합성 구조)은 사용자가 부적합하다고 판정해 삭제한다.
> **사양(정본)**: `docs/plan/2026-10-09-lua-skin-compat.md` · **결정**: `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md` · **조사**: `docs/reference/skin-compat/README.md`(색인). 스킨 작업 전에 원본을 통독하지 말고 색인에서 절을 찾아 읽는다. 조사 문서는 코드가 바뀌면 같은 단위 안에서 갱신한다.
> 운용: 위임은 Workflow 전용(Agent 단독 호출·general-purpose 금지), Opus 는 effort high 이하·Sonnet 은 xhigh 이하, 사양·판정·Git 은 메인 직접. 커밋은 웨이브가 게이트를 통과할 때마다 메인이 커밋·푸시한다(D7, 이 작업에 한해 §9 의 "사용자 요청 시에만"을 대체). 이름 금지 규칙은 폐지됐다(D2).
> 자동 시작: 2026-10-09 21:54 KST 예약(세션 한정 예약이라 세션을 닫으면 사라진다). 예약이 사라졌으면 새 세션에서 사양 §10 "자동 시작 절차"대로 L3 웨이브 1 부터 시작한다.

- [x] L1. 조사 — 외부 묶음의 화면별 요구 기능, 레퍼런스 구현의 스킨 엔진 계약(Lua 환경·객체·속성·화면별 로더), 현 엔진·렌더·앱 배선의 격차를 실제 파일 근거로 확보했다. Workflow `wf_43ad1f8d-42b`(조사자 14 + 종합 1 + 비판 1, 오류 0). 종합의 R-BMS 현황 26행과 레퍼런스 의미론 19항목은 비판 단계가 원본과 대조해 전부 일치를 확인했고, 작업 분해의 결함(소유 파일 누락·빠진 선행·검증 수단 부재)과 빠진 기능 9건·고유 기능 구멍 15건은 `99-critique.md`에 있다.
- [x] L1-b. 조사 결과 영속화(사용자 지시 2026-10-09) — 보고서 16개 전문(1.5MB)과 색인 `README.md`를 저장했다(미커밋: 보고서에 레퍼런스·외부 묶음 이름이 있어 이름 규칙 결정 대기). 원 지시: 조사 보고서 전문과 색인을 `docs/reference/skin-compat/`에 저장해 이후 세션이 원본을 다시 통독하지 않게 한다. 색인에는 보고서별 다루는 범위와 찾는 법을 적는다. 저장한 조사 문서는 살아 있는 문서로 관리한다: L3~L5 의 각 구현 단위가 끝날 때 그 단위가 바꾼 코드를 다루는 절(특히 현 엔진·렌더·앱 현황과 격차 표)을 같은 단위 안에서 갱신하고, 문서 상단에 최종 갱신일과 대응 단계를 적는다.
- [x] L2. 사양·결정 확정 — 사용자 결정 7건(D1 기본 스킨 자작, D2 이름 규칙 폐지, D3 구 스킨 삭제, D4 영어·사이드메뉴 2패널, D5 mp4 포함, D6 캡처 없이 좌표 대조, D7 웨이브별 커밋·푸시)을 받아 사양과 결정 문서를 기록했다.
- [x] L3. 엔진 구현 — 사양 §6 의 웨이브 1~2.
  - [x] 웨이브 1A — 철거(W1-1a·b·c, W1-2, W1-3, W1-6): 구 번들 3세대와 혼합 합성 삭제(사운드 22개는 `assets/skins/rbms-default/` 로 이동), 설정 스키마 3 과 마이그레이션(파일·계정 블롭), 모델 확장 필드 삭제, Lua 5.2 전환, 폰트 캐시 패밀리 분리. Workflow `wf_575fa3b4-9ac`(7 에이전트), 리뷰 major 2·minor 3 전부 수정. 게이트: fmt·clippy·`cargo test --workspace` 3,111 통과·0 실패·ignored 3·`git diff --check`, 내장 골든 불변.
  - [x] 웨이브 1B — 기반(W1-4, W1-5, W1-7~W1-10): 타이머 µs(OFF = `i64::MIN`)·stretch 11종·오프셋 r 부호·acc 선언 순서, 장면 시계와 전환 리셋, 마우스 release·drag·휠, GPU 논리 크기 = 물리 뷰포트와 `ScaledRenderer`, 표면 바이트 통과(E10 확정), START/SELECT(A/W), 해상도·창 모드 설정, 캡처 하니스 `stage/capture.rs`. Workflow `wf_e03c0a9e-2fd`(8 에이전트), 리뷰 major 2·minor 5 처리. 게이트: fmt·clippy·`cargo test --workspace` 3,262 통과·0 실패·ignored 3·`git diff --check`, 골든 불변, GPU 대 CPU 픽셀 비교 통과. 메인이 1920x1080 GPU 캡처를 직접 열어 확인. 실창 확인(내장 화면 밝기 변화 포함)은 사용자 절차로 남음. 원래 항목: 구 번들·혼합 합성 삭제, Lua 5.2 전환, 타이머 µs·stretch·오프셋 정합, 장면 시계, GPU 논리 크기 런타임화, 해상도 설정, 캡처 하니스, START/SELECT.
  - [x] 웨이브 2A — Lua 런타임과 로더 본체(W2-0~W2-5): Lua 5.2 런타임(`crates/rbms-skin/src/lua/`: 환경, `require`/`dofile` 가두기, io 쓰기 오버레이, os, luajava facade, `main_state`, `timer_util`), `SkinHost`·`MapHost`, 값 참조 타입 확장, Lua 값 → `SkinDef` 변환, 2패스 로더, 구 샌드박스 삭제. Workflow `wf_402f94ef-6d0`(10 에이전트), 리뷰 critical 3·major 3·minor 3 처리. 게이트: 테스트 3,475 통과·0 실패. **ModernChic `.luaskin` 9/10 로드**(keyconfig 는 원본 결함), pcall 실패 0, destination 수 play7 286·select 1914·result 195·decide 69, 스킨 폴더 무변경.
  - [x] 웨이브 2B — `rbms-cli skin-dump` 와 프레임 비용 실측(선곡 1,801호출 평균 0.28ms·최대 0.65ms), 앱의 스킨 팩 폴더 지정(`skin.pack`, `RBMS_SKIN_PACK`, SKIN 탭 PACK FOLDER), 오버레이 총 크기 상한, 외부 스킨 정지 프레임(`crates/rbms-render/tests/skin_external.rs`). Workflow `wf_c034401f-ce6`(6 에이전트), 리뷰 major 3·minor 8 처리. 게이트: 테스트 3,530 통과·0 실패. 메인이 ModernChic 결과·선곡 1920x1080 캡처를 직접 확인(결과는 거의 다 나옴, 선곡은 곡 바 목록과 텍스트 의미론이 빠짐 → `docs/reference/skin-compat/v1-first-render.md`). 원래 항목: 환경·io 오버레이·luajava, `SkinHost`·`main_state`, Lua 값 변환, 2패스 로드, 덤프 CLI, 픽스처, 스킨 팩 폴더 지정, 결정 화면 정지 프레임.
- [x] L4. 화면·UI 전환 — 사양 §6 의 웨이브 3~7.
  - [x] 웨이브 3A — 공통 그리기 의미론(W3-0~W3-3, W3-5, W3-6): prepare/draw 2단계, `SkinHost` 직접 그리기, 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·센티널, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 `skin_host/` 군집 A·I·M. Workflow `wf_02e2af85-d93`(10 에이전트), 리뷰 major 2·minor 5 처리. 게이트: 테스트 3,716 통과·0 실패, 골든 불변. **ModernChic 결정 화면이 조사 문서의 좌표·색·페이드와 일치**(리뷰어와 메인이 캡처를 직접 확인). 선곡·결과의 공통 객체도 좌표대로.
  - [x] 웨이브 3B — 텍스처 관리자(W3-4: 참조 source 만 디코드, 화면 이탈 해제, 화면당 1 GiB 예산)와 Decide Stage·장면 수명·로딩 분리(W3-7). Workflow `wf_28b3ee25-269`(4 에이전트), 리뷰 major 1·minor 4 처리. 게이트: 테스트 3,776 통과·0 실패, 골든 불변. 앱 경로에서 실제 차트로 찍은 ModernChic 결정 화면이 좌표·색·페이드와 일치(리뷰어 판정, 메인이 페이드 프레임 확인). 실창 확인은 사용자 절차.
  - [x] 웨이브 4 — 결과와 코스 결과(W4-1~W4-7): 스킨 클릭·드래그 디스패치와 이벤트 분배, gauge·결과 그래프, 엔진 기록 확장(전 게이지 종류 이력·판정별 early/late), Result·CourseResult Stage(타이머 150~152, 입력 잠금, FADEOUT), 호스트 군집 B·G·H·E 일부, 효과음 버스. Workflow `wf_985f3fc9-e64`(9 에이전트), 리뷰 critical 1·major 2·minor 8 전부 수정. 게이트: 테스트 4,065 통과·0 실패, 골든과 엔진 테스트 불변. 실제 플레이 결과로 찍은 ModernChic 결과·코스 결과 화면이 좌표·수치와 일치(리뷰어 대조, 메인이 클리어 결과 캡처 확인), Info 클릭·방향키 메뉴 전환 동작.
  - [x] 웨이브 5 — 선곡(W5-1~W5-7): songlist 재작성(원형 인덱스·패스별 그리기·스크롤 보간), 선곡 상태 모델, 원본 키 표와 패널 1~3·옵션 이벤트, 호스트 군집 F·H·E, 슬라이더 쓰기와 편집 텍스트(IME), 선택 BGM, 스킨 위 시스템 오버레이와 단축키 안내. Workflow `wf_2dfe3d4c-d13`(10 에이전트), 리뷰 major 6·minor 8 처리. 게이트: 테스트 4,309 통과·0 실패, 골든 불변. 실제 라이브러리로 찍은 ModernChic 선곡 화면과 옵션 패널을 메인이 직접 확인. 패널이 닫힌 상태의 F 키 문제는 사용자 결정 D8(즐겨찾기만)로 정리됐고 웨이브 8 에서 구현한다.
  - [x] 웨이브 6 — 플레이 SP·DP(W6-1~W6-8): note·judge·커버·bga·비주얼라이저 재작성, 플레이 상태기계(로딩 → READY → 플레이 → 완주 페이드/폐점), 타이머 드라이버와 오프셋 1~5, 호스트 군집 C·D, 내장 레이아웃 의존 제거와 구 어댑터·드라이버 삭제. Workflow `wf_1f1087ff-ad5`(10 에이전트), 리뷰 major 1·minor 14 처리. 게이트: 테스트 4,589 통과·0 실패, 골든·곡 시계·판정 경로 불변. 오토플레이로 찍은 ModernChic 7/5/10/14키 화면이 좌표·엔진 수치와 일치(리뷰어 대조, 메인이 7키 프레임 확인). 릴리스 프레임 비용 7키 약 0.06ms. 선곡의 종류별 노트 수는 사용자 결정 D9 로 별도 작업이 됐다(이번 작업에서는 빈칸).
  - [x] 웨이브 7A — 비트맵 폰트(W7-3, W7-4)와 디코더 조사(W7-1): `.fnt` 로더(libGDX 1.9.9 바이트코드 기준 조판), type 0 과 distance field 그리기(CPU·GPU 동일 계산). Workflow `wf_46905912-776`(5 에이전트), 리뷰 major 1·minor 7 처리. 게이트: 테스트 4,676 통과·0 실패, 골든 불변. 디코더는 OpenH264 소스 빌드 + `re_mp4` 로 선정(`docs/reference/skin-compat/v2-video-decoder.md`, 결정 문서 "동영상 디코더" 절).
  - [x] 웨이브 7B — 동영상 source(W7-2): `crates/rbms-video`(OpenH264 + `re_mp4`), 스킨 동영상 source 재생. Workflow `wf_1a4c0731-1e7`(3 에이전트), 리뷰 major 1·minor 5 수정. 게이트: 테스트 4,776 통과·0 실패(피처 끈 구성 포함). ModernChic mp4 8개 전부 디코드(프레임 수 일치), 시스템 디코더와 화소 대조 일치, 앱 플레이 화면의 범용 BGA 동영상을 메인이 직접 확인.
  - [x] CI 복구(2026-10-11) — 웨이브 1B~7B 동안 확인하지 않은 CI 실패(Ubuntu 1B~, Windows 2A~, macOS 7A~)를 고쳤다: 할당 상한(텍스트 블록·그래프·차트 길이), Windows 경로·줄바꿈·장면 시계, 느린 테스트(실행 파일 해시), 벽시계 의존 테스트. Workflow `wf_c1ed92d1-93e`·`wf_40d5eac7-aed`. run `38077937158`(`cf47348`)에서 세 OS 전부 통과, 작업당 4~8분. 이후 푸시마다 `gh run list` 로 확인한다(사양 §8, `docs/feedback/2026-10-11-ci-not-checked-after-push.md`).
- [x] L5. 기본 스킨 — 사양 §6 의 웨이브 8. 실행 분할: 8A(W8-0, W8-2a) → 8B(W8-2b 선곡, W8-2c 플레이, W8-2d 결과) → 8C(W8-1 죽은 설정 행·기기 로컬 경로·레터박스 기본값, W8-3 내장·설치·릴리스·제3자 고지, W8-4 폴백, W8-5 테스트 재편, W8-6 문서). 위임은 Workflow 전용, 모델은 Opus(high 이하)와 Sonnet(xhigh 이하)만 쓴다(2026-10-11 사용자 재확인: Fable 금지).
  - [x] 웨이브 8A — 선곡 키 표의 NEXT_REPLAY 제거(D8, 숫자 4 만 슬롯을 넘김)와 기본 스킨 설계서 `docs/plan/2026-10-11-default-skin-design.md`·골격(`assets/skins/rbms-default/`: 공통 Lua, 이미지 생성 스크립트, 결정 화면). Workflow `wf_38f63f0c-00f`(4 에이전트), 리뷰 major 4·minor 9 처리. 게이트: 테스트 4,812 통과·0 실패, 골든 불변. 메인이 결정 화면 캡처를 직접 확인. ModernChic Lua 136개와 대조해 복사 없음(D1).
  - [x] 웨이브 8B — 선곡, 플레이 SP(5/7/9)·DP(10/14), 결과·코스 결과 문서와 통합. Workflow `wf_57262b5f-803`(10 에이전트), 화면별 리뷰 major 7·minor 21 처리. 게이트: 테스트 4,815 통과·0 실패, 골든 불변. 문서 9개 덤프 `--strict` 통과, 캡처 80장(메인이 선곡·플레이 7/9/14키·결과 확인), 이미지 93 KiB. 엔진 수정 2건(5·10키 결과의 턴테이블 역할, 선곡 op 190~195). 9키 스킨 플레이가 처음 그려짐.
  - [x] 웨이브 8C — 기본 스킨 내장(`apps/rbms-player/build.rs`)·설치(`install-manifest.txt`)·기본 선택, 로드 실패 알림과 24키 폴백, 설정 행 표지·`skin` 그룹 기기 로컬화·레터박스 기본값, 제3자 고지와 릴리스 묶음, 기본 팩 게이트 테스트와 환경 변수 테스트 일반화, 문서(`docs/skin.md` 재작성, architecture·crates·development·README·HANDOFF, 수동 확인 목록). Workflow `wf_04db2a8b-10f`(7 에이전트), 리뷰 major 1·minor 12 중 10건 수정·3건 보류(사양 웨이브 8C 기록). 게이트: 테스트 4,878 통과·0 실패, 골든 불변. 깨끗한 HOME 의 첫 실행이 기본 스킨으로 그려짐을 메인이 캡처로 확인.
- [x] L6. 검증 — 최종(2026-10-11): 로컬 게이트 통과(테스트 4,878 통과·0 실패), CI run `38099911787`(`d0af473`)에서 세 OS 전부 통과, ModernChic 덤프 9/10 load ok·pcall 0(keyconfig 는 원본 결함), 웨이브 8 뒤 다시 찍은 ModernChic 선곡·플레이·결과 캡처가 웨이브 7B 최종본과 픽셀 동일, 기본 스킨 문서 9개 `--strict` 통과와 화면 캡처 확인. 실제 창 확인은 사용자 절차(`docs/quality-assurance/2026-10-11-default-skin-manual-check.md`).
- [x] L7. 문서·이력 — `docs/skin.md` 재작성, architecture·crates·development·README·HANDOFF 갱신, 수동 확인 목록, 이력 `docs/history/2026-10-11-lua-skin-compat.md`. 조사 문서는 웨이브마다 상단 반영 사항으로 갱신했다.
- 사용자 결정 대기(이력 문서의 표): 기존 설치의 레터박스 마이그레이션, `Cargo.lock` 추적과 `--locked`, 고지 파일의 Apache-2.0 선택 표기, 내장 화면 테마 색 재조정. 웨이브 9 는 별도 지시가 있을 때 시작한다.

---

## 완료 — 실행 스크립트 `start.sh` 재작성 (2026-10-09)

- [x] 루트 `start.sh`(gitignore 대상, 로컬 전용) 작성 — 인자 없이 또는 곡 폴더·차트·옵션을 넘기면 `cargo build --release -p rbms-player` 후 바이너리를 호출 위치 기준으로 실행하고, `./start.sh web` 은 `web/.env.local` 존재를 확인한 뒤 `bun install` → `bun run dev` 를 실행한다. 검증: `bash -n`·`shellcheck` 통과, 릴리스 빌드 1분 성공(`target/release/rbms-player`). GUI 창 기동과 웹 서버 기동은 실행하지 않았다.

---

## 현재 작업 — 스킨 시스템 완성과 기본 스킨 전면 제작 (2026-09-17)

> 사용자 지시: 제공된 외부 스킨 묶음(구조 분석 입력, 저장소 복사 금지)과 화면 12장을 참고해 기본 스킨·스킨 구조·커스터마이징을 완성하고 필요한 기능은 `docs/`에 사양을 두고 구현한다. 아래 V7-C·V8은 이 작업에 흡수한다. 운용: 조사·구현·적대 리뷰 = Workflow(Opus max, 주제당 질문 3개·10분 상한), 사양·판정·Git = Fable 직접.

- [x] K1. 조사 — R1 엔진 격차(그리는 객체 8종/미지원 15종, 중첩 destination·행/시계열 상태·타이머 3대 차단), R2 앱 배선(대체는 결과 전용, 커스터마이즈 문서경로 키·세대 이동 유실, 사운드 폴더 단일), R3 외부 묶음(좌표 17표·옵션 55/파일 65/오프셋 28·결함 13건) 보고서 확보(Workflow `wf_32fd88cf-707`).
- [x] K2. 사양 확정 — `docs/plan/2026-09-17-skin-system-completion.md`(§1~§12)와 결정 `docs/acknowledge/2026-09-17-skin-system-decisions.md`(D1~D12) 기록.
- [x] K3. 엔진 구현 — Workflow `wf_b03ec12f-3f9`(14 에이전트): 로더 중첩 트랙·`replace`·`hotspot`·`scope`, 렌더 객체 13종(note/gauge/judge/songlist/covers/graphs 7종/density), `FrameExtra` 상태, 설정 공유 스코프·세대 목록 설치·custom/shared 이동·번들 사운드 폴더, v3 자산(이미지 21·사운드 22·생성 스크립트). 적대 리뷰 31건 중 critical/major 19건 반영. 게이트: fmt·clippy·`cargo test --workspace` 3,118 통과·금지 명칭 0건. 앱 배선(대체 게이트·상태 조립·타이머·SKIN 탭)은 K4 로.
- [x] K4. 앱 배선 + 기본 스킨 제작 — Workflow `wf_1a582c5d-df7`(14 에이전트, 사용량 제한으로 1회 재개): W2-0 게이트 일반화·핫스팟·공유 오프셋·옵션 행 → W2-a/b/c/d(play·select·result·SKIN 탭) → W3-1~4 문서(선택·결정/결과·단일 5/7/9·이중 10/14) → 리뷰 23건(major 9) 수정 → 게이트 통과(fmt·clippy·`cargo test --workspace` 0 실패·캡처 20장·금지 명칭 0). 메인 직접 보강: 플레이 문서 오프셋 결함, 결과 `hint`/`ir` 대체 단위, 상태 id 확장(플레이·선택·결정), 네이티브 BGA 사각형 게이트, 이중 필드 RON 폴백, 캡처 프로브.
- [x] K4-b. 후속 Workflow `wf_136680dc-fb5` — 단일 플레이 `bga` 객체·PLAY SIDE/BGA SIZE/GRAPH POSITION·2P/NEAR 프레임, 문서 minor 일괄, 게이트(3,181 통과·0 실패).
- [x] K5. 검증 — 헤드리스 캡처 12장(`docs/quality-assurance/2026-09-17-skin-system/captures/`) 메인 육안 판정, 전체 테스트·fmt·clippy 통과. **실제 GPU 창 캡처는 터미널 화면 녹화 권한 부재로 미완(체크리스트 §3 사용자 절차).**
- [x] K6. 문서·이력·커밋 — `docs/skin.md`·history·QA·README 갱신(2026-09-17), 2026-09-18 핸드오프 준비로 `docs/HANDOFF.md` 재작성·결정 D13~D23·정합성 정정. 사용자 지시(2026-09-18 "커밋 푸시")로 5단위 커밋: `1535b0a` feat(skin) · `4f1a338` feat(config) · `3704b8c` feat(render) · `3cd72f4` feat(player)+v3 자산 · docs(skin)(이 문서 포함, 다음 커밋). author 단독·트레일러 0건 확인.

---

## 현재 작업 — 기본 스킨 실화면 불일치 수정 (2026-09-17)

> 사용자 제공 실행 화면 두 장에서 1차 구현의 배치·가림 결함이 확인됐다. 이전 완료 표시는 1차 파일 설치와 자동 검사에만 해당하며 실화면 합격을 의미하지 않는다. 이번 작업은 `docs/plan/2026-09-17-skin-visual-correction.md`에 사양을 먼저 고정한 후 기능을 고치고 기본 파일을 수정한다.

- [x] V1. 실화면 결함 재현·원인 확인 — 선택 화면의 좌우 배치/옵션 패널, 재생 화면의 필드/BGA/그래프 겹침을 실제 코드·좌표와 대조했다.
- [x] V2. 상세 사양 선기록 — `docs/plan/2026-09-17-skin-visual-correction.md`에 화면별 영역·상태·레이어·편집 파일·기존 설정 전환·검증 장면과 실패 기준을 구현 전에 기록했다.
- [x] V3. 엔진 기능 수정 — 번들 버전·설정 전환·문서 게이트와 배경→내장 UI→전경 합성, 파일 기반 옵션/패널 투명도, 플레이 배치 연동 테두리를 구현하고 관련 검사로 확인했다.
- [x] V4. 기본 파일 재제작 — `assets/skins/steel-neon-v2/`의 선택·결정·단일/이중 재생·결과 문서, RON, 팔레트, 생성 스크립트, PNG를 만들고 배포 목록에 연결했다.
- [x] V5. 실화면 검증·결과 기록 — 선택/옵션/7키/14키/결과 픽셀 캡처와 차트 영상 보존을 확인하고 `docs/history/2026-09-17-skin-visual-correction.md`에 결과와 남은 범위를 기록했다.
- [x] V6. 개별 객체 사양 확정 — 상태 원천, 파일 스키마, 네이티브 요소의 대체 조건, 입력 좌표와 화면별 완료 기준을 `docs/plan/2026-09-17-skin-object-spec.md`에 기능 구현 전에 기록했다.
- [ ] V7. 개별 객체 기능 구현 — 곡 목록/상세, 플레이 노트·판정·게이지·HUD, 결과 패널/그래프, 옵션 행의 문서 구동과 상태·입력 연결을 검증한다.
  - [x] V7-A. 결과 점수·클리어·판정·목표 텍스트는 개별 JSON5 객체로 이동하고 필요한 객체가 없을 때 네이티브 출력으로 복귀한다.
  - [x] V7-B. 선택 옵션 11행의 글자/값 위치·색을 테마 RON에서 행별로 수정할 수 있고 설정 descriptor와 연결된 상태를 유지한다.
  - [x] V7-C. 결과 등급·그래프·하단 힌트, 곡 목록/상세, 플레이 HUD·노트·판정·게이지, 옵션 패널의 나머지 요소를 개별 객체로 전환한다. — 위 K3·K4 에 흡수해 완료(대체 단위 표 `docs/skin.md` §2.2).
- [x] V8. 기본 스킨 객체화 — 엔진 V7 완료 후 기본 JSON5를 요소별 파일 값으로 구성하고 5/7/9/10/14키와 선택/옵션/결과 실화면을 확인한다. — 위 K4·K5 에 흡수(`steel-neon-v3`, 헤드리스 캡처 12장). 실제 GPU 창 확인은 K5 와 같이 미완.

---

## 현재 작업 — 기본 스킨 범위 재점검 (2026-09-17)

> 사용자 질문: 1차 적용이 빠르게 끝난 이유와 필요한 기능의 완결 여부를 실제 코드에 비춰 분명히 한다. 세부 근거와 후속 순서는 `docs/plan/2026-09-17-default-skin.md`에 기록한다.

- [x] G1. 지원 범위 재감사 — 문서 렌더러·앱 입력·설정 화면에서 파일로 바꿀 수 있는 요소와 아직 코드에 고정된 요소를 확인했다.
- [x] G2. 범위·후속 사양 정정 — 1차 완료와 전체 고도화 미완료를 문서에 분리해 적고 후속 기능의 구현 순서를 명시했다.
- [x] G3. 문서 검증·상태 보고 — 금지 명칭 검색 0건과 `git diff --check` 통과를 확인하고 현재 상태를 답한다.

---

## 현재 작업 — 기본 스킨 고도화 (2026-09-17)

> 기준: 이 문서, 프로젝트 `AGENTS.md`, `/Users/gkn/.codex/llm-rules/ai-process.md` 및 실제 스킨 로더·렌더러 코드. 제공된 화면 자료와 외부 원본 파일은 구조를 분석하는 입력으로만 사용하며, 저장소 산출물의 명칭과 그래픽은 독립적으로 구성한다.

- [x] S1. 제공 자료와 현 기본 스킨 경로 분석 — 화면별 구조·옵션·자산·기능 차이와 배포 제약을 실제 파일 근거로 확인했다.
- [x] S2. 구현 사양 선기록 — `docs/plan/2026-09-17-default-skin.md`에 화면·편집·합성 기능·기본 적용·기존 설정 보존·검증 기준을 기록했다.
- [x] S3. 필요한 스킨 기능 추가 — 화면 합성·곡 선택 열 배치·설정 기능을 구현하고 관련 최소 테스트로 확인했다.
- [x] S4. 파일 기반 기본 스킨 제작·적용 — 곡선택·결정·단일/이중 플레이·결과 파일을 제공하고 새 설치의 기본값에 연결했다.
- [x] S5. 검증·결과 기록 — 문서 로드·컴파일 경고, 화면 합성, 사용자 편집 보존과 기존 설정 호환을 확인하고 `docs/history/2026-09-17-default-skin.md`에 결과를 기록했다.

---

## 현재 작업 — 문서 정합성·고도화 마무리 (2026-09-16)

> 사용자 지시: `docs/` 전체를 실제 코드·Git·검증 결과와 대조해 정합성을 완성하고, 이번 라운드의 문서 이력과 지도까지 마무리한다. Phase R은 실제 키가 필요한 사용자 동반 배포 작업이므로 미착수 상태를 보존한다.

- [x] D1 문서 인벤토리와 정본·보존 이력 구분 — `README.md`, `docs/roadmap.md`, `docs/PROCESS.md`, `docs/history/`, `docs/plan/`, `docs/reference/`의 역할·중복·현재성 표기를 실제 파일 기준으로 판정했다.
- [x] D2 활성 문서의 코드·Git·실행·Phase 상태 정합성 수정 — 루트 안내와 `docs/{PROCESS,architecture,crates,ci-release}.md`의 명령·경로·브랜치·테스트 수·완료 상태를 현재 `dev`와 구현에 맞췄다.
- [x] D3 계획·참조·QA·링크/경로 교차검증과 오래된 지시 정리 — 활성 문서의 깨진 Markdown 링크, 폐기된 실행 지시, 완료 기능의 미구현 표기를 보존 이력과 구분해 정정했다.
- [x] D4 최종 작업 이력·문서 지도·PROCESS 완료 상태 정리 — 감사 근거와 Phase R 경계를 `docs/history/2026-09-16-documentation-finalization.md` 및 문서 지도에 기록했다.
- [x] D5 최종 검증·적대 리뷰·커밋/푸시 — Markdown 링크·오래된 패턴·`git diff --check`와 독립 문서 검토를 통과했다. 이 문서 변경을 docs 전용 커밋으로 `dev`에 푸시하고 원격 CI를 확인한다.

---

## 현재 작업 — 전면 고도화 감사·계획 수립 (2026-09-09~)

> 사용자 지시: 프로젝트를 정확히 읽고 고도화 계획 수립. 관점 = (1) 스킨 완전 커스터마이징 (2) IIDX(~34) 기능 전수 확인 (3) 레퍼런스 구현 판정 동일성 + 설정 노출 (4) crate 분리·역할 (5) 파일 길이·Rust 관용성 (6) 메모리 누수 (7) UI/UX(프리징·미리듣기·디버그·시작 전 옵션·target 그래프) (8) 성능 (9) 시작 전 설정 vs 인게임 옵션.
> 운용: 위임은 **Workflow만**(Agent 단독 호출 금지, 사용자 지시 2026-09-09). 리서치·구현·적대 검증 = Opus, 기계적 수정(clippy/문서 경로/테스트 분리) = Sonnet, 검토·종합·핵심 판단 = Fable 직접. 병렬 가능한 것은 항상 병렬. 산출 = 계획 문서 `docs/plan/2026-09-09-enhancement-plan.md`.

- [x] a. 인라인 정찰 — PROCESS.md·크레이트 맵·LOC 핫스팟·레퍼런스 구현 소스 배치(play/skin/json·lr2·lua 로더)·현 SkinConfig 스키마 확인
- [x] b. 리서치 워크플로 실행(`wf_1911d201-312`) — 9관점 보고서 + 관점별 검증 9편 완료(반박 2건: 디버그 오버레이 부재·LN unwrap 패닉), IIDX 30~34 웹 조사 완료. 완전성 비평 대기
- [x] c. Fable 검토 — 전 보고서·검증 통독, 직접 코드 확인 6건(見逃し POOR 슬롯·윈도우 표·오디오 클럭·FLOATING·그린넘버 LIFT·assist 플래그). 검토 노트 `scratchpad/review-notes.md`
- [x] d. 계획 문서 확정 — `docs/plan/2026-09-09-enhancement-plan.md`(관점별 갭 표·Phase A~G·결정 12건·비평 반영·문서 정정 목록·한계). 근거 보고서 24편 `docs/plan/research-2026-09-09/`. 후속 조사 4건(IR·커스텀 판정 정책·Renderer diff·cpal 실측) 병합, `cargo test` baseline 889/0
- [x] e. 사용자 결정 12건 전부 추천안 채택 → `docs/acknowledge/2026-09-09-enhancement-decisions.md`

### Phase A — 정확성 핫픽스 (2026-09-09 착수, Workflow `rbms-phase-a`)

> 파일 소유권 분리로 병렬: 1차 core(model/parser/chart/judge/play)·audio·ir·render 4갈래 동시 → 2차 app 통합 → 갈래별 적대 리뷰 5개 병렬 → 수정 → 문서 정정(Sonnet) → Fable 최종 검증(`cargo test --workspace` + clippy). 항목 정의는 계획 §2 Phase A.

- [x] A-core: J1+J2+J3, J4·J5·J7·J8, J10·J11, J12-1, J13·J15·J16·J18·J19, J14 `#DEFEXRANK`, C7 NaN 소절 거부, `#SWITCH/#CASE/#SKIP/#DEF`(결정 10) — 리뷰 반영까지 전부 완료. 보류 2건: J17/J20/J21/J23 상세·J24(계획대로 Phase D 범위), 지뢰 damage 스케일(1차 출처 미확인, 현 값 유지)
- [x] A-audio: A5 기본 gain 하향(0.5, 레퍼런스 구현 정확 일치), A7 스트림 사망 감지, P7 카운터·scratch 선할당, A13 채널 키 피치, 클럭 페어 seqlock화 — 전부 완료. 보류 1건: A6 start/stop 램프는 계획대로 Phase B 범위
- [x] A-ir: `http.rs` 타임아웃 강등 제거, HTTP mock 테스트, degraded 경로·8엔드포인트 URL 검증 — 완료. 보류 1건: URL percent-encoding은 `url` 크레이트 신규 의존 필요해 문서화만(코드 미변경)
- [x] A-render: P1 폰트 캐시 상한, K9 결과 팔레트 스킨화(후방호환 API), 골든 하네스 격자·감도 강화 — 완료. 보류 1건: RSS 상한 계측 하네스는 계획 문서(소유 밖) 반영만 남음
- [x] A-app: A3 키음/판정 시각 분리, IR 제출 게이트(autoplay/replay/judge rate>100), assist 플래그, U3 Root Esc, U4 조작 저장, P6 cancel, 원자적 저장 + `rule_version`, U6 URL 중복, P3 디바운스, 스트림 사망 폴백, 그린넘버 매직상수 통합 — 완료. 보류 1건: 어시스트 램프 강등(`LightAssistEasy`)은 `rbms_judge::ClearType` 변형 신설이 필요해 Phase D 이월(대신 `best_clear_for_md5`가 어시스트 기록을 보수적으로 제외)
- [x] 적대 리뷰 5갈래 → 수정 (core/audio/ir/render/app 전 갈래 반영 완료, `scratchpad/phase-a/review-*.md`·`fix-*.md`)
- [x] 문서 정정(계획 §4) + divergences.md J1~J26 섹션
- [x] 커밋(2026-09-09, 사용자 지시 "갈래별로 co-author 없이"): `b9e25f6` parser · `c8f6b1b` judge · `cfc41ee` audio · `18eefc3` ir · `3179b2d` render · `e730209` player · `2c7adef` docs (author 단독, 트레일러 0, Conventional Commits — 커밋 훅이 요구)
- [x] Fable 최종 검증(2026-09-09 실측): `cargo test --workspace` **1,060 통과 · 0 실패 · 2 ignored**(착수 전 889), `cargo clippy --workspace` 경고 51(기준 동일), `--all-targets` 94(기준 101). 에이전트가 추가한 `//` 주석 약 210줄은 Sonnet 워크플로로 제거·history 문서로 이동. 실기 가청 1회(사용자)는 미실시 — history §5 절차 참조

### Phase W — web/ Next.js 단독 서버 + 웹 FE (2026-09-09 착수, Phase A 와 병렬)

> 사용자 지시: 백엔드를 Next.js 단독 서버로 이 레포 `web/` 에, Vercel `bms.hyuns.uk`. better-auth + MySQL(.env 추후) + Drizzle, 전부 최신, TanStack prefetch + Next 캐시/revalidate, design.md 토큰 + shadcn 목록화. 결정 → `docs/acknowledge/2026-09-09-enhancement-decisions.md` §웹.
> 워크플로: W0 설계(`docs/web/architecture.md`·`tasks.md`·`components.md`) ∥ 스캐폴드(web/ 생성·토큰·providers·drizzle·better-auth·health) → Fable 검토 → W1 api-core ∥ web-ui(mock) → W2 api-ext ∥ ui 통합 → 리뷰·수정 → 검증(typecheck·lint·test·build) → 문서.

- [x] W0 설계 문서(`docs/web/{architecture,components,tasks}.md`) + 스캐폴드(`web/`, Next 16.3.4·React 19.2.8·Tailwind 4.3.3·shadcn 4.21·Drizzle 0.45·better-auth 1.7.3·TanStack 5.102, typecheck/lint/test/build 통과). 미결 6건 결정 → decisions 문서
- [x] W1 API 코어 + 웹 UI(MSW) — 리뷰·수정 완료, typecheck/lint 0·테스트 155 통과·빌드 19 라우트. `drizzle-kit migrate` 로 사용자 DB에 21테이블 적용(2026-09-09). **실 DB 스모크(Fable)**: `/api/health`·`/api/version` 200, register 201+토큰, guest 제출 201(unranked: guest), 토큰 제출 rank 1, 더 나쁜 재제출 시 베스트 유지, autoplay 제출 unranked, ranking raw 배열·best raw 객체·chart 메타 정상, `/api/auth/me` Bearer 정상. 계약 보정 4건 도출 → W2a: 미등록 차트 ranking `[]`/best `null`(404 대신), played_at ±7일 → 미래 5분만 거부, `.env.example` 키 보강; 후속: 빈 sha256 허용(md5 단독 클라 지원)
- [x] W2 API 확장 ∥ UI part1 → UI part2 → 리뷰 3·수정 완료. Fable 최종 게이트: typecheck 0·lint 0·테스트 196 통과(2 skip)·빌드 35 페이지. 실 DB 스모크 전 항목 정상(→ `docs/history/2026-09-09-web-nextjs-ir-server.md`)
- [x] 커밋(web 갈래별) + push origin dev (2026-09-09)
- [x] Vercel 배포(2026-09-09): 프로젝트 `rbms-web`(b-hs) 링크, Production env 6종(DATABASE_URL·BETTER_AUTH_SECRET·BETTER_AUTH_URL·NEXT_PUBLIC_APP_URL·ALLOW_GUEST·REPLAY_MAX_BYTES) 등록, `vercel --prod` 성공. 프로덕션 별칭 `https://rbms-web.vercel.app` 에서 health/version/랭킹/FE 통계/페이지 7종 200 확인(실 DB). 배포 해시 URL은 배포 보호(SSO)로 302 — 정상
- [x] 도메인 `bms.hyuns.uk`: 사용자가 Cloudflare 레코드를 Vercel CNAME(`*.vercel-dns-016.com`, DNS only)으로 교체(2026-09-09). `https://bms.hyuns.uk/api/health` 200 확인(Vercel IP 216.150.1.65 경유)
- [x] CI(dev) 골든 실패 수정 완료: 원인은 곡선택 폴더 행 마커 U+25B8 가 임베드 Inter 에 없어 OS 시스템 폰트로 폴백된 것. 골든 테스트를 임베드 폰트 전용 결정적 렌더(`TextEngine::embedded_only`, `use_embedded_fonts_only`)로 수정 — 커밋 `61b9a97`, CI run 34307010505 에서 ubuntu/macos/windows + fmt/clippy 전부 통과
- 후속: md5 단독 제출(LR2IR 어댑터), Vercel Blob, 라이트/다크 스크린샷 확인, 슈퍼셋 서버의 누락 필드 `.default()`

### 사용자 지시 (2026-09-09): "멈추라 할 때까지 멈추지 말고 계획대로 전부 구현" — 클라 GUI 로그인 등 IR 연동은 CLI 가 아니라 GUI 에서 전부 가능해야 함

> 실행 순서(파일 충돌 회피): Phase I(클라 GUI 연동) → B(오디오) → C(구조) → D ∥ E1~E2 ∥ F → E3~E6 ∥ G → R(릴리스). 각 Phase = Workflow 1개(구현 Opus max, 파일 소유권 분할 → 적대 리뷰 Opus → 수정 → Fable 게이트 `cargo fmt/test/clippy` + web `bun run verify`) → 갈래별 Conventional Commit → push → CI 확인 → PROCESS/history 갱신.

### Phase I — 클라 ↔ IR 서버 GUI 연동 (Workflow `rbms-phase-i`)
- [x] I-ir(커밋 `a1987ba`): `rbms-ir` DTO 확장(`SubmitResponse` ranked/flags/is_new_best/score_id) + `HttpScoreServer` register/login/me/replay download/settings get·put(낙관적 잠금)/rivals put/course ranking + 에러 변형(401/409/413/429) + mock HTTP 테스트 + `docs/reference/ir-api.md`
- [x] I-app(커밋 `6226f0c`): NETWORK 탭 GUI — 계정 상태·EMAIL·PASSWORD(마스킹, 비영속)·LOGIN/REGISTER/LOGOUT(백그라운드, 토큰 영속) · `build_server` 토큰 전달 · 결과 화면 IR 결과(rank/new best/unranked 사유) · 곡선택 IR 랭킹 패널(비동기 캐시·라이벌 행) · 리플레이 자동 업로드 + 랭킹 행에서 다운로드 재생 · 설정 동기화(업로드/다운로드/충돌) · 라이벌 관리
- [x] I-web(커밋 `a91f73f`): `/guide` 를 GUI 절차 기준으로 재작성(CLI 제거) + README Web 절 정정. 워크스페이스 `rustfmt.toml`(max_width 160) 추가 + 포맷 커밋 `0f12ae8`, CI 3-OS 통과
- [x] 적대 리뷰 28건(critical 2: 비밀번호가 SERVER URL 로 유출·랭킹 패널 LOADING 고착) → Rust 측 전건 수정 → 검증(테스트 1,277 통과). **GUI 실기 검증(2026-09-09, TAIDE 접근성 권한 후 키 자동화)**: 곡선택 → SETTINGS/NETWORK 13행 렌더 → SERVER URL 입력 → REGISTER 로 일회용 계정 `gui-e2e-871795` 생성(ACCOUNT "logged in as", PASSWORD 소거, rivals 갱신) → 서버 `/api/players/{id}` 조회 확인 → IR RANKING 패널 상태 표시 확인. 사용자 settings.ron 은 백업 후 복원
- [x] 서버 측 계약 결함 수정(커밋 `7fa8106` web · `beb2210` ir · `a1edc71` player · `f98899b` docs): submit 응답에 ranked/flags/is_new_best/score_id, 리플레이 다운로드에 gauge·chart.md5, settings PUT 200 `{updated_at}`(204 하위호환), guest 상수 확인. 로컬 dev 서버 e2e(`crates/rbms-ir/tests/e2e_local.rs`, 계정 `e2e-phase-i-18d391944f88`) 통과. `vercel --prod` 재배포 후 `https://bms.hyuns.uk` health/version/guide/홈 200, Chrome 실렌더 `/guide` 라이트·다크 양쪽 확인(2026-09-09). history: `docs/history/2026-09-09-phase-i-client-ir-integration.md`

### Phase B~G + 릴리스 (계획 `docs/plan/2026-09-09-enhancement-plan.md` §2)
- [x] B 오디오 클럭 재설계(2026-09-09, 가청 확인만 사용자 몫) — 보간 클럭·룩어헤드 스케줄·단일 AudioEngine·볼륨 3분리/#VOLWAV/보이스 스틸/램프·오디오 설정 노출·판정 오차 하네스/언더런 카운터/소크
  - [x] 구현 (명세 `docs/plan/2026-09-09-phase-b-spec.md`)
  - [x] 적대 리뷰 23건 반영 — 보간 클럭 계단형 붕괴·룩어헤드 부족·콜백 내 PCM free·xrun 오탐 사망·스틸 클릭·검증 지표 상수화 등. → `docs/history/2026-09-09-phase-b-audio-review-fixes.md`, 명세에 항목별 정정 블록
  - [x] §4.3 소크 13분(6세그먼트, HOME 격리, 수정 반영 후): underruns/drops/steals/hard_steals/late/ts_fallbacks 0, interp_sd 최대 9.17µs(계단형이면 3,079µs), RSS 기울기 −1.99MB/분, 로그 panic/error 0. 실기 보간 잔차 sd 3,073µs → 12.5µs
  - [x] 게이트(Fable 실측): fmt 통과, `cargo test --workspace` 1,475 통과·0 실패·3 ignored(착수 전 1,284), clippy 신규 경고 0. 커밋 `ec4d9d2` audio · `b53f99b` play · `b70fc56` parser/#VOLWAV · `b683e88` player
  - [ ] 실기 가청 확인 1회(키음 즉시성·BGM 온셋·클릭 없음·프리뷰 전환) — 사용자 몫
- [x] C 구조 개편(2026-09-09, 명세 `docs/plan/2026-09-09-phase-c-spec.md`, 커밋 `3813075`..`a243ce5`; 게이트 실측 fmt 통과·clippy `-D warnings` 0·`cargo test --workspace` 1,656 통과·0 실패·3 ignored(착수 전 1,474)·리뷰 30건 반영·에이전트 `//` 주석 693줄 제거·history `docs/history/2026-09-09-phase-c-structure.md`) — Stage enum·PlaySession→rbms-play·rbms-config·설정 descriptor 테이블·판정/게이지 데이터화·rbms-store/rbms-library 이관·CI lint 게이트/thiserror/forbid(unsafe)
  - [x] Wave 0 (P1~P6 병렬): judge 데이터화(`data/judge.ron` 6모드·`data/gauge.ron` BEAT_7K + 패리티 가드) + `JudgeAlgorithm` 4종 + `clear_type_id` 수신 · `rbms-table` `match_levels`/`TableError` · render `SkinError`·전역 인자 주입 · parser/chart/model clippy 0 · 루트 `[workspace.lints]`/`[workspace.dependencies]` · rbms-play clippy 0 + `Player::judge()` 접근자
  - [x] Wave 1 (S1→S4 직렬): `rbms-store`(scores/replay/write_atomic, md5 인덱스로 O(N) 조회) · `rbms-library`(취소 가능 스캔·`Library`) · `[lib]` 타깃 + `run() -> ExitCode` + `rbms-cli scan`/`scores` · `ir_map`→`rbms-ir` feature `mapping` · `rbms-config`(단일 `Config`+`schema_version`+마이그레이션, 픽스처 7종) · `App{shared,stage}` 분해 + `StageHandler`/`Transition`(`self.stage = Stage::` **0곳**) · `PlaySession`+`SoundSink` · 설정 descriptor 테이블(정수 인덱스 상수·`SETTING_TABS` 삭제)
  - [x] Wave 2 H0: 앱 잔여 clippy 0, `keyconfig.rs` 인라인 테스트 → `keyconfig_tests.rs` 분리
  - [x] Wave 2 H1 게이트 플립(실측 2026-09-09): 전 14크레이트 `[lints] workspace = true`, 전 크레이트 루트 `#![forbid(unsafe_code)]`(플레이어 포함 — `bytemuck` derive 와 충돌 없음이 실측으로 확인돼 §11 미확인 1 해소), CI `continue-on-error` 제거 + `cargo fmt --all --check` 게이트 + `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `rust-toolchain.toml` `channel = "1.95.0"` 고정(CI 도 동일 버전 설치). 검증: clippy **0 경고**, fmt 통과, `cargo test --workspace` **1,634 통과 · 0 실패 · 3 ignored**(착수 전 1,475)
  - [x] 문서(§9 step 14): `docs/crates.md`(신규 3크레이트 + judge `data`/`algorithm` + `match_levels` + `PlaySession` + 플레이어 절 재작성), `docs/architecture.md`(크레이트 맵·Stage/PlaySession·데이터화·lint 게이트), `docs/acknowledge/reference-divergences.md` §Phase C(C-D1 후보선택 기본값 `Duration`·C-D2 trait→enum·C-D3 게이지 6종)
  - **예외 1건**: `crates/rbms-audio/tests/rt_safety.rs` 는 계수 global allocator 로 콜백 무할당을 증명하는 테스트라 `unsafe` 가 필수 → 그 테스트 크레이트 루트만 `#![allow(unsafe_code)]`. 나머지 전 타깃은 forbid/deny 유지
  - [x] 적대 리뷰 25건 반영(2026-09-09) — major 9 · minor 16. 결함 수정: RESULT 화면에서 오디오 재오픈 가드가 풀려 키음 뱅크가 버려지던 것(`stage_owns_chart_audio` 신설) · 상위 스키마 동기화 blob 을 v1 로 강등 파싱해 계정 설정을 파괴하던 것 · 마이그레이션이 구 `settings.ron` 을 백업 없이 덮어써 롤백 불가였던 것(`settings.ron.v{from}.bak`) · 오디오 장치가 없을 때 press 가 리플레이에 안 남아 press 없는 release 만 담기던 것 · IR 제출의 `judge_algorithm` "Combo" 하드코딩(실제 정책 `Duration` 과 모순) · 기동 실패 `unwrap`(`Gpu::new -> Result` + `ExitCode::FAILURE`). 배선 누락: `rbms-cli config` 신설 · `judge.ron`/`gauge.ron` 런타임 소비 전환(내장 const 는 폴백 겸 패리티 기준) · `AppShared.songs` → `rbms_library::Library`(md5 인덱스) · NETWORK 탭 정수 행 인덱스 13개 + `NETWORK_SETTING_ROWS` 삭제(전부 `SettingId` 라우팅). 위생: `ConfigError::Read` 실사용 · `AudioOptions` `PartialEq` 에서 `reopen_pending` 제외 · 도달 불가 `AUDIO_PENDING_STATUS` 제거 · `Player::judge` 비공개 · `rbms-ir` 글롭 re-export → 명시 목록 + 워크스페이스 dep 통합 · `render_playfield` shim/`#[allow(too_many_arguments)]` 제거 · `tablesrc` `Result<_, String>` → 타입 에러 + `rbms-table` shim 2개 삭제 · `rbms-store` 의 UI 문자열 → 앱 `format.rs`(`is_stale_rule_version` 만 노출), `ScoreBook.records` 비공개화 · 매직넘버 2건 상수화 · 신규 파일 `//` 주석 제거 · `stage/select.rs` → `select/{mod,preview,scene}.rs`, `lib.rs` → `assets.rs` 분리
  - [x] 리뷰 후 게이트(Fable 실측): fmt 통과, `cargo test --workspace` **1,656 통과 · 0 실패 · 3 ignored**(리뷰 전 1,634), `cargo clippy --workspace --all-targets --all-features -- -D warnings` 0 경고, `cargo run -p rbms-cli -- config <v0 settings.ron>` 스모크 통과
  - [x] 스펙·문서 정정: 스펙 §4(`rbms-config → rbms-store`)·§4.1(백업/`ConfigError::Read`/blob 상한)·§4.7·§6.1(소비 전환), `docs/acknowledge/reference-divergences.md` §Phase C 스펙 대비 의도적 이탈 C-S1~C-S4(목록 상태 `AppShared` 유지 · `dyn StageHandler` 단일 match · `rbms-config → rbms-store` · `App`/`AppShared` 크레이트 루트 유지)
  - 미실시: 실기 GUI 확인(H1 은 게이트·문서 변경이라 렌더 경로 무변경)
- [x] D 판정 패리티 완성 + JUDGE 탭 노출(2026-09-10, 명세 `docs/plan/2026-09-09-phase-d-spec.md`; D0~D5 순차 구현 + 적대 리뷰 23건 반영, history `docs/history/2026-09-09-phase-d-judge-parity.md`, 발산 `docs/acknowledge/reference-divergences.md` §Phase D) — J17 알고리즘 4종(기본값 `Combo` 전환)·J20 9게이지+J26 GAS·J21~J23(5세트×9원소 게이지+PMS fixjudge)·J9/A10 정역 2키(BSS/MSS 앱 배선)·J24 CN/HCN 2단계 폴드·J25 LN MODE(모델+리플레이+스코어 키 전부 동일 축)·J6 24K Mode 신설·어시스트 램프 강등(`rule_version` 게이팅). 게이트(Fable 실측): fmt 통과, `cargo test --workspace` **1876 통과 · 0 실패 · 3 ignored**(D 착수 전 1,656), `cargo clippy --workspace --all-targets -- -D warnings` 0 경고, 금지어 grep 0, 신규 `//` 주석 0. HEAD `0c20fa9` 위 워킹트리 변경(미커밋)
- [x] E1~E6 스킨 완전 커스터마이징(2026-09-09~10, 명세 `docs/plan/2026-09-09-phase-e-spec.md`; 웨이브 0 스캐폴드 → 웨이브 1 E-prim(프리미티브 6종+배칭 규칙, 골든 PNG 3장)+E2(타이머 151개·프로퍼티 968개 생성기 커밋)+DST → 웨이브 2 E-prop(레지스트리+상태원천 라우팅)+E-load(모델·JSON5·Lua `mlua` 샌드박스·경로해석, 로더 통합테스트 신규작성 중 결함 2건 발견 수정) → 웨이브 3 E-screen(`skin_render/` 신규, BGA 특수경로 제거)+E-ui(SKIN 탭+문서탐색/선택/커스터마이즈+`rbms-config` 영속화) → 적대 리뷰 28건(critical 4) 전건 테스트 붙여 수정, history `docs/history/2026-09-09-phase-e-skin.md`, 발산 `docs/acknowledge/reference-divergences.md` §Phase E(착수전 5건+웨이브3 통합 9건+리뷰반영 7건, 총 21건 E-D1~E-D21)) — 최종 게이트(Fable 실측, 이 세션 재검증, 수정 없음): fmt 통과(no-op), `cargo test --workspace` **2633 통과 · 0 실패**, clippy `-D warnings` 0 경고, 금지어 grep 0, 신규 `//` 주석 0(유일 매칭은 raw string 픽스처 리터럴 내부 텍스트). 골든 PNG 4장 합계 23KB(200KB 예산 이내, 이번 세션 무변경). 기본화면/픽스처스킨 헤드리스 PNG 확인 완료. 미구현(후속): Note/Judge/Gauge류 오브젝트(E-D9)·레인별 타이머(E-D14)는 플레이 세션 데이터 배선이 Phase E 범위 밖이라 경고 후 드롭
- [x] F UX·기능 고도화(2026-09-09~10, 명세 `docs/plan/2026-09-09-phase-f-spec.md`; F0~F4 5갈래 병렬 구현 → 통합(`docs/history/2026-09-10-phase-f-integration.md`) → 적대 리뷰 21건(critical 1·major 다수) 반영 19건·발산 등록 2건 → 최종 검증, history `docs/history/2026-09-09-phase-f-ux.md`, 발산 `docs/acknowledge/reference-divergences.md` §Phase F) — 옵션 오버레이(결정 3, 11행)·토스트/상태줄·백그라운드 로딩(폴더스캔/표fetch/키음·BGA 디코드 워커화)·정렬 12종(레퍼런스 `BarSorter` 패리티, 오름차순+무기록 마지막)·필터 패널(레벨/모드/클리어/즐겨찾기)·타깃 8종(고정레이트 11종 전체선택+RANK NEXT)/PACEMAKER 실시간·결과 3분할 그래프(GAUGE/TIMING/JUDGE)·27분위 랭크바. 최종 게이트(Fable 실측, 수정 없이 재검증): fmt 통과(no-op), clippy `-D warnings` 0 경고, `cargo test --workspace` **2232 통과 · 0 실패 · 2 ignored**(테스트 바이너리 38개 전부 ok), 금지어·금지주석 0건. 옵션 오버레이/결과+그래프/필터+토스트 3화면 헤드리스 PNG 육안 확인(임시 하네스, 확인 후 원복). 미구현 2건(BPM 정렬=시작BPM vs 레퍼런스 최고BPM · 차트파싱 미워커화)과 LANE OPTION FLIP/BATTLE 미배선은 발산 문서에 사유·해소조건 등록 후속(경미): 토스트가 곡선택 우하단 힌트 줄과 겹침 — 토스트를 힌트 위로 올리거나 표시 중 힌트를 숨길 것
- [x] G 데이터 스케일·롱테일 — 곡DB(rusqlite)·스코어DB·코스·연습 모드·gilrs/MIDI·시스템 사운드·복수 IR
  - [x] G0~G7·G9 갈래 산출물과 중단 시점 G8 배선을 `wip/phase-g`에서 복구하고 `cargo check -p rbms-player --tests` 성공 확인
  - [x] G8 실제 배선 완전성 감사와 첫 clippy 실패 분류 — G2·G5·G6 필수 배선 완료, G1·G3·G4·G7·G9 부분 배선; CLI 구형 스캔 단절·코스 이월·연습 시작 게이지/배속·복수 IR 오프라인 게이트를 후속 구현으로 확정
  - [x] 미배선 기능을 의존 순서대로 연결하고 갈래별 최소 테스트 통과
    - [x] G1·G9 CLI를 SQLite 증분 스캔과 bmson 차트 요약 경로로 전환 — `cargo test -p rbms-cli` 12통과, clippy `-D warnings` 통과
    - [x] G3 코스 gauge·standing combo를 다음 스테이지에 이월 — judge/play/player 단위 테스트 통과, 관련 clippy `-D warnings` 통과
    - [x] G4 연습 모드 시작 gauge와 재생 배속을 실제 세션·오디오에 적용 — 구간 경계 LN 클리핑·시작 BGA 프레임·BGA 소유권 복원까지 반영
      - [x] 시작 gauge 종류·값을 세션 생성과 판정 설정 재적용 뒤에도 보존
      - [x] 50~200% 배속과 중간 구간 시작 원점을 판정·스케줄·키음 시간축에 동일 적용
      - [x] 연습 재생에서도 로딩한 BGA 이미지를 복제하지 않고 플레이 종료 시 복원
    - [x] G7 추가 IR만 구성한 오프라인 fan-out과 primary 선택 흐름 완성
      - [x] extra-only 결과 fan-out·완료 상태 수렴·랭킹 조회·리플레이 다운로드를 primary 기준으로 전환
      - [x] primary 프로필 선택 UI와 선택 변경 시 랭킹 캐시·진행 중 요청 무효화
    - [x] G5 포함 테스트 전용 API·dead-code 허용 제거 후 player clippy 복구 — test-only 접근자는 `cfg(test)`, 미사용 선행 추상화 삭제, player all-targets clippy 통과
  - [x] workspace fmt·test·clippy 게이트, 적대 리뷰, 실행 스모크, history 갱신 — fmt·clippy·workspace test 실패 0, CLI 임시 폴더 스모크 `charts=1/read=1`, 최종 적대 리뷰 blocker 0 (`docs/history/2026-09-16-phase-g-data-long-tail.md`)
- [x] H 경미 후속 일괄(사용자 결정 2026-09-10: G 다음에 모아서 처리)
  - [x] H1 곡선택 토스트를 하단 힌트와 겹치지 않는 위치에 배치, 헤드리스 테스트 통과
  - [x] H2 `app_input::tests::the_settings_row_and_the_in_play_key_stop_at_the_same_ceiling`를 경계값 검사로 축소 — 16.93초 → 3.01초(82% 감소)
  - [x] H3 실제 오디오와 Phase G 기능의 실기 확인 절차·합격 기준 문서화 — `docs/quality-assurance/2026-09-16-phase-h-manual-checks.md`; 실기 자체는 실제 장치·서버 환경에서 후속
  - [x] H4 최종 fmt·clippy·test·적대 리뷰·history와 실행 방법 정본화 — fmt·clippy·workspace test·release build·`git diff --check` exit 0, 적대 리뷰 blocker 0; 이력 `docs/history/2026-09-16-phase-h-minor-followups.md`
- [ ] R 릴리스(prod 브랜치·브랜치 보호·v0.1.0 태그·서명) — **사용자가 실제 키와 함께 추후 진행**(자택 보관 키 사용, 2026-09-10 결정). 에이전트는 착수하지 않음

---

## 보존 이력 — 전체 로드맵 실행 당시 기록 (2026-06-03~)

> 이 블록은 당시의 계획·실행 스냅샷을 보존한다. 체크박스와 미구현 표기는 작성 시점의 사실이며 현재 상태를 나타내지 않는다. 현재 정본은 이 문서 상단과 `docs/roadmap.md`이며, Phase A~H는 완료되고 Phase R만 실제 키를 기다리는 미착수 상태다.
>
> 당시 사용자 지시: "계획을 docs에 반영 후 Phase 0→7까지 멈추지 말고 진행." 확정 순서(9-에이전트 적대적 분석·비평으로 도출, 비평 정정 반영). 당시 forward plan은 `docs/roadmap.md`, 결정 대기 항목은 `docs/acknowledge/`.
> 효력: S<1일 · M=1~3일 · L≈1주 · XL=수주. `(선택)`/`(BLOCKED)` 표기.

**Phase 0 — 위생 & 사실 정정 (전부 S, 즉시) ✅ 완료(2026-06-03)**
- [x] 문서 사실 정정 1패스 — 테스트 수 `132/92/100 → 880`(PROCESS·CLAUDE·ci-release), CN/HCN 판정 미차별 명확화(`divergences.md` 추가)
- [x] `ci-release.md` stale 정정 — 리모트·dev 존재 반영, prod 브랜치+첫 태그만 잔여
- [x] 루트 `LICENSE`(GPL-3.0 전문) 추가
- [x] `docs/roadmap.md`·README 배포 섹션 dev/prod·Actions 상태 정정 (역할: roadmap=로컬 워킹, docs/roadmap=공개 ledger)
- [x] 미커밋 `docs/` + `LICENSE`를 dev 커밋(`1d3fb13`, 작성자 Hyunseok Byun, co-author 없음; gitignore 항목 제외)
- [x] dev → origin 푸시 (CI 3-OS 트리거됨)

**Phase 1 — 클라 정확성(핵심 패리티) + 마무리**
- [x] `PlayOptions.lntype`를 헤더에서 유도 (app_play.rs 하드코딩 제거 → `ir_map::ir_lntype`, 0=LN/1=CN/2=HCN, 테스트)
- [x] #PREVIEW: `config.debug` 6분기 계측 + `samples/preview-demo` 픽스처 + dead_code 제거 (가청 확인은 수동 1회 — `docs/bug/2026-06-03-preview-playback.md`)
- [x] (2026-06-07) **곡선택 하이브리드 미리듣기** — `#PREVIEW` 있으면 파일, 없으면 곡 **autoplay 미리듣기**(백그라운드 코디네이터 스레드: 파싱→스케줄→키음 디코드, 취소가능, 위상연속 루프). `load()`와 디코드 헬퍼 공통화. 2R 적대적 리뷰 반영. → `docs/history/2026-06-07-select-autoplay-preview.md` (가청 1회는 사용자 몫)
- [x] ⭐ `LnKind` 판정 전파 + CN/HCN 2-판정 모델(head@press + end@release, 이른릴리스=end 판정·미히트=2 Miss, 분모 CN=2) + 합성 픽스처 5종. `Cn`/`Hcn` 게이트 → LN/Normal byte 불변. HCN 200 ms 틱·CN deferral·BSS/MSS는 **Phase D에서 완료**(→ `docs/history/2026-09-09-phase-d-judge-parity.md`).

**Phase 2 — 첫 릴리스(인프라)**
- [ ] prod 브랜치 생성·푸시 + 브랜치 보호 전략 결정(필수, `docs/acknowledge` 기록)
- [ ] 첫 GitHub Release (v0.1.0 수동 태그, macOS 유니버설+Windows+sha256)

**Phase 3 — 클라 마지막 기능**
- [x] NETWORK 설정 탭 — SERVER URL/PLAYER ID 행(22/23), text_input 인플레이스 편집(Enter 커밋·Esc 취소·버퍼 표시), `PlaySettings` 영속(라운드트립 테스트), `build_server` 추출로 ScoreServer 재구성. 빈 URL=오프라인·빈 ID=guest.

**Phase 4~6 — 백엔드 + 웹 FE (2026-09-09 Phase W 로 대체·완료, 별 레포 대신 이 레포 `web/`)**
- [x] IR 계약 동결(/api 프리픽스·settings 경로·필드명) → decisions W7
- [x] 백엔드 M1·M1.5·M2·M3 전부 `web/` Next.js 단독 서버로 구현(38 라우트: health/version/auth/scores/charts/players/replays/settings/rivals/courses/tables/admin/fe) + ranked 정책·build allowlist·audit + Vercel 프로덕션 배포
- [x] 웹 FE: 검색·차트·리더보드·플레이어·인증(로그인/가입)·홈·tables·courses·리플레이 뷰어·settings(토큰/동기화/라이벌)·admin/builds
- [ ] 클라 `SubmitResponse` DTO 확장 — 현 `rbms-ir` DTO 는 accepted/rank/previous_best/message 뿐, 서버가 주는 ranked/flags/is_new_best/score_id 미수신
- [ ] 클라 로그인 UI/토큰 — `apps/rbms-player/src/main.rs` `build_server` 가 `HttpScoreServer::try_new(url, None)` 으로 token=None → 플레이어 제출은 전부 guest(unranked). NETWORK 탭에 로그인(register/login → 토큰 저장) 필요. `rbms-ir` 는 Bearer 지원 완료
- [ ] 클라 랭킹 패널·리플레이 업/다운로드·설정 동기화 토글·rival UI — 서버 엔드포인트는 전부 준비됨, 클라 호출부 없음
- [ ] (선택) 설정 화면 마우스 스테퍼 UX

**Phase 7 — 롱테일 / 선택 폴리시 & 하드닝**
- [x] ~~HCN 연속 게이지~~ · ~~스크래치 BSS/MSS~~ — **Phase D에서 완료**(CN deferral·HCN 200 ms 틱 포함, → `docs/history/2026-09-09-phase-d-judge-parity.md`)
- [ ] (선택) 게이지 5K/PMS
- [ ] (선택/BLOCKED) 결과·메뉴 레이아웃 RON화 · (선택) 테마 chrome 색 완성
- [ ] (선택) 백엔드 M4 LR2IR READ 어댑터 · (선택/BLOCKED) FE 리플레이 뷰어 · (선택) 백엔드 M5 안티치트
- [ ] (선택) macOS 서명/공증·Windows Authenticode · (선택) .app/.dmg+무인자 picker
- [ ] (선택) 코스메틱: 리사이즈 리플로우·BGA 비디오(mpg)·글리프 아틀라스(P3a)·웹폰트(P4) — 판정/점수 영향 0

> 결정 대기(요약): CN/HCN 테스트 코퍼스 범위 · prod 보호+첫 버전 · 서명 비용. (IR 계약·백엔드 소유권·리플레이 뷰어는 Phase W 로 해소.) 다음 착수 후보: ① 클라 로그인/토큰 + DTO 확장(MVP 루프 완성) ② 고도화 계획 Phase B~G(`docs/plan/2026-09-09-enhancement-plan.md` §2) ③ prod 브랜치·첫 릴리스.

---

## 0. 목표 / 현재 상태

레퍼런스 구현(Java/libGDX) **코어 PLAY**를 Rust로 재구현. **완전 플레이 가능 + 레퍼런스 구현/IIDX 확장 다수 완성.**
흐름: BMS 로드 → (노트옵션 셔플) → 로딩화면 → 하이스피드 스크롤 → 키음 샘플정확 재생 → 입력/판정/게이지 → HUD → 결과 → (옵션)스코어 서버 전송. GUI 곡선택(폴더·난이도표 네비)·설정(탭)·키설정 에디터·리플레이 포함.

판정/게이지/TOTAL은 **레퍼런스 구현과 byte/수치 단위로 대조 검증됨**(§6 검증 참고). 미구현은 §7 한계.

## 1. 확정 결정

- **그래픽**: `wgpu 29 + winit 0.30`, 네이티브 인스턴스드 쿼드(`fill_rect`=1인스턴스). 렌더는 `Renderer` trait 추상화(CPU 백엔드 `CpuCanvas`도 존재 → 테스트/예제). 레퍼런스 좌표공간 **1280×720(16:9)** 고정, GPU 유니폼이 NDC로 매핑.
- **오디오**: `cpal 0.17 + symphonia 0.5 + 커스텀 RT 믹서`. **마스터 클럭 = 재생된 샘플 수**(vsync 아님) — 레퍼런스 구현의 vsync 양자화 자체는 피했으나, 클럭이 **오디오 콜백 주기(실측 512프레임/10.667ms 고정)로 양자화**되는 점은 남아 있다(판정 타임스탬프 오차 평균 5.33ms·최대 10.67ms). 근본 해결은 Phase B에서 마지막 콜백 시각 기준 연속 보간으로 처리 예정 — `docs/plan/2026-09-09-enhancement-plan.md` §1.2 A1.
- **시간 전역 µs(i64)**. 차트 해시 = raw 바이트 MD5+SHA-256(byte-exact).
- **모드/스킨/키맵/설정은 데이터 주도**. 모드=`Mode` 구조체, 스킨=`SkinConfig`(RON), 키=`KeyConfig`(RON), 플레이옵션=`PlaySettings`(RON).
- **키맵 기본 = 레퍼런스 구현 Z열**(사용자 결정). 차트 모드 자동감지로 프리셋 선택, CLI/인앱 오버라이드.
- **사용자 설정 파일**(`~/.config/rbms/`): `settings.ron`(플레이옵션) · `keyconfig.ron`(키) · `tables.ron`(난이도표 목록) · `scores.ron`(로컬 플레이 기록) · `replays/`(리플레이). 모두 없으면 기본값 자동생성, 파싱실패 시 `.bak` 백업.
- **테스트 데이터**: `/Users/hyunseokbyun/Documents/personally/1/`(발광 ★1, 727차트). **커밋 금지(저작권)**. 발광1 난이도표 = `https://darksabun.club/table/archive/insane1/data.json`(로컬 86곡/18레벨 매칭 실측).

## 2. 모드 프로파일 (확장성)

`rbms_model::Mode`(name/key/player/scratch/channel_assign) + 상수 `BEAT_7K/5K/10K/14K/POPN_9K`. `detect_mode(src, filename)` 자동판별. 새 키모드 = 데이터 추가(엔진 분기 없음).
**14K 레인(주의)**: BEAT7 table 기준 P1 키=lane0-6/스크=7, **P2 키=lane8-14/스크=15**.

## 3. 크레이트 맵 (의존: 위→아래)

```
apps/rbms-player  ← winit+wgpu 윈도우·게임루프·GUI(셀렉트/설정탭/키설정/난이도표·로컬기록모달·마우스·디버그). 모든 크레이트 사용.
                    main.rs + 모듈: keyconfig.rs(키설정) · settings.rs(플레이옵션 영속) · replay.rs(리플레이) · tables.rs(난이도표목록) · scores.rs(로컬 기록 ScoreBook).
apps/rbms-cli     ← 차트 정보 출력(파싱 검증 도구).
crates/
  rbms-play       ← 통합 드라이버: 키음 스케줄러·autoplay·press/release·auto_lanes·키빔상태·**키봄상태(bomb: 노트히트 판정 기록)**·set_judge_rate·simulate_autoplay.
  rbms-render     ← Renderer trait + CpuCanvas + skin(SkinConfig/Skin) + playfield(노트·키빔·레인장식·커버) + hud(콤보/판정/가로게이지) + result + **select**(SelectView/SelectHot/render_select: 곡선택 행리스트·상세·밀도그래프·기록·모달, 헤드리스 예제) + **render_key_bomb**(노트히트 폭발, SkinConfig 데이터주도) + font(**cosmic-text 다국어**: FontSystem+SwashCache, fill_rect 알파, load_font/set_ui_family).
  rbms-table      ← 난이도표(BMS table): dto(TableEntry/Header)·DifficultyTable(fetch/fetch_or_cache·by_level)·md5 매칭. reqwest blocking.
  rbms-audio      ← decode(symphonia)·mixer(RT voice pool,stride 리샘플)·engine(cpal, SAMPLES_PLAYED 클럭, **sample_duration_us**=프리뷰 루프용).
  rbms-ir         ← IR-슈퍼셋 클라이언트: dto(serde)·ScoreServer trait·HttpScoreServer·NullScoreServer. (클라 계약=docs/reference/ir-api.md, **서버 전체설계=docs/backend/**)
  rbms-judge      ← windows(µs 윈도우·rank·scaled)·matcher(JudgeEngine: 매칭·콤보·EX·미스·LN·fast/slow·**空POOR empty_poor**·set_windows)·gauge(GrooveGauge·ClearType).
  rbms-chart      ← detect_mode·to_model(마디→µs·채널→레인·LN/mine)·scroll(visible/constant_offsets·그린넘버)·shuffle(NoteOption)·count_playable_notes·**note_density**(레퍼런스 구현 SongInformation 포팅: 1초빈 히스토그램+peak/avg/end).
  rbms-parser     ← BMS lexical: base36/62·Shift-JIS·#mmmCC·#RANDOM/#IF·MD5/SHA-256 → BmsSource.
  rbms-model      ← 순수 타입: Mode·Note·TimeLine·Model·LnKind·NoteKind.
```

## 4. 완료 기능 (전부 검증됨)

**코어:** 파서(727곡 MD5 byte-exact) · 타이밍모델 · 오디오(실디바이스) · 스크롤/렌더 · 판정엔진 · autoplay · 입력 · 라이브 윈도우.
**판정/게이지:** 판정 윈도우(레퍼런스 구현 `JudgeProperty.SEVENKEYS` 일치) · `#RANK→judgerank`(`[25,50,75,100,125]%`, NORMAL=#RANK2=75%) · 게이지 6종+모디파이어 · 클리어램프(색=레퍼런스 구현 `LAMP`) · LN 풀판정(head hold→release) · **空POOR(이른 빈 POOR)=노트 미소실·콤보 유지·MS페널티**(레퍼런스 구현 `judgeVanish[5]=false`·`combo[5]=true`, `empty_poor` 카운터).
**게임옵션:** 모드 자동감지 · **노트옵션**(OFF/MIRROR/RANDOM/S-RANDOM/R-RANDOM/ROTATE, 시드 결정적·DP 사이드분리) · **하이스피드 고정**(FLOATING/CONSTANT 그린넘버) · **판정 오프셋**(±200ms) · **오토 캘리브레이션**(정확판정 평균오차로 결과 시 offset 보정, per-run 수렴) · **JUDGE WIDTH**(judge_rate 50~200% 윈도우 스케일) · **TOTAL**(override/노트수 기본) · lift · lane cover(sudden) · scratch side/auto.
**렌더/UI:** 1280×720 랜드스케이프(필드 center-left+우측 BGA) · 키 빔 · 레인 outline+구분선(opacity/onoff) · **가로 게이지**(레인 아래 0~100, IIDX식) · **다국어 폰트**(cosmic-text+Inter+시스템폴백 → 일/한/중/태/아랍 등 모든 언어 AA 렌더, `font_path`/`--font`로 교체) · 플레이 HUD(콤보·직전판정·FAST/SLOW·판정카운트·EX·게이지) · 결과화면 · 일반/와이드 스킨(RON, 임베드+`--skin`) · BGA(이미지) · BGA on/off.
**셀렉트/설정/키:** GUI 곡선택(재귀 스캔·폴더 네비 Root→ALL SONGS/난이도표→레벨→차트, ↑↓·←상위/→진입·Enter·**마우스 클릭**) · 우측 정보패널+**로컬 기록 인라인 리스트·상세 모달**(`R`/클릭, 리플레이 재생) · 폴더 선택(rfd) · **난이도표 인앱 관리**(다중표 tables.ron·URL텍스트입력/파일rfd 추가·제거) · **설정 탭**(PLAY/GAUGE/JUDGE/DISPLAY/INPUT, 탭/값 **마우스 클릭**) · **통합 키 설정**(파일+인앱 에디터, 레인+조작키 전부 재바인딩·충돌검출) · **설정 영속**(settings.ron, AUTO REPLAY 토글 포함) · **리플레이 저장·재생**(`--replay`/모달, 시드/옵션 복원해 동일 재현) · **로컬 스코어 영속**(scores.ron, 서버 무관).
**IR:** IR 슈퍼셋 클라이언트(스코어 제출, `--server`·`--player`) — DTO 슈퍼셋 확장(JudgeBreakdown **early/late(epg…lms)·avgjudge·empty_poor**, PlayOptions **전체옵션**, ScoreSubmission **seed/algo/rule/skin/`client_build_sha256`(자기-빌드 sha2)·`client_platform`**, ReplayData **µs구조**(`ReplayEvent`), settings/replay-dl/auth/course 메서드 stub). 전부 serde default 후방호환. **백엔드 서버는 설계 완료·구현 후속**.
**ROADMAP 클라이언트 9종(2026-05-31 완료):** 폴더 SCANNING 로딩·**스코어 랭크 그래프(IIDX 9분법 `dj_rank`+랭크바, 결과+셀렉트)**·**점수 ΔEX 비교(직전/베스트)**·**리플레이 분석모드(재생바·일시정지·배속·재시뮬 시크·노트별 ms-off)**·**데이터 주도 HUD 스킨**(판정 팔레트/라벨/게이지 임계·색/요소위치 RON화)·**폰트 P3**(무할당 중첩 캐시·말줄임)·**노트옵션 ALL-SCRATCH/H-RANDOM**(시간임계 40/125ms)·**그린넘버 표시**·**CN/HCN(`#LNMODE`)**·**듀얼필드 14K(P1좌·P2우)**. SCORE GRAPH·REPLAY ANALYSIS 설정 토글. → `docs/history/2026-05-31-roadmap-client-features.md`.
**UI(IIDX/LR2 지향 재설계, 2026-05-31):** 폴더 영속(`songs_folder`)·폴더 스캔 백그라운드 스레드+애니메이션 LOADING·노트 상단 클리핑(`[top_y,judge_y]`, 프레임 상단서 흘러나옴)·플레이 IIDX 레이아웃(좌 정보, 중앙 **라이브 스코어 그래프**, 우 판정카운트/BGA)·결과 IIDX 레이아웃(거대 DJ LEVEL+스코어 리포트, PGREAT 핫핑크)·곡선택 행별 클리어램프 LED·**곡선택 상세 메타 고도화**(부제/아티스트/장르·제작자 + 2열 스탯그리드: BPM범위·DIFFICULTY명·NOTES(+LN)·JUDGE(#RANK명+%)·LENGTH·TOTAL; 포커스 곡만 lazy `to_model`로 노트수/길이/BPM범위 산출, `#MAKER` 파싱, bms-rs 메타 모델 참조). DP(14K)는 BGA 비키도록 좌측 앵커. 레퍼런스 `docs/reference/ui/`·스펙 `docs/reference/ui-design.md`. → `docs/history/2026-05-31-ui-redesign-iidx.md`.
**곡선택 전면 재설계(레퍼런스 구현 modern chic, 2026-05-31):** 렌더링을 **`rbms-render::render_select`로 추출**(`SelectView`/`SelectHot`, `CpuCanvas` 헤드리스 PNG 검증 가능, `main.rs` 인라인 제거) — 행 KEY/레벨 배지·클리어램프 LED(좌바+우세로바)·포커스 연출(시안테두리+노랑타이틀)·중앙포커스 스크롤, 상세 **커버(`#STAGEFILE`→`#BANNER`, 단일 BGA슬롯 쿼드뒤·포커스당1회 디코드)**+제목블록(2줄 래핑)+2열 스탯그리드+**노트 밀도 히스토그램**(`note_density`, 서브픽셀·긴곡 오버플로없음·PEAK/AVG/END notes/sec)+기록(베스트바·DJ랭크바·최근행 EX+랭크+추세)+기록모달. `main.rs` `build_select_view` 조립 + `SelectKey` 캐시(매프레임 재할당 방지)+hot 매핑. 파서 `#BANNER`/`#PREVIEW` 추가(`#PREVIEW`는 데이터만, 재생 후속). **2라운드 적대적 멀티에이전트 리뷰**(1R 16건 반영·2R 0건). → `docs/history/2026-05-31-select-redesign.md`, 스펙 `docs/reference/ui-select-redesign.md`, 타깃 `…/ui/provided/06-reference-select-target.png`.
**곡선택 후속(2026-05-31): 스탯그리드 잘림 수정·#PREVIEW·KEY BOMB.** (1) 상세 스탯그리드 **2열×3행→3열×2행** 압축(하단 LENGTH/TOTAL이 DENSITY 구분선에 잘리던 것 해결). (2) **`#PREVIEW` 프리뷰 재생 (TODO — 실재생 미동작, 2026-06-07 해소)** — 포커스 settle(디바운스 20프레임) 시 `#PREVIEW` 디코드+루프(경계 재트리거, mixer `play(key)`가 동일키 먼저 stop하므로 선스케줄 대신 자연종료 후 재발화) 로직·select 전용 `AudioEngine`(플레이 엔진과 분리, `load()` 진입부·select 이탈 시 정지)·`AudioEngine::sample_duration_us`·**DISPLAY 탭 PREVIEW 토글**(`PlaySettings.preview`)까지 배선했으나 **포커스해도 소리 안 남**(추후 디버깅, 코드 유지). (3) **KEY BOMB** — 노트 히트(judge≤3, 空POOR/miss 제외) 시 판정선 판정색 확장+소멸 버스트. `Player.bomb`(press/release/autoplay 전 경로 기록)·`rbms_render::render_key_bomb`·**`SkinConfig` bomb_enabled/height/duration_ms 데이터주도**(기존 RON serde default 호환). 적대적 리뷰 1건(LOW: 리플레이 직접실행 시 cpal 스트림 1프레임 공존) 근본수정. → `docs/history/2026-05-31-select-redesign.md`.
**CI/배포:** GitHub Actions(`.github/workflows/ci.yml`·`release.yml`) — 자동 버전·macOS 유니버설·Windows 빌드·릴리스(→`docs/ci-release.md`). 실제 동작은 GitHub 원격 push 시.

**2026-06-03 세션:** IR `lntype` 차트 유도(`ir_map::ir_lntype`, 0=LN/1=CN/2=HCN; 기존 하드코딩 제거) · **`#PREVIEW` 계측**(6 silent 분기 `config.debug` 로그)+`samples/preview-demo` 픽스처+dead_code 제거(가청 확인만 수동 잔여) · **CN/HCN 2-판정**(head@press + end@release, `Cn`/`Hcn` 게이트, 분모 2, 픽스처 5종; LN/Normal byte 불변) · **NETWORK 설정 탭**(SERVER URL/PLAYER ID 인앱 편집·`PlaySettings` 영속·`build_server` 재구성) · **Windows 설정 경로**(`config_dir` HOME→USERPROFILE). + Phase 0 위생(LICENSE·테스트수 정정·docs 커밋·dev 푸시) · 릴리스/백엔드 결정 기록(보류). → `docs/history/2026-06-03-session.md`.

**2026-06-07 세션:** **곡선택 하이브리드 미리듣기**. 사용자 의도 정정 — 원하던 건 `#PREVIEW` 파일만이 아니라 "포커스한 곡이 들리는 미리듣기". 원본 레퍼런스 구현도 곡 autoplay 미리듣기는 없음(=#PREVIEW 파일 or 메뉴 BGM)이라, **하이브리드 신규 구현**: `#PREVIEW` 있으면 파일, 없으면 **백그라운드 코디네이터 스레드**가 차트 파싱→throwaway `Player`로 autoplay 키음 스케줄 추출→전체 키음 병렬 디코드(취소가능)→메인 스레드가 `PreviewMsg` 채널 드레인 후 클럭 앵커·dispatch-먼저-루프(+2초 tail, 위상연속 재-앵커). 키음 디코드 fan-out/job빌드는 `load()`와 공통 헬퍼(`keysound_jobs`/`spawn_keysound_decode`)로 통합. 헤드리스 통합테스트(`tests/autoplay_preview.rs`)+디코드→믹스 테스트 추가(889통과). **2라운드 적대적 리뷰**(1R 8건·2R 1건 전부 반영). → `docs/history/2026-06-07-select-autoplay-preview.md`. (실기 가청 1회는 사용자 몫)

**2026-06-07 세션 (2) — 첫 실행/온보딩(.dmg 대비):** `.dmg` 더블클릭(터미널 없음) 대비. (1) **무인자 진입** — 인자·기억폴더 없을 때 `usage`+`exit` 대신 GUI 진입. (2) **초기 스캔 백그라운드화** — `App::new` 동기 블로킹(창도 안 뜸) → 백그라운드 스레드+`scan_rx`/`apply_scan`, SCANNING 화면에 **곡 수 카운트** 표시. (3) **첫 실행 빈 곡선택 + 온보딩 CTA**(`SelectView.empty_hint`: WELCOME/add-folder, 헤드리스 렌더 시각확인). (4) **Esc-중-스캔** 종료 회귀 수정(취소 복귀). 집중 적대 리뷰 1건 반영. → `docs/history/2026-06-07-first-launch-onboarding.md`. (`.app`/`.dmg` 패키징·공증은 release.yml 측 후속)

상세 이력 → `docs/history/2026-05-31-*.md`·`2026-06-03-session.md` (§8 docs맵). 당시 다음 할 일 → `docs/roadmap.md`.

## 5. 실행

```bash
cargo run --release -p rbms-player --                    # 기억 폴더 → RBMS_SONGS → 빈 GUI
cargo run --release -p rbms-player -- "<폴더>"            # GUI 곡선택
cargo run --release -p rbms-player -- "<차트.bms>" --auto # 단일 차트 autoplay
cargo run --release -p rbms-player -- "<차트.bms>" --interactive
./target/release/rbms-player "<폴더>"                    # macOS / Linux 빌드 바이너리
.\target\release\rbms-player.exe "<폴더>"               # Windows PowerShell 빌드 바이너리
```
- **곡선택**: ↑↓ 이동 · →/Enter 열기/플레이 · ←/Esc 뒤로/상위 · **Tab 설정** · **O 폴더선택**(`+ ADD FOLDER` 선택 → Enter → Esc로 저장·재스캔) · **T 난이도표 관리** · **R 기록 모달** · **마우스**(행 1클릭 선택·재클릭 열기, 우측 기록 클릭→모달). 모달: ↑↓ 이전/다음·Enter 리플레이·Esc 닫기.
- **설정(Tab)**: **Tab으로 탭 전환**(PLAY/GAUGE/JUDGE/DISPLAY/INPUT) · ↑↓ 이동 · ←→ 값변경 · Enter(KEY CONFIG 진입) · Esc 저장후복귀.
  - PLAY: AUTOPLAY·HI-SPEED·SPEED FIX(FLOATING/CONSTANT)·RANDOM·**AUTO REPLAY**. GAUGE: GAUGE·TOTAL. JUDGE: JUDGE OFFSET·JUDGE WIDTH·AUTO CAL. DISPLAY: SKIN(NORMAL/WIDE)·**FONT**(DEFAULT/CUSTOM, Enter/우/클릭=파일선택 라이브 적용·좌=기본)·LIFT·LANE COVER·BGA·**DEBUG MODE**. INPUT: SCRATCH SIDE·SCRATCH AUTO·KEY CONFIG. (탭/값 마우스 클릭 가능)
- **플레이 중**: Esc=뒤로(중도 포기) — 단 **남은 노트가 없으면 Esc로 곧장 결과화면**(아웃트로 대기 스킵). **DEBUG MODE** 시 좌상단 FPS/RAM/프레임시간/노트·콤보·게이지 등 수치 오버레이.
- **키 설정**: 설정→`KEY CONFIG`→`Stage::KeyConfig`(EDIT MODE 전환·Enter 캡처 재바인딩·충돌 빨강·Esc 저장) 또는 `~/.config/rbms/keyconfig.ron` 직접 편집.
- **기본 레인 키(레퍼런스 구현 Z열)**: 7K=`Z S X D C F V`+LShift(스크) · 5K=`Z S X D C`+LShift · 9K/PMS=`Z S X D C F V G B`(스크없음) · 14K=좌손 P1(`Z S X D C F V`+LShift)·우손 P2(`M K , L . ; /`+RShift).
- **기본 조작 키(플레이 중, 재바인딩 가능)**: ↑/↓=hi-speed · →/←=lane cover · ]/[=lift. Esc=뒤로.
- **CLI 옵션**: `--interactive|--auto --sc-left --sc-auto --lift F --hispeed F --gauge ... --keys Z,S,X,... --skin file.ron --font file.ttf --table URL --keyconfig path.ron --replay file.ron --server URL --player ID`.

## 6. 검증 메모 (판정/게이지 — 사용자 "빡세다" 검증)

- 판정 윈도우 = 레퍼런스 구현 `JudgeProperty.SEVENKEYS`(PG±20/GR±60/GD±150/BD-280~220/MS-150~500 µs @judgerank100) **정확 일치**.
- `#RANK 0~4 → judgerank [25,50,75,100,125]%` = `JudgeWindowRule.NORMAL`과 일치. **#RANK 2(NORMAL)=75%=PGREAT ±15ms**는 레퍼런스 구현과 동일.
- 게이지/TOTAL = `deltas * total/notes`(그루브) 일치. 자동미스는 BAD 늦은경계 이후. **예외(2026-09-09 감사, Phase A에서 수정 완료)**: 見逃し POOR(스윕 미스)가 게이지 MS 슬롯(idx5)으로 잘못 들어가던 결함(J1)이 있었다 — 코드 4(PR)가 아니라 코드 5(MS)로 처리돼 게이지 페널티가 실제보다 컸다. `crates/rbms-judge/src/matcher.rs`에서 슬롯을 정정했다 → `docs/acknowledge/reference-divergences.md` "판정 윈도우·게이지 발산" 섹션.
- 결론: "빡센" 건 NORMAL 윈도우가 원래 타이트 + 입력/표시 지연 미보정. → **JUDGE OFFSET** 수동 또는 **AUTO CAL**(한 곡 플레이로 자동 보정).

## 7. 알려진 한계 / 다음 후보

- ~~CJK 폰트 없음~~ **해결**(cosmic-text). ~~캐시키 할당~~/~~말줄임~~ **P3 해결**(무할당 중첩 캐시·`fit_text`). **P3a run-length 병합 보류**(프로파일 ROI~11%, AA 텍스트 픽셀별 alpha 상이 → 본 해법은 글리프 텍스처 아틀라스, 후속). **P4 웹폰트(URL)** 후속.
- ~~14K 단일필드~~ **듀얼필드 해결**(`dual_field`, P1좌·P2우·바깥 스크래치, 필드별 judge라인/구분선/게이지 1개). 비활성화(`dual_field:false`) 시 레거시 단일필드.
- ~~ALL-SCRATCH/H-RANDOM 미구현~~ **해결**(시간임계 40/125ms). ~~green-number 미반영~~ **표시·반영**(HUD). ~~CN/HCN 미구분~~ **`#LNMODE`→`LnKind` 구분 + 종단 2-판정 차별화 적용**. HCN 200 ms 틱·CN deferral·스크래치 BSS/MSS는 **Phase D에서 완료**(→ `docs/history/2026-09-09-phase-d-judge-parity.md`). 스크래치 회전(2키 교대) 단순화는 잔존.
- 윈도우 리사이즈 UI 리플로우 없음(논리 1280×720 고정). BGA 비디오(mpg) 미지원. 게이지 5K/PMS 변종·judgerank 커스텀 일부 미반영. 난이도표 추가 fetch는 동기(1개씩).
- **미구현(2026-09-09 감사, `docs/plan/2026-09-09-enhancement-plan.md` 참조)**: 곡DB(매 실행 전량 스캔·증분 없음) · 컨트롤러/MIDI/마우스 스크래치(의존성 0) · 코스/단위 · 연습 모드 · 곡선택 난이도·모드 필터/즐겨찾기/랜덤 선택 · 볼륨 3분리(system/key/bg, `#VOLWAV` 미반영, 현재 마스터 게인 1계통뿐).
- **IR 백엔드 = 전체 설계 완료(`docs/backend/`, 문서 단계)·구현 후속**. ~~클라 슈퍼셋 확장 필요~~ **클라 DTO 슈퍼셋 확장 완료**(§4 IR). 서버 미구현이라 신규 메서드(settings/replay-dl/auth/course)는 호출 시 `Unsupported`. `PlayOptions.lntype`에 실제 LN모드 전파는 후속(모델에 lnmode 미보유).
- ~~UI 곡선택 재설계~~·~~KEY BOMB~~ **완료**(§4). ~~#PREVIEW 파일만~~ **곡선택 하이브리드 미리듣기 완료**(2026-06-07): `#PREVIEW` 있으면 파일, 없으면 곡 **autoplay 미리듣기**(백그라운드 코디네이터·취소가능·위상연속 루프). 실기 가청 1회만 사용자 몫 → `docs/history/2026-06-07-select-autoplay-preview.md`. 남은 UI 후속: **스킨 데이터화**(결과/메뉴 패널까지 — 곡선택 추출·KEY BOMB 데이터화로 진척). ~~`.dmg` 첫 실행 UI~~ **첫 실행/온보딩 런타임 UI 완료**(2026-06-07: 무인자 진입·백그라운드 스캔+곡수·온보딩 CTA·Esc 취소 → `docs/history/2026-06-07-first-launch-onboarding.md`); 남은 건 **`.app`/`.dmg` 패키징·무인자 진입·macOS 공증**(release.yml 측).
- ~~Windows 설정 경로~~ **해결**(`config_dir` HOME→USERPROFILE, → `docs/reference/windows-compat.md`). ~~CN/HCN 판정 차별화~~ **종단 2-판정 적용**. HCN 200 ms 틱·CN deferral·BSS/MSS는 **Phase D에서 완료**(→ `docs/history/2026-09-09-phase-d-judge-parity.md`). 남은: **F5 결과/메뉴 패널 위치 RON화**(현재 HUD 표면만 데이터화), **P3a 글리프 아틀라스·P4 웹폰트**, **FE 프로젝트**(별 저장소·MIT) → 당시 `docs/roadmap.md`. 적대적 리뷰 보류 차이 → `docs/acknowledge/reference-divergences.md`.

> **세션 이력(완료)**: 입력판정 근본수정(空POOR)·폴더 ←→ 네비·마우스·로컬기록 모달·레퍼런스 구현 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE → `docs/history/2026-05-31-input-judge-nav-mouse-records.md`. 다국어 폰트(cosmic-text) → `…-multilingual-font.md`. 백엔드 IR-슈퍼셋 설계 + CI/CD → `…-backend-ir-ci.md`. (모두 §4 완료기능·§8 docs맵에 반영됨)

## 8. docs 맵

- `docs/reference/` — mechanics(레퍼런스 구현 메커닉)·rust-stack·architecture·wgpu29-winit030-api·ir-api(서버계약)·_appendix-raw·**cn-hcn-judgment**(CN/HCN 판정 레퍼런스 구현 대조 구현 스펙·후속)·**windows-compat**(Windows/크로스플랫폼 현황: CI 보장·USERPROFILE 수정·런타임 검증 잔여)·**ui-design**(IIDX/LR2/레퍼런스 구현 UI 레이아웃 스펙)·**ui-select-redesign**(곡선택 정밀 스펙: render_select 추출·밀도 포팅·레이아웃)·**ui/**(레퍼런스 스크린샷: provided 6 + fetched 18 + README).
- `docs/plan/` — **2026-09-09 전면 고도화 계획**: `2026-09-09-enhancement-plan.md`(관점별 갭 표·판정 발산 J1~J26·Phase A~G 실행계획·결정 12건·문서 정정 목록, 이 문서의 정본) + `research-2026-09-09/`(근거 보고서·검증·후속조사·비평 24편, 원 리서치 산출물 보존용).
- `docs/acknowledge/` — decisions(확정결정)·reference-divergences(보류차이)·empty-poor-local-scores(空POOR처리·로컬기록 스키마).
- `docs/font-cjk-support.md` — 다국어 폰트 지원(cosmic-text 결정·레퍼런스 구현 폰트 파악·P1/P2 완료·P3/P4 후속).
- `docs/backend/` — **백엔드 IR-슈퍼셋 서버** 설계: README·PRD·api-spec(전 엔드포인트)·data-model(Drizzle)·endpoint-tasks·compatibility(LR2IR/레퍼런스 구현 매핑+출처)·**contract-freeze**(Phase 4 게이트: 클라 경로 대조·계약 동결·M0 단계). (Hono/Bun/Drizzle)
- `docs/ci-release.md` — GitHub Actions(ci/release): 자동 버전·macOS 유니버설·Windows 빌드·릴리스. 주의(원격/브랜치보호/LICENSE/서명).
- `docs/bug/` — 버그/진단 기록(예: `2026-06-03-preview-playback` #PREVIEW 계측·검증절차).
- `docs/memory/` — test-library(라이브러리 실측).
- `docs/history/` (시간순):
  - `m0-m1-parser-chart` · `m2-audio` · `m3-render-scroll` · `m6-judge-m4-autoplay` · `m7-live-window` — 코어 마일스톤.
  - `features-mode-gauge-ln-result-gpu-config-select` · `skin-bga-ir` · `font-hud-gui` — 1차 확장.
  - `beam-keymap-720p-table` — 키빔·모드별키맵·720p·재귀선택·난이도표.
  - `options-settings-folders` — 노트옵션·설정영속·게이지·BGA·하이스피드고정·오프셋·폴더선택·리플레이·다중표.
  - `judge-verify-tuning-skins` — 판정검증·JUDGE WIDTH/TOTAL·설정탭·일반/와이드스킨·레인장식·가로게이지·오토캘.
  - `input-judge-nav-mouse-records` — 空POOR 판정 근본수정·폴더 ←→ 네비·마우스 입력·로컬 기록(scores)+상세 모달·레퍼런스 구현 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE.
  - `multilingual-font` — 다국어 폰트(cosmic-text+Inter+시스템폴백, 모든 언어 AA)·`font_path`/`--font` 교체·레퍼런스 구현 폰트 파악.
  - `backend-ir-ci` — 백엔드 IR-슈퍼셋 전체 설계(LR2IR+레퍼런스 구현 IR+설정동기화·µs리플레이·빌드해시무결성·FE) + GitHub Actions(자동버전·macOS유니버설·Windows)·라이선스/서명 정리.
  - `roadmap-client-features` — ROADMAP 클라이언트 9종: 폴더 SCANNING 로딩·스코어 랭크 그래프(IIDX 9분법)·점수 ΔEX 비교·리플레이 분석모드·데이터 주도 HUD 스킨·폰트 P3(캐시·말줄임)·IR 슈퍼셋 DTO·자기-빌드 SHA-256·노트옵션(ALL-SCR/H-RAN·green-number·CN/HCN·듀얼필드14K). 웨이브별 적대적 멀티에이전트 리뷰.
  - `ui-redesign-iidx` — IIDX/LR2 지향 UI 재설계: 폴더 영속·백그라운드 스캔(애니 로딩)·노트 상단 클리핑(근본 수정, 베젤 hack 제거)·플레이 IIDX 레이아웃+라이브 스코어 그래프·결과 IIDX 레이아웃·곡선택 클리어램프 LED+상세 메타 고도화(bms-rs 참조, `#MAKER`). DP 레이아웃 BGA 비킴. 적대적 리뷰로 DP충돌·stale폴더·perf 수정. 레퍼런스 수집·디자인 스펙.
  - `select-redesign` — 곡선택 전면 재설계(레퍼런스 구현 modern chic): `rbms-render::render_select` 추출(헤드리스 검증)·`note_density`(SongInformation 포팅)·커버(단일 BGA슬롯)·KEY/레벨 배지·밀도 히스토그램·타이틀 2줄·`build_select_view` 캐시·파서 `#BANNER`/`#PREVIEW`. 2라운드 적대적 리뷰(16건→0건).
  - `2026-06-03-session` — 위생(LICENSE·테스트수 정정·docs 커밋·dev 푸시)·IR lntype 차트 유도·#PREVIEW 계측+픽스처·CN/HCN 2-판정·NETWORK 설정 탭·Windows 설정 경로(HOME→USERPROFILE). 릴리스/백엔드 결정 기록(보류).
  - `2026-06-07-select-autoplay-preview` — 곡선택 **하이브리드 미리듣기**(#PREVIEW 파일 or 곡 autoplay): 백그라운드 코디네이터 스레드(파싱→autoplay 키음 스케줄→전체 키음 병렬 디코드, 취소가능)·위상연속 루프(+2초 tail)·`load()`와 디코드 헬퍼 공통화. 2R 적대적 리뷰(1R 8건·2R 1건 반영). 헤드리스 통합테스트 추가(889통과).
  - `2026-06-07-first-launch-onboarding` — `.dmg` 대비 **첫 실행/온보딩**: 무인자 GUI 진입(usage+exit 제거)·초기 스캔 백그라운드화(SCANNING+곡수 카운트, 창 즉시 표시)·첫 실행 빈 곡선택 온보딩 CTA(`empty_hint`)·Esc-중-스캔 종료 회귀 수정. 집중 적대 리뷰 1건 반영.
  - `2026-09-09-phase-a-accuracy-hotfix` — Phase A 정확성 핫픽스: 판정 윈도우·게이지 발산 J1~J19+J14(見逃し POOR 슬롯·5K/PMS 윈도우·LN 마진 등, 상세는 `reference-divergences.md`)·오디오 마스터 게인 재조정+seqlock 클럭·IR 타임아웃 강등 제거·폰트 캐시 LRU+골든 하네스 강화·`#SWITCH`계열 결함·앱 통합(IR 제출 게이트·assist 플래그·원자적 저장 등). 갈래별 병렬 구현→적대적 리뷰 1라운드→반영.
  - `2026-09-09-phase-d-judge-parity` — Phase D 판정 패리티 완성: `JudgeAlgorithm` 4종·9게이지+GAS·BSS/MSS·CN/HCN 2단계 폴드·LN MODE 전축 통일·24K Mode 신설·어시스트 램프 강등. 적대 리뷰 23건 반영.
  - `2026-09-10-phase-f-integration` — Phase F 갈래 합류: F0~F4 교차 배선(결과 fast/slow 스크래치 분리·모드 라벨·LETTERBOX 매프레임 반영·텍스트에디터 통일·PACEMAKER HUD·re-export·정렬 방향 확인). `stage/play.rs` 800행 초과 분리.
  - `2026-09-09-phase-f-ux` — Phase F 최종 검증(코드 수정 없음, `cargo fmt --all`만 적용): fmt/clippy(`-D warnings`)/`cargo test --workspace`(2232통과·0실패) 전부 green, 금지어·금지주석 0건, 옵션 오버레이·결과 그래프·필터+토스트 3화면 헤드리스 PNG 육안 확인.
- **새 세션 진입점 = 이 PROCESS.md**(CLAUDE.md가 지정). 별도 글로벌 하네스 메모리는 사용 안 함 — SSOT는 docs/.

## 9. 작업 규칙(요약)

각 스텝 검증 후 진행(테스트/렌더 PNG). 모호·범위변경 시 멈추고 질의. 큰 변경 후 적대적 멀티에이전트 리뷰. 커밋은 사용자 요청 시에만, **co-author 미포함**. `target`/`Cargo.lock`/라이브러리 차트 커밋 금지(.gitignore).
