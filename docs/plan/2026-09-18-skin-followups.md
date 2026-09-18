# 스킨 후속 기능 전면 구현 사양 (2026-09-18)

상태: 착수 사양(연구 전). 사용자 지시(2026-09-18): "커밋 푸시하고, 요약한 내용의 남은 기능들도 다 구현 시작해서 커밋·푸시·머지". 남은 기능은 `docs/HANDOFF.md` §8 TODO 3(기본 번들 미세 결함)과 TODO 4(사양 §12 후속)이다. 실제 GPU 창 확인(TODO 2)은 터미널 화면 녹화 권한이 필요해 이 작업에 넣지 않는다. 브랜치 `feat/skin-followups`에서 작업하고 단위별 커밋 뒤 `dev`에 머지·푸시한다(`prod`는 Phase R, 키 필요).

`[미확인]` 표시는 연구(L1)에서 레퍼런스 구현 원본과 코드로 확정한 뒤 이 문서를 갱신한다. 레퍼런스 구현과 외부 묶음의 이름은 어디에도 적지 않는다.

## 1. 범위와 완료 조건

| 항목 | 내용 | 완료 조건 |
| --- | --- | --- |
| F1 번들 결함 | (a) 10키 전용 `frame-dp-10k.png` (b) 플레이 NOTES 수치 경계 물림 (c) 선택 화면 판정별 카운트 6행 (d) BGA SIZE OFF 를 `bga` 선언 게이트로 | 캡처 `play-10k`에서 프레임이 필드와 일치, NOTES 수치가 패널 안, 선택 상세에 판정 6행이 최고 기록 값으로 표시, OFF 시 투명 목적지 없이도 네이티브 BGA 사각형이 그려지지 않음 |
| F2 BGM 루프 | 곡 선택 화면 BGM 루프 재생(`select` 스템 또는 `bgm` 스템), 결정 화면 BGM, 화면 이탈 시 정지 | 선택 화면 진입 후 스템 길이를 넘겨도 끊김 없이 이어지고 플레이 진입 시 정지. 단위 테스트로 루프 스케줄 검증 |
| F3 문서 이벤트 | `customTimers`(Lua 타이머), `customEvents`(action/condition/minInterval), 객체 `click`/`act`(내장 이벤트 id + 커스텀 이벤트) | 픽스처 문서에서 커스텀 타이머 값이 dst 에 반영되고, 클릭 사각형이 `act` 이벤트를 발화해 조건·간격을 지키며, 내장 이벤트(폴더 열기/정렬 등)가 기존 `Hot` 경로로 이어진다 |
| F4 화면·입력 | `skinpreview` 객체, `practice` 객체(연습 패널 행), 옵션 패널 마우스 조작 | SKIN 탭이 문서 헤더 `preview` 이미지를 보여주고 문서의 `skinpreview` 객체가 같은 이미지를 그림. 연습 화면이 플레이 문서의 `practice` 객체로 행을 그리고 없으면 네이티브 유지. 옵션 패널 행 클릭·값 클릭이 키 조작과 같은 결과 |
| F5 24키 | 문서 타입 id 확정 `[미확인]`, `play-24k.json5` 실제 문서, 모드 활성(bmson `mode_hint`) 경로 캡처 | 24키 bmson 차트로 헤드리스 캡처 1장, `is_supported_skin_type` 이 24키를 포함 |
| F6 CSV 스킨 | 외부 CSV 형식 스킨 직접 로드(`.csv`·`.lr2skin`) → `SkinDef` 변환, 옵션·파일·오프셋 헤더 노출 | 픽스처 CSV 가 로드돼 이미지·숫자·텍스트·슬라이더·그래프·버튼이 기존 렌더러로 그려지고 SKIN 탭에 나타남 |
| F7 pmchara | 팝픈 캐릭터 객체(`src`·`color`·`type`·`side`) 상태별 애니메이션 `[미확인]` | 픽스처 시트로 상태(대기·굿·배드·피버·승·패) 전환 픽셀 테스트 |
| F8 비디오 BGA | 비디오 파일 BGA 재생(`#BMPxx` 가 동영상일 때) | 순수 Rust 디코더(의존성 결정은 L1 연구 뒤) 로 MPEG-1 클립이 재생·동기화되고 헤드리스 캡처에서 프레임이 바뀜. 지원 밖 컨테이너는 경고 후 정지 이미지 폴백 |

