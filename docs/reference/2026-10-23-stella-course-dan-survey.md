# 2026-10-23 조사: 추가 요구 1~8 의 현재 코드와 외부 출처
> 최종 갱신 2026-10-23 · 대응 커밋 e2da295
> 출처: workflow wf_45a3a684-82f(Sonnet, 읽기 전용) 보고를 그대로 옮겼다. 파일:줄 근거는 조사 시점(e2da295 와 같은 코드) 기준이다. 사용자 결정은 [PROCESS.md](../PROCESS.md) 의 "Stella·코스·단위" 체크리스트에 있다. 외부 사이트의 내용은 2026-10-23 관찰값이다.

## 0. 요구 원문(사용자, 2026-10-23)
1. 스크래치 오토의 색상과 opacity 도 조절 가능해야 해
2. 플레이 도중에 (노트를 하나 이상 친 상황)에 esc 를 누르면 그냥 바로 대기화면으로 넘어가버리는데 하나라도 쳤고, 서버로 정보를 보내는 상황이면 result 화면을 띄워야해
3. 폴더에서, 예를들어서 발광 3폴더에서 esc 를 누르면 발광 3 폴더에 focus 되어있어야하는데 폴더 목록의 가장 첫번째로 가게되네, 이것도 현재 폴더로 focus 되게해놔
4. 추가적인 난이도표를 넣을거야 https://stellabms.xyz/st/table.html 이건데 stella 난이도표야. 그러면 폴더의 폴더를 들어가는 재귀적 폴더가 필요하겠지
5. stellabms 에는 DL 도 지원하는데 곡 다운로드 폴더를 폴더 지정과함께 곡이 없는경우 해당 곡은 opacity 처리 + opacity 처리된 곡을 플레이하려하면 모달 + 확인으로 곡을 다운로드 받겠냐등의 안내문구를 적어서 다운로드 가능하게 만들어
6. stellabms 는 내서버가 아니라 출전가 명확하므로 stella bms 발광표에서 항상 가져오고. 이건 업데이트가 진행되는 난이도표니 한국시간 매일 새벽 0시에 업데이트 하도록 cron 도 vercel 에 workflow 나 다른것들을 이용해서 최신화 하게 만들어
7. 현재 가지고있는 발광 전곡들도 내 서버에 올리면 좋을듯한데 .. 올려서 stella 의 다운로드 방식을 갖추면 좋을듯해. cloudflare 에 올리면 되니까 검토해보고
8. 코스모드를 구현해야해. 발광단위인정 등을 구현하고싶거든. https://www.bms-ir.org/new/dan?table=genocide2018 여기에 발광 제노사이드 표가 있는데 이것들로 들어가서 정확하게 모든 단위를 가져오고 서버에 등록해놔, 이것도 출처등록을 해두면 좋을거고 해당 단위를 획득하면 내정보에 단위를 넣도록하고, 다른 코스들도 생성하게 만들고. 마이페이지에서 내가 클리어한 코스를 설정가능하도록해. 게이지도 정확하게 구현하고 (iidx모드, 비토라자모드, lr2모드 등이고 아마 비토라자랑 lr2의 코스 게이지가 비슷할텐데 lr2를 기준으로 게이지를 넣으면될거야, 이또한 설정에서 변경가능하게 만들어놔야하고.)

## 1. 문서-코드 대조(2026-10-22 ADR 5건)

### ext-keys

일치 확인 17건. 불일치:
- [line-number-only] docs/acknowledge/2026-10-22-adr-option-panel-layout.md 1절 (제거된 SelectKey 변종 아래 두 번째 항목): 문서 "`stage/select/events.rs:83-85` `SettingEvent::Random` 문서" / 코드 Random 변종의 문서 주석은 84-85행이고 변종 선언은 86행이다. 83행은 Gauge 변종이다 (apps/rbms-player/src/stage/select/events.rs:84-86)

메모: 내용 불일치는 찾지 못했고 줄 번호 하나만 어긋났다. 위 대조는 읽기 전용으로만 했고 빌드, 테스트, 플레이어 실행은 하지 않았다. 따라서 테스트 이름의 존재와 단언 내용은 소스를 읽어서 확인했고 실제 통과 여부는 확인 못함. 확인 못함: docs/reference/option-panels.md 및 기타 참조 문서, 기본 스킨 Lua의 패널 그림, 실제 창에서의 키 동작. 참고로 이 ADR이 말하는 `forget_turntable_changes`는 패널이 닫힐 때(Closed로 전환)도 호출되는데, 문서는 '열리거나 다른 패널로 바뀔 때'라고만 적었다. 동작상 문제는 아니라 불일치로 분류하지 않았다. an_option_panel_is_called_up은 문서에서 테스트처럼 읽힐 수 있으나 헬퍼 함수다. 줄 번호 근거(track.rs, main_state.rs, names.rs, guide.rs, side.lua)는 모두 일치했다. 또 확장 id ADR 2절 소리가 OptionChange이고 패널을 닫을 때 저장한다는 설명 중 저장 시점(settle_option_changes, events.rs:410 부근)은 함수 시그니처만 확인했고 전체 동작은 확인 못함.

### grade

일치 확인 14건. 불일치:
- [wording] docs/acknowledge/2026-10-22-adr-assist-lamp-grade.md 4절 첫 항목(비교): 문서 "merge_best 는 best.lamp() < log.lamp() 일 때 clear·assist_grade·option·seed·gauge·assist 를 그 판의 값으로 바꾼다" / 코드 조건은 lamp_counts && best.lamp() < log.lamp() 이고, lamp_counts = updates_best || !is_stale_rule_version(log.rule_version) 이다. 즉 최고 기록을 못 세우는 판(updates_best=false)이 규칙 세대 2 미만이면 램프·등급이 더 높아도 갱신하지 않는다. 필드 목록과 대입은 문서와 같다. 문서는 이 보조 조건을 적지 않았다 (crates/rbms-store/src/scoredb.rs:654-661)
- [wording] docs/HANDOFF.md 6절 14번 및 ADR 6절 마지막 항목: 문서 "코스 제출과 코스 DB 에 게이지가 없다" / 코드 웹 course_score 에 gauge_value(float, 끝난 게이지 수치) 열과 코스 DTO 의 gauge_value 가 있다. 없는 것은 게이지 종류(id)이므로 등급을 파생할 수 없다는 결론은 맞다. 'DB 에 게이지가 없다'는 표현만 부정확하다 (web/src/server/db/schema.ts:290, web/src/server/dto/course.ts:54)

메모: 읽기 전용으로 대조했고 파일 변경, 빌드, 테스트 실행은 하지 않았다. 동작이 달라지는(behavior) 불일치는 발견하지 못했고, 줄 번호만 틀린 항목(line-number-only)도 없었다. 위 두 건은 모두 표현 수준이다. 확인 못한 것: (1) refold 의 동률 규칙 '먼저 친 판'은 코드상 scorelog.id 오름차순이다. 가져오기·동기화로 id 순서가 친 순서와 다를 수 있는지는 확인하지 않았다(merge_best 는 엄격한 > 비교라 먼저 처리된 판이 남는다). (2) web/tests 의 assist-grade 테스트 3건과 format.rs 테스트 본문은 실행하지 않았고, 표가 같은 것은 소스 대조로만 확인했다. (3) ADR 5절 중 이번 범위 밖인 trace_backfill, record_sync, 결과 화면 쪽 주장은 대조하지 않았다. (4) ADR 에 적힌 운영 적용 run id 와 399행 같은 수치(7절)는 외부 상태라 확인하지 않았다."

### skin-lua

일치 확인 16건. 불일치:
- [wording] docs/acknowledge/2026-10-22-adr-skin-colour-rows.md 2절 두 번째 항목: 문서 "노트는 순백 시트의 그림을 고르고 note.tint 의 그 칸에 id 를 넣는다. 아무 레인도 custom 이 아니면 note.tint 를 만들지 않는다." / 코드 note.tint 는 어떤 레인에든 note_tint(노트 행)가 있을 때만 만들어진다. 노트 행은 저장되지 않고 LN 행만 custom 이면 lane.long_tint 만 세팅되어 note.tint 는 만들어지지 않고 long_tint 만 만들어진다(순백 그림은 NOTE_IMAGES 의 tint 키에 따라 LN 계열만 고른다). 문서의 '아무 레인도 custom 이 아니면' 은 노트 행 기준으로 읽어야 정확하고, LN 행만 custom 인 경우를 말하지 않는다. (assets/skins/rbms-default/lua/play/field.lua:139-166 (tinted 는 148, long_apart 는 149, tint 표 생성 154-159, long_tint 표 생성 160-165), colors.lua:150-154)
- [wording] docs/acknowledge/2026-10-22-adr-option-panel-layout.md 6절 두 번째 항목: 문서 "SCRATCH AUTO 아래 `ON: THE LAMP IS ASSIST GRADE` (문서 문구는 'ON: THE LAMP IS AN ASSIST GRADE' 한 줄)" / 코드 실제 렌더는 'ON: THE LAMP IS AN' 과 'ASSIST GRADE' 두 줄로 나뉘어 그려진다(열이 좁아 lines 표 두 항목). (assets/skins/rbms-default/lua/select/sheet.lua:373)

메모: 읽기 전용으로 대조했고 파일을 만들거나 고치지 않았으며 빌드·실행도 하지 않았습니다. 행 수는 lua 인터프리터가 없어 keys_of 의 표를 손으로 센 값입니다. 5·7·10·14키는 모두 scratch=true 이고 17행, 9키는 scratch=false 라 22행입니다. 행 목록은 모드의 scratch 여부에만 달려 있어 5·10·14키가 7키와 같은 17행인 것은 코드로 확인했습니다. 행 목록, id, def 값, 9키 라벨, 판정선 회색 102, 알파 순서의 코드 줄은 모두 일치했고 줄 번호 불일치는 없었습니다. 확인하지 못한 것은 다섯 가지입니다. (1) 연결선이 서로 엇갈리지 않는다는 주장은 좌표를 계산해 보지 않았고 캡처로도 보지 않았습니다. (2) 호스트가 21140~21148 불린을 실제로 답하는지는 Rust 쪽이라 이번 범위 밖입니다. (3) 첫 번째 mismatch 는 문서 문장이 LN 행만 custom 인 경우를 직접 말하지 않는 문구 차이일 뿐 동작 결함은 아닙니다. (4) HANDOFF 6절 13 세 번째 항목의 '색 알파는 불투명도가 남긴 알파의 곱' 은 엔진 쪽(dst.rs:630-633)만 보았고 노트 경로(rbms-render)는 열지 않았습니다. (5) 기본 스킨 Lua 는 현재 working tree 기준입니다. git status 상 수정된 파일은 docs/HANDOFF.md·docs/PROCESS.md·docs/ci-release.md 뿐이라 대조한 Lua 파일은 HEAD 와 같습니다.

### font

일치 확인 24건. 불일치:
- [wording] docs/acknowledge/2026-10-22-adr-ui-text-device-pixels.md 2) 굵기를 고르는 API 둘째 항목: 문서 "굵기를 요청하지 않은 줄은 face 자기 굵기 키를 쓴다" / 코드 굵기를 요청하지 않으면(None) layout_key 는 face 의 실제 굵기가 아니라 고정 센티널 OWN_WEIGHT_KEY = 0 을 쓴다. 따라서 plain 호출(키 0)과 TextStyle::regular(키 400)는 같은 화소를 내지만 레이아웃 캐시 항목은 따로 생긴다(테스트 a_heavier_weight... 가 cache_stats 로 가중치당 1개를 확인) (crates/rbms-render/src/font.rs:153-157 (layout_key), font.rs:~149-151 (OWN_WEIGHT_KEY=0))
- [wording] docs/acknowledge/2026-10-22-adr-ui-text-device-pixels.md 1) 결정, device_text_scale 판정 설명: 문서 "(1,1) 이면 None, 두 축 차이가 반올림 범위 안이면 Some(min(sx,sy)), 아니면 None (판정식만 기술)" / 코드 판정식 외에 sx, sy 가 유한(is_finite)하고 0 초과여야 Some 이 되는 가드가 추가로 있으나 문서에 없다. 동작 불일치는 아님(비정상 입력은 None) (crates/rbms-render/src/font.rs:185-186)

메모: 읽기 전용으로 대조했고 파일 수정, git 상태 변경, 빌드/테스트/실행은 하지 않았다. 따라서 '1x 헤드리스 출력이 이전과 바이트 동일', '골든 상수가 그대로임', 사각형 수·CPU 시간(22,275개, 45~72us)·RSS(14.6→139.2MB)·이미지 279→271,503 같은 측정값, 테스트가 통과한다는 사실, Retina·Windows 실기 결과는 확인 못함. 동작을 바꾸는 불일치는 없었고, 줄 번호 인용은 font.rs:175 하나뿐이며 정확하다. 문서의 유일한 줄 번호 인용(font.rs:175)은 현재 코드와 일치한다. 조사 문서(font-rendering.md)와 docs/architecture.md, docs/crates.md 의 글자 경로 서술이 구버전이라는 ADR 81행의 자기 고백은 이번 범위 밖이라 열어 보지 않았다(확인 못함). HANDOFF.md 3.0절의 글꼴 행 외 줄(14, 89, 130행)은 ADR 과 같은 내용만 훑었고 상충 사항은 없었다."

## 2. 조사 보고

### client-1-3

#### 1. SCRATCH AUTO 초록 노트의 색과 불투명도 조절

**현재 상태**

1) 초록은 Rust 상수다. 이미지나 Lua 가 아니다. crates/rbms-render/src/skin_render/notes.rs:141 AUTO_LANE_NOTE = #30E050 (a=255), :144 AUTO_LANE_LONG_BODY = 같은 초록 a=0xA0, :147 AUTO_NOTE_IMAGE = NoteImage::Solid(AUTO_LANE_NOTE), :150-156 AUTO_LONG_IMAGES (머리·끝은 노트색, 몸통은 0xA0). 기본 스킨 lua/play 에서 'auto' 는 info.lua 의 AUTOPLAY/TIMING AUTO 뿐이고, 30E050 는 Rust·문서·테스트에만 있다(grep). 즉 image/play.png, colors.lua, field.lua 는 초록과 무관하다.
2) 그리는 길: 앱이 LaneNotes.auto_lanes(notes.rs:308)에 session.auto_lanes() 를 채운다(stage/play/mod.rs:1194, :1706, stage/play/skin_preview.rs:785, stage/capture/panels.rs:148, ghost.rs:246). 판정은 plays_itself(notes.rs:341). 일반 노트는 notes.rs:1028-1031 에서 put(r, &AUTO_NOTE_IMAGE, at, alpha, None), 롱노트는 :1099 에서 (&AUTO_LONG_IMAGES, None). 두 곳 모두 tint 가 None 이라 색 행 틴트(NoteLane.tint/long_tint)는 AUTO 에 닿지 않는다(틴트는 :1099 의 else 가지에서만 쓰임). 스킨 없는 내장 필드도 같은 상수(crates/rbms-render/src/playfield.rs:105, :127).
3) 불투명도: put()(notes.rs:943-974)이 alpha = alpha x self.opacity x 255 (:951)로 Solid 도 그리고(:970 lane_tinted), self.opacity 는 note 객체 dst 알파 + 오프셋(place.tint.a, :1264)이다. 그래서 기본 스킨의 Note Opacity 행(assets/skins/rbms-default/lua/play/layout.lua:81)이 AUTO 노트에도 이미 곱해진다. AUTO 만 따로 조절하는 값은 없다.
4) 외부 스킨: 스킨이 고른 노트 그림이 차지할 사각형(레인 폭, 높이는 스프라이트 셀 높이 notes.rs:669-674)에 엔진이 단색 사각형을 칠한다. 그림 모양은 사라지고 스킨의 Lua·색 행·tint 는 영향이 없다(docs/skin.md:703, docs/play-options.md:505 의 10.5절, QA docs/quality-assurance/2026-10-11-default-skin-manual-check.md 3.17 X1 ModernChic 확인).
5) 색 행·오프셋 체계: 색 행은 문서가 헤더 color 로 선언하고(ColorDef, rbms-skin model.rs:333), 저장은 SkinCustomisation.colors: BTreeMap<i32, SkinColor>(crates/rbms-config/src/schema.rs:323, settings.ron skin.custom[문서 경로].colors), 읽기는 OffsetSource::color(id). 기본 스킨은 Scratch Note id 12, Scratch LN id 42 등을 선언하나(colors.lua) 노트 tint 로만 흐른다. 엔진이 모든 play 스킨에 자동으로 붙이는 것은 오프셋 4종뿐이고(crates/rbms-skin/src/loader.rs:296-301 AUTOMATIC_OFFSETS) 자동 색 행은 지금 없다. SCRATCH AUTO 설정은 config.play.scratch_auto(schema.rs:580, settings.rs:1049/1230, play_menu/rows.rs:402)이며 PLAY 메뉴 CHART 묶음에 있다(docs/play-options.md 7.2절).
6) ADR 관계: docs/acknowledge/2026-10-22-adr-skin-colour-rows.md:42 가 'SCRATCH AUTO 초록은 사용자 색보다 우선한다(기존 동작 유지)'라고 못박았다. 이 ADR 은 '색 행 틴트가 AUTO 노트를 칠하지 않는다'(의미색)는 결정이고, 새 요구('AUTO 색을 사용자가 정한다')는 그 중 '고정 초록' 부분만 바꾼다. 상태가 '유효(사용자 미확인)'이므로 갱신 대상이다.

**관련 파일**: /Users/hyunseokbyun/development/R-BMS/crates/rbms-render/src/skin_render/notes.rs, /Users/hyunseokbyun/development/R-BMS/crates/rbms-render/src/playfield.rs, /Users/hyunseokbyun/development/R-BMS/crates/rbms-skin/src/loader.rs, /Users/hyunseokbyun/development/R-BMS/crates/rbms-config/src/schema.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play/mod.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play/skin_preview.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play_menu/rows.rs, /Users/hyunseokbyun/development/R-BMS/assets/skins/rbms-default/lua/play/colors.lua, /Users/hyunseokbyun/development/R-BMS/assets/skins/rbms-default/lua/play/layout.lua, /Users/hyunseokbyun/development/R-BMS/docs/acknowledge/2026-10-22-adr-skin-colour-rows.md, /Users/hyunseokbyun/development/R-BMS/docs/skin.md, /Users/hyunseokbyun/development/R-BMS/docs/play-options.md

