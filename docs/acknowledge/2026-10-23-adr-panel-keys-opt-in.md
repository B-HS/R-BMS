# ADR: 선곡 옵션 패널의 재배정 키는 헤더 `rbms_extensions` 가 있는 스킨에서만

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 5 는 사용자 결정, 세부 M5-1~M5-4 는 메인 결정이고 사용자 확인 2026-10-24(미확인 16 지금대로)) · 결정일 2026-10-23 · 결정자 사용자(결정 5) + 메인(세부) · 커밋 `9e39262`(제목은 마이그레이션 0012 이지만 웨이브 1 의 클라이언트 변경 전체가 들어 있다)
> 관계: [옵션 패널 배치 ADR](2026-10-22-adr-option-panel-layout.md) 7절의 "키 표 재배정은 모든 스킨에 적용된다"와 "외부 스킨의 키 재배정 위험은 남긴다"를 대체한다. 키 배치 자체(어느 칸에 무엇을 두는가)와 [확장 id ADR](2026-10-22-adr-rbms-extension-ids.md) 의 id 대역은 그대로다.

## 배경

- 2026-10-22 에 원본 키 표(`MusicSelectKeyProperty`)의 죽은 칸 열 곳을 SCRATCH AUTO·KEEP GREEN NUMBER·LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM·GAUGE 이전·HI-SPEED FIX 이전에 재배정했다. 그 표는 어느 스킨이 선곡을 그리든 쓰였다.
- 통합 리뷰 `panels-1` 이 짚은 위험: beatoraja 용 외부 스킨(예 ModernChic)은 그 칸들을 JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE·DP OPTION 같은 원본 라벨로 그린다. 그 스킨 위에서 키를 누르면 화면에는 변화가 없는데 OptionChange 소리가 나며 저장 설정이 바뀌었고(다음 판이 SCRATCH AUTO 로 시작해 어시스트 램프가 될 수 있다), 이를 막을 통로가 없었다. 옵션 패널 ADR 7절이 "사용자 확인이 필요하다(미확인)"로 남겼다.
- 7절은 방법 셋을 적었다. (a) 헤더나 맞춤 행에 확장 지원 표시를 두고 로더가 읽어 `KeyLayout` 이 따른다. (b) 번들 기본 팩 경로일 때만 확장 키를 켠다. (c) 실행 중에 확장 id 읽기를 기록한다. 이번 결정은 (a)다.

## 결정

1. **표시 형식.** 스킨 문서 최상위 정수 키 `rbms_extensions`. 1 이상이면 "이 스킨은 R-BMS 확장 id(이벤트 21000~21099, 불린 21100~21199)와 그에 딸린 선곡 패널 키 재배정을 안다"는 뜻이다(`SkinDef::knows_rbms_extensions`, 상수 `RBMS_EXTENSIONS_PANEL_KEYS = 1`). 없거나 0(`NO_RBMS_EXTENSIONS`)이면 모른다. JSON 은 `"rbms_extensions": 1`, Lua 는 헤더 표의 `rbms_extensions = 1` 이고 헤더 패스의 값이 이긴다(이름·작성자와 같은 규칙). beatoraja 는 두 형식 모두 모르는 키를 버리므로 겸용 스킨에 넣어도 안전하다.
2. **표시가 있는 스킨**은 지금 표(`BEAT_TABLE`·`POPN_TABLE`)를 쓴다. 기본 스킨은 `lua/props.lua` 의 `props.rbms_extensions = 1` 을 `lua/select/main.lua` 의 `screen.header()` 에 실어 보낸다. 선곡 문서에만 넣는다(읽는 곳이 선곡뿐이다).
3. **표시가 없는 스킨**은 원본 표(`REFERENCE_BEAT_TABLE`·`REFERENCE_POPN_TABLE`, 원본 `MusicSelectKeyProperty` 에서 `NEXT_REPLAY` 만 뺀 것)로 읽는다. 원본 역할 일곱 변종(`OptionDpUp`·`OptionDpDown`·`Option2Up`·`JudgeArea`·`MarkNote`·`BpmGuide`·`NoMine`)을 되살렸다.
   - 패널 1 건반 4 는 DP OPTION 다음, 건반 6 은 RANDOM 이전(원본 역할, 2026-10-22 이전 동작). DP OPTION 값은 저장되지만 플레이에 적용되지 않는 기존 차이는 그대로다.
   - 패널 2 의 JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE 칸은 죽은 칸이다. 패널이 누름을 가져가고 아무 일도, 소리도 없다(`press_act` 가 `None`).
   - 패널 2·3 의 턴테이블에는 역할이 없다. 패널 1 의 턴테이블은 TARGET 이다.
   - 키 안내(`H`)의 패널 줄에서 `SELECT + TURNTABLE = HIDDEN+ ON/OFF`·`START+SELECT + TURNTABLE = JUDGE ALGORITHM` 두 줄은 재배정 표일 때만 보인다(`KeyLayout::is_extended`).