공통 완료 조건: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` 0 실패, 금지 명칭 0건, `git diff --check`, `docs/skin.md`·history·acknowledge 갱신.

## 2. 결정 (착수 시점, L1 뒤 확정)

- E1 새 의존성은 순수 Rust 만 추가한다. 시스템 라이브러리(ffmpeg 등)가 필요한 경로는 구현하지 않고 사용자 결정 항목으로 보고한다(CI 3-OS 유지).
- E2 CSV 스킨은 별도 로더 모듈이 `SkinDef` 로 변환해 기존 렌더러를 그대로 쓴다(렌더러 분기 금지).
- E3 커스텀 이벤트는 `SkinScreen` 이 소유한다(핫스팟과 같은 경로). 내장 이벤트 id 는 레퍼런스 번호를 따르고 rbms 가 없는 동작은 경고 1회.
- E4 BGM 루프는 오디오 엔진의 루프 보이스로 구현한다(앱 재스케줄 금지: 프레임 지연이 끊김을 만든다).
- E5 24키 문서 타입 id 는 레퍼런스 번호를 따른다(현재 표의 16은 `[미확인]`).

## 3. 파일 소유권과 웨이브

| 웨이브 | 작업 | 소유 파일 |
| --- | --- | --- |
| L1 연구 | 레퍼런스 의미(F3·F4·F5·F7·F6 명령 표), 비디오 디코더 선택지, 오디오 루프·BGA 경로 사실 | 스크래치 보고서만 |
| L2-A | F1 | `assets/skins/steel-neon-v3/{tools/generate-assets.py,palette.json,images/frame-dp*.png,play-10k.json5,shared/objects-play.json5,select.json5}`, `crates/rbms-render/src/skin_render/state.rs`(선택 판정 id), `apps/rbms-player/src/stage/select/mod.rs`(판정 값 조립), `apps/rbms-player/src/skin_screen.rs`(BGA 게이트) |
| L2-B | F2 | `crates/rbms-audio/src/**`, `apps/rbms-player/src/syssound.rs`, `apps/rbms-player/src/stage/select/mod.rs`(진입·이탈 훅만), `apps/rbms-player/src/stage/loading.rs` |
| L2-C | F3 | `crates/rbms-skin/src/{model.rs,loader.rs,lua.rs,timer.rs}`, `crates/rbms-render/src/skin_render/{screen.rs,state.rs}`, `apps/rbms-player/src/skin_screen.rs`(이벤트 디스패치), `apps/rbms-player/src/stage/select/mod.rs`(이벤트→Hot) |
| L2-D | F4 | `apps/rbms-player/src/{settings_ui.rs,settings_view.rs,app_options.rs,skin_select.rs}`, `apps/rbms-player/src/stage/{practice.rs,settings.rs}`, `crates/rbms-render/src/skin_render/{object.rs,draw.rs,mod.rs}`(skinpreview·practice 객체) |
| L2-E | F5 | `crates/rbms-skin/src/loader.rs`(타입 표), `assets/skins/steel-neon-v3/play-24k.json5`, `apps/rbms-player/src/assets.rs`(테스트), 24키 캡처 테스트 |
| L3-F | F6 | 신규 `crates/rbms-skin/src/csv/**`, `crates/rbms-skin/src/lib.rs`, `apps/rbms-player/src/skin_select.rs`(확장자), 픽스처 |
| L3-G | F7 | 신규 `crates/rbms-render/src/skin_render/pmchara.rs`, `object.rs`·`draw.rs`·`mod.rs`·`state.rs`(pmchara 상태) |
| L3-H | F8 | 신규 `crates/rbms-video/**`, `apps/rbms-player/src/{assets.rs,app_play.rs}`, `apps/rbms-player/src/stage/play/mod.rs`(프레임 갱신), 픽스처 클립 |
| L4 | 적대 리뷰 3갈래 → 수정 → 게이트 → 문서 → 단위 커밋 → `dev` 머지·푸시 | 전체 |

L2 갈래끼리 같은 파일을 만지는 곳(`select/mod.rs`·`skin_screen.rs`·`state.rs`)은 갈래별로 다른 함수·절만 수정하고 충돌 시 메인이 통합한다. L3 는 L2 통합 뒤 시작한다.

## 4. 검증

- 각 갈래: 최소 단위 테스트 + `cargo test -p <crate>`. 화면이 바뀌는 갈래는 헤드리스 캡처 PNG 를 `docs/quality-assurance/2026-09-18-skin-followups/` 에 남긴다.
- 통합: 공통 완료 조건(§1) + `RBMS_SKIN_CAPTURE_DIR` 캡처 재생성.

## 5. 비목표

- 실제 GPU 창 확인(권한 필요), 시스템 라이브러리 기반 비디오 디코딩, 외부 CSV 스킨 자산 동봉, `prod` 머지.
