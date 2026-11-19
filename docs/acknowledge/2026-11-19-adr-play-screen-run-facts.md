# ADR: 플레이 화면에 출발 폴더, 그 판의 실제 게이지·배치·어시스트, 현재 NPS 를 보인다

> 최종 갱신 2026-11-19 · 대응 커밋 `3c546d3`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 세 줄뿐이고 M6·M7·M17 과 "구현에서 정한 것"은 메인·구현 단위의 결정이며 사용자 확인 전. 실제 앱 창에서 CJK 출발 폴더와 좁은 머리를 확인하지 않음) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M6·M7·M17), 구현 단위 U6(구현에서 정한 것) · 커밋 `3c546d3`
> 관계: 판의 실제 값을 스킨에 주는 길은 [R-BMS 확장 id](2026-10-22-adr-rbms-extension-ids.md)·[확장 id 공간](2026-10-24-adr-rbms-extension-id-spaces.md)의 길이다. 배치 이름은 [LAYOUT PICK](2026-10-29-adr-layout-pick.md)·[배치 플레이](2026-10-24-adr-layout-play.md)가 빌리는 배치를 포함한다. 코스 게이지는 [코스 게이지 모드](2026-10-23-adr-course-gauge-modes.md). 계획 U6, 조사 2·3·11절, 보고 [U6.md](../history/2026-11-19-feature-upgrade-unit-reports/U6.md).

## 배경

- 사용자 요청 요지(2026-11-19): 2 플레이 화면에 출발 폴더 표시, 3 플레이 화면에 선택한 옵션(랜덤·정배치·게이지) 표시, 11 판정 위 줄 오른쪽에 NPS 와 on/off.
- 플레이 화면에는 HI-SPEED·GREEN·COVER·LIFT·AUTOPLAY/REPLAY·배속 표지만 있었다. 게이지 종류·배치 옵션·어시스트 옵션 이름은 없었다. 스킨이 읽을 수 있던 설정 인덱스(`GAUGE_1P` 40, `RANDOM_1P` 42, `RANDOM_2P` 43)는 설정값이지 그 판의 실제 값이 아니다. 빌린 배치(LAYOUT PICK·리플레이 배치)는 설정과 달리 RANDOM + 시드로 달리고, 코스는 CLASS 계열 게이지, GAUGE AUTO SHIFT 는 판 도중 게이지가 바뀐다.
- 선곡 경로 문자열(`STRING_DIRECTORY` 1000)은 선곡 화면 클러스터만 답했다. 선곡 폴더 스택은 앱에 있다.
- 플레이 중 현재 NPS 값은 없었다(선곡의 밀도 값은 있다).

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | 플레이 화면에 출발 폴더를 보인다 |
| D2 | 플레이 화면에 선택한 옵션(랜덤·정배치·게이지)을 보인다 |
| D3 | 판정 위 줄 오른쪽에 NPS 를 보이고 켜고 끌 수 있게 한다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M6 | 플레이 머리 둘째 줄 오른쪽에 출발 폴더를 강조색으로 쓴다. 값은 선곡 경로의 마지막 두 마디(`発狂BMS / ★8`), 루트면 빈 값. 난이도 이름 앞에 두는 안은 좁은 머리(DP)에서 난이도·레벨을 밀어내므로 쓰지 않는다 |
| M7 | 필드 아래(HI-SPEED·GREEN 밑 줄)에 그 판의 게이지·배치·어시스트 옵션을 쓴다. 값은 설정이 아니라 판의 실제 값(빌린 배치, 코스 게이지, GAUGE AUTO SHIFT 반영). 설정 인덱스(40·42·43)로 스킨이 그리는 안은 빌린 배치·코스에서 틀리므로 쓰지 않는다 |
| M17 | 판정 위 줄의 FAST/SLOW 오른쪽에 현재 NPS(직전 1초 동안 판정선을 지난 노트 수)를 보인다. 스킨 옵션 `NPS Display` On(기본)/Off. 기본 Off 는 요청한 기능이 숨으므로 쓰지 않는다 |

### 3) 구현에서 정한 것(구현 단위가 정함, 사용자 미확인)

#### 앱 쪽

