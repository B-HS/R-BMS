# Lua 스킨 완전 호환과 기본 스킨 교체 (2026-10-09 ~ 2026-10-11)

> 최종 갱신 2026-10-11 · 대응 커밋 `36c52a5` ~ `d0af473`(브랜치 `dev`) · 사양 `docs/plan/2026-10-09-lua-skin-compat.md` · 결정 `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`

## 무엇을 했나

| 단계 | 내용 | 끝 커밋 |
| --- | --- | --- |
| 조사(L1) | 외부 스킨 ModernChic, beatoraja 스킨 엔진, R-BMS 현황의 격차 조사. 보고서 16개를 `docs/reference/skin-compat/` 에 저장 | `36c52a5` |
| 웨이브 1A·1B | 구 번들 3세대와 혼합 합성 삭제, Lua 5.2 전환, 타이머 µs, 장면 시계, 창 크기와 해상도 설정, 캡처 하니스 | `6e5bb37` |
| 웨이브 2A·2B | Lua 런타임(격리, 쓰기 오버레이, 예산), 2패스 로더, `SkinHost`, `rbms-cli skin-dump`, 스킨 팩 지정 | `032a435` |
| 웨이브 3A·3B | 공통 그리기 의미론, 텍스처 관리자, 결정 화면 | `1fe0123` |
| 웨이브 4 | 결과와 코스 결과 | `6b95bfd` |
| 웨이브 5 | 선곡 | `721d196` |
| 웨이브 6 | 플레이 SP·DP | `3566189` |
| 웨이브 7A·7B | 비트맵 폰트, mp4 동영상 source(`crates/rbms-video`) | `2aae20a` |
| CI 복구 | 웨이브 1B~7B 동안 확인하지 않은 세 OS 의 테스트 실패 수정 | `366cf32` |
| 웨이브 8A | 선곡 키 표의 NEXT_REPLAY 제거(D8), 기본 스킨 설계서와 골격 | `039ced3` |
| 웨이브 8B | 기본 스킨의 선곡·플레이 5/7/9/10/14키·결과·코스 결과 | `d4cd23f` |
| 웨이브 8C | 기본 스킨 내장·설치·기본 선택, 설정 정리, 제3자 고지, 테스트 재편, 문서 | `d0af473` |

## 끝 상태

- beatoraja 용 풀 Lua 스킨 ModernChic 을 파일 수정 없이 읽어 선곡·결정·플레이(5/7/10/14키)·결과·코스 결과를 그린다. `.luaskin` 10개 중 9개가 로드되고(keyconfig 는 스킨 쪽 결함으로 원본에서도 실패) 스킨의 pcall 이 삼킨 오류는 0 이다.
- 자작 기본 스킨 `assets/skins/rbms-default/`(문서 9개, Lua 27개, 시트 5장 93 KiB)가 바이너리에 내장되고 첫 실행 때 `<설정 폴더>/skin/rbms-default/` 에 설치된다. 팩을 지정하지 않으면 이 스킨으로 전 화면이 그려진다.
- 화면 흐름은 선곡 → 결정 → 플레이(PRELOAD → READY → PLAY → FAILED/FINISHED) → 결과다. 내장 화면은 24키, 키 설정, 스킨 로드 실패의 폴백으로 남아 있다.

## 최종 검증(L6, 2026-10-11)

| 항목 | 결과 |
| --- | --- |
| 로컬 게이트 | fmt, clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 4,878 통과·0 실패·ignored 5, `git diff --check` |
| CI(run `38099911787`, 커밋 `d0af473`) | ubuntu·macos·windows 의 test 와 fmt + clippy 전부 통과(작업당 1~7분) |
| ModernChic 덤프 | 9/10 load ok, pcall 0, 함수 값 실패 0. keyconfig 만 본체 패스 실패(`Decide/lua/require/textproperty.lua:37`) |
| ModernChic 화면 회귀 | 웨이브 8 뒤 다시 찍은 선곡·플레이 7키·결과 캡처가 웨이브 7B 최종 캡처와 픽셀 동일 |
| 기본 스킨 덤프 | 문서 9개 `--strict` 통과, 읽기 전용 사본에서도 같음 |
| 기본 스킨 화면 | 캡처 80장과 게이트 캡처 9종. 메인이 결정·선곡·플레이 7/9/14키·결과·코스 결과·첫 실행 선곡을 직접 확인 |
| ModernChic·beatoraja 폴더 | 변경 없음 |

## 확인하지 못한 것

실제 GPU 창에서의 동작은 자동으로 확인할 수 없다. 사용자가 확인할 항목은 `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 에 있다(마우스 조작, 한글·일본어 제목, 창 크기, 외부 팩 전환, 내장 화면의 밝기 등). beatoraja 실행 캡처가 없으므로 글자 위치의 픽셀 단위 일치와 색감은 보장 범위 밖이다(사용자 결정 D6).

## 사용자 결정이 필요한 채 남은 것

| # | 내용 |
| --- | --- |
| 1 | 레터박스 기본값은 새 설치에만 적용된다. 기존 설치(이전 빌드가 `letterbox: false` 를 적어 둠)도 켜려면 설정 스키마 4 마이그레이션이 필요하다 |
| 2 | `Cargo.lock` 이 추적되지 않아 릴리스 빌드의 의존 버전이 `THIRD-PARTY-NOTICES.md` 와 어긋날 수 있다. 추적하고 `--locked` 로 빌드할지 |
| 3 | 고지 파일은 Apache-2.0 을 선택지로 주는 크레이트를 Apache-2.0 으로 받는다고 적었다 |
| 4 | 내장 화면이 실제 창에서 전보다 어둡게 보인다(표면을 바이트 통과로 바꾼 결과, 사양 E10). 테마 색을 다시 맞출지 |

## 후속 후보(웨이브 9, 별도 지시)

IR 랭킹·라이벌의 스킨 연결, 스킨 설정 화면(type 9), customTimers/customEvents, 24키 문서, bilinear 전용 셰이더, skinpreview, BGA 동영상, 선곡의 종류별 노트 수·밀도·주 BPM(D9), 선곡의 표 이름·레벨, 선곡 위 앱 오버레이와 스킨의 정합(검색 상자, 랭킹 패널 폭), 캡처 하니스의 CJK 글꼴.