**차이와 위험**

1) 기존 틴트(곱셈)로는 못 한다. tinted 는 채널 곱셈이라 초록(30,E0,50)에 사용자 색을 곱하면 어두워지고 빨강(FF0000)은 (30,00,00)이 된다. 색을 바꾸려면 Solid 의 색 자체를 사용자 값으로 대체해야 하는데 AUTO_NOTE_IMAGE·AUTO_LONG_IMAGES 가 const(notes.rs:147-156)라 프레임마다 값을 받는 형태로 바뀌어야 한다. put()·NoteImage::Solid(Color)는 이미 임의 색을 받으므로 그리기 쪽 변경은 작다.
2) 저장 위치가 설계의 핵심이다. (A) 전역 play 설정: AUTO 는 엔진이 그려 스킨과 무관하다는 현 설계 취지에 맞고 외부 스킨에도 똑같이 적용된다. (B) 문서가 선언하는 색 행: 기본 스킨에서만 메뉴에 나오고 외부 스킨에는 행이 없어 외부 스킨 AUTO 는 조절 불가. (C) 엔진이 모든 play 문서에 자동으로 붙이는 색 행: AUTOMATIC_OFFSETS 선례는 있으나 색 행 자동 부착은 신규 메커니즘이고 값이 문서 경로별로 저장돼 스킨을 바꾸면 값이 따로 논다.
3) 기존 색 행의 알파 규칙은 '색의 알파 = 그 부위 불투명도의 몇 분의 몇'(ADR 1절, docs/play-options.md 7.4절 표)이므로 RGBA 한 값으로 색과 불투명도를 함께 표현할 수 있다. 별도 불투명도 행을 따로 두면 Note Opacity(이미 곱해짐)와 3중 곱이 된다.
4) 롱노트 몸통 알파 0xA0(약 63%)은 고정 비율이다. 사용자 알파와 합성하는 규칙(곱할지, 몸통도 사용자 값으로 통일할지)을 정해야 한다.
5) 미리보기: PLAY 메뉴 미리보기는 SCRATCH AUTO 를 그리지 않는다(docs/play-options.md:346). 행을 만들어도 그 자리에서 색이 보이지 않아 실기 확인은 실제 플레이 또는 미리보기 확장이 필요하다.
6) 혼동 위험: AUTO 중에는 턴테이블을 사람이 치지 않고 AUTO 노트는 Scratch Note(id 12/42) 틴트를 받지 않으므로, 사용자가 Scratch Note 색을 바꿔도 AUTO 켠 판에서는 변화가 없다. 도움말·문서 문구가 필요하다.
7) 표시 선호이므로 리플레이·trace·서버 본문에 싣지 않는 쪽이 자연스럽다(play_trace.rs:165 의 auto_lanes 는 판정용 레인 표시이고 색이 아니다. 이 판단은 코드 확인 아니라 추론).
8) 영향 범위: playfield.rs:331 테스트가 AUTO_LANE_NOTE 화소를 단언한다. crates/rbms-render/tests/golden_play.rs:73 은 auto_lanes: &[] 라 골든은 영향이 없는 것으로 읽었다(확인한 범위). 저장값이 없으면 현재 화소와 같게 둘 수 있다. 문서 갱신 대상: ADR colour-rows 4절, docs/skin.md:703, docs/play-options.md:505, docs/reference/skin-color-customization.md:290, QA 3.17 X1.
9) 실제 GPU 창 확인은 읽기 전용 조사라 하지 못했다(확인 못함).

**결정 지점(조사 당시)**

- 저장 위치: A 전역 play 설정(추천: 스킨 무관, 외부 스킨 포함, 현 설계 취지) / B 기본 스킨 색 행만 / C 엔진 자동 색 행
- 색과 불투명도 표현: RGBA 한 값(추천: 알파 = 불투명도, 기존 색 행 규칙과 동일) / 색과 별도 불투명도 행
- 롱노트 몸통: 사용자 알파에 현재 비율 0xA0/255 를 곱함(추천: 머리와 몸통 구분 유지) / 머리와 같은 값 / 몸통 따로 조절
- 기본값과 호환: 저장값이 없으면 지금과 화소 동일 #30E050 알파 255(추천) / 새 기본값
- 조절 UI 위치: PLAY 메뉴 CHART 의 SCRATCH AUTO 행 옆 + 헥스 입력(추천) / 선곡 옵션 패널·SETTINGS INPUT 에도 노출
- Scratch Note 색 행(id 12/42)이 AUTO 노트에도 적용되게 할지(추천: 아니오, 전용 값만)

#### 2. 노트를 하나 이상 친 판의 Esc 후 결과 화면

**현재 상태**

1) 사용자 보고는 코드와 문서의 의도된 동작과 일치한다. Esc 는 PlayState::escape_key(apps/rbms-player/src/stage/play/mod.rs:652)에서 ESCAPE 설정 Immediate/Hold/Double 모두 leave_run(:709)으로 간다. 스킨 장면 경로는 scene_escape(:1460): 닫히는 중이면 enter_result(:1464), 남은 노트 없으면 stop_finished 로 닫힘 진행, 그 외는 escape_key. leave_run 에는 이 외에 :1430, :1554 의 Esc 길게 누르기 경과 호출뿐이다(다른 진입 없음).
2) leave_run 순서(:709-728): PlayStop 소리, save_settings, 연습이면 Back, has_ended()(:731 = all_notes_resolved 또는 장면이 closing)이면 enter_result, 코스 중이면 app_result::leave_course(app_result.rs:595), 단일 곡이면 keeps_a_left_run && !has_judged_nothing 일 때 record_left_run(app_result.rs:489) 뒤 ctx.shared.leave_play()(app_play.rs:779-787 -> Transition::Back = 선곡). 따라서 노트를 쳤고 남은 노트가 있으면 결과 화면 없이 곧장 선곡으로 간다. 코드 주석이 '결과 화면은 보이지 않는다: Escape stays the key that leaves at once'(app_result.rs:480-488 부근)라고 의도를 적었다.
3) 저장·전송은 이미 된다. record_left_run 은 session.summary().left_unfinished()(crates/rbms-play/src/session.rs:577: 남은 노트 = 늦은 POOR, 램프 FAILED)로 record_run(app_result.rs:219)을 돌려 기록·리플레이·제출 본문을 남기고, send_without_a_screen(app_result.rs:636)이 프로필 서버로 보내고 start_record_sync(Logs([id]))로 주 서버 기록 동기화를 시작한다(로그인한 경우). 즉 서버로 가는 부분은 동작하고 결과 화면만 없다. 코스 도중은 leave_course 가 같은 길로 스테이지를 FAILED 곡 기록으로 남기고 FAILED 코스 기록을 보낸 뒤 end_course(lib.rs:310 -> Back).
4) 문서상 근거: docs/play-options.md 10.2절 '도중 포기(Esc)' 행(:480 부근)은 '결과 화면을 거치지 않고 곧장 선곡으로 간다(Esc 는 지금도 바로 나가는 키다)'. docs/acknowledge/reference-divergences.md:236(결과 화면)·:239(코스 도중 Esc)가 이를 원본과 다른 점으로 적었고, ADR 2026-10-15-adr-escape-keeps-every-run.md 기각 표에 '코스 Esc 때 코스 결과 화면을 보인다 - Esc 는 바로 나가는 키라는 기존 결정과 어긋난다', 2026-10-14-adr-abandoned-run-saved.md 기각 표에 '결과 화면을 거친다 - 빨리 나가는 키의 체감이 바뀐다'가 있다. 읽은 범위에서 사용자 원문이 '결과 화면을 생략하라'고 지시한 기록은 찾지 못했다(사용자 결정은 저장·전송이다). 전체 대화 기록은 확인 못함.
5) 결과 진입 경로 enter_result(app_result.rs:70-160): play.summary()(left_unfinished 아님)와 record_run 으로 기록하고, 서버 전송은 결과 화면용 spawn_profile_submits/spawn_score_submit 또는 ir_status::Sending, 코스면 fold_course_stage 뒤 다음 스테이지/finish_course, 단일 곡은 Stage::Result(ResultState …). 결과 화면 데이터는 ResultRun::of(stage/result.rs:669)가 shared.run_records.last()(keep_run_record)의 PlayRecord 에서 ResultInput::from_record 로 읽는다.
6) 원본(beatoraja): Esc 는 ControlInputProcessor.java:205-206 에서 player.stopPlay(). BMSPlayer.java:938-970 stopPlay 는 남은 노트가 있으면 state = STATE_FAILED, TIMER_FAILED 를 켠다. STATE_FAILED 분기(:693-742)에서 skin.getClose() 경과 뒤(:711) resource.setScoreData(createScoreData())(:717), 점수 데이터가 null 이 아니면 RESULT(:737-738), null 이면 선곡(:739-741). createScoreData(:858-861)는 코스가 아니고 PG·GR·GD·BD(빠름+늦음) 합이 0 이면 null 이다. 즉 원본도 '하나라도 쳤으면 결과 화면, 하나도 안 쳤으면 선곡'이고 R-BMS 의 has_judged_nothing 기준과 같다. 코스는 null 이 되지 않아 항상 RESULT 로 가고, 실패 때 게이지 로그 꼬리에 0 을 채우므로(:722-727) MusicResult.java:179-199 에서 게이지 마지막 값이 0 이하라 COURSERESULT(불합격)로 간다. 원본은 START+SELECT 1초 누름도 stopPlay(ControlInputProcessor.java:196-201)지만 R-BMS 에는 그 동작이 없다(docs/play-options.md 5.2절).

**관련 파일**: /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play/mod.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play/scene.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/app_result.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/app_play.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/result.rs, /Users/hyunseokbyun/development/R-BMS/crates/rbms-play/src/session.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/app_result/record_tests.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/play/tests.rs, /Users/hyunseokbyun/development/R-BMS/docs/play-options.md, /Users/hyunseokbyun/development/R-BMS/docs/acknowledge/2026-10-15-adr-escape-keeps-every-run.md, /Users/hyunseokbyun/development/R-BMS/docs/acknowledge/2026-10-14-adr-abandoned-run-saved.md, /Users/hyunseokbyun/development/R-BMS/docs/acknowledge/reference-divergences.md, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/BMSPlayer.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/ControlInputProcessor.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/result/MusicResult.java

**차이와 위험**

1) 요구는 의도된 차이를 되돌리는 것이다. 영향 문서: play-options.md 10.2절 도중 포기 행과 코스 행, reference-divergences.md:236·:239, 두 ADR 의 기각 표, docs/HANDOFF.md:74. 뒤집히는 테스트: app_result/record_tests.rs:936(leaving_mid_song_keeps_a_failed_run_with_the_rest_as_poor 가 Result·Stay 가 아님을 단언)과 :1184(leaving_mid_course_keeps_the_stage_and_a_failed_course 가 Result·CourseResult 가 아님을 단언). stage/play/tests.rs:88 escape_abandons_a_run_on_the_first_press_by_default 는 leaves_the_chart 로 단언하는데 이 헬퍼가 Result 를 포함하는지는 읽지 않아 확인 못함.
2) 수치 불일치 위험: 저장되는 값은 summary().left_unfinished()(남은 노트가 POOR)인데 enter_result 는 summary()(미조정)를 쓰고, 결과 화면의 판정 수·그래프는 PlayRecord(record_left_run 도 state.session.record() 미조정을 넘김)에서 ResultInput::from_record 로 읽는다. 결과 화면이 저장·전송된 값과 같은 수를 보이려면 조정된 summary/record 를 넘겨야 한다. from_record 가 어떤 필드를 읽는지 전부는 읽지 않아 불일치 범위는 확인 못함.
3) 이중 전송 위험: enter_result 는 결과 화면 전송(spawn_score_submit 등), record_left_run 은 기록 동기화로 보낸다. 두 길을 모두 타면 같은 판을 두 번 보낼 수 있으므로 한 길만 타야 한다. 코스는 leave_course(FAILED 코스 직접 저장·전송 후 end_course)와 enter_result 의 fold_course_stage/finish_course 가 겹치므로 어느 쪽이 코스 기록을 남길지 정해야 한다.
4) 장면 연출: 원본은 FAILED 닫힘 연출(close ms) 뒤에 결과로 간다. R-BMS PlayScene 에는 남은 노트가 있을 때 Esc 로 Failed 로 닫는 길이 없고(scene.rs:209 stop_finished 는 남은 노트 없을 때만), 게이지 사망 때만 PlayPhase::Failed 로 간다(scene.rs:~270 play()). 즉시 결과로 가면 소리·타이머 정리(SceneStep::Failed 가지의 stop_chart_sounds, stage/play/mod.rs:1415-1418)를 별도로 해야 한다.
5) 코스 중 Esc: 원본은 코스 결과(불합격)까지 간다. R-BMS 는 현재 leave_course 가 end_course 로 코스를 끝낸다. 코스 결과 화면 진입은 finish_course(app_result.rs)가 담당하므로 FAILED 코스를 그 길로 흘릴 수 있는지는 확인 못함.
6) 노트 0개 판(has_judged_nothing)은 사용자 지시(2026-10-19)로 아무것도 남기지 않고 선곡이며 원본의 null 점수와 같으므로 그대로 둔다. 앱 종료 길(keep_run_cut_short_by_quitting)은 화면이 없어 영향 없다.
7) 사용자 문장 '대기화면'은 선곡 화면을 뜻한다고 가정했다(확인 못함). '서버로 정보를 보내는 상황이면'의 해석이 갈린다: 로컬 저장은 로그인과 무관하게 되고 서버 전송 가능 여부는 AppShared::sends_to_primary_ir_server(app_ranking.rs:107)로 가려진다.

**결정 지점(조사 당시)**

- 결과 화면 조건: 노트 1개 이상 친 사람이 친 판 전부(추천: 이미 로컬 저장되므로 로그인 무관) / 서버 전송 가능할 때만(sends_to_primary_ir_server), 비로그인은 지금처럼 곧장 선곡
- 코스 도중 Esc: 그 스테이지 결과 후 FAILED 코스 결과까지(추천: 원본과 같음) / 지금처럼 곧장 선곡
- 진입 연출: 즉시 결과(추천: 변경 최소) / 원본식 FAILED 닫힘 연출 뒤 결과
- 결과 화면의 판정 수: 저장값과 같게 남은 노트를 POOR 로 포함(추천: 2026-10-14 사용자 결정과 일치) / 원본식으로 미포함
- 결과 화면에서 다시 하기(R, N) 노출 여부: 노출(원본은 START/SELECT 재시작 허용) / 숨김
- ADR 처리: 2026-10-15 escape-keeps-every-run 과 2026-10-14 abandoned-run-saved 의 기각 항목 '결과 화면 생략'을 새 ADR 로 대체

#### 3. 폴더에서 Esc 로 나올 때 방금 있던 폴더에 커서 복원

**현재 상태**

1) 구조: 선곡의 폴더 위치는 스택이 아니라 고정 깊이 enum 이다. apps/rbms-player/src/lib.rs:458-466 SelectView { Root, AllSongs, TableLevels(usize), TableLevel(usize, usize), UserFolders, UserFolder(usize) }. 부모 정보도 커서 기억도 없다. 목록은 AppShared.select_items 와 sel 이며 rebuild_select_items_with(stage/select/list.rs:276)가 select_view 로 만든다: Root = ALL SONGS, 레벨이 있는 표마다 표 폴더, MY FOLDERS(:279-); TableLevels(ti) = 레벨마다 폴더 'LV {level} (n)' 로 target TableLevel(ti, li)(:297-), 목록 인덱스 == li 이며 필터로 폴더가 숨겨지지 않는다.
2) 진입: enter_row(stage/select/mod.rs:421)가 Folder 항목에서 select_view = target; sel = 0; rebuild.
3) 이탈의 모든 길이 한 곳이다. Esc·왼쪽 화살표: select_escape(mod.rs:851) -> select_back. 컨트롤러 검은 건반: close_folder(mod.rs:382) -> select_back. 스킨 키 FolderClose·마우스 CloseBar: close_bar(stage/select/skinned.rs:349) -> select_back(:354). select_back(mod.rs:866-895)은 AllSongs/TableLevels/UserFolders -> Root(:878-882), TableLevel(ti, _) -> TableLevels(ti)(:883-887), UserFolder(_) -> UserFolders(:888-892) 모두 shared.sel = 0 을 넣고 rebuild 한다. 그래서 발광 ★3 레벨 폴더(TableLevel)에서 Esc 하면 레벨 목록의 첫 줄(LV 최저)로, 표 폴더(TableLevels)에서 Esc 하면 Root 의 첫 줄 ALL SONGS 로 간다. AllSongs -> Root 만 우연히 일치한다. 사용자 보고와 코드가 일치한다.
4) 플레이 후 복귀(leave_play -> rebuild_select_items, app_play.rs:779-787)는 select_view 와 sel 을 유지하고 clamp 만 한다(list.rs:334-336). 그래서 폴더 안 차트에서 플레이 뒤 Esc 해도 같은 현상이다.
5) 코드 안의 선례: 검색 이탈은 위치를 복원한다. exit_search(stage/select/search.rs:205-)가 ListPlace(apps/rbms-player/src/tablesrc.rs:1041-1097)로 보던 레벨(이름)과 커서의 폴더를 다시 찾아 view 와 row 를 되돌린다. FocusedFolder(tablesrc.rs:990-1022)가 SelectItem::Folder 를 Level(표, 레벨 이름)/UserFolder(id)/Leading(view)로 안정적으로 식별한다.
6) 테스트: 폴더 이탈은 select_view 만 단언한다(user_folder_tests.rs:123-128, stage/select/tests.rs:480-490, table_tests.rs). 읽은 범위에서 폴더 이탈 후 sel 을 단언하는 테스트는 없다.
7) 원본(beatoraja select/BarManager.java): dir(Queue<DirectoryBar>, 재귀 디렉터리 스택)과 sourcebars(Queue<Bar>, :73)가 있다. 진입할 때 sourcebars.addLast(prevbar)(:376)로 진입 직전에 커서가 있던 바를 저장한다. close()(:490-500)는 updateBar(parent)를 부르고, updateBar 는 dir 를 되감으며 sourcebar = sourcebars.removeLast()(:298-303)를 꺼낸 뒤 새 목록에서 그 바를 찾아 selectedindex 로 삼는다(:424-452): SongBar 는 sha256 일치, 그 외는 같은 클래스+같은 제목의 첫 항목, 못 찾으면 0. Root 로 갈 때(:284-287)는 prevbar = dir.first() 로 최상위 폴더를 찾는다.