| 항목 | 내용 |
| --- | --- |
| 확장 id | 텍스트 21010 `PLAY_ORIGIN`(출발 폴더), 21011 `PLAY_GAUGE`(게이지 이름), 21012 `PLAY_ARRANGE_1P`, 21013 `PLAY_ARRANGE_2P`(사이드별 배치 이름), 21014 `PLAY_ASSIST`(어시스트 요약), 숫자 21085 `PLAY_NPS`. Play 클러스터가 답한다(`skin_host/play.rs`, `skin_host/mod.rs` 의 `ROUTES`). 텍스트 공간의 21010~21014 는 이벤트 공간의 21010~21014(형제 차트 슬롯)와 공간이 달라 겹치지 않는다. 스킨 옵션 916 `NPS_ON`·917 `NPS_OFF`(옵션 이름 `NPS Display`) |
| 숫자 21086 | 계획의 `PLAY_GAUGE_KIND`(게이지 종류)는 만들지 않았다. 플레이 화면의 인덱스 40(`BUTTON_GAUGE_1P`)을 Play 클러스터가 이미 엔진의 실제 게이지로 답하므로 스킨은 그 값으로 이름(`theme.gauge_name`)과 램프 색(`lamp_for_gauge` 와 같은 대응)을 고른다. 텍스트 21011 은 외부 스킨용으로 같은 이름을 준다(`GAUGE_NAMES`, `theme.lua` 와 같은지 시험으로 확인) |
| `RunSetup` | `PlayLive` 에 `setup: RunSetup { origin, arrangements: [&'static str; 2], assists }` 를 더했다. 텍스트는 `Arc<str>` 로 공유해 프레임마다 할당하지 않는다. 그 대가로 `PlayLive` 는 `Copy` 가 아니라 `Clone` 이다. `PlayState` 는 스킨이 그 판을 처음 그릴 때 한 번 정하고(`ensure_setup`), `PlaySample`(PLAY 메뉴·SKIN 메뉴 미리보기)은 매 프레임 정한다(설정을 바꾸면 미리보기가 따라가도록). 새 파일 `stage/play/run_setup.rs` |
| 출발 폴더 | 선곡 폴더 스택(`open_folders`)의 마지막 두 마디(`ORIGIN_DEPTH` = 2)를 `" / "` 로 잇는다. 코스, 리플레이 보기, 루트, 선곡을 한 번도 열지 않고 명령줄로 시작한 판은 빈 값이다. 폴더 스택은 재시도·다음 곡에서 바뀌지 않으므로 값이 유지된다(그 흐름을 직접 확인하는 통합 시험은 없음) |
| 게이지 | 엔진이 지금 선택한 게이지의 이름이다. 그래서 코스의 CLASS 계열 게이지와 GAUGE AUTO SHIFT 가 판 도중 바뀌는 것을 그대로 따른다 |
| 배치 | 연습 판은 그 판의 `option`, 그 밖에는 판이 시작될 때 설정에 들어 있는 값(`config.play.random`)이다. 빌린 배치와 리플레이는 load 때 이미 설정에 들어가 있다. 이름은 `NoteOption::label`(`theme.random_name` 과 같음). DP 는 하나의 배치 옵션이 두 사이드를 각자의 시드로 섞으므로 1P·2P 에 같은 이름을 쓰고 싱글은 2P 가 빈 값이다. 배치가 OFF 여도 `OFF` 라고 쓴다. LaneOption(FLIP 등)은 플레이에 적용되지 않으므로 쓰지 않는다 |
| 어시스트 | SCRATCH AUTO, LN MODE, EX JUDGE, CONSTANT, LEGACY NOTE 순서로 켜진 것만 `" · "` 로 잇는다. 이름은 선곡 옵션 시트의 이름(`LN_MODE_LABELS` 의 CN·HCN 포함). LN MODE 는 LN 이 아니고 **그 모드가 차트의 롱노트를 정할 때만**(`ln_mode_key != SCORE_LN_MODE_FROM_CHART`) 쓴다. 차트가 롱노트 종류를 정한 곡에서는 아무것도 바꾸지 않기 때문이다. `PlaySample` 은 상수 `SAMPLE_LN_MODE_DECIDES = true` 로 설정값을 보인다. CONSTANT·LEGACY NOTE 는 노트 필드가 읽는 곳과 같게 설정에서, SCRATCH AUTO·EX JUDGE·LN MODE 는 세션에서 읽는다 |
| NPS 계산 | `notes_in_the_last_second`(`skin_host/play/shown.rs`): 플레이 타이머가 가리키는 차트 시간 기준으로 직전 1,000 ms(`NOTES_PER_SECOND_WINDOW_US`, 경계는 `현재 - 1초` 초과·`현재` 이하) 동안 판정선에 온 일반 노트와 롱노트 시작을 센다. 지뢰, 보이지 않는 노트, 롱노트 끝은 세지 않는다. 판정선의 차트 시간으로 플레이 타이머 경과 시간을 쓰므로 연습과 재생 속도가 바뀐 판에서도 차트 시간이다. 판정 오프셋만큼 앞서 그려지는 필드 시각은 쓰지 않는다 |
| NPS 표시 | DP 에서는 첫 사이드에만 그린다(차트 전체 값이라 두 번 쓰지 않음). Judge Display 가 Hide 여도 NPS 는 따로 켜고 끈다 |