4. **어느 표를 쓰는지는 선곡 문서로 정한다**(`KeyLayout::of_browser`): 모드의 턴테이블 유무 × 선곡을 그리는 문서의 표시. 선곡 문서가 아직 없으면(읽는 중) 원본 표다. 그동안은 스킨 선곡의 입력이 돌지 않는다. 스킨 없는 내장 선곡의 목록 역할은 두 표가 같다.
5. **확장 id 자체는 모든 스킨에 답한다.** 이벤트·불린 id(켬끔 값, 어시스트 등급, 닿지 않는 키)는 표시와 상관없이 답한다. 표시는 키 재배정만 가른다.

### 메인 결정(설계서 client-small 8절, 사용자 확인 2026-10-24)

| id | 결정 | 이유 |
| --- | --- | --- |
| M5-1 | 최상위 정수 키 `rbms_extensions`(1 이상 = 안다), JSON·Lua 같은 이름 | 확장 키 선례 `color` 처럼 최상위이고, 정수라 뒤에 확장 묶음이 늘면 값을 올릴 수 있다 |
| M5-2 | 표시 없는 스킨의 패널 1 건반 4 는 원본 역할(DP OPTION 다음), 건반 6 은 RANDOM 이전 | 결정 5 "원본 역할". 원본 스킨이 그 칸에 DP OPTION 값을 그리므로 표시 없이 바뀌는 일이 없다 |
| M5-3 | 표시를 넣을 기본 스킨 문서는 `select.luaskin` 만 | 읽는 곳이 선곡뿐이다 |
| M5-4 | 선곡 문서가 아직 없을 때는 원본 표 | 그동안 스킨 선곡 입력이 돌지 않아 체감 차이가 없고, 모르는 쪽이 안전하다 |

## 이유

- 사용자 결정 5(옵션 패널 ADR 7절의 (a)). 스킨이 확장 id 를 안다는 사실을 스킨 자신이 알리게 하면 외부 스킨의 원본 라벨과 키 동작이 어긋나지 않고, 기본 스킨의 IIDX 식 패널 그림은 그대로 맞는다.
- 로더에 이미 `color` 같은 R-BMS 확장 헤더 키 선례가 있고 `SkinDef` 가 알 수 없는 키를 거절하지 않는다. 새 id 를 만들지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| (b) 번들 기본 팩 경로일 때만 확장 키를 켬 | 사용자가 고친 사본, `RBMS_SKIN_PACK`, 테스트가 쓰는 저장소 경로 팩에서 꺼진다 |
| (c) 실행 중에 확장 id 읽기를 기록 | 프레임마다 다시 만드는 `ScreenHost` 와 Options·Select 클러스터에 셀을 통과시켜야 하고 첫 프레임은 알 수 없다 |
| 불린 `rbms_extension_ids: true` / 중첩 표 `rbms = { extensions = 1 }` | 정수 한 키가 단순하고 값을 올릴 수 있다 |
| 패널 1 건반 4 를 죽은 칸으로 | 원본 스킨이 그 칸에 DP OPTION 을 그린다. 결정 5 는 원본 역할을 말했다 |
| 기본 팩 문서 전부에 표시를 넣음 | 읽는 곳이 선곡뿐이라 의미 없는 키가 는다 |
| 설정 항목(OPTION PANEL KEYS)으로 고름 | 설정 분기가 늘면 테스트와 문서가 둘이 된다(2026-10-22 에 이미 기각) |
| 표시 없는 스킨에서 바뀐 옵션을 토스트로 알림 | 2026-10-22 에도 추가하지 않았다. 목표는 표시 없는 스킨에서 키가 설정을 바꾸지 않는 것이라 알림이 필요 없다 |