**관련 파일**: /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/mod.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/skinned.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/list.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/search.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/tablesrc.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/lib.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/app_play.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/user_folder_tests.rs, /Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/stage/select/tests.rs, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/select/BarManager.java

**차이와 위험**

1) 원인은 select_back 의 세 갈래 모두 sel = 0 을 쓰는 것이다(mod.rs:880, 885, 890). 이탈 길이 select_back 한 곳이라 고칠 지점은 하나다. 폴더 항목의 목록 위치는 TableLevel(ti, li) -> TableLevels(ti) 에서 li 와 같고, TableLevels(ti) -> Root 에서는 '레벨이 있는 표'만 세는 순서(list.rs:283-)이며, UserFolder(at) -> UserFolders 에서 at, UserFolders -> Root 에서는 마지막 줄이다. 이미 있는 FocusedFolder/ListPlace 가 이름·id 기준 재탐색을 제공하므로 인덱스를 직접 세지 않고 '방금 떠난 폴더를 가리키는 항목'을 찾는 방식이 표가 갱신돼도 안전하다(설계 판단).
2) 요구 4번(폴더의 폴더, 재귀)과 직결된다. 지금의 고정 깊이 enum 으로는 임의 깊이를 표현하지 못하고, 이탈마다 부모를 알 수 없다. 이번 수정을 enum 위에 임시로 얹으면 재귀 도입 때 다시 갈아엎게 된다. 원본처럼 '진입 시 (폴더, 커서에 있던 바)를 스택에 쌓고 이탈 시 꺼내 복원'하는 구조가 3번과 4번을 함께 풀며, 이 경우 SelectView·ListPlace·표 갱신 보정(tablesrc.rs:1099 show_the_list_again)·skin_host/select/bridge.rs:124 의 view 매칭을 같이 손봐야 한다. 이번 조사에서 4번 구조는 읽지 않았다(확인 못함: Stella 표의 실제 폴더 계층).
3) 검색 이탈, 표 갱신 보정, 결과/플레이 복귀는 이미 위치를 유지하므로 별개다. 복원 규칙이 스택으로 바뀌면 search_resume(mod.rs:173)의 ListPlace 와 중복되지 않게 정리해야 한다.
4) 스킨이 그리는 휠은 shared.sel 과 select_gen 을 키로 갱신하므로(skinned.rs:464, scene.rs:362) sel 만 맞게 두면 휠이 따라온다. 휠 애니메이션 상태가 별도로 있는지는 읽지 않았다(확인 못함).
5) 숨은 폴더: 레벨 폴더는 필터로 숨겨지지 않지만(list.rs:297-) 표/MY FOLDERS 는 조건부로 나온다. 떠난 폴더가 새 목록에 없으면(표 갱신으로 레벨 사라짐) 기존 ListPlace 규칙처럼 0 또는 clamp 로 떨어뜨리는 규칙이 필요하다.
6) 실제 GPU 창에서 Esc 를 눌러 본 확인은 하지 못했다(확인 못함).

**결정 지점(조사 당시)**

- 복원 방식: 지금 enum 위에서 FocusedFolder/ListPlace 로 '떠난 폴더 항목'을 찾는 최소 수정(추천 단기) / 폴더 스택(진입 시 푸시, 이탈 시 팝, 원본 BarManager.dir+sourcebars 방식)으로 4번 재귀 폴더와 함께 도입(추천 장기)
- 3번만 먼저 최소 수정하고 4번에서 스택으로 다시 바꿀지, 처음부터 스택으로 3번과 4번을 한 번에 할지
- 폴더를 못 찾을 때(표 갱신으로 레벨 사라짐 등): 첫 줄(현재와 같음) / 가장 가까운 줄
- Root 로 나갈 때 AllSongs·표 폴더·MY FOLDERS 도 같은 규칙으로 해당 폴더에 커서를 둘지(추천: 예, 원본과 같음)

### tables-courses

#### 4. Stella 난이도표 추가와 폴더 안의 폴더(재귀 폴더)

**현재 상태**

[표 등록] 표 원본은 TableSource{name, location} 한 쌍이고(crates/rbms-config/src/schema.rs:437-440) 설정 library.tables(871)와 seeded_default_tables(874)에 들어갑니다. 기본 표는 DEFAULT_TABLES 한 건(발광, id insane1, 사본 https://bms.hyuns.uk/tables/insane1)뿐이고(crates/rbms-config/src/tables.rs:36-37), 한 번만 넣고 지운 표는 다시 넣지 않습니다(tables.rs:79-93). 바이너리 내장 스냅샷도 insane1 한 건 고정 배열입니다(apps/rbms-player/src/tablesrc/seed.rs:31 `SNAPSHOTS: [Snapshot; 1]`, include_bytes! 로 web/data/tables/insane1/{header,data}.json). 사용자는 표 관리 화면에서 주소를 직접 등록할 수 있어(apps/rbms-player/src/stage/tables.rs, register_table tablesrc.rs:860) Stella 주소를 넣으면 현재 코드로도 받아집니다.
[받기] fetch_with 가 응답 첫 글자로 본문/머리/페이지를 판별하고 페이지는 <meta name="bmstable"> 를 읽으며(crates/rbms-table/src/fetch.rs:78-95, html.rs:25), header.data_url 은 헤더 기준 상대 주소로 합쳐 받습니다(fetch.rs:106-107). 저장본은 설정 폴더 옆 tables/<주소해시>.json 이고(tablesrc.rs:57-58, 238-260) 한 번 받으면 사용자가 R/A 를 누를 때만 다시 받습니다(stage/tables.rs 머리 주석 15-17행). 표 레벨은 level_order 가 없으면 숫자 순으로 만듭니다(crates/rbms-table/src/lib.rs:194-207).
[곡 매칭] 표 곡은 md5 로만 라이브러리와 맞춥니다(lib.rs:173-191, Md5Index). 선곡에는 곡이 아니라 '소유한 곡의 인덱스 목록'만 올라갑니다(TableLevels = Vec<(레벨, Vec<usize>)>, tablesrc.rs:42).
[폴더 깊이] 선곡 위치는 평평한 Copy enum SelectView{Root, AllSongs, TableLevels(표), TableLevel(표,레벨), UserFolders, UserFolder(번호)}(apps/rbms-player/src/lib.rs:458-466)이고 스택이 아닙니다. 목록은 이 값으로 match 해서 만들고(apps/rbms-player/src/stage/select/list.rs:276-333), 뒤로가기는 TableLevel→TableLevels→Root 로 하드코딩되어 있습니다(stage/select/mod.rs:866-899, 들어가기 421-441). 즉 깊이는 뿌리 > 표 > 레벨 > 곡 최대 3단이고 폴더 안의 폴더 변형이 없습니다. SelectItem 도 Song(usize)/Folder{label,target} 둘뿐입니다(lib.rs:470-473). 표 위치를 새로고침 뒤 다시 찾는 ListPlace/FocusedFolder 도 이 세 변형을 직접 다룹니다(tablesrc.rs:990-1100). MY FOLDERS 도 뿌리 > MY FOLDERS > 폴더 > 곡 2단이며 ADR 이 '폴더 안의 묶음은 없다'고 못 박았습니다(docs/acknowledge/2026-10-14-adr-user-folders.md 결정 2, apps/rbms-player/src/user_folders.rs:1-5, 44).
[웹의 표 제공] 웹은 표를 코드에 정적 import 한 스냅샷으로만 제공합니다. RAW_SNAPSHOTS 는 insane1 한 건 하드코딩(web/src/server/service/domain/table-snapshot/table-snapshot.data.ts:1-17). 표준 header.json/data.json 은 /tables/[tableId]/ 아래 두 라우트(web/src/app/tables/[tableId]/header.json/route.ts, data.json/route.ts → web/src/server/route/table-snapshot.ts:91-105)가 내고, 머리는 name/symbol/data_url/level_order/last_update 다섯 필드만 만들어 course 는 나가지 않습니다(table-snapshot.service.ts:93-99, 스냅샷 머리 스키마 web/src/server/dto/table-snapshot.ts:8-13 이 course 를 버림). 항목 스키마는 md5 가 유효 해시여야 하고 id 등 다른 키는 버립니다(dto:15-25). 이와 별도로 DB 기반 /api/tables(관리자 세션 업서트)와 difficulty_table/table_folder/table_chart/table_course 테이블이 있지만 table_chart.chart_sha256 이 NOT NULL 이라 md5 만 있는 표를 못 담는다고 ADR 에 적혀 있습니다(web/src/server/db/schema.ts:325-360, web/src/server/route/table.ts:20-30, docs/acknowledge/2026-10-14-adr-default-insane-table.md '이유').
[외부 관찰, 코드 아님. 2026-10-23 curl/WebFetch 로 공개 응답만 읽음, 저장소·로컬 파일 변경 없음] https://stellabms.xyz/st/table.html 은 <meta name="bmstable" content="header.json"> 를 가진 정적 페이지입니다. 같은 사이트에 /sl/(Satellite) 표가 형제로 있습니다(client 테스트도 sl 주소를 예로 씀: tables.rs:132). st/header.json 은 name Stella, symbol st, data_url "score.json"(data.json 아님), level_order 없음, course 필드를 '배열의 배열'로 가지며 코스 약 78개("name" 키 79개에서 표 이름 1 제외한 추정)입니다. score.json 은 2350곡, 키는 id/md5/sha256/title/artist/url/url_diff/level 이고 name_diff·comment 없음, 레벨 0~12, 약 840KB, Cache-Control public, max-age=60 이며 Last-Modified/ETag 는 관찰되지 않았습니다.

**관련 파일**: crates/rbms-config/src/tables.rs, crates/rbms-config/src/schema.rs, crates/rbms-table/src/lib.rs, crates/rbms-table/src/dto.rs, crates/rbms-table/src/fetch.rs, crates/rbms-table/src/html.rs, crates/rbms-table/src/saved.rs, apps/rbms-player/src/tablesrc.rs, apps/rbms-player/src/tablesrc/seed.rs, apps/rbms-player/src/stage/tables.rs, apps/rbms-player/src/lib.rs, apps/rbms-player/src/stage/select/list.rs, apps/rbms-player/src/stage/select/mod.rs, apps/rbms-player/src/user_folders.rs, apps/rbms-player/src/skin_host/select/bridge.rs, web/src/server/service/domain/table-snapshot/table-snapshot.data.ts, web/src/server/service/domain/table-snapshot/table-snapshot.service.ts, web/src/server/dto/table-snapshot.ts, web/src/server/route/table-snapshot.ts, web/scripts/snapshot-table.ts, web/data/tables/insane1/source.json, docs/acknowledge/2026-10-14-adr-default-insane-table.md, docs/acknowledge/2026-10-14-adr-user-folders.md

**차이와 위험**

1) 클라이언트가 Stella 를 받는 것 자체는 막힘이 보이지 않습니다(meta 태그 있음, 상대 data_url 지원, 알 수 없는 키 id 는 무시, md5+sha256 둘 다 있어 md5 매칭 가능). 실제로 받아 보는 시험은 이번 조사에서 하지 않았으므로 '확인 못함'입니다.
2) 폴더 안의 폴더는 구조 변경입니다. SelectView 가 Copy 인 평평한 enum 이고, 목록 생성(list.rs:276-333)·뒤로가기(mod.rs:866-899)·위치 복원(tablesrc.rs ListPlace/FocusedFolder)·스킨 브리지의 열린 폴더 제목(skin_host/select/bridge.rs:117-140)·scene.rs:156-167·list.rs:578-579,659 가 모두 변형 목록을 직접 match 합니다. 임의 깊이로 가려면 경로(스택/트리) 모델로 바꿔야 하고 위 호출처 전부와 골든·스킨 계약 테스트가 걸립니다. 어느 구조를 재귀로 만들지가 요구 문장만으로는 정해지지 않습니다(예: 'Stella > st/sl > 레벨 > 곡' 같은 표 묶음인지, 표 header 의 course 묶음인지, 사용자 임의 폴더인지).
3) 표 항목에 sha256 만 있는 곡은 매칭되지 않습니다(lib.rs:179 가 md5 만 봄). Stella 는 md5 가 모두 있어 영향이 없지만 다른 표에는 해당될 수 있습니다.
4) 웹은 한 표 고정입니다(table-snapshot.data.ts:15 정적 import). 두 번째 표(Stella)를 서빙하려면 데이터 파일 추가, RAW_SNAPSHOTS 확장, 머리 DTO 에 course 보존이 필요하고, 항목 스키마가 id·sha256 을 버립니다. 클라이언트가 Stella 를 원본 주소로 직접 받으면 웹 제공은 필수가 아닙니다(웹 사본은 오프라인 첫 실행과 원본 장애 대비용이었음: tables.rs:33-35 주석).
5) 기본 등록을 Stella 로 넓히면 DefaultTable.id/location 은 릴리스 뒤 바꾸면 안 된다는 규칙(tables.rs:19-26)과, 갱신되는 표를 실행 파일에 박으면 낡는다는 점(seed.rs 가 include_bytes! 로 내장, 갱신은 사용자 R/A 만)이 부딪힙니다.
6) 표 출처 표기 정책이 ADR 로 이미 '다른 사이트 흔적 제거'로 정해져 있어(docs/acknowledge/2026-10-14-adr-table-page-no-source-notice.md '현재 상태') 이번 '출처 등록' 요구와 충돌 소지가 있습니다.

**결정 지점(조사 당시)**

- 폴더 안의 폴더가 정확히 무엇을 가리키는지: A) 표 묶음(Stella 아래 st·sl 등 형제 표를 한 폴더로) / B) 표 header 의 course 묶음 / C) 임의 깊이 일반 트리(SelectView 를 경로 스택으로 전면 교체).
- Stella 를 기본 표로 넣을지(바이너리 내장 + 한 번 등록, 갱신은 수동 R/A) 아니면 사용자가 주소를 직접 등록하게 두고 기본은 안내만 할지, 그리고 sl(Satellite) 도 함께 넣을지.
- 웹이 Stella 를 사본으로도 서빙할지(웹 정적 import 확장 필요) 아니면 클라이언트가 stellabms.xyz 에서 직접 받게 할지.
- 출처 표기 정책: 발광 표는 '흔적 제거'가 유효인데 Stella·단위 표는 출처를 표기할지(표마다 다르게 둘지).

#### 5. 곡 다운로드(표의 url·url_diff, 없는 곡 흐리게, 확인 모달, 곡 폴더 지정)

**현재 상태**

[표 곡 데이터] 클라이언트 TableEntry 는 md5/sha256/level/title/artist/url/url_diff 일곱 필드를 읽고 저장본에도 넣습니다(crates/rbms-table/src/dto.rs:11-26, saved.rs:19-32, 테스트 saved.rs:157-158). name_diff 필드는 클라이언트 구조체에 없어 읽는 순간 버려집니다(dto.rs:11-26 에 name_diff 없음. serde 기본이 알 수 없는 키 무시). 반면 웹 스냅샷과 API 는 name_diff 를 유지합니다(web/scripts/snapshot-table.ts:22, web/src/server/dto/table-snapshot.ts:24,58). 그리고 클라이언트에서 entry.url / url_diff 를 읽는 코드는 테스트 밖에 없습니다(grep: rbms-table 테스트와 saved.rs 테스트뿐, apps/rbms-player/src 에는 소비처 없음).
[없는 곡 표시] 표 항목 중 라이브러리에 없는 곡은 막대가 없습니다. match_levels_in 은 라이브러리 인덱스만 돌려주고(crates/rbms-table/src/lib.rs:173-191), 없는 레벨도 빈 목록으로 남지만 곡 막대는 만들지 않습니다(lib.rs:164-166, 레벨 막대 라벨은 'LV n (소유 수)' list.rs:304-306). SelectItem 엔 Song(usize)/Folder 뿐이라 '없는 곡' 변형이 없습니다(apps/rbms-player/src/lib.rs:470-473). 즉 opacity 처리된 없는 곡 막대도, 그 곡을 열었을 때의 모달도 없습니다. MY FOLDERS 도 같아서 가지지 않은 곡은 폴더 줄의 (가진 곡/전체) 숫자로만 보입니다(user_folders.rs:1-12).
[곡 폴더 설정] 곡 라이브러리 폴더 목록은 config.library.folders 이고(crates/rbms-config/src/schema.rs:855), 곡 폴더 관리 화면에서 추가·삭제합니다(apps/rbms-player/src/stage/folders.rs, 추가는 rfd 네이티브 선택창 dialog::pick_song_folder, 나가면 재스캔 folders.rs:90). 쓰기 대상 폴더 개념(다운로드 폴더)은 없습니다.
[다운로드·압축 해제 기능] 없습니다. 저장소에서 외부 프로세스 실행(Command::new)이 0건이고, 압축 해제·아카이브 코드도 없습니다(grep 결과 해당 없음. lr2crs·zip 지원 코드 0건). 브라우저로 링크 열기도 없습니다.
[의존성] HTTP: reqwest 0.12 (blocking, json, rustls-tls)가 workspace 의존성이고(Cargo.toml workspace.dependencies) rbms-table(crates/rbms-table/Cargo.toml)과 rbms-ir 가 직접 씁니다. rbms-player 는 reqwest 를 직접 선언하지 않고 두 크레이트로 간접 사용합니다(apps/rbms-player/Cargo.toml). 압축: Cargo.lock 에 zip/sevenz/unrar/tar/bzip2/xz/lzma 는 없습니다. flate2 1.1.10 과 miniz_oxide·crc32fast 는 있으나 png 크레이트(image 의 png feature)의 하위 의존으로만 들어와 있어 R-BMS 어느 크레이트도 직접 선언하지 않았습니다(Cargo.lock 의 png 항목이 flate2·crc32fast·miniz_oxide 를 의존). flate2 는 deflate 스트림 라이브러리이지 ZIP 컨테이너 리더가 아니고 rar/7z/lzh 는 다룰 수 없습니다. 기존 표 받기용 HTTP 코드는 응답 64MB 상한, 전체 30초 제한이라 큰 곡 압축본에 그대로 쓸 수 없습니다(crates/rbms-table/src/fetch.rs:22-37, 141-161).
[외부 관찰, 코드 아님] Stella score.json 2350곡의 url_diff 분포(내가 계산): stellabms.xyz/upload/<번호> 636건(실제로 한 건 HEAD/GET 해 보니 Content-Type application/zip, content-disposition attachment 로 ZIP 바로 내려줌), 확장자가 zip 인 직접 링크 577, rar 41, bms/bme/bml 단일 차트 파일 123, 빈 값 278, 그 밖의 HTML 페이지(드롭박스·web.archive.org 등) 약 695. url 쪽은 zip 149, rar 116, 7z 7, lzh 2. 자주 쓰이는 호스트는 stellabms.xyz 636, bms.hexlataia.xyz 457, darksabun.club 133, web.archive.org 88 입니다.