#### 기본 스킨 쪽

| 항목 | 내용 |
| --- | --- |
| 머리 | `lua/play/info.lua` 의 `head`: 둘째 줄 오른쪽 끝에 출발 폴더를 강조색, SHRINK, 머리 너비의 40%(`origin_share` 0.4) 이내로 쓴다. 출발 폴더가 있을 때만 아티스트 칸을 줄인다. 좁은 머리(DP·9키 넓은 레인)에서 재생 속도 표지가 둘째 줄 오른쪽을 차지할 때는 출발 폴더를 그 왼쪽에서 끝낸다. 14키 넓은 레인처럼 머리가 아주 좁으면 출발 폴더가 있을 때 아티스트 글자가 SHRINK 로 읽기 어려울 만큼 줄어든다(계획대로 "그만큼 줄인다") |
| 옵션 줄 | `foot_options`: 필드 아래(HI-SPEED·GREEN 밑 줄)에 `게이지 · 배치 · 어시스트`. 게이지 이름은 그 게이지가 주는 램프 색으로 쓰고, 이름 뒤 글자의 x 위치는 Inter 18px 로 잰 게이지 이름 폭(+약 3px)의 표로 둔다(`HEAD.name_widths` 와 같은 방식). AUTOPLAY/REPLAY 는 같은 줄 오른쪽으로 옮겼다. DP 는 둘째 사이드 발치에 2P 배치를 쓴다 |
| NPS | `lua/play/field.lua` 의 `nps`: FAST/SLOW(또는 ms) 칸 오른쪽에 숫자와 작은 `NPS` 라벨로 항상 그린다. 판정 타이머에 묶지 않았고 판정 위치 오프셋은 따라간다. 스킨 옵션 `NPS Display`(916 On 기본·917 Off, `layout.OPTION_NAME.NPS`)로 끈다 |
| 번역 | 새 번역 문구는 없다. NPS, 게이지·배치·어시스트 이름, `NPS Display`/On/Off 는 번역하지 않는 게임 용어와 스킨 옵션 이름이다 |

## 이유

- 판의 실제 값(`RunSetup`)을 앱이 정해 스킨에 주면 빌린 배치·코스·GAUGE AUTO SHIFT 가 항상 맞다. 설정 인덱스로 스킨이 알아내는 방식은 이 셋에서 틀린다.
- 출발 폴더는 선곡 폴더 스택에서 문자열 자르기 없이 만들 수 있다(제목 목록의 마지막 두 마디).
- NPS 는 요청대로 "지금 얼마나 빠른가"를 보이는 값이라 직전 1초의 이동 창으로 정했다. 선곡 정보창의 NPS 평균·최고(`DENSITY_AVERAGE` 364·`DENSITY_PEAK` 360, [선곡 정보창 ADR](2026-10-29-adr-select-info-overflow.md))와는 정의가 다르다. 선곡 값은 차트를 1초 칸으로 나눠 롱노트 몸통을 걸친 초마다 세고, 이 값은 이동 창에서 롱노트 시작만 센다.
- 기본 On 은 요청한 기능이 숨지 않게 한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 난이도 이름 앞에 출발 폴더 | 좁은 머리(DP)에서 난이도·레벨을 밀어낸다 |
| 설정 인덱스(40·42·43)로 스킨이 옵션을 그리기 | 빌린 배치·코스에서 틀린다 |
| 숫자 21086 `PLAY_GAUGE_KIND` | 인덱스 40 이 이미 판의 실제 게이지를 답해 필요 없다 |
| NPS 기본 Off | 요청한 기능이 숨는다 |
| NPS 를 판정 이미지 타이머에 묶기 | 판정이 없을 때 NPS 가 사라진다 |
| `PlayLive` 를 `Copy` 로 유지하고 텍스트를 매번 복사 | 프레임마다 할당한다. `Arc<str>` 로 공유하고 `Clone` 으로 바꿨다 |