## 영향과 관련 파일

- 스킨: `crates/rbms-skin/src/model.rs`(`SkinDef.rbms_extensions`, 상수 둘, `knows_rbms_extensions`), `crates/rbms-skin/src/loader/lua_skin.rs`(헤더 값 복사), 기본 스킨 `assets/skins/rbms-default/lua/props.lua`·`lua/select/main.lua`.
- 선곡: `apps/rbms-player/src/stage/select/{keys,panel,guide,skinned}.rs`(`KeyLayout` 네 변종, `of_browser`, `is_extended`, 원본 표 둘, `PLAY_PANEL_PRESSES` 10·`ASSIST_PANEL_PRESSES` 12 는 두 표의 합집합).
- 테스트: `keys.rs`(`the_reference_tables_are_the_references_without_next_replay`, `only_a_skin_that_names_the_extensions_reads_the_reassigned_table`, `each_family_of_tables_carries_only_its_own_panel_roles`), `panel.rs`(`the_reference_roles_of_the_first_panel_step_the_double_option_and_the_random_back`, `the_reference_assists_this_player_has_not_got_ask_for_nothing`), `panel_tests.rs`·`overlay_tests.rs`·`skinned_tests.rs`(픽스처에 표시를 넣음), `crates/rbms-skin/tests/skin_loader.rs`·`skin_luaskin.rs`.
- 남은 위험(확인하지 못한 것 포함):
  - 사용자가 고쳐 둔 기본 팩 `lua/select/main.lua` 사본(설치 관리자가 덮지 않는 파일)에는 표시가 없어 원본 표가 된다. 기본 스킨의 패널 그림은 확장 칸을 그리므로 그림과 키가 어긋난다. 사용자 보고가 있을 때 다룰 테스트 부채 후보다.
  - JSON 스킨이 `rbms_extensions` 에 정수가 아닌 값을 쓰면 문서 전체가 거절된다(설계서의 판단이고 시험하지 않았다). 대상은 R-BMS 용 JSON 스킨 작성자뿐이다. `docs/skin.md` 에 "정수"를 적는 일은 이 ADR 을 쓴 작업의 범위 밖이라 하지 않았다.
  - 외부 팩(ModernChic)으로 키를 누르는 확인은 `RBMS_SKIN_PACK` 이 있을 때만 돌아 CI 에서 하지 않았고, 실제 창에서도 보지 않았다(QA 3.20 Q8·3.17 X1 실기).
- 설계서와 다른 점: 키 표·표시·호출처를 `docs/plan/2026-10-23-client-small.md` 3.3절과 대조한 범위에서는 없다.

## 날짜와 사용자 원문

2026-10-23.

- 결정 회신(원문): "1-11 추천대로". 결정 5 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): panels-1 은 스킨 헤더의 확장 지원 표시가 있을 때만 재배정 키.
- 앞선 질문: 옵션 패널 ADR 7절의 "사용자 확인이 필요하다(미확인): 지금대로 둘지, (a)~(c) 중 하나로 외부 스킨에서는 원본 표를 지킬지."