**관련 파일**: crates/rbms-table/src/dto.rs, crates/rbms-table/src/saved.rs, crates/rbms-table/src/lib.rs, crates/rbms-table/src/fetch.rs, crates/rbms-table/Cargo.toml, apps/rbms-player/src/lib.rs, apps/rbms-player/src/stage/select/list.rs, apps/rbms-player/src/stage/select/mod.rs, apps/rbms-player/src/stage/folders.rs, apps/rbms-player/src/dialog.rs, apps/rbms-player/Cargo.toml, crates/rbms-config/src/schema.rs, Cargo.toml, Cargo.lock, web/scripts/snapshot-table.ts, docs/acknowledge/2026-10-14-adr-table-page-no-source-notice.md

**차이와 위험**

1) 가장 큰 격차: 선곡에 '없는 곡' 막대 자체가 없습니다. 표 항목 수준으로 목록을 보여 주려면 목록 모델(SelectItem), 스킨 쪽 막대 종류(list.rs:578-579 BarKind), 정렬·필터·검색, 소유 수 계산(shown_count)이 모두 바뀝니다. 외부 Lua 스킨이 그리는 막대와의 호환(BarKind, 곡 막대 속성) 영향이 큽니다.
2) 압축 해제는 새 의존성 없이는 불가능합니다. 규칙('새 의존성 금지', 사용자 규칙 및 convention 6.6)상 zip 크레이트 등은 사용자 승인이 필요하고, rar·7z·lzh 는 Rust 쪽 순수 구현이 사실상 없거나 별도 의존이 큽니다(확인 못함: 후보 크레이트의 현재 상태·라이선스는 조사하지 않음). Stella 링크는 ZIP 이 가장 많지만(위 통계) rar 157, 7z 7, lzh 2 가 있어 ZIP 만 지원하면 일부는 '열기 불가'가 됩니다. 단일 .bms/.bme 링크 123건은 압축 해제 없이 곡 폴더에 저장하는 것만으로 충분하지만 키음원·BGA 가 없는 단독 차트라 플레이 불가일 수 있습니다.
3) url_diff 가 가리키는 것이 '다운로드 가능한 파일'이 아닌 경우가 약 695건(HTML 페이지)이라, 앱 안 자동 다운로드는 일부만 가능합니다. name_diff 는 클라이언트가 이미 버리므로 필요하면 DTO 에 추가해야 합니다(저장본 호환: saved.rs 의 #[serde(default)] 방식 필요).
4) 안전: 외부 사이트가 준 압축 파일을 받아 곡 폴더에 풀기 때문에 zip-slip(경로 이탈), 압축 폭탄(크기 상한), 확장자 필터, 덮어쓰기 정책, 다운로드 중 UI 비차단(워커 스레드, 프레임 루프 보호 — docs/acknowledge/2026-10-14-adr-play-hitch-criteria.md) 설계가 필요합니다. 현재 http_get 의 64MB 상한·30초 전체 제한은 용도가 달라 재사용이 안 됩니다.
5) 다운로드 후 라이브러리 반영(재스캔)과 표 막대 즉시 갱신은 지금 구조상 곡 폴더 관리 화면을 나갈 때 Loading::rescan 으로만 일어납니다(folders.rs:90). 다운로드 완료 → 부분 재스캔 경로는 없습니다.
6) 재배포·저작권: 이 앱이 임의 사이트의 곡을 받아 푸는 기능이며, 호스팅 아님. 법적 판단은 이번 조사 범위 밖(확인 못함).

**결정 지점(조사 당시)**

- 새 의존성 허용 여부: ZIP 압축 해제용 크레이트 한 개(예: zip)를 승인할지. rar·7z·lzh 는 '지원 안 함, 브라우저로 열기만' / '외부 도구 호출 금지 유지' 중 선택.
- 다운로드 가능한 링크의 범위: A) stellabms.xyz/upload 와 확장자가 zip 인 직접 링크만 / B) rar·7z 까지(의존성 추가) / C) HTML 페이지는 시스템 브라우저로 열기만 — 그러려면 브라우저 열기 수단(현재 없음, 새 의존성 또는 OS 명령)도 정해야 함.
- 없는 곡 표시 방식: A) 선곡 목록에 '없는 곡' 막대를 새로 만들어 흐리게(스킨 호환 영향 큼) / B) 레벨 폴더 안에 별도 '받을 수 있는 곡' 하위 목록 / C) 표 관리 화면에서만 받기.
- 다운로드 폴더: 곡 폴더 목록 중 하나를 고르게 할지, 전용 '다운로드 폴더'를 설정에 새로 둘지(설정 스키마 버전이 오릅니다: crates/rbms-config 의 CURRENT_SCHEMA_VERSION).
- 받은 직후 동작: 자동 재스캔으로 막대를 바로 살릴지, 사용자가 곡 폴더 관리에서 확인하게 둘지.

#### 6. 매일 KST 0시 표 갱신(cron·workflow)

**현재 상태**