## 영향과 관련 파일

- 코드: `apps/rbms-player/src/skin_host/{extension.rs, mod.rs, play.rs, play/shown.rs, select/bridge.rs}`(`open_folders` 를 `pub(crate)` 로), `stage/play/{mod.rs, skin_preview.rs, run_setup.rs, run_setup/tests.rs}`, 기본 스킨 `lua/{props.lua, play/field.lua, play/info.lua, play/layout.lua, play/main.lua}`. `crates/rbms-skin/src/property/extension.rs` 는 대역이 이미 열려 있어 고치지 않았다.
- 소유 밖을 고친 것: `stage/capture/play_parts.rs`·`play_parts/judge_sizes.rs`(상수 `NO_NOTES_PER_SECOND` 와 `NPS Display` Off 를 판정·파트 측정 실행에만 건 것. NPS 가 판 내내 필드 위에 그려져 "필드 위에는 판정만 있다" 류 시험 6개를 깼기 때문이다. 모든 `FieldRun` 에 일괄로 걸면 사용자 커스터마이즈가 저장되어 내보낸 팩 시험이 깨져 그렇게 하지 않았다), `stage/capture/play_rows.rs`(`CAPTURED_FOLDERS` 상수와 선곡 경로 열기 한 줄: 캡처에 출발 폴더가 보여야 겹침을 확인할 수 있어서다. 코스 캡처는 그대로 빈 값).
- 시험: `run_setup/tests.rs`(빌린 배치, 연습 배치, OFF 표기, DP, 출발 폴더 루트·1개·3개, 코스·리플레이 빈 값, 코스 EX CLASS 게이지, GAUGE AUTO SHIFT HARD 에서 NORMAL, 어시스트 순서와 LN MODE 조건), `shown/tests.rs`(NPS: 지뢰·숨김·롱노트 끝 제외), `skin-dump --strict --frames 120` 플레이 문서 5개 x 세 언어 exit 0, 플레이 캡처 46장면(7키·7키 넓은 레인·9키·14키·14키 넓은 레인·코스에서 겹침 없음).
- 알려진 위험(테스트 부채 3.176·3.177, 실기 QA 3.29 G8):
  - 캡처 하니스가 내장 글꼴만 읽어 CJK 출발 폴더가 네모로 나온다. 앱은 시스템 글꼴로 대체해 그리지만 실제 앱 화면으로는 확인하지 못했다(`rbms-player` 를 직접 띄우지 않았다).
  - 게이지 이름 폭 표는 PIL 로 잰 값에 여유를 둔 것이다. 렌더러 측정과 몇 px 다를 수 있다.
  - 코스 캡처의 게이지가 NORMAL 로 나오는 것은 캡처가 코스 게이지 없이 세션을 만들기 때문이다. 코스 게이지는 단위 시험으로만 확인했다.
  - 출발 폴더는 스킨이 그 판을 처음 그릴 때의 선곡 경로다. 재시도와 다음 곡의 유지를 직접 확인하는 통합 시험은 없다.
  - `PlayLive` 가 `Copy` 가 아니다. 이 타입을 복사해 쓰는 코드는 `clone` 이 필요하다.
- 문서: [skin.md](../skin.md) 1.4절 확장 id 표와 3.4절 스킨 옵션 표, [play-options.md](../play-options.md) 22절.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "2 플레이 화면에 출발 폴더 표시, 3 플레이 화면에 선택한 옵션(랜덤·정배치·게이지) 표시, 11 판정 위 줄 오른쪽에 NPS on/off". 원문 전문은 이 저장소에 없다.