[스케줄 현황] 웹의 vercel.json 에는 crons 가 없습니다(web/vercel.json 전체: $schema, framework nextjs, regions icn1, ignoreCommand 뿐). .github/workflows/ 다섯 개(ci.yml, release.yml, signing-check.yml, web-db.yml, web.yml) 중 schedule 트리거는 0건이고, web-db.yml 은 workflow_dispatch 전용입니다(.github/workflows/web-db.yml:6-14). ci.yml/web.yml 은 push·pull_request, release.yml 은 prod push·태그·수동입니다. 저장소에서 'cron' 문자열은 AI 주간 요약 설계문서의 선택지 한 줄(docs/plan/2026-10-19-web-ai-features.md:696)에만 나옵니다. 웹 환경변수 스키마에도 CRON_SECRET 류는 없습니다(web/src/server/lib/env.ts:6-23).
[표 갱신 방식 현재] 커밋된 스냅샷입니다. web/data/tables/insane1/{header,data,source}.json 을 코드가 정적 import 하므로(web/src/server/service/domain/table-snapshot/table-snapshot.data.ts:1-3) 표 내용은 배포 단위로만 바뀝니다. 스냅샷은 사람이 `bun scripts/snapshot-table.ts <주소> data/tables/insane1` 을 실행하고 git diff 를 본 뒤 커밋하는 흐름입니다(docs/web/2026-10-14-insane-table.md:175, web/data/tables/insane1/source.json 의 method.command). 이 스크립트는 단일 표·data.json 고정 파일명(snapshot-table.ts:10-12)이고 comment 를 비우고 웹 주소를 지웁니다(ENTRY_FIELDS_*, 같은 파일 21-24).
[배포 경로] Vercel 은 Git 연결, 운영 브랜치 dev, 루트 web 이고 ignoreCommand 가 'web/ 가 이전 배포 커밋과 같으면 건너뛴다'입니다(web/vercel.json, docs/acknowledge/2026-10-14-adr-vercel-git-deploy.md 결정 3). web/data/tables/** 만 바뀐 커밋은 Rust CI(ci.yml 의 예외 경로)도 같이 돕니다(.github/workflows/ci.yml:5-14의 'web/data/tables/**'). 이유는 플레이어가 그 두 파일을 실행 파일에 넣고 테스트가 읽기 때문입니다(apps/rbms-player/src/tablesrc/seed.rs 머리 주석).
[운영 DB 쓰기 경로] 운영 DB 는 GitHub Actions 의 수동 web-db.yml + 저장소 시크릿 WEB_DATABASE_URL 로만 만집니다(docs/acknowledge/2026-10-14-adr-production-db-operations.md). 입력은 고정된 action 목록(status, migrate, cleanup-*, chart-notes-*, admin-seed-count, assist-grade-backfill*)이고 SQL 은 web/scripts/*.sql 만 실행합니다(run-sql.ts). 웹 코드에서 DB 에 쓰는 관리용 HTTP 경로는 관리자 '세션'을 요구하는 POST /api/tables(web/src/server/route/table.ts:20-30)와 POST /api/courses/meta(web/src/server/route/course.ts:100-110) 두 개뿐이고, API 토큰(Bearer) 방식이 아니라 쿠키 세션입니다(with-session.ts:18-26 withAdmin). 스케줄러가 쓸 수 있는 서버 토큰 경로는 없습니다.
[클라이언트 쪽 갱신] 클라이언트는 표를 저절로 다시 받지 않습니다(stage/tables.rs 머리 주석 15-17: 'Nothing fetches a table again by itself: R does ... A for every table'). 30일 지난 저장본에 날짜를 눈에 띄게 하는 알림만 있습니다(COPY_OLD_AFTER_DAYS = 30, tablesrc.rs:68).
[외부 사실] KST(UTC+9, 서머타임 없음) 0시는 UTC 15:00 이므로 크론 식으로는 `0 15 * * *` 입니다. Vercel 요금제(Hobby/Pro)는 저장소·문서에서 확인되지 않았습니다(docs/reference/ai-features/client.md:307, docs/plan/2026-10-19-web-ai-features.md:697 '확인하지 못했다'). 요금제별 크론 제한은 공식 문서 확인 못함.

**관련 파일**: web/vercel.json, .github/workflows/web-db.yml, .github/workflows/web.yml, .github/workflows/ci.yml, .github/workflows/release.yml, web/scripts/snapshot-table.ts, web/scripts/run-sql.ts, web/data/tables/insane1/source.json, web/src/server/service/domain/table-snapshot/table-snapshot.data.ts, web/src/server/route/table.ts, web/src/server/route/course.ts, web/src/server/lib/with-session.ts, web/src/server/lib/env.ts, apps/rbms-player/src/tablesrc/seed.rs, docs/acknowledge/2026-10-14-adr-production-db-operations.md, docs/acknowledge/2026-10-14-adr-vercel-git-deploy.md, docs/ci-release.md

**차이와 위험**

1) 일일 갱신을 하려면 '표 내용이 어디에 사는가'가 바뀌어야 합니다. 지금은 빌드 시점에 굳는 정적 JSON 이라 크론이 아무리 돌아도 재배포 없이는 사이트가 안 바뀝니다. 선택지는 (가) 크론이 스냅샷을 받아 저장소에 커밋·푸시 → Vercel 재배포(ignoreCommand 가 web/ 변경 시에만 빌드하므로 맞음, 단 Rust CI 도 같이 돎, 워크플로에 contents: write 권한과 커밋 계정이 필요하고 '자동 커밋' 정책·AI 트레일러 금지 규칙과 맞춰야 함), (나) 표를 DB 나 런타임 캐시/저장소에 넣고 읽는 쪽을 바꾸기(ADR 이 DB 에 안 넣은 이유는 sha256 NOT NULL 때문이므로 스키마 변경 필요), (다) 웹은 건드리지 않고 클라이언트가 원본 주소에서 직접 받기(서버 작업 불필요, 하지만 클라이언트는 자동 재조회가 없음) 입니다.
2) Vercel 크론은 production 배포의 GET 경로를 호출하는 방식이라 vercel.json 에 crons 항목과 인증된 라우트(CRON_SECRET 비교)를 새로 만들어야 하고, 그 라우트가 DB 에 쓰려면 지금은 서버 토큰 인증이 없습니다. 운영 DB 쓰기를 코드에서 자동화하는 것은 현재 ADR(운영 DB 는 수동 web-db.yml 만)과도 어긋나 별도 합의가 필요합니다. 자동 모드는 운영 DB 변경과 release.yml 실행을 거부하므로(사용자 규칙) 명령 승인이 필요합니다.
3) GitHub 스케줄은 UTC 기준이고 지연·누락이 있을 수 있으며(공식 문서 확인 못함), 장기간 활동 없는 저장소는 스케줄이 꺼질 수 있음(확인 못함).
4) 외부 표를 매일 받아 자동 게시하면 변조·장애 응답을 그대로 배포할 위험이 있습니다. 항목 수 급감, 해시 형식 위반 시 중단하는 검증이 필요합니다(snapshot-table.ts 에 --check 는 있으나 변경 허용 범위를 판단하는 규칙은 없음).
5) 웹이 Stella 를 서빙하지 않고 클라이언트가 원본 주소로 직접 받는 경우, '매일 최신화'는 클라이언트 쪽 자동 재조회 정책(현재 없음)의 문제가 됩니다.

**결정 지점(조사 당시)**

- 최신화 방식: A) GitHub Actions 크론(UTC 15:00)이 스냅샷을 받아 dev 에 커밋·푸시 → Vercel 자동 배포 / B) Vercel Cron + 관리용 라우트로 DB 또는 저장소(Blob 등)에 쓰고 런타임에 읽기 / C) 웹은 사본을 만들지 않고 클라이언트가 원본 주소에서 직접 받되 클라이언트에 자동 재조회를 추가.
- 운영 DB·운영 사이트 쓰기를 자동 크론으로 허용할지(현재 ADR 은 수동 web-db.yml 만). 허용하면 서버 간 인증 수단(CRON_SECRET 또는 API 토큰)을 새로 정해야 함.
- Vercel 요금제(Hobby/Pro) 확인: 크론 주기·횟수 제한과 함수 시간에 영향. 사용자만 대시보드에서 확인 가능.
- 자동 갱신 때 변경 검증 기준(항목 수 최소 유지, 해시 형식, 변경량 상한)과 실패 시 알림 방식.

#### 7. 곡 파일 호스팅(Cloudflare) 검토

**현재 상태**

[저장소의 Cloudflare 흔적] R2·Workers·wrangler 설정이나 코드는 없습니다. 'cloudflare' 문자열은 두 곳뿐이고 둘 다 '원본 사이트가 Cloudflare 를 쓴다'는 설명입니다(web/scripts/snapshot-table.ts:203, web/data/tables/insane1/source.json:61 '페이지 html 은 Cloudflare 가 요청마다 스크립트를 주입해 해시가 매번 달라 저장하지 않는다'). wrangler.toml 등 설정 파일도 저장소에 없습니다(grep 결과 없음).
[웹 호스팅·저장 방식] 운영 웹은 Vercel 입니다(web/vercel.json 리전 icn1, docs/acknowledge/2026-10-14-adr-vercel-git-deploy.md). 응답 헤더로 보면 bms.hyuns.uk 는 Cloudflare 를 거치지 않고 Vercel 이 직접 응답합니다(2026-10-23 curl -I /tables/insane1/header.json → server: Vercel, x-vercel-id, cf-* 헤더 없음. 외부 관찰, 코드 아님). 파일 저장 추상화는 리플레이 하나뿐입니다. ReplayStorage{kind, save, load}(web/src/server/service/shared/storage/replay-storage.ts:6-10)에 구현은 MySQL 인라인 저장 하나(createMysqlReplayStorage, replay.data mediumtext 컬럼: web/src/server/db/schema.ts:166-197)입니다. 환경변수 REPLAY_STORAGE 는 'db'|'blob' 을 받지만(web/src/server/lib/env.ts:19-20) 컴포지션은 'db' 가 아니면 'is not implemented; only db is supported' 로 던집니다(web/src/server/compose/replay.ts:45-46). @vercel/blob 의존성은 package.json 에 없습니다. 리플레이 크기 상한 기본 4MB(env.ts:3 DEFAULT_REPLAY_MAX_BYTES). 즉 큰 바이너리(곡 압축본) 저장 경로는 웹에 없습니다.
[웹 보안 제약] 프로덕션 CSP 가 connect-src 'self', img-src 'self' data: blob: 이라(web/next.config.ts:3-12) 웹 화면에서 외부 호스트 파일을 fetch 하려면 CSP 를 넓혀야 합니다. 플레이어 앱은 CSP 와 무관하게 HTTP 를 직접 합니다.
[외부 관찰] stellabms.xyz 자체가 Cloudflare 뒤에 있습니다(응답 헤더 server: cloudflare, cf-ray). Stella 업로드 파일(stellabms.xyz/upload/<번호>)은 ZIP 을 attachment 로 바로 내려줍니다.

**관련 파일**: web/vercel.json, web/next.config.ts, web/package.json, web/src/server/lib/env.ts, web/src/server/compose/replay.ts, web/src/server/service/shared/storage/replay-storage.ts, web/src/server/db/schema.ts, web/scripts/snapshot-table.ts, docs/acknowledge/2026-10-14-adr-vercel-git-deploy.md, docs/acknowledge/2026-10-14-adr-default-insane-table.md

**차이와 위험**

1) 구현 전무: R2 버킷·서명 URL·업로드 파이프라인·메타데이터 테이블·관리 화면이 모두 새로 필요합니다. 기존 REPLAY_STORAGE 'blob' 자리는 이름만 있고 미구현이며 Vercel Blob 용으로 보입니다(R2 와 다름. 의도는 확인 못함).
2) 법적·정책 위험이 가장 큽니다. 전 곡을 내 서버에 올려 재배포하는 것은 저작권·제작자 허락 문제입니다. 저장소 ADR 은 발광 표의 '메타데이터(표)'를 사본 서빙할 때도 원본에 재배포 허락/금지 문구가 없다는 점을 따졌습니다(docs/acknowledge/2026-10-14-adr-default-insane-table.md 배경, Q1). 곡 파일은 표 메타보다 훨씬 민감하며 이번 조사에서 판단하지 않았습니다(확인 못함). Stella 쪽은 자체 업로드(stellabms.xyz/upload)로 이미 배포 중이므로 '그쪽 방식을 갖춘다'는 것은 업로드 허락 규칙을 따로 확인해야 한다는 뜻입니다.
3) 규모: 발광 표 1,035곡, Stella 2,350곡 중 일부만이 해당된다고 해도 곡당 수~수십 MB 의 압축본이라 용량·요금(R2 저장·Class A/B 요청·이그레스 정책)을 따져야 합니다. 요금 수치는 이번 조사에서 확인하지 못했습니다(확인 못함: 공식 문서로 재확인 필요).
4) 인증·남용: 다운로드를 공개 주소로 열면 대역폭 남용, 로그인 사용자에게만 서명 URL 을 내면 인증 체계(better-auth 세션·API 토큰)와 레이트리밋(web/src/server/lib/with-rate-limit.ts) 확장이 필요합니다. 업로드 쪽은 파일 형식·크기·악성 압축 검증이 필요합니다.
5) 로컬에 '발광 전곡'이 이미 있다면 그 곡들의 md5 와 표의 매칭, 중복 제거, 곡 폴더 구조(압축본 단위 vs 곡 폴더 단위)를 정해야 합니다. 이 저장소에는 곡 폴더를 압축·업로드하는 도구가 없습니다.
6) 운영 변경이므로 자동 모드가 거부하는 영역(운영 사이트 쓰기, 시크릿)이 많습니다. 비밀 파일 접근 금지 규칙상 Cloudflare 토큰은 사용자 쪽 입력이 필요합니다.

**결정 지점(조사 당시)**

- 곡 파일 재배포를 하기로 할지(권리 확인 책임은 사용자). 한다면 대상: 발광 전곡 / 허락이 명확한 곡만 / 사용자가 직접 올린 곡만.
- 저장소 선택: Cloudflare R2(+서명 URL) / Vercel Blob(env 에 이미 자리 있음) / 현 DB 외 다른 곳 — 요금 확인은 사용자 몫.
- 공개 범위: 로그인한 R-BMS 사용자만 / 누구나 / 표 항목 단위 제한.
- 다운로드 기능(항목 5)과 어떤 순서로 만들지: 외부 링크 다운로드 먼저(호스팅 없이) → 내 서버 미러는 나중에, 또는 처음부터 미러.

#### 8. 코스 모드·단위 인정(발광 제노사이드 코스 등록, 내 정보의 단위, 코스 게이지 설정)

**현재 상태**

[클라이언트 코스 기능은 이미 상당 부분 있음] 코스 파일 모델은 beatoraja CourseData 를 그대로 옮겼습니다. 이름/차트(md5·sha256·title)/제약 14종/트로피/release(crates/rbms-course/src/model.rs:243-254), 제약 토큰 grade, grade_mirror, grade_random, no_speed, no_good, no_great, gauge_lr2, gauge_5k, gauge_7k, gauge_9k, gauge_24k, ln, cn, hcn(model.rs:92-109)을 소문자 토큰과 기준 구현 이름(CLASS 등) 둘 다 읽습니다(model.rs:154-159). 코스 id 는 R-BMS 자체 규칙 sha256(이름\n차트해시\n…)입니다(model.rs:304-313, 이름이 다르면 id 가 달라짐). 파일은 <설정폴더>/courses/*.json 에서만 읽고(apps/rbms-player/src/course_ui.rs:66-71, 선곡 COURSE 탭 stage/select/mod.rs:463-469, crates/rbms-course/src/load.rs:47-58), rbms_course::save 는 호출처가 앱 코드에 없습니다(grep 0건. 가져오기·만들기 UI 없음). .lr2crs 형식 지원 코드는 없습니다(grep 0건). 코스 막대는 별도 탭의 목록(CourseList)이고 곡이 하나라도 없으면 시작할 수 없습니다(course_ui.rs:63-118, 'N STAGES - M MISSING'). 코스 시작은 apps/rbms-player/src/lib.rs:270-282, 제약 적용은 CourseOverrides(course_ui.rs:285-312)입니다. NO GOOD/NO GREAT 는 표시만 하고 적용하지 않습니다(course_ui.rs:300 unapplied, 주석 '아직 강제하지 않는다'). course_ui.rs 머리 주석(14-17행)은 '아직 호출되지 않는다'고 쓰여 있으나 실제로는 호출됩니다(lib.rs:115, 270-282) — 낡은 주석입니다.
[표의 course 필드] 표 header 의 course 는 읽지 않고 원문 serde_json::Value 로만 보관·저장본에 씁니다(crates/rbms-table/src/dto.rs:51-54 '지금은 코스를 플레이하는 것이 없다', lib.rs:27-29, saved.rs:19-32). 변환기가 없습니다. 또한 모양이 다릅니다: 표의 코스는 `md5: ["…", …]`(문자열 배열)인데 rbms_course::Course 의 charts 는 {md5,sha256,title} 객체 배열이라 그대로는 읽히지 않습니다(model.rs:243-254, 테스트 픽스처 crates/rbms-course/tests/fixtures/courses_array.json 이 객체형). 외부 관찰: Stella header.json 의 course 는 배열의 배열이고 `constraint: ["grade_mirror","gauge_lr2","ln"]`, trophy silvermedal/goldmedal(missrate 5.0/2.5, scorerate 70/85), md5 문자열 배열 4개 구성입니다. 웹은 표 머리 스키마가 course 를 버려 표준 header.json 으로 내보내지 못합니다(web/src/server/dto/table-snapshot.ts:8-13, table-snapshot.service.ts:93-99).
[코스 게이지] GaugeIndex 아홉 개 중 Class(6), ExClass(7), ExHardClass(8)가 코스 게이지입니다(crates/rbms-judge/src/gauge.rs:23-33, 인덱스 58-69, is_course 98-100). 사용자 설정은 GaugeKind 여섯 개(AssistEasy~Hazard)뿐이고 코스 게이지는 선택 불가입니다(gauge.rs:9-16, 수치 설명 20-22). 코스 진행 중에는 항상 Class 로 시작합니다(app_play.rs:578 initial_gauge: Class). 시작 게이지 값은 게이지 표 Class 행의 init 입니다(lib.rs:259-261). 수치(GAUGE_TABLE, crates/rbms-judge/src/gauge_tables.rs:130-184): 7키 표는 Class [PG 0.15, GR 0.12, GD 0.06, BD -1.5, PR -3.0, MS -1.5] 완충 GUTS_CLASS, ExClass [0.15,0.12,0.03,-3,-6,-3], ExHardClass [0.15,0.06,0,-5,-10,-5](gauge_tables.rs:149-151 근처 7키 행). LR2 표는 Class [0.1,0.1,0.05,-2,-3,-2] GUTS_LR2, ExClass [0.1,0.1,0.05,-6,-10,-2] GUTS_LR2, ExHardClass [0.1,0.1,0.05,-12,-20,-2](gauge_tables.rs:182-184). 번들 data/gauge.ron 과의 일치는 테스트가 지킨다고 되어 있으나 이번엔 실행하지 않음(확인 못함). 이 코스 게이지는 beatoraja·LR2 표를 옮긴 것이고, IIDX 식 별도 게이지는 코드에 없습니다(grep 'iidx' 0건).
[게이지 방식 설정] 설정 화면 JUDGE 탭의 GAUGE 구역에 GAUGE SET(AUTO|LR2), GAUGE AUTO SHIFT, BOTTOM SHIFTABLE GAUGE, TOTAL 행이 있습니다(apps/rbms-player/src/stage/settings/catalog.rs:44, 값 목록 crates/rbms-config/src/judge.rs:28-31 GAUGE_SET_CYCLE = [None, Some(Lr2)]). 일반 GAUGE 행은 PLAY 설정 쪽 6종 순환입니다(crates/rbms-config/src/settings.rs:144-147). 코스 전용 게이지 방식 설정은 없고, 코스 제약 gauge_lr2/5k/7k/9k/24k 가 그 코스의 게이지 표를 강제합니다(course_ui.rs:299-307, app_play.rs:459-462 judge_setup.gauge_set = overrides.gauge_set_for(...)). 제약이 없으면 설정 GAUGE SET 또는 곡의 모드가 정합니다. IIDX/beatoraja/LR2 세 가지를 고르는 설정은 없습니다. beatoraja 표(5K/7K/PMS/24K)는 이미 모드별 자동 선택이고 LR2 만 설정으로 노출됩니다.
[코스 결과·제출] 코스 결과 화면은 EX, 점수율, 최대 콤보, MISS COUNT, 남은 게이지, 실패 스테이지, 트로피를 보입니다(course_ui.rs:~370-385, apps/rbms-player/src/stage/course_result.rs). 제출 본문은 course_hash, 점수 합계, 차트 해시 목록, trophy 이름, 어시스트를 싣지만 코스의 이름·제약·트로피 규칙 정의는 싣지 않습니다(apps/rbms-player/src/course_ir.rs:71-108). 로컬 DB 에는 코스 최고 기록 테이블이 없고 제출 본문 보관소 course_submission 뿐입니다(crates/rbms-store/src/scoredb/courses.rs:105-118). 코스 스테이지는 Class 게이지로 저장됩니다(courses.rs:120-140).
[웹] 코스 테이블 course(코스 정의: 이름·lntype·constraint json·trophy json), course_chart(순서+sha256), course_score, course_best(코스·사용자·SCRATCH AUTO 구분 기본키, 램프·EX·BP·콤보, assist_grade)가 있습니다(web/src/server/db/schema.ts:238-322). 연결 테이블 table_course 가 표↔코스를 잇습니다(schema.ts:353-359). 제출 API POST /api/courses(Bearer 토큰), 순위 /api/courses/[hash]/ranking, 내 최고 /api/courses/[hash]/best, 상세 /api/courses/[hash]. 코스 정의 등록 POST /api/courses/meta 는 관리자 세션만 가능하고(web/src/server/route/course.ts:100-110) 클라이언트는 이 경로를 호출하지 않습니다(crates/rbms-ir, apps/rbms-player 에서 'courses/meta' 호출 0건). 제출은 정의 등록 없이도 받습니다(course.service.ts submit 에 존재 확인이 없음, 255-345행 부근). 웹 화면은 코스 상세 한 쪽(web/src/app/(app)/courses/[courseHash]/page.tsx)뿐이고 코스 목록 화면은 없습니다. course_best.assist_grade 는 항상 0 입니다(course.service.ts:143-147, docs/HANDOFF.md:132).
[사용자 단위(段位) 자리] user 테이블에는 rank varchar(32) 기본 '' 와 rank_points double(기본 0)이 있고(web/src/server/db/auth-schema.ts:18-19), better-auth additionalFields 로 input:false 선언되어 있으며(web/src/server/lib/auth.ts:26-27), 플레이어 프로필 응답이 extra:{rank: row.rank} 로 이미 내보냅니다(web/src/server/service/domain/player/player.service.ts:22-24). 그러나 이 열을 쓰는 코드는 없습니다(docs/web/2026-10-14-audit.md:48 '쓰기 0건', docs/HANDOFF.md:122). user.extra json 도 있으나 프로필 출력에 쓰지 않습니다. '마이페이지' 설정 탭은 프로필/API 토큰/동기화 blob/라이벌/AI 키이고(web/src/widgets/settings/settings-tabs.tsx:27-31) 단위 선택 화면은 없습니다. 클리어한 코스 목록을 사용자별로 모으는 조회는 course_best 를 course_hash+user 로 읽으면 가능하지만 '단위 등급 → 코스' 대응 테이블은 없습니다.
[외부 관찰] BMS-IR 단위 표 https://www.bms-ir.org/new/dan?table=genocide2018 는 WebFetch 요약상 코스 이름이 初段~十段, 発狂初段~十段, 発狂皆伝, Overjoy 입니다(요약 모델의 읽기라 정확한 개수·이름은 확인 못함). 페이지는 '表示中の段位ZIP' 링크 /new/dan/download.zip?table=genocide2018 를 제공하고, HEAD 로 확인한 응답은 200, application/zip, content-disposition filename="BMS-IR_dan_courses_genocide2018.zip", content-length 10329 입니다. ZIP 안의 형식(.lr2crs 로 추정됨)은 내려받아 열지 않아 확인 못함. 개별 코스는 /new/course?hash=… 페이지에서 .lr2crs 를 받을 수 있다고 페이지가 말합니다. API 주소는 페이지에 없었습니다.

**관련 파일**: crates/rbms-course/src/model.rs, crates/rbms-course/src/load.rs, crates/rbms-course/src/run.rs, crates/rbms-course/tests/fixtures/courses_array.json, crates/rbms-table/src/dto.rs, crates/rbms-table/src/saved.rs, crates/rbms-judge/src/gauge.rs, crates/rbms-judge/src/gauge_tables.rs, crates/rbms-config/src/judge.rs, crates/rbms-config/src/settings.rs, crates/rbms-config/src/schema.rs, crates/rbms-store/src/scoredb/courses.rs, apps/rbms-player/src/lib.rs, apps/rbms-player/src/course_ui.rs, apps/rbms-player/src/course_ir.rs, apps/rbms-player/src/app_play.rs, apps/rbms-player/src/stage/course_result.rs, apps/rbms-player/src/stage/select/mod.rs, apps/rbms-player/src/stage/settings/catalog.rs, web/src/server/db/schema.ts, web/src/server/db/auth-schema.ts, web/src/server/lib/auth.ts, web/src/server/dto/course.ts, web/src/server/route/course.ts, web/src/server/service/domain/course/course.service.ts, web/src/server/service/domain/player/player.service.ts, web/src/widgets/settings/settings-tabs.tsx, docs/acknowledge/2026-10-14-adr-assisted-course-submission.md, docs/acknowledge/2026-10-15-adr-course-stage-combo.md, docs/HANDOFF.md

**차이와 위험**

1) 코스를 '들여오는' 길이 없습니다. 표 header 의 course 는 보관만 하고 변환기가 없으며 모양(md5 문자열 배열 vs 객체 배열)이 달라 그대로는 못 읽습니다. BMS-IR 단위는 ZIP(추정 .lr2crs)로만 보이고 .lr2crs 파서는 없어 별도 구현이 필요합니다. 어느 쪽 형식을 원천으로 삼을지가 정해져야 합니다.
2) 코스 id(course_hash)가 이름에 의존합니다(model.rs:304-313). 서버에 코스를 미리 등록해 두려면 클라이언트가 같은 이름·같은 차트 순서로 같은 해시를 만들어야 하므로, 원천(표 header, lr2crs)의 이름 표기(예: 'GENOSIDE 2018 段位認定 五段' 등 테스트 픽스처의 이름)와 정규화가 한 글자라도 다르면 순위가 갈라집니다.
3) 서버 코스 등록 경로가 사람 손에 묶여 있습니다. 코스 정의 등록은 관리자 세션의 POST /api/courses/meta 뿐이고 클라이언트·스크립트에 쓰는 토큰 경로가 없습니다. 제출은 정의 없이도 받으므로 '출처 등록'을 하려면 course 테이블의 extra json 이나 새 열을 정해야 합니다(스키마 변경 = 마이그레이션, 운영 DB 변경은 자동 모드가 거부).
4) 단위 획득 판정 규칙이 없습니다. 서버에는 '코스 클리어 → 단위' 대응도, user.rank 에 쓰는 코드도 없습니다. 클리어 정의(코스 램프가 FAILED 가 아님? 어시스트·SCRATCH AUTO 클리어 포함 여부? 조건 gauge/옵션?)를 정해야 하고, 모든 판을 저장·전송하는 기존 원칙과 어시스트 처리(course_best 는 SCRATCH AUTO 별도 칸, assist_grade 는 항상 0)와 맞춰야 합니다. user.rank 는 varchar(32) 단일 문자열이라 여러 체계(발광 단위·Stella·기타 코스)를 담기 부족하면 새 테이블이 필요합니다.
5) '마이페이지에서 내가 클리어한 코스를 설정'은 UI 신규 구현입니다(현재 설정 탭 5개에 없음). 자기 신고 방식인지 서버가 증명하는 방식인지에 따라 부정 방지 설계가 달라집니다.
6) 게이지: 코스 게이지 값 자체(Class/ExClass/ExHardClass 와 LR2 표)는 이미 beatoraja·LR2 원본 수치로 구현되어 있어 '정확하게 구현' 과제는 주로 선택 UI 와 매핑입니다. 지금은 코스 도중 항상 Class 로 시작하고(app_play.rs:578) 제약으로 표만 바뀝니다. Stella 코스(grade_mirror + gauge_lr2 + ln)가 요구하는 대로 LR2 표 Class 행이 적용되는지는 코드상 경로(제약 → gauge_set Lr2 → Class 게이지)로 이어지지만 이번 조사에서 실행·검증은 하지 않았습니다(확인 못함). 사용자의 'IIDX 모드'에 해당하는 게이지 표는 코드에 없습니다. 이를 만들려면 새 수치 정의와 해당 모드의 근거(원본 자료)가 필요하고, 판정·점수 의미가 원본과 달라지면 먼저 확인받는 규칙이 적용됩니다.
7) 비토라자(beatoraja) 모드와 LR2 모드의 코스 게이지가 비슷하다는 사용자 판단은 수치로는 같지 않습니다(7키 Class 의 BD -1.5/PR -3.0 대 LR2 Class 의 BD -2/PR -3 등, gauge_tables.rs:149 대 182). LR2 를 기준으로 삼는다면 gauge_set 을 LR2 로 고정하는 의미가 됩니다.
8) NO GOOD/NO GREAT 제약은 미적용입니다(course_ui.rs:300). 발광 단위가 이 제약을 쓰는지는 확인 못함.
9) 이전 ADR 들은 서버가 램프 id 0~10·와이어 이름 변경을 못 하게 합니다(HANDOFF '절대 하지 말 것'). 단위를 램프와 섞으면 안 됩니다.

**결정 지점(조사 당시)**

- 코스 원천: A) 표 header course(Stella 등, 변환기 신규) / B) BMS-IR 단위 ZIP(.lr2crs 파서 신규, ZIP 해제는 새 의존성) / C) 둘 다. 발광 제노사이드 코스 정의를 어느 파일에서 가져와 서버 course 테이블에 넣을지, 그리고 코스 이름 표기를 어떻게 정규화할지(해시 안정성).
- 서버 등록 자동화: 관리자 세션 API 를 쓰는 일회성 스크립트 / web-db.yml 의 새 action 으로 SQL 적재 / 새 토큰 인증 관리 라우트. 어느 쪽이든 운영 DB 쓰기는 사용자 승인 또는 '!' 직접 실행이 필요합니다.
- 단위 데이터 모델: user.rank(문자열 한 칸) 재사용 / 새 테이블 user_dan(체계·단위·획득 코스·획득 시각, 여러 체계 지원). 표시 위치(프로필, 랭킹, 클라이언트 내 정보)와 마이페이지 선택 방식(자기 신고 vs 서버가 기록으로 자동 판정).
- 단위 인정 조건: 코스 클리어 램프 기준(NORMAL 이상? EXHARD?), 어시스트·SCRATCH AUTO 판 인정 여부, 지정 게이지·제약 준수 여부, ranked 판만 인정할지.
- 게이지 모드 선택지: 'iidx / beatoraja / lr2' 세 가지를 설정으로 신설할지(iidx 게이지 수치의 원천 자료와 판정 의미 확인 필요), 아니면 지금처럼 코스 제약(gauge_lr2 등) + GAUGE SET(AUTO|LR2)을 유지하고 코스 전용 게이지 설정만 추가할지. 코스 기본값을 LR2 로 고정할지.
- course_best.assist_grade 항목(HANDOFF 6절 14번)과 연동: 코스에 게이지를 저장해 등급을 줄지, 열을 지울지.
- 클라이언트에서 코스를 가져오는 방법: 파일 직접 두기(현재) / 표 안에서 자동 생성 / 서버에서 내려받기 UI.

### external

#### A-1. Stella 난이도표 파일 구조와 필드 실측 (요구 4·6)

**현재 상태**

- 체인: https://stellabms.xyz/st/table.html 6행 `<meta name="bmstable" content="header.json">` -> https://stellabms.xyz/st/header.json -> 46행 `fetch(header.data_url)` -> https://stellabms.xyz/st/score.json (data.json 이 아니라 score.json). 확인: 2026-10-23 curl 로 3개 모두 HTTP 200, application/json.
- header.json 실제 최상위 키는 name="Stella", symbol="st", data_url="score.json", course 4개뿐. level_order, original_url, last_update 키는 없음(비교: beatoraja 기본 표 header_insane.json 에는 level_order, last_update, original_url 이 있음).
- header.course 는 그룹 1개에 코스 26개: "Stella Skill Simulator 4th st0"~"st12" 13개 + "STELLA技能訓練 st Boys / st ??? / st Girls / stMuscle / st Hexagon / st裏皆伝 / stMarathon / st「B」 / st overjoy- / st Other / stProfessional / stMaster / st JADONG" 13개. 24개는 md5 4곡, stMuscle 과 stMarathon 은 5곡. 코스 필드: name, constraint, trophy, md5(배열). 26개 모두 constraint=["grade_mirror","gauge_lr2","ln"], trophy=[silvermedal(missrate 5.0, scorerate 70.0), goldmedal(missrate 2.5, scorerate 85.0)] 동일.
- score.json 한 항목의 실제 값(첫 항목): {"id":6691,"md5":"9188a4c9876386173ba35158edf23a15","sha256":"da6e707a6a87ee7732f7d9b399b429904b4935c29e6e6c3c6defc75971969494","title":"#B2FFFF [SP Celeste Colored Strawberry]","artist":"tkqn14 mov. WIC / obj: Dignitas","url":"https://manbow.nothing.sh/event/event.cgi?action=More_def&num=94&event=146","url_diff":"https://stellabms.xyz/upload/4536","level":"0"}. 2350개 전체에서 키 집합은 id(정수), md5, sha256, title, artist, url, url_diff, level(문자열) 8개로 동일. name_diff, comment, lr2_bmsid 등은 하나도 없음. md5 중복 0개, url 빈 값 0개.
- 표시 페이지의 열: LV(`st`+level), TITLE(링크 https://ir.stellabms.xyz/charts/{md5}), ARTIST(url 이 있으면 링크), CHART(`url_diff` 가 비어 있지 않으면 DL 링크), SONG INFO(https://stellabms.xyz/song/{id}) (table.html 96·112-118행).
- HTTP 특성: cache-control public, max-age=60. ETag, Last-Modified, Access-Control-Allow-Origin 헤더는 응답에 없음(HEAD 확인). 즉 브라우저 교차 출처 fetch 불가, 서버 쪽 fetch 와 내용 해시 비교가 필요. score.json 840,194 바이트(비압축).

**관련 파일**: https://stellabms.xyz/st/table.html, https://stellabms.xyz/st/header.json, https://stellabms.xyz/st/score.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/st_table.html, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/header.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/score.json, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/TableDataAccessor.java

**차이와 위험**

- 이 표의 course 는 beatoraja 형식의 constraint/trophy 를 가진다. 표 파서가 data.json 이름과 level_order 에 기대면 깨진다(level 은 문자열 "0"~"12" 이고 파일 안에서 숫자 순서로 정렬돼 있음).
- beatoraja 가 이 course 를 읽는 방식은 TableDataAccessor.java:188-210(constraint 문자열을 CourseDataConstraint.getValue 로 변환, 모르는 값은 버림, trophy 의 name/missrate/scorerate 복사). R-BMS 가 같은 형식을 받으려면 grade_mirror, gauge_lr2, ln 이 코스 제약으로 필요하다.
- 한 곡이 여러 레벨에 들어가지 않는다는 보장은 md5 중복 0개로 확인했으나(2350개 기준), sub/rec/vote 표 사이에는 겹친다(아래 A-2).
- 표 HTML 에 'Please register this page with BeMusicSeeker.' 문구가 있다. 표를 도구로 등록하라는 뜻이며 이용 허락 문구는 아니다.

**결정 지점(조사 당시)**

- 어느 Stella 표를 기준으로 삼을지: table.html(정식 2350곡, 사용자가 준 주소) / table_rec.html(추천 137곡) / table_sub.html(전부 3247곡) 중 선택. 추천안: 사용자가 준 table.html(정식) 하나를 기본으로 하고 rec 는 선택 항목으로 후속.
- 표 안의 course 26개(Stella Skill Simulator 4th 등)를 R-BMS 코스 기능에서도 함께 가져올지(요구 8의 코스 모드와 연결): 가져온다 / 레벨 표만 가져온다.

#### A-2. 표 규모, 레벨, Satellite 등 관련 표, 갱신 주기 (요구 4·6)

**현재 상태**

- Stella 정식(table.html): 2350곡, 레벨 문자열 0~12(13개). 레벨별 곡 수: 0=236, 1=304, 2=283, 3=315, 4=278, 5=256, 6=177, 7=154, 8=126, 9=100, 10=66, 11=34, 12=21 (합 2350). 표시 기호는 header.symbol "st" 를 붙인 st0~st12.
- Stella 추천(table_rec.html -> header_rec.json -> score_rec.json): 137곡, 레벨 0~10. Stella 서브(table_sub.html -> header_sub.json -> score_sub.json): 3247곡, 레벨 0~12. 각 header 의 course 는 26개로 동일(길이만 확인, 내용 동일성은 확인 못함).
- Satellite 존재 확인: https://stellabms.xyz/sl/table.html 200, `<title>Satellite</title>`, meta bmstable=header.json. header.json: name "Satellite", symbol "sl", data_url "score.json", course 13개("Satellite Skill Analyzer 2nd sl0"~). sl/score.json 2497곡, 레벨 0~12(0=253, 1=277, 2=269, 3=222, 4=210, 5=176, 6=151, 7=148, 8=159, 9=144, 10=129, 11=170, 12=189), 필드는 Stella 와 같음.
- /link 페이지(https://stellabms.xyz/link)에 있는 다른 표 묶음: Stella(st), Satellite(sl), DP Stella(dpst), DP Satellite(dp), Supernova(sn), Solar(so), Starlight(sr), 폐지된 EXOPLANET(fr). 각 표마다 table.html(정식), table_rec.html(추천), table_vote.html, table_rec_vote.html, table_sub.html 5종이 있다.
- 갱신 주기(원문, https://stellabms.xyz/introduce 의 '난이도표의 갱신'): "관리자 또는 심사위원이 승인대기/거절대기를 진행한 스레드는 정기적으로 2주 간격으로 결과를 난이도표에 반영합니다. 갱신 후 2주일 뒤, 일요일 오후 12시에 결과를 반영합니다." 또 "제안 시 즉시 해당 난이도표의 Sub에 등록되며"(같은 페이지). 즉 정식 표는 2주 단위, Sub 표는 즉시 바뀐다.
- 서버가 CDN 캐시를 60초만 둔다(max-age=60).

**관련 파일**: https://stellabms.xyz/link, https://stellabms.xyz/introduce, https://stellabms.xyz/sl/table.html, https://stellabms.xyz/sl/header.json, https://stellabms.xyz/sl/score.json, https://stellabms.xyz/st/header_rec.json, https://stellabms.xyz/st/score_rec.json, https://stellabms.xyz/st/score_sub.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/txt_link.txt, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/txt_introduce.txt

**차이와 위험**

- 요구 6(매일 한국시간 0시 cron)은 과하다. 정식 표는 2주 간격(일요일 12시 반영)으로만 바뀐다고 사이트가 밝혔다. 다만 요구대로 매일 돌려도 무방하며, 요청 1회가 840KB 이므로 부담은 작다. ETag 와 Last-Modified 가 없어서 내용 sha256 비교로 변경을 판단해야 한다.
- 한국시간 0시는 UTC 15:00 전날이다(UTC cron 환경이면 변환 필요). 사이트 안내에서 '일요일 오후 12시'가 어느 시간대인지는 확인 못함(다른 공지에는 GMT+9 표기가 있음).
- 표를 새로 받을 때 md5 가 사라지거나 레벨이 바뀌는 곡이 생긴다(Stella 규칙: 레벨 변경, 삭제, 다른 표로 이동이 정기 반영됨). 서버는 이전 스냅샷과 비교해 삭제와 이동을 처리해야 한다.
- 사이트 점검 공지가 있다(예: 9/19 오전 0~1시 점검, https://stellabms.xyz/notice). 한국시간 0시 cron 은 이런 점검 시간대와 겹칠 수 있어 재시도가 필요하다.
- robots.txt 는 존재하지 않는다(https://stellabms.xyz/robots.txt 가 SPA HTML 200 을 돌려줌). API 이용 약관 문구도 찾지 못했다.

**결정 지점(조사 당시)**

- 갱신 주기: 매일 한국시간 0시 그대로(추천: 요구대로, 변경 없으면 저장 안 함) / 주 1회 / 월요일 한국시간 새벽만.
- Satellite(sl) 표도 함께 가져올지: 지금은 Stella 만 / Stella+Satellite 둘 다(Satellite 는 같은 형식이라 비용이 작음).
- sub 표(3247곡)까지 받을지: 정식만(추천) / sub 포함.

#### A-3. 'DL' 의 정체, 호스팅, 이용·재배포 문구 (요구 5·7)

**현재 상태**

- 'DL' 은 곡 패키지가 아니라 `url_diff`(차분 URL) 링크다: table.html 112-113행 `if (info[i].url_diff != '') ... <a href='${info[i].url_diff}'>DL</a>`. 곡 본체는 `url` 필드(사이트 표기 '곡 URL')이며 표에서는 ARTIST 칸에 링크로만 쓰인다(104행 부근). 곡 정보 페이지(https://stellabms.xyz/song/6691)에도 '곡 URL' 과 '차분 URL' 두 항목만 있고 내려받기 버튼이 없다.
- url_diff 분포(2350곡, 확장자·호스트 기준 분류): 빈 값 278, stellabms.xyz/upload/{id} 636(id 34~8276), 직접 압축파일(.zip/.rar/.7z/.lzh) 618, 직접 BMS 파일(.bms/.bme/.bml/.pms) 123, HTML 페이지 또는 클라우드(Dropbox, Google Drive, OneDrive, 개인 사이트 등) 627, bms.hexlataia.xyz 의 비압축 링크 68.
- 호스트 상위: stellabms.xyz 636, bms.hexlataia.xyz 457, darksabun.club 133, web.archive.org 88, akred.web.fc2.com 79, www.dropbox.com 73, absolute.pv.land.to 61, parksu.darksabun.club 51, drive.google.com 45, gnqg.rosx.net 44 (이하 다수의 개인 사이트).
- Stella 자체 호스팅: 있다. HEAD 로 확인한 https://stellabms.xyz/upload/4536 와 /upload/4782 는 200, content-type application/zip, content-disposition attachment; filename=004536.zip / 004782.zip. 사이트 '차분 업로더'(https://stellabms.xyz/uploader, 이 주소는 내 요청에서 HTTP 500 이었음)가 근거이며 소개 문구는 "자신이 제작한 BMS 차분을 업로드 하는 곳입니다. 별도로 가입하지 않아도 업로드 할 수 있습니다.".
- 호스팅되는 것은 차분(chart difference) 파일만으로 보인다: 미러 https://bms.hexlataia.xyz/mirror/gnqg-upload/01162.zip 은 19,385 바이트, .../04611.zip 은 37,969 바이트(HEAD content-length). 음원 포함 패키지 크기가 아니다. 곡 파일은 내려받지 않았고 압축 안의 내용 구성은 확인 못함.
- 곡 본체 출처(`url`)는 manbow.nothing.sh 이벤트 페이지, 개인 사이트 등 외부 페이지 링크다(예: 첫 항목). 패키지 일괄 다운로드나 API 는 없다. 외부 IR 링크 https://ir.stellabms.xyz/charts/{md5} 는 내 HEAD 요청에서 403.
- 원문 인용(https://stellabms.xyz/rules): "곡을 추가할 시, 올바른 곡 URL 및 차분 URL을 제시하여 주십시오." / "상업용 곡의 BMS, 무키음 BMS, osu convert 등 저작권에 위배되는 곡의 제안을 금지합니다." / "모든 사항에 대해서는 기본적으로 미디어를 보유하고 있는 개인/단체/기업 등의 가이드라인(2차 창작 가이드 또는 이용 약관)을 참고하는 것으로 합니다." / 차분 업로더 규칙: "자기자신이 만든 차분만 업로드해주세요. (단, 차분제작자가 허가한 경우 대리 업로드가 가능한 것으로 합니다.)", "저작권에 위배될수 있는 차분의 업로드를 금지합니다."
- https://stellabms.xyz/link 원문: "각 난이도표의 링크는 난이도표 내용을 따로 출력하지 않으며, 난이도표 내부의 곡 링크, 차분 링크는 곡 정보 페이지를 참고하거나, BeMusicSeeker 등에 등록 후 해당 프로그램에서 확인을 부탁드립니다."
- 표 JSON(score.json, header.json)의 재배포·가공 허락 또는 금지를 직접 밝힌 문구는 찾지 못했다(rules, introduce, link, notice, 첫 화면 확인).

**관련 파일**: https://stellabms.xyz/rules, https://stellabms.xyz/introduce, https://stellabms.xyz/link, https://stellabms.xyz/song/6691, https://stellabms.xyz/upload/4536, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/st_table.html, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/page_rules.html

**차이와 위험**

- 요구 5(곡이 없으면 다운로드 안내 모달)의 실제 다운로드 대상은 `url_diff` 뿐인데 이것은 차분이라 음원이 들어 있지 않다(미러 zip 19~38KB). 곡이 로컬에 없을 때 `url_diff` 만 내려받아서는 플레이할 수 없다. 음원 패키지는 `url`(이벤트 페이지 등 외부 사이트 링크)에서 사람이 받아야 하고, 직접 다운로드 링크가 아닌 페이지가 많다(627+278 곡은 직접 파일이 아니거나 빈 값).
- 따라서 안정적인 자동 다운로드는 stellabms.xyz/upload/{id}(636곡)와 직접 압축파일(618곡) 일부로 제한된다. Dropbox, Google Drive, OneDrive 링크는 로그인·확인 화면 때문에 자동화가 불안정할 수 있다(확인 못함). 외부 링크는 사라진다(사이트 자체에 'URL 유실 신고' 게시판이 있고, 278곡은 url_diff 가 비어 있음).
- 요구 7(발광 전곡을 내 서버에 올려 Cloudflare 에서 배포)는 권리 문제가 가장 크다. 근거: Stella 규칙의 위 인용(저작권 가이드를 따름, 대리 업로드는 제작자 허가 시에만), GENOCIDE 운영 사이트 about.html 의 "またこちら側で入手不可能なbms、差分の保管は原則しません" 와 "各自保管し合い入手できる様にするのは自由ですが、作者の方に連絡取れるなら取ってからしましょう", GENOSIDE 패키지 페이지의 "BMSパッケージ化に許可頂いた作者様" 감사 문구(작자별 허가를 받아 패키지화했다는 뜻), BMS-IR 약관의 "解析譜面や、権利侵害のおそれがある楽曲・譜面データの登録・投稿・配布" 금지. 제작자별 허가 없이 R-BMS 서버에 올리면 이 관행과 어긋난다.
- Stella 가 호스팅하는 /upload/{id} 파일을 R-BMS 서버로 복제하는 것의 허락 여부는 사이트 문구에서 확인 못함.

**결정 지점(조사 당시)**

- 곡 다운로드 방식: (A, 추천) R-BMS 가 파일을 호스팅하지 않고 곡 정보에 `url`/`url_diff` 외부 링크를 열어 주는 안내 모달 / (B) stellabms.xyz/upload/{id} 와 직접 압축 링크만 앱이 받기 / (C) Cloudflare 에 자체 미러.
- 요구 7 자체 미러: 작자·차분 제작자 허가를 받을 수 있는 범위(예: GENOSIDE 패키지처럼 허가를 받은 묶음, 또는 본인 제작물)에 한정할지 / 허가 없이 전곡을 올리지 않기(추천) / 그래도 올리기(법적 책임이 사용자에게 있음).
- 곡이 없을 때의 UI 동작: 모달에 외부 링크만 열기 / 차분만 내려받고 음원은 안내 / 다운로드 폴더 지정 후 자동 압축 해제까지(자동 해제는 보안상 별도 설계 필요).

#### B-1. GENOSIDE 2018 SP 단위인정 22개: 이름, 4곡 제목, md5 (요구 8)

**현재 상태**

- https://www.bms-ir.org/new/dan?table=genocide2018 은 JavaScript 가 그리는 페이지가 아니라 서버가 HTML 표를 직접 내려준다(HTTP 200, cache-control no-store). 별도 JSON 주소는 없다. 같은 페이지가 주는 내려받기: 개별 `/new/course/download?hash={160자 hex}`(.lr2crs, 396바이트), 표 전체 `/new/dan/download.zip?table=genocide2018`(10,329바이트, 22개 .lr2crs). 이번 조사에서 22개 코스 페이지를 모두 1초 간격으로 가져와 4곡씩 읽었고(22/22 확인), 각 코스 해시의 md5 4개와 페이지 곡 md5 가 모두 일치했다.
- 코스 해시 구조: 32자 접두사 `00000000002000000000000000005190` + md5 32자 x 4 = 160자(22개 모두 길이 160, 접두사 동일).
- 단 이름(Course ID): ☆01 初段 11120, ☆02 二段 11119, ☆03 三段 11118, ☆04 四段 11117, ☆05 五段 11116, ☆06 六段 11115, ☆07 七段 11114, ☆08 八段 11113, ☆09 九段 11112, ☆10 十段 11111, ★01 発狂初段 11110, ★02 発狂二段 11109, ★03 発狂三段 11108, ★04 発狂四段 11107, ★05 発狂五段 11106, ★06 発狂六段 11105, ★07 発狂七段 11104, ★08 発狂八段 11103, ★09 発狂九段 11102, ★10 発狂十段 11101, ★★ 発狂皆伝 11100, (^^) Overjoy 11099. 전체 제목은 모두 "GENOSIDE 2018 段位認定 {이름}".
- 4곡 (제목=md5, 곡 순서대로):
☆01 初段: Agito=ae11930223a8f24e91633f1a7864e1eb / Beat Cannon (KHTP Remix)=cb661ee05c67990012bfd12703ba656c / ZetsueN (Radio Edit)=328dbf235f3b6cb2791e71bfb62951f8 / Scaffold=7c6ca04aa14d07b2c081f4dd35432844
☆02 二段: Reach to the sky=56528c5f77d140bebf5c3b7070471808 / さいこ゛のたたかい / たひ゛のおわり=d3ed28d0958cd17a93635c6a5604d0aa / Arithmetic of Sounds=daab2e157ac86ff4374ffff00a129f85 / Asteria=a0aac381114f31691c16b864a401d926
☆03 三段: 夏色の舞う蒼色のセカイ=ad6a62789520c5591c37dada7a063892 / GOODBOUNCE=fba939901ecf81d6d0ffe6c605f17676 / The Formula=d89ea641d2ae9502d43f6905a5ede5a0 / DESPERATE=29ff7e29c35b7d72cb1a7437fc09b36d
☆04 四段: Heterochromia Iridis=935ac8a2d9a3e2307a6e0206f87c22ad / Lonely Cat=b177143ae68115a0702db737a7e7c504 / EUROBEATアイドル橘花音のテーマ=8e17f96a8440ff15923bfe228409c9b6 / SUNRISE BEACH -暁星の雫-=6dddd7873ce15935d891730e6f17b992
☆05 五段: 空想都市トーキョー=c3912d126e1af67d32d236397a7510cb / Blooming=dbf71bc8344d4fd406e154e599c5c27c / Aeventyr=2e962e84f9ceeb87535fe50a382a380a / Last Feud ~the period~=d2f44370fe455dc1fbffd8c5c6505e77
☆06 六段: 君の記憶、風の唄=33c6dd9cfe024f2600c2cfe6ae42df43 / Mirage Imitation=c72d51c6d648ea7b5863ca3bfe52a66c / GO BACK 2 YOUR RAVE=f2ef13a310d8d500b918c5126e13ccb1 / アリスの空虚で愉快な人形劇=52650f7ca6b989917f095f0635c71da4
☆07 七段: ホーリーサンバランド=4d87528933213d3427ee870c71791538 / Larix (BMS Edit)=761863c63ecfc1a579f87be40632cce1 / Castorpollux=af07aa8958ee69c4b1ba0876adab86ea / Laplace=e9d2bc9465b6172c27df0d7da71f10d6
☆08 八段: 88D=07193291a7222c7a1d1f542b6c081953 / BLAZES ASSAULT=cb95efd400d207ab019bb481c508060a / Fly Again (bms edit)=86030ae0cb6945ba653176e021fca4fd / Evanescent=5dc2ecda7d5a5615ed10a132ac3cd4bd
☆09 九段: ラケシスと永遠の祈祷者=18aa482e76edfda904b3cf3d646c59bf / Precious!!!=07baa34abf34131ee20ab40b70354037 / Gamegame=7f31675fdb02c22e392e5b98de1baec4 / Shooting Silver Bullet=77dd6afb7bfb291774fbdcf5aa0db282
☆10 十段: AirTrip-SF#2-=f14357d72c2c11081fd45dc00aa4cffa / 絵空事の世界と私=ce73f117ef9c4a3ff40fb8069e1c3dda / Between Sanity and Insanity=2356c21a671f059293a24d4cf14f0db2 / Blink=5f6f1c5d3fca5edba8edd537d9927c61
★01 発狂初段: SOLROS=f872dd65dd08638b06d80470a3233fb9 / Calamity Fortune=1b72e8f6439a698e78f94be16470b789 / TURNDOWN=2371263af3b0d644fba62526c9f49481 / Dstorv=8a8a6c2f3511eb0876a6c9a2027d7bbe
★02 発狂二段: Dai-katsugeki〔AMAZE Remix〕=d70e2549e841819708d279d2478bb1e2 / Engine=5e3711f2f576f3929156c9594b87efac / YAKUI=a56ddb2169246817fb7527363fb9c687 / Meteor Lights=cefb4735c26f9488438021b4d7fdca31
★03 発狂三段: Trahison [E]=19fbf62d711282d81208f97f7135e2a2 / Danse de Romani [Professional]=3791d42c00cdfa135df3779eb0f78505 / UnFaked phantOm=b46b23fa8e2ad96e02763f687bb5a494 / CO5M1C R4ILR0AD [ON3 M0RE 3XTR4]=efae90d527a71d753ea7ad2847ec2006
★04 発狂四段: 私のヒトミ=47de06762f16d164426e3197c5de967c / suffruti=a718de9a2cd2f88deafbd40f5929abb4 / Jack o' Lantern=5077cd14b4759d0e8d61e854eac4e88a / Le Reve=c87682922d1b87622b34fe7a7ffba52c
★05 発狂五段: Y=e9fc0fbbde68bfd10f2d289751a1fc6a / Infinite Galaxy=3a500560e6ace69e08b51736ff1be0ca / SCARY BANQUET=8a8019d432ab4dd9bde4e65faf1b663f / Toki (Sound piercer's Timesucker Remix)=7fd4d1b5a767fd97343129157782a95f
★06 発狂六段: IKAROS=f96cf3af3d9356407bff483d27f4aecc / Fairytale=a29f9500cebd16cbb8d14b69ace79db1 / Wizdomiot=871c6aed53c5dd3ac35686d9a92af4f4 / 先天性エゴサ依存症候群=40673cc8ea628a9452f6ae9ac74d5817
★07 発狂七段: Inherited World=a4b59ca055a856b9a200ccfd1bf5c7bc / We are the xxxx=a0f48e96286e0ce275a33fd8cb851ff0 / ATHAZA=c4134c29746e1ece168159ba4ae88d45 / Dreadnought=67a76e7d26f3c3ea99fdc6f912f83021
★08 発狂八段: Lemuria=f0feb9654ae044c0e95ebf68e2497c2e / 現実をカットアップしてみた=51e0a29d5c7fc9cdf5e5e8eee1ad1124 / FIONA=1cf2e138abd7199d6e4656447cfaed6c / FUXXING YOUR BRAIN=8e0df85dbf59c4753346f1582e2d4422
★09 発狂九段: 硬核機動=603aae2c3681cc2562b9b53b50408c69 / ひつぎとふたご=79dc444b48299814462ff23a2bf3ab79 / hypocrite=ff5d7235ba643bc85b804a3df2778590 / CHMR=451ed38cdba0323388027129f5929645
★10 発狂十段: 多重極展開=d6fea1389a86fc14eb354fcc5e60e03e / Altale=e58e3f718d089ec37caa0f2c7149a54b / 3.14 (TT mix)=1e049d189ef9bef79824f55764517a58 / 天空の城ヴェリエル=fb975909eb0f60a9190c7a1007515d4b
★★ 発狂皆伝: Doppelganger=10420967a85371e65db57967e6c696cd / ダークサンバランド=ebc5946c3de24a01048d1c90cbd5c9d6 / キミとボクへの葬送歌=45446f6c96bdb081a2054b80cd8f7208 / Love & Justice=39d0732ef7fee9ad0c8b044ccbe8a396
(^^) Overjoy: Legend of Seeker=cfad3baadce9e02c45021963453d7c94 / リジットXXパラダイス=77d23be22b2370925c573d922276bce0 / 運命論=188a99f74ab71804f2e360dcf484545c / FREEDOM DiVE=c46a81cb184f5a804c119930d6eba748
- 같은 선택 목록에 DP 판이 따로 있다: table=genocide2018_dp(GENOSIDE 2018 DP段位認定), .lr2crs 22개(`<line>14</line>`). SP 는 `<line>7</line>`. 그 외 bms-ir 단위 표: new_generation_dan, ln3_dan, solomon_dan, starlight2023_dan, stella_skill_simulator_4th/3rd/2nd/1st, satellite_skill_analyzer_2nd/1st, dp_satellite_skill_analyzer_1st, scramble_dan, genocide2012, genocide2009, genocide2007, genocide_initial_0.
- 교차 검증: GENOCIDE 운영자 사이트의 공식 코스 파일 http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip(4,084바이트, Last-Modified 2018-04-15)에 코스 44개(SP 22 + DP 22)가 있고, 해시가 bms-ir 의 SP 22개·DP 22개와 모두 같다(44/44). beatoraja 기본 난이도표(miraiscarlet.github.io/bms/table/genocide_normal, genocide_insane)의 'GENOSIDE 2018' 코스 22개의 md5 목록도 위 22개와 모두 같다(22/22).

**관련 파일**: https://www.bms-ir.org/new/dan?table=genocide2018, https://www.bms-ir.org/new/dan/download.zip?table=genocide2018, https://www.bms-ir.org/new/dan?table=genocide2018_dp, http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip, https://nekokan.dyndns.info/~lobsak/genocide/grade.html, https://miraiscarlet.github.io/bms/table/genocide_insane/header_insane.json, https://miraiscarlet.github.io/bms/table/genocide_normal/header_normal.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/genocide_courses.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/dan.html, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/crs_zip/x/GENOSIDE_2018, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/crs_official/x/test.lr2crs

**차이와 위험**

- 요구 8의 '발광 제노사이드 표의 모든 단위를 정확히 가져오기'는 위 22개(SP)와 DP 22개로 끝난다. 확인한 코스는 SP 22개 전부, DP 는 해시 목록 대조만 했고 DP 의 곡 제목과 md5 개별 확인은 하지 않았다(확인 못함).
- 코스 하나는 곡 md5 4개의 순서 있는 목록이다. 서버에 등록할 때 곡 순서가 의미를 가지므로 순서를 보존해야 한다. bms-ir 의 곡 페이지(/new/song)는 robots.txt 가 Disallow 라서 곡 제목은 코스 페이지의 표기를 쓰되 대량 호출하지 말 것.
- 등록한 코스를 플레이하려면 이 md5 곡 파일이 모두 로컬에 있어야 한다. 발광 패키지는 2018 StarterPackage(6.16GB, 159곡)와 Append(125MB)로 배포되며, 일부 코스는 이전 판 GENOSIDE-StarterPackage 도 필요하다(공식 페이지 문구: "*一部前回のGENOSIDE-StarterPackage-が必要になります"). 예: 皆伝 4번째 'Love & Justice' 와 Overjoy 4번째 'FREEDOM DiVE' 는 공식 2018 곡 목록(159곡)에 보이지 않는다(제목 대조, 완전한 대조는 아님).
- 일부 곡 제목은 bms-ir 표기와 공식 표기가 다르다(예: 공식 'Agito', 'Laplace' 는 각각 같으나 'ひつぎとふたご' 같은 곡은 공식 곡 목록에서 제목이 달라 보임). md5 를 키로 쓰고 제목은 표시용으로만 써야 한다.
- 인정 단위 이름(初段, 発狂皆伝 등)을 사용자 정보에 넣으려면 클리어 판정이 필요한데, 이를 위해서는 B-2 의 게이지와 제약(미러만 허용 등)이 정해져야 한다.

**결정 지점(조사 당시)**

- DP 단위(22개)도 등록할지: SP 22개만(추천, R-BMS 가 7키 중심이면) / DP 포함(14키 클리어 지원이 있어야 의미 있음. 클라이언트 DP 지원 여부는 이번에 확인하지 않음).
- 가져오는 방식: 공식 코스 파일(GENOCIDE 운영자 2018_course.zip, 추천: 출처가 운영자) 파싱 / bms-ir 22개 페이지 파싱 / 정적 JSON 으로 코드에 내장. 세 출처의 해시가 같음을 확인했으므로 기능 차이는 없음.
- 곡 제목과 이름 표기: 일본어 원문 유지 / 한국어 병기.

#### B-2. 단위인정의 게이지·제약 정보가 어디에 적혀 있는가 (요구 8: 게이지)

**현재 상태**

- bms-ir 의 dan 페이지와 코스 페이지에는 게이지 종류·옵션 제약 항목이 없다. 코스 페이지(예: /new/course?hash=...&view=both)의 문구는 출처·LN 모드("現在の表示設定: 新旧表示=新旧混合 / 表示機種=LR2のみ / ロングノート=LN")와 곡 4개, 랭킹 표뿐이다. 랭킹 표의 'HARD', 'FULL COMBO' 는 플레이어가 낸 램프이지 코스 제약이 아니다.
- .lr2crs 에는 `<title>`, `<line>`(SP 7, DP 14), `<hash>`, `<type>`(내가 본 bms-ir 파일 22+22+13+12개, 공식 44개 모두 2)만 있다. 게이지·미러·랜덤 제한을 나타내는 필드가 없다. `<type>2` 의 의미는 확인하지 못했다(LR2 파일 형식 명세를 찾지 못함).
- 반면 beatoraja 용 표 형식(header.json 의 course)에는 제약이 명시돼 있다. 발광 표 beatoraja 기본 설정(Config.java:192-193 의 miraiscarlet.github.io)의 header_normal.json 31개, header_insane.json 49개 코스 전부 constraint=["grade_mirror","gauge_lr2","ln"]. 'GENOSIDE 2018' 코스 22개도 동일. 트로피: ☆(normal) 쪽은 bronzemedal(missrate 7.5, scorerate 0.0), silvermedal(5.0, 70.0), goldmedal(2.5, 85.0); ★(insane) 2018 12개는 silvermedal(5.0, 70.0), goldmedal(2.5, 85.0)만. Stella 쪽 header.json 코스 26개도 같은 constraint 이다(A-1).
- 제약 값의 뜻(beatoraja CourseData.java): grade_mirror(type 0: 단위, 미러 허용), gauge_lr2(type 3: LR2 게이지 사용), ln(type 4: LN 모드 고정). 적용 코드 MusicSelector.java:470-505: MIRROR 이면 랜덤 옵션이 미러(1)일 때만 유지하고 아니면 랜덤 해제, LN 은 lnmode 0 으로 고정. isClassCourse 는 CLASS, MIRROR, RANDOM 중 하나가 있으면 참(CourseData.java:84-91).
- 사이트 안내: beatoraja QA(https://mirai-yokohama.sakura.ne.jp/bms/beatoraja/) "ここで設定されている段位認定はLR2段位ゲージが設定されているため、減少量等はLR2と同じ感覚でプレイできます。(判定仕様が異なるため、完全にLR2と同一とはなりません)". iidx.org(https://iidx.org/misc/iidx_lr2_beatoraja_diff) 각주 3: "most DAN courses are tagged with gauge_lr2 to force enable beatoraja to use the LR2 DAN gauge instead."

**관련 파일**: https://miraiscarlet.github.io/bms/table/genocide_insane/header_insane.json, https://miraiscarlet.github.io/bms/table/genocide_normal/header_normal.json, https://iidx.org/misc/iidx_lr2_beatoraja_diff, https://mirai-yokohama.sakura.ne.jp/bms/beatoraja/, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/Config.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/CourseData.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/select/MusicSelector.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/TableDataAccessor.java, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/mirai_header.json, /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/web/mirai_normal_header.json

**차이와 위험**

- .lr2crs 만 가져오면 게이지·미러·LN 제약이 사라진다. 코스 정의에는 R-BMS 가 따로 gauge=LR2, 미러 허용, LN 고정을 부여해야 한다. 근거 출처는 beatoraja 기본 표의 constraint(발광 2018 코스 22개 확인).
- 사용자가 말한 '비토라자(beatoraja)와 LR2 코스 게이지가 비슷할 것'은 수치로는 같지 않다(C 항목 표). 다만 beatoraja 에서도 발광·Stella 코스가 gauge_lr2 로 태그돼 실제 플레이는 LR2 단위 게이지로 돌아간다. 따라서 기본값을 LR2 로 두는 것은 beatoraja 와도 동작이 같다.
- bronzemedal 이 있는 코스(☆)와 없는 코스(★)가 섞여 있어, 트로피 등급을 단일 규칙으로 가정하면 안 된다. trophy 의 missrate/scorerate 가 어떤 비교인지(미스율 이하, 점수율 이상 등)는 beatoraja 소스에서 평가하는 위치를 찾지 못해 확인 못함(TableDataAccessor.java:205-206 는 값 복사만).
- `<type>2` 와 `<line>` 외의 LR2 코스 속성(LR2 에서 코스 게이지를 어떻게 정하는지)은 확인 못함.

**결정 지점(조사 당시)**

- 코스 데이터에 제약을 어떻게 둘지: beatoraja 형식 그대로 constraint 배열(grade_mirror, gauge_lr2, ln) 저장(추천) / R-BMS 전용 필드로 정리.
- 트로피(은·금 메달)를 단위 판정이나 사용자 표시에 쓸지: 단위 취득은 '클리어(게이지 0 초과로 종료)'만으로 하고 트로피는 후속(추천) / 트로피까지 구현.

#### B-3. 출처 등록과 이용 조건 (요구 8: 출처 등록)

**현재 상태**

- 원 출처(코스를 만든 쪽): GENOCIDE(GENOSIDE) 운영 사이트 https://nekokan.dyndns.info/~lobsak/genocide/grade.html . 그 페이지 안에 'コースファイル(LR2IR用)' 링크 http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip 이 있고(4,084바이트, 44코스), 문구 "BMS段位認定の為の全159曲のパッケージです / SP/DP/SP発狂/DP発狂に対応 / PackageとAppendを導入する事で段位認定全44コースプレイ可能". 연락 경로: https://nekokan.dyndns.info/~lobsak/genocide/ (Contact), Twitter @GENOCIDE5000, BBS http://jbbs.livedoor.jp/game/39018/ . 저작권 표기: "©GENOCIDE 2013 All rights reserved".
- 이 페이지가 밝히는 패키지 배포 방식: GENOSIDE -BMS Starter Package 2018-(6.16GB)은 Amazon Drive 공유(https://www.amazon.co.jp/clouddrive/share/bsf6KACRFzwy51MJ2SFSP0FWBmHkXMQZwfuMcGosLZ3)와 Google Drive 미러, Append(125MB)는 Amazon Drive 와 운영자 사이트(http://nekokan.dyndns.info/~lobsak/genoside/GENOSIDE2018_SP_append.zip). 감사 문구: "BMSパッケージ化に許可頂いた作者様 / 差分作者様". 한국어 요약: 곡·차분 작가의 허가를 받아 패키지화했다.
- bms-ir.org 는 사용자 질의의 URL 이지만 비공식 사이트다. 약관(https://www.bms-ir.org/terms, 규약 버전 2026-06-13-1): "本サービスは、LR2の旧インターネットランキング（LR2IR）の終了を受け…個人が運営している非公式の互換βサービスです。旧LR2IRの公式後継ではありません。" 금지 사항에 "サーバーやネットワークへ過度の負荷をかける行為" 와 "解析譜面や、権利侵害のおそれがある楽曲・譜面データの登録・投稿・配布" 가 있다. 코스 데이터의 재배포 허가·금지 문구는 약관에 없음. robots.txt(https://www.bms-ir.org/robots.txt)는 "Disallow: /archive/", "Disallow: /new/song", "Disallow: /new/client_view" (본문에 \n 이 글자 그대로 들어 있는 형태로 보였음). /new/dan 과 /new/course 는 Disallow 에 없음. 이 조사는 /new/dan, /new/course, 다운로드 zip 외에 /new/song 은 호출하지 않았다.
- bms-ir dan 페이지의 '出典 新/旧/両方'는 코스의 출처가 아니라 랭킹 기록의 출처(새 BMS-IR 기록과 옛 LR2IR 보관 기록) 선택 필터다(표의 '出典' 열이 旧/新 배지).
- 다른 후보 출처: beatoraja 기본 난이도표 제공자 miraiscarlet.github.io (header 의 original_url 이 http://nekokan.dyndns.info/~lobsak/genocide/normal.html 과 insane.html 을 가리킴), GitHub MsrLab-org/LR2IRCourses (README 에 라이선스·재배포 문구 없음, 약 75개 .lr2crs).
- GENOCIDE 사이트 about.html 의 안내: 표 자체는 LR2IR 정보를 가져와 만든 것이며 "更新はLR2IRに負荷が掛かる為二ヶ月〜三ヶ月に一回"; 책임 한정: "リンク先で生じた問題は一切責任を負いません".

**관련 파일**: 

**차이와 위험**

- 출처 필드에 쓸 값은 '원 제작자 = GENOCIDE 운영(lobsak), 입수 경로 = 운영자 공식 코스 파일(해시 44/44 일치), 검증 = bms-ir 와 beatoraja 기본 표'로 나눠 적는 편이 정확하다. bms-ir 는 비공식 호환 서비스이고 약관상 출처로 쓰는 것을 금지하지는 않지만 안정성(서비스 변경·중단 가능)을 보증하지 않는다(약관 '現状有姿で提供').
- 코스 파일 자체의 재배포 허가 문구는 찾지 못했다. 공식 사이트는 코스 파일을 직접 내려받게 하고 있으나 라이선스는 명시하지 않아, 서버에 보관할 때는 출처 URL, 가져온 날짜, 해시를 함께 저장하고 요청이 오면 삭제할 수 있는 형태가 안전하다. 법적 판단은 확인 못함.
- 패키지(곡 파일 6.16GB)를 R-BMS 서버에 올리는 것은 위 '작가별 허가' 구조와 충돌한다(요구 7 참고: A-3).
- bms-ir 에서 호출한 요청 수는 약 30회(코스 페이지 22개, zip 5개 내외, 기타 몇 개)로 모두 1초 이상 간격을 뒀다. 자동 갱신 작업을 만들 때는 약관의 '過度の負荷' 금지에 맞춰 호출 수를 제한해야 한다.

**결정 지점(조사 당시)**

- 서버에 저장하는 출처 항목: (추천) 제작 주체, 입수 URL, 가져온 날짜, 해시 / 제작 주체와 URL 만.
- 운영자(GENOCIDE)에게 코스 데이터의 앱 내 사용을 미리 알리거나 허락을 받을지: 먼저 연락한다 / 출처 표기만 하고 진행(외부 게시나 연락은 초안 승인 뒤에만 가능).

#### C. 단위인정(코스) 게이지 사양 비교: LR2 / beatoraja / IIDX (요구 8: 게이지 모드)

**현재 상태**

수치 순서는 PG, GR, GD, BD, PR(미스 POOR), 빈 POOR(MS). 7키 기준, 단위는 게이지 %.

[1] LR2 단위 게이지 (beatoraja 의 gauge_lr2 코스 게이지 = LR2 판 CLASS)
- GaugeProperty.java:123 CLASS_LR2: 증가 PG +0.10, GR +0.10, GD +0.05 / 감소 BD -2, PR -3, MS -2, min 0, max 100, 시작값 100, 클리어 경계 0(남아 있으면 클리어), modifier 없음(TOTAL 보정 없음). 구간 보정 guts {30, 0.6}: 게이지가 30 미만이면 감소량 x0.6(BD -1.2, PR -1.8, MS -1.2). 증가량에는 보정 없음.
- 근거 일치: https://ralba-gear.hateblo.jp/entry/2023/11/06/141139 '段位ゲージ' 표 0.1/0.1/0.05/-2.0/-3.0/-2.0 와 감소 보정(30,20,10% 에서 0.6배); https://iidx.org/misc/iidx_lr2_beatoraja_diff 의 LR2/DAN 행 "0.1 0.1 0.05 -2 -3 -2 Yes" 와 'DAN gauge adjustment' 표(30 미만 -1.2/-1.8/-1.2). 두 사이트 값이 beatoraja 코드와 같다(beatoraja 가 LR2 값을 이식한 것).
- 같은 LR2 세트의 코스용 상위 게이지: EXCLASS_LR2 (124행) 0.10/0.10/0.05/-6/-10/-2 + guts {30,0.6}; EXHARDCLASS_LR2 (125행) 0.10/0.10/0.05/-12/-20/-2, guts 없음.
- LR2 본체의 시작값·곡 간 이월·사망선은 LR2 가 비공개 소스라 직접 확인 못함. 확인된 것은 beatoraja 이식 코드(시작 100, 곡 간 이월, 0 이하 사망)와 iidx.org 비교표뿐이다. iidx.org 는 "LR2oraja 최신 릴리스에 따르면 LR2 감소 보정은 30%가 아니라 32%에서 시작하고 2%에서 스테이지 실패. 다른 출처로 확인되지 않음" 이라고 적었고, ralba 는 "LR2HARDでは2を下回ると失敗になるらしい？orajaは0で失敗" 라고 적어 LR2 사망선이 2%일 가능성이 있으나 미확인.

[2] beatoraja 기본(모드별) 코스 게이지: 코스에 gauge_* 제약이 없을 때 쓰는 것 (GrooveGauge.java:123-158, BMSPlayerRule.java:13-21)
- 7키(GaugeProperty.java:93-95): CLASS 0.15/0.12/0.06/-1.5/-3/-1.5, guts {5:0.4, 10:0.5, 15:0.6, 20:0.7, 25:0.8}(값 미만에서 곱함); EXCLASS 0.15/0.12/0.03/-3/-6/-3, guts 없음; EXHARDCLASS 0.15/0.06/0/-5/-10/-5, guts 없음. 모두 시작 100, min 0, max 100, 경계 0, modifier 없음. (iidx.org 의 raja/EX-DAN 0.15 0.12 0.03 -3 -6 -3 은 코드와 같고, raja/EXHARD-DAN 의 GR 값은 iidx.org 가 0.08 이라 적었으나 코드는 0.06: 코드를 따름.)
- 5키(83-85행): CLASS_5 0.01/0.01/0/-0.5/-1.0/-0.5, EXCLASS_5 0.01/0.01/0/-1/-2/-1, EXHARDCLASS_5 0.01/0.01/0/-2.5/-5/-2.5. PMS(103-105행): CLASS_PMS 0.15/0.12/0.06/-1.5/-3/-3 등. 키보드(113-115행): CLASS_KB 0.2/0.2/0.1/-1.5/-3/-1.5 등.
- 사용자 게이지 설정이 코스 게이지로 바뀌는 규칙(GrooveGauge.java:160-166, 164행 `id = type <= 2 ? 6 : (type == 3 ? 7 : 8)`): ASSIST EASY, EASY, NORMAL 설정 -> CLASS, HARD -> EXCLASS, EXHARD 이상 -> EXHARDCLASS. 코스 제약이 GAUGE_LR2 이면 LR2 세트, GAUGE_5KEYS/7KEYS/9KEYS/24KEYS 이면 해당 세트(GrooveGauge.java:128-147, CourseData.java 의 제약 정의).

[3] 곡 사이 이월·사망 (beatoraja)
- 새 곡을 시작할 때 GrooveGauge.create(model,type,resource)(GrooveGauge.java:123-158)가 PlayerResource 에 저장된 직전 곡의 게이지 기록 마지막 값으로 모든 게이지 슬롯을 세팅(151-155행): 이월한다. 첫 곡은 시작값(100).
- Gauge.setValue(GrooveGauge.java:218-222): 현재 값이 0 이하이면 더는 값을 바꾸지 않는다(한번 0 이 되면 고정). 감소 보정은 update(229-240행)에서 inc<0 일 때만 guts 적용.
- 사망: 게이지 0 이면 STATE_FAILED(BMSPlayer.java:655; 게이지 자동 이동이 NONE 일 때). 곡 클리어 판정은 `state != STATE_FAILED && gauge.isQualified()`(BMSPlayer.java:864)이며 isQualified 는 `value > 0 && value >= border`(GrooveGauge.java:246-248), CLASS 계열 border 는 0 이므로 0 초과로 끝나면 클리어. 실패한 판의 게이지 로그는 0 으로 채워져 저장(BMSPlayer.java:722-730).

[4] IIDX AC 단위 게이지 (https://iidx.org/compendium/gauges_and_timing 표, https://iidx.org/beginner/gauge)
- DAN: PG +0.16, GR +0.16, GD +0.04 / BD -1.5, PR -2.5, MS -1.5, 게이지 30% 이하에서 BAD·POOR 감소 절반(-0.75/-1.25/-0.75). 시작 100%. 곡 간 이월("The gauge amount carries over from one stage to the next."). 실패선은 두 페이지가 다르다: compendium 은 "you start at 100%, and get a stage fail if you drop to 0%", beginner 는 "fail out if it falls below 2%".
- EX-DAN (VEFX 로 전환): 0.16/0.16/0.04/-3.0/-5.0/-3.0, 30% 보정 없음.
- 본 확인 못함: iidx.org 외 공식 자료(atwiki 등 확인 시도는 HTTP 403 으로 열리지 않음).

[5] 차이 요약 (7키, PG/GR/GD/BD/PR/MS)
- LR2 DAN: +0.10/+0.10/+0.05 / -2/-3/-2, 보정 30 미만 x0.6 한 단계.
- beatoraja 7K CLASS: +0.15/+0.12/+0.06 / -1.5/-3/-1.5, 보정 5 단계(5,10,15,20,25 미만에서 0.4~0.8배). 증가량이 더 크고 BAD·빈 POOR 가 가볍다.
- IIDX DAN: +0.16/+0.16/+0.04 / -1.5/-2.5/-1.5, 30% 이하 x0.5 한 단계.
- 증가량: PG 는 IIDX 0.16 > beatoraja 0.15 > LR2 0.10. PG 와 GR 이 같은 값인 것은 LR2·IIDX, beatoraja 는 GR 이 0.12 로 낮다.
- 감소량: BAD 는 LR2 가 가장 무겁고(-2), beatoraja·IIDX 는 -1.5. POOR 는 beatoraja·LR2 가 -3, IIDX 가 -2.5. 빈 POOR 는 LR2 -2, beatoraja·IIDX -1.5.
- 사망선: beatoraja 0 초과 유지, IIDX 0%(compendium) 또는 2% 미만(beginner), LR2 미확인(2% 가능성).

**관련 파일**: /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/GaugeProperty.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/GrooveGauge.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/BMSPlayer.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/BMSPlayerRule.java, /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/CourseData.java, /Users/hyunseokbyun/development/R-BMS/crates/rbms-judge/src/gauge.rs, /Users/hyunseokbyun/development/R-BMS/crates/rbms-judge/src/gauge_tables.rs, https://iidx.org/misc/iidx_lr2_beatoraja_diff, https://iidx.org/compendium/gauges_and_timing, https://iidx.org/beginner/gauge, https://ralba-gear.hateblo.jp/entry/2023/11/06/141139

**차이와 위험**

- R-BMS 는 이미 beatoraja 와 같은 9슬롯 게이지(GaugeIndex::Class=6, ExClass=7, ExHardClass=8, gauge.rs:20-63)와 LR2 세트를 갖고 있다. gauge_tables.rs:15 `GUTS_LR2 = [(30.0, 0.6)]` 이 beatoraja 의 LR2 보정과 같고, gauge_tables.rs:23-24 주석은 "[`GaugeSetId::Lr2`] is never picked that way because the reference reaches it only through a course constraint ... so this engine exposes it as a user setting" 이라고 적고 있다. 즉 요구 8의 'LR2 기준 + 설정에서 변경'은 LR2 세트를 코스에 연결하는 문제가 중심이다. 이번 조사에서 설정 화면에 그 항목이 실제로 있는지, 코스 플레이 경로(곡 간 이월, 이월 실패)가 어디까지 구현돼 있는지는 코드 실행 없이 읽지 않아 확인 못함.
- R-BMS 에는 IIDX 세트가 없다(GaugeSetId 는 FiveKeys, SevenKeys, Pms, Keyboard, Lr2 5종, gauge_tables.rs:41-47). 'IIDX 모드' 코스 게이지를 넣으려면 새 표(위 [4]의 DAN/EX-DAN 값)와 30% 구간 보정, 실패선(0 또는 2) 결정이 필요하다. 근거 자료는 iidx.org 한 곳뿐이고 두 페이지가 실패선에서 서로 다르다.
- 사용자 추측 '비토라자와 LR2 코스 게이지가 비슷'은 반만 맞다: 수치는 [5]처럼 다르지만, 발광·Stella 코스가 gauge_lr2 로 태그돼 beatoraja 에서도 LR2 값으로 돌아간다. 설정에서 'beatoraja 모드'를 고르면 7키 CLASS(수치가 LR2 와 다름)가 되어 단위 클리어 난이도가 LR2 보다 쉽거나(BAD) 어려워진다(PG 증가가 큼으로 이득). 어느 쪽이 쉬운지는 판 내용에 따라 달라 한 방향으로 단정하지 못함.
- guts 경계 차이: 코드는 `value < gut[0]` (엄격히 미만, GrooveGauge.java:233), ralba 표는 30 행을 0.6배로 표기. 정확히 30.0 일 때만 갈린다. 부동소수 값 비교이므로 테스트 영향은 낮음.
- beatoraja 가 코스 시작값으로 100 을 쓰는 것(init)은 코드에서 확인했지만 원본 LR2 단위 게이지가 시작 100%인지는 확인 못함. 단위 취득 판정을 '원본과 같게'라 말하려면 이 부분은 불확실.
- 판정·점수 의미를 바꾸는 변경은 사용자 확인이 필요하다는 작업 규칙: 코스 게이지 기본을 LR2 로 하는 것은 새 기능이므로 해당하지 않지만, 기존 코스 스테이지 저장을 CLASS 게이지로 옮긴 변경(HANDOFF 6절 14)과의 관계는 이번 조사 범위 밖.

**결정 지점(조사 당시)**

- 코스 게이지 모드 기본값: LR2(추천: 발광·Stella 코스가 gauge_lr2 태그이고 R-BMS 에 LR2 세트와 GUTS_LR2 가 이미 있음) / beatoraja 7키 CLASS / IIDX.
- 설정으로 고를 수 있는 모드 목록: LR2 + beatoraja 7K 두 가지(코드 값이 확실, 추천) / IIDX 추가(자료가 iidx.org 한 곳, 실패선 불일치) / 코스의 constraint 가 지정하면 그것을 우선하고 설정은 constraint 없는 코스에만 적용.
- IIDX 모드를 넣는다면 실패선: 0% (compendium) / 2% 미만 (beginner 페이지).
- 사용자 게이지 설정(ASSIST EASY~EXHARD)과 코스 게이지 선택의 관계: beatoraja 식(NORMAL 이하=CLASS, HARD=EXCLASS, EXHARD 이상=EXHARDCLASS, 추천: R-BMS 가 이미 이 규칙을 가짐) / 코스 전용 단일 게이지.
