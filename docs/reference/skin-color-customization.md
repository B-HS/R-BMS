# 스킨 색·불투명도 조절 조사 (레인·노트 색, 부위별 불투명도, 스킨 세부 조절)

> 최종 갱신 2026-10-22 · 대응 커밋 0323bd9

사용자 요청 4(레인·노트 색상을 각각 조절, 불투명도 조절, 스킨 세부 조절 확대)를 구현하기 전에 기본 스킨 `rbms-default` 와 엔진이 색을 어떻게 다루는지 읽기 전용으로 조사한 결과다. 같은 탐색을 다시 하지 않도록 이름, id, 계산식, 값 목록을 적었다. 코드는 한 줄도 고치지 않았고, 근거는 `파일:줄` 이다. 문서와 코드가 다른 곳은 1.2절에 모았다. 원본(beatoraja)은 HEAD `8320241d`, 외부 스킨 ModernChic 은 읽기만 했고 내용을 복사하지 않았다.

## 0. 결론

1. 색이 PNG 에 구워진 부위는 노트 계열(일반 노트, LN 머리·꼬리·몸통, HCN, 지뢰)뿐이다. 레인 면, 레인 구분선, 마디선, 판정선, 레인 커버·리프트·HIDDEN+, 키빔, 폭발, 진행 막대, 화면 바탕은 Lua 가 destination 의 색(r,g,b,a)으로 입힌다(2절).
2. 엔진은 노트를 그릴 때 destination 색을 읽지 않고 알파만 남긴다(`crates/rbms-render/src/skin_render/notes.rs:131-133, 902-935`, beatoraja 의 `LaneRenderer` 가 `setColor(WHITE)` 로 초기화하는 것과 같다). 그래서 노트 색은 Lua 만으로 자유롭게 바꿀 수 없다. 방법은 엔진에 노트 색 입히기를 더하거나, 구운 색 묶음을 늘리는 것뿐이다(5.4절).
3. 키빔·폭발·펄스·`block`·`bar_*`·숫자 시트는 순백(255,255,255) 마스크라 곱셈 색이 어떤 RGB 든 정확히 나온다(3절 실측). 노트 시트는 순백이 아니다(흰 노트 `F1F4F8`, 그림자 행이 구워져 있다). 곱셈으로 바꾸려면 노트 시트를 순백 바탕으로 다시 생성해야 한다(3절).
4. 오프셋은 x, y, w, h, 각도, 알파만 바꾸고 RGB 는 바꾸지 못한다(`crates/rbms-skin/src/dst.rs:472-480, 563-570`). 사용자가 입력한 숫자를 색으로 받는 경로가 지금 없다. beatoraja 의 커스터마이즈 종류도 property, filepath, offset 셋뿐이다(5.2절).
5. Lua 는 본체 패스에서 `skin_config.option` 과 `skin_config.offset` 을 읽어 dst 색에 쓸 수 있다. 그러나 옵션 변경은 문서를 다시 읽고 오프셋 변경은 다시 읽지 않아서, 오프셋을 색 입력으로 쓰면 값이 다음 로드까지 반영되지 않는다(4.6절, 5.3절).
6. 이미 있는 것: 색 옵션 넷(Theme, Accent, Note Colors, Bomb Color), 부위별 불투명도 18행(오프셋 101~119 중 105 제외), JUDGE POSITION Y, 엔진 오프셋 15행, 모두 `settings.ron` 의 `skin.custom[문서경로]` 에 저장된다(4절).
7. 설계 대안은 여섯이다: A 기본 스킨 전용 Lua(로드 시 반영), B 엔진 수준 덮어쓰기, C 엔진 확장 + 기본 스킨 채택(실시간 반영, 메인 추천), D 가산 3층 혼합(엔진 변경 없음, 비추천), E 프리셋 확대, F 파일 변종(6절).
8. 결정이 필요한 것은 10개다(8절). 전부 메인 추천안대로면 C 안을 3단계로 구현한다(6.8절).

## 1. 조사 방법

### 1.1 읽은 것

| 구분 | 내용 |
| --- | --- |
| 기존 문서 | `docs/skin.md`, `docs/theme.md`, `docs/play-options.md`(7절 PLAY 메뉴), `docs/HANDOFF.md`, `docs/reference/skin-compat/README.md`, `b3-play-objects.md`(3.3.5절 노트), `docs/plan/2026-10-11-default-skin-design.md`(머리말) |
| 기본 스킨 | `assets/skins/rbms-default/lua/{theme,ui,parts,props}.lua`, `lua/play/{main,layout,field,info}.lua`, `lua/atlas/{play,common}.lua`, `tools/{generate-images,sprites}.py`, `tools/sections/{play,common}.py`, `image/{play,common}.png` |
| 엔진 | `crates/rbms-skin/src/{dst,model,loader,lua/env,loader/lua_skin,loader/from_lua,loader/track}.rs`, `crates/rbms-skin/src/model/objects.rs`, `crates/rbms-render/src/{lib,cpu}.rs`, `crates/rbms-render/src/skin_render/notes.rs`, `apps/rbms-player/src/gpu/mod.rs` |
| 앱 | `apps/rbms-player/src/{skin_select,number_row,skin_preview,skin_screen}.rs`, `skin_select/adjustments.rs`, `skin_host/{mod,play_timers}.rs`, `stage/{skin_menu,play_menu/rows}.rs`, `crates/rbms-config/src/schema.rs` |
| 원본 | `beatoraja/src/bms/player/beatoraja/{SkinConfig,skin/SkinHeader,skin/SkinObject,launcher/SkinConfigurationView,skin/json/JSONSkinLoader}.java`, ModernChic `Play/lua/sp/notes.lua`(note.dst 의 키만 확인) |
| 실측 | `target/release/rbms-cli skin-dump`(기본 팩 로드 시간), 캡처 테스트 `the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched`(scratch HOME, `RBMS_SKIN_CAPTURE_DIR`, 7K 프레임 확인), PNG 픽셀 덤프(Pillow 12.3.0 `uv run --with`) |

### 1.2 문서와 코드의 차이

`ea698a9` 이후 `assets/skins`, `crates/rbms-skin`, `crates/rbms-render`, 스킨 메뉴·PLAY 메뉴 코드를 바꾼 커밋은 없다(`git log ea698a9..HEAD` 에 문서 커밋 `e2c829f` 하나). 그래서 `docs/skin.md` 3.4절의 값(옵션 항목, 부위 id 18개, 색 값, 파일 65개)이 코드와 모두 일치했다. 확인한 일치 항목은 다음과 같다.

| 항목 | 문서 | 코드 |
| --- | --- | --- |
| 부위 불투명도 | 18행, id 101~119 중 105 없음 | `layout.lua:54-94` 에 18개, 105 없음 |
| 기본 팩 파일 수 | 65개 | `skin-dump` 의 "73 files" 에서 `tools/` 8개를 뺀 값 |
| SCRATCH AUTO 초록 | `#30E050`, 몸통 알파 0xA0 | `notes.rs:141-144` |
| 엔진 오프셋 | 15행 | `loader.rs:253-258` (4+1+5+5) |

차이로 적을 만한 것은 하나다. `docs/skin.md` 6절 "플레이" 행은 SCRATCH AUTO 노트에 "그림의 알파로 색을 입히는 셰이더가 렌더러에 없다" 고 적고 후속으로 남겼다. 이 조사의 노트 색 입히기(5.4절, 6절)도 같은 결핍에 걸린다.

## 2. 플레이 필드의 부위별 그리기

### 2.1 그리기 순서와 좌표

`play/main.lua:100-121` 의 `build` 순서다: `theme.apply(option)` → 바탕(`parts.backdrop`) → 정보 패널(`info.build`) → `field.under` → `field.over` → FAILED 덮개 → 페이드. `field.under` 는 레인 면, 구분선, 키빔, 노트 필드 순이고(`field.lua:349-354`), `field.over` 는 커버, 판정선, 폭발, 판정 글자, 알림 순이다(`field.lua:356-362`). 위에 그린 것이 아래 것을 덮는다. 좌표는 1920x1080 의 위쪽 기준 `{x, top, w, h}` 이고 `ui.lua:88-117` 의 `frames_of` 가 beatoraja 의 아래쪽 기준 y 로 바꾼다.

색이 dst 에 실리는 경로는 하나다. `ui.lua:88-117` 의 `frames_of` 가 `opts.color` 를 키프레임마다 `r`, `g`, `b` 로, `opts.a`(기본 255)를 `a` 로 적고, `Builder:rect`(`ui.lua:219-221`)는 엔진 내장 흰 점 이미지 `props.image.WHITE`(-111, `props.lua:20`)에 그 색을 곱한다. `s:image` 와 `s:number` 도 같은 경로이고, 그림이 흰 마스크이면 결과는 지정한 색 그대로다.

### 2.2 부위 표

레인 크기(1920 기준)는 `layout.lua:99-103`: 흰 건반 58, 검은 건반 46, 스크래치 100, 9키 버튼 62, 레인 사이 2px, Wide 는 1.5 배, 노트 두께 건반 16·버튼 24. 필드는 y 0~904(노트 영역), 904~912(판정선, `layout.FIELD`, `layout.lua:96-97`).

| 부위 | 구현 | 그리는 방식 | 색 출처 | 알파 조절(오프셋 id) | Lua 만으로 자유 RGB |
| --- | --- | --- | --- | --- | --- |
| 화면 바탕 | `parts.lua:101-103` | 화면 전체 `s:rect` | `theme.color.bg`(Mono `000000`, Slate `0C0F14`) | 없음(항상 100%) | 가능 |
| 레인 면 | `field.lua:95-101` | 레인마다 `s:rect`, 높이 0~912 | `lane.shaded and theme.color.bg or theme.color.panel`. shaded 는 검은 건반과 9키의 짝수 버튼(`layout.lua:280`). Mono 에서 bg `000000`, panel `161616` | Lane Background(101) | 가능 |
| 레인 구분선 | `field.lua:103-117` | 인접 레인 사이 2px 틈마다 `s:rect` | `theme.color.line`(Mono `3C3C3C`) | Lane Divider(102) | 가능 |
| 마디선 | `field.lua:148-153` | 필드 폭 x 2px `s:rect`, `note` 의 `group` | `theme.color.line`, 오프셋 LIFT(3) | Bar Line(108) | 가능 |
| 판정선 | `field.lua:159-171` | 8px 막대 둘(어두운 바탕 `accent_dim`, READY 타이머로 400ms 가운데에서 열리는 `accent`)과 위 16px 펄스 이미지 | 강조색(Accent 옵션: White `FFFFFF`, Sky `4CC9F0`, Amber `F2B33D`, Green `59D185`, Pink `F0689E`)과 어두운 짝(`theme.accent_dim`) | Judge Line(103) | 가능 |
| 키빔 | `field.lua:119-128` | `beam` 이미지(8x64, 아래로 갈수록 진한 알파 마스크)를 판정선 위 280px(y 624~904)에, 누르면 KEYON 타이머, 떼면 120ms 페이드 | `lane.tint`(`NOTE_TINT`, `layout.lua:110-119`) | Key Beam(104) | 가능 |
| 폭발 | `field.lua:207-219` | `bomb`(1024x128, 8프레임 고리, 128px) ADD 합성, 판정선 가운데 | `view.bomb_tint or lane.tint`(`layout.lua:356`) | Bomb(106) | 가능 |
| 일반 노트 | `field.lua:130-157` | `note` 객체 하나가 레인마다 `note_<색>` 스프라이트를 늘려 그림 | PNG 에 구움(2.5절). dst 색 무시 | Note(107, 객체 알파) | 불가(엔진 변경 필요) |
| LN 머리·꼬리 | 같음 | 같은 `note_<색>` | 같음 | 107 | 불가 |
| LN 몸통 | 같음 | idle `long_<색>`, 누르는 중 `held_<색>` | 같음 | 107 | 불가 |
| HCN | 같음 | 머리·꼬리 `note_<색>`, 몸통 idle `hell_<색>`, 누르는 중·회복 `hell_held_<색>`, 닳는 중 `hell_drain` | 같음 | 107 | 불가 |
| 지뢰 | 같음 | `mine` | 장미색 `C2455E` 고정, 위·아래 두 행 불투명, 가운데 알파 0x82 | 107 | 불가 |
| 레인 커버·리프트·HIDDEN+ | `field.lua:177-205` | `block`(8x8 흰색) 이미지를 `slider`, `liftCover`, `hiddenCover` 객체로, 가장자리 2px 강조색 막대, 수치는 숫자 시트 | 면 `theme.color.raised`(Mono `262626`), 수치 `theme.color.sub`, 초록 수치 `theme.tone.green` | Lane Cover(109) | 가능 |
| 진행 막대 | `info.lua:399-405` | 4px 트랙(`raised`)과 `bar_v` 그래프(강조색) | `raised`, `accent` | Progress Bar(111) | 가능 |
| 게이지 | `info.lua:322-339` | 노드 스프라이트 72장(6종 x 6상태 x 테마 2) | PNG 에 구움 | Gauge(110) | 불가(범위 밖) |
| 판정 글자·콤보 | `field.lua:221-259` | `judge_*`, `combo_*` 시트 | PNG 에 구움 | 불투명도 부위 아님 | 불가(범위 밖) |

판정 글자·콤보와 게이지는 요청 범위(레인·노트)가 아니라 표에만 적었다.

### 2.3 레인 종류와 색 매핑

| 모드 | 레인 종류(`layout.lua:108-109`, `223-239`) |
| --- | --- |
| 5K, 7K, 10K, 14K | `BEAT_KEYS` = 흰, 검, 흰, 검, 흰, 검, 흰(7K·14K 는 7개, 5K·10K 는 앞 5개) + 스크래치. 모든 모드에서 흰 건반은 `white`, 검은 건반은 Note Colors 의 `black`, 스크래치는 Note Colors 의 `scratch` |
| 9K | `BUTTON_COLORS` = white, yellow, green, blue, red, blue, green, yellow, white(9개, 색 역할 5종). Note Colors 행이 없다 |
| DP(10K, 14K) | 1P·2P 필드가 같은 종류 구성을 쓴다(`double`, `layout.lua:328-347`) |

`lane.tint` 는 `NOTE_TINT[colors[kind]]`(`layout.lua:234-236`)이고, 키빔과 폭발 색이 이 값을 따른다. Note Colors 항목은 `layout.NOTE_SETS`(`layout.lua:44-50`)에 있다.

| 항목(op) | 검은 건반(`black`) | 스크래치(`scratch`) |
| --- | --- | --- |
| White-Blue(930, 기본) | cyan `4CC9F0` | red `F0564C` |
| White-Deep Blue(931) | blue `568CF5` | red |
| White-Yellow(932) | yellow `F2B33D` | red |
| White-Green(933) | green `59D185` | red |
| Mono(934) | gray `8C8C8C` | silver `C4C4C4` |

흰 건반은 언제나 `white` = `F1F4F8`(`theme.lua:49`)이다. 9키 버튼 색은 `theme.tone`(red `F0564C`, orange `F0885A`, yellow `F2B33D`, green `59D185`, blue `568CF5`, violet `B26BF0`, rose `C2455E`, `theme.lua:93-101`)과 `NOTE_TINT`(`layout.lua:110-119`)에서 온다.

### 2.4 팔레트 값 전부(`lua/theme.lua`)

| 이름 | 값(RGB) | 줄 |
| --- | --- | --- |
| `theme.mono` | bg 0,0,0 / panel 22,22,22 / raised 38,38,38 / line 60,60,60 / text 242,242,242 / sub 184,184,184 / muted 128,128,128 / bar 200,200,200 / lamp_off 68,68,68 | 8-18 |
| `theme.slate` | bg 12,15,20 / panel 20,25,34 / raised 29,36,48 / line 43,53,69 / text 241,244,248 / sub 167,178,193 / muted 102,115,135 / bar 92,115,160 / lamp_off 58,69,86 | 20-30 |
| `theme.accent` | white 255,255,255 / sky 76,201,240 / amber 242,179,61 / green 89,209,133 / pink 240,104,158 | 32-38 |
| `theme.accent_dim` | white 102,102,102 / sky 31,95,120 / amber 97,72,24 / green 36,84,53 / pink 96,42,63 | 40-46 |
| `theme.note` | white 241,244,248 / cyan 76,201,240 / gray 140,140,140 / silver 196,196,196 / shade 12,15,20 | 48-54 |
| `theme.OPTION` | THEME_MONO 920, THEME_SLATE 921, ACCENT_WHITE 922, SKY 923, AMBER 924, GREEN 925, PINK 926 | 56-64 |

`theme.apply`(`theme.lua:312-322`)는 본체 패스에서 `theme.color` 표의 값을 **제자리에서** 바꾼다(`painted`, `theme.lua:291-295`). 화면 모듈은 색 표를 참조로만 쥐므로, 한 번 `apply` 하면 모든 부위가 같은 값을 읽는다. 그러나 `layout.lua:110-119` 의 `NOTE_TINT` 는 `theme.note.*` 와 `theme.tone.*` 를 직접 쥐므로 Theme·Accent 의 영향을 받지 않는다.

### 2.5 노트 시트(`image/play.png`, 1024x570, 36,843 B)

생성식은 `tools/sections/play.py`다. 색은 `lua/theme.lua` 에서 정규식으로 읽는다(`tools/sprites.py:51-57`).

| 스프라이트 | 크기 | 만드는 식 | 줄 |
| --- | --- | --- | --- |
| `note_<c>` | 4x8 | 7행은 색 그대로, 맨 아래 1행은 `mixed(c, SHADE, 0.55)`. `SHADE` = (12,15,20), `mixed(c, t, s) = round(t + (c - t) * s)` | `play.py:26-28, 85-89` |
| `long_<c>`(idle 몸통) | 8x4 | 열 1~6 불투명 `mixed(c, SHADE, 0.4)`, 열 0·7 투명 | `play.py:29-33, 92-100, 116` |
| `held_<c>`(LN 누르는 중) | 8x4 | `mixed(c, SHADE, 0.8)` | `play.py:117` |
| `hell_<c>`(HCN idle) | 8x4 | 바깥 열 `mixed(c, SHADE, 0.3)`, 가운데 열 3·4 `mixed(c, SHADE, 0.8)` | `play.py:118` |
| `hell_held_<c>`(HCN 누르는 중) | 8x4 | 바깥 `mixed(c, SHADE, 0.4)`, 가운데 `mixed(c, WHITE, 0.5)`(색보다 밝다) | `play.py:119` |
| `hell_drain` | 8x4 | 바깥 `mixed(rose, SHADE, 0.3)`, 가운데 rose(`C2455E`) | `play.py:113` |
| `mine` | 4x8 | rose, 위·아래 2행 알파 255, 가운데 4행 알파 130(0x82) | `play.py:35-37, 103-109` |

`<c>` 는 8종(`blue`, `cyan`, `gray`, `green`, `red`, `silver`, `white`, `yellow`, `play.py:16-25`)이다. `notes()`(`play.py:112-120`)는 색마다 `note_`, `long_`, `held_`, `hell_`, `hell_held_` 다섯 장을 만들어 8색 x 5 = 40장이고, `mine` 과 `hell_drain` 이 더해져 모두 42장이다. 아틀라스 좌표는 `lua/atlas/play.lua:86-132` 이고 노트 4x8 은 y=511, 몸통 8x4 는 y=566 한 줄에 있다. 스프라이트를 10배로 확대해 눈으로 확인하고(Pillow 로 만든 임시 그림) 8x4 몸통 열의 값을 직접 읽은 결과, 노트는 단색 면과 아래 한 줄 그림자뿐이고 그라데이션이나 하이라이트가 없다. 하이라이트는 `hell_held_<c>` 의 가운데 두 열(색과 흰색을 반반 섞음)과 게이지 노드의 끝 칸(`GAUGE_TIP_WHITE_SHARE` 0.5) 둘뿐이다.

나머지 시트: `beam`(8x64, 알파 1~118), `pulse`(8x64, 알파 1~71), `bomb`(1024x128, 8프레임, 알파 5~255, 4배 표본 BOX 축소), `bar_h`(256x4), `bar_v`(4x256), 판정 글자 5장·콤보 숫자 3줄(색 구움), 게이지 노드 72장(`play.lua:14-85`)이다. `common.png` 의 `block`(8x8)과 `num18`~`num96`, `snum24`, `snum32` 는 순백이다(3절).

## 3. 흰 이미지 x 곱셈 색 검증(실측)

엔진의 곱셈은 CPU 가 `a * b / 255` 정수 내림(`crates/rbms-render/src/cpu.rs:26-31`), GPU 가 `textureSample * tint`(`apps/rbms-player/src/gpu/mod.rs:88`)다. 곱셈은 알파까지 채널마다 적용된다(`QuadParams.tint`, `lib.rs:282-283`). 표면 포맷은 sRGB 가 아니라서(docs/skin.md E10) 바이트가 그대로 통과한다. 흰 텍셀 255 는 어떤 tint 채널 t 에 대해서도 `255 * t / 255 = t` 라 정확하다(0~255 전부 확인).

픽셀을 직접 읽은 결과(`image/play.png`, `image/common.png`, Pillow 로 RGB 집합과 알파 범위 계산):

| 스프라이트 | RGB | 알파 | 곱셈 결과 |
| --- | --- | --- | --- |
| `block`(8x8) | 순백 1색 | 255 | 정확 |
| `num18/24/32/44/64/96`, `snum24/32` | 순백 1색 | 1~255(글자 가장자리) | 정확(알파는 그대로) |
| `beam` | 순백 1색 | 1~118 | 정확, 최대 알파 118(46%) |
| `pulse` | 순백 1색 | 1~71 | 정확 |
| `bomb`(8프레임) | 순백 1색 | 5~255 | 정확 |
| `bar_h`, `bar_v` | 순백 1색 | 255 | 정확 |
| `judge_pgreat`, `combo_pgreat` | `84CEFF` | 1~255 | 구운 색(곱하면 어두워짐) |
| `judge_poor` | `F25200` | 2~255 | 구운 색 |
| `note_white` | `F1F4F8` 몸통, `8A8D91` 그림자 | 255 | 순백이 아니라 부정확 |

노트 계열을 "순백 바탕 x 사용자 색" 으로 바꿀 때의 오차를 `play.py` 의 식으로 계산했다(비교 대상은 지금 구워 둔 값. 그림자 색 계산을 그대로 두지 않고 순백 기준 회색 사다리로 만든 경우):

| 부위 | 구운 값(예: cyan) | 순백 바탕 x 사용자 색 | 최대 오차(채널당) |
| --- | --- | --- | --- |
| 노트 몸통(7행) | (76,201,240) | (76,201,240) | 0 |
| 노트 그림자 행(0.55) | (47,117,141) | (42,110,132) | 9 |
| LN idle(0.4) | (38,89,108) | (30,80,96) | 12 |
| LN held, HCN 가운데(0.8) | (63,164,196) | (61,161,192) | 4 |
| HCN 바깥(0.3) | (31,71,86) | (23,60,72) | 14 |
| HCN held 바깥(0.4) | (38,89,108) | (30,80,96) | 12 |

오차는 8색 전부에서 15 이하이고(255 만점), 원인은 구운 값이 색을 검정이 아니라 `SHADE`(12,15,20) 쪽으로 섞기 때문이다. 곱셈은 검정으로 간다. 사다리 회색 값을 조금 올려 평균을 맞출 수 있지만 채널마다 다르게 어긋나 완전히는 못 맞춘다. 현재 `F1F4F8` 바탕으로 곱하면 몸통부터 채널당 2~13 어두워진다(cyan (76,201,240) 이 (72,192,233), red (240,86,76) 이 (227,82,74)). `hell_held` 의 가운데 열은 색보다 밝은 하이라이트(`mixed(c, WHITE, 0.5)`)라 곱셈으로는 만들 수 없고, 흰색에 가까운 별도 레일 스프라이트를 틴트 없이 겹치거나 모양을 단순화해야 한다. 지뢰·`hell_drain` 은 레인 색을 따르지 않는 의미색(장미)이라 틴트에서 빼야 한다.

## 4. 이미 있는 사용자 조절

### 4.1 색 옵션 행 넷(웨이브 14)

| 옵션 | 항목(op) | 가진 문서 | 정의 |
| --- | --- | --- | --- |
| Theme | Mono(920), Slate(921) | 9개 전부 | `theme.lua:68-71, 305-310` |
| Accent | White(922), Sky(923), Amber(924), Green(925), Pink(926) | 9개 전부 | `theme.lua:73-79` |
| Note Colors | White-Blue(930), White-Deep Blue(931), White-Yellow(932), White-Green(933), Mono(934) | play5, play7, play10, play14(스크래치 있는 모드) | `layout.lua:44-50`, `play/main.lua:50-56` |
| Bomb Color | Note(935), White(936) | 플레이 5개 | `play/main.lua:57` |

옵션은 `property` 로 선언하고 항목마다 op 번호를 가진다. 값을 바꾸면 문서를 다시 읽는다(스킨 메뉴·PLAY 메뉴 모두). 이름과 항목 이름이 문서 사이에서 같아야 "같은 옵션을 모든 화면에 적용"(`A` 키, `skin_select.rs:1241`)과 내보내기가 동작한다.

### 4.2 부위별 불투명도 18행

`layout.lua:54-94` 의 `PART_ROWS`. 알파 축만 허용한 오프셋 하나(`{ name=..., id=..., a=true }`, `play/main.lua:61-69`)이고 Lua 는 `s:within(part, ...)` 로 부위 안의 모든 destination 에 그 id 를 붙인다(`ui.lua:171-190`).

| id | 이름 | 덮는 것 |
| --- | --- | --- |
| 101 | Lane Background | 레인 면 |
| 102 | Lane Divider | 레인 사이 세로선 |
| 103 | Judge Line | 판정선과 펄스 |
| 104 | Key Beam | 키빔 |
| 106 | Bomb | 폭발 |
| 107 | Note | 노트, LN, 지뢰(객체 알파) |
| 108 | Bar Line | 마디선 |
| 109 | Lane Cover | 커버·리프트·HIDDEN+ 면과 수치 |
| 110 | Gauge | 게이지와 숫자 |
| 111 | Progress Bar | 진행 막대 |
| 112 | Song Header | 곡 머리 |
| 113 | BGA | BGA 와 틀 |
| 114 | Score Graph | 스코어 그래프(SP 문서만) |
| 115 | Score | 점수 패널 |
| 116 | Judge Count | 판정 수 |
| 117 | Timing | 타이밍 |
| 118 | Distribution Graph | 분포 그래프 |
| 119 | Speed Display | 발치의 HI-SPEED·GREEN, 커버 수치 |

저장값은 알파 오프셋 `a`(-255~+255)이고 UI 는 퍼센트다. `opacity_percent(a) = clamp(round(100 + a * 100 / 255), 0, 200)`, `opacity_alpha(p) = round((p - 100) * 255 / 100)`(`skin_select.rs:256-269`). 더하는 방식이라 0%와 100%만 모든 destination 에 같게 듣는다(`dst.rs:486-499` 의 `offset_alpha`/`shifted_alpha`, 최종 알파 = clamp(dst 알파 + 오프셋 알파, 0, 255)). 노트 객체는 destination 알파 오프셋만큼 필드 전체가 흐려진다(`notes.rs:892-894` 의 `opacity`, 원본은 이것도 읽지 않는다).

### 4.3 위치·크기 오프셋

| 이름 | id | 축 | 비고 |
| --- | --- | --- | --- |
| Judge Position | 100 | y | 기본 스킨 선언. 판정 글자·콤보·위 줄 |
| All offset(%) | 10 | x,y,w,h | 엔진이 모든 플레이 스킨에 붙임(`loader.rs:253-258`) |
| Notes offset | 30 | h | 노트 두께(`notes.rs` 의 `note_nudge`) |
| Judge offset | 32 | x,y,w,h,a | |
| Judge Detail offset | 33 | x,y,w,h,a | |

선언은 beatoraja 의 `CustomOffset`(id, x, y, w, h, r, a 허용 여부)과 같다. 범위는 위치·크기 ±2000, 각도 ±360, 알파 ±255(`skin_select.rs:90-97`)이고, 원본의 JavaFX 스피너는 정수 ±9999 이다(`SkinConfigurationView.java:454`). 예약된 id: 1, 2(스크래치 각도), 3(리프트), 4(레인 커버), 5(HIDDEN), 10, 30, 32, 33, 50, 51. 유효 범위는 0~199(`main_state.offset(id)` 가 범위 밖에서 오류, `lua/main_state.rs:40-41`)이다. 기본 스킨이 쓰는 id 는 100~119 라서 120~199 가 비어 있다.

### 4.4 저장 위치와 형식

`settings.ron` 의 `skin.custom[문서 경로]` 가 `SkinCustomisation`(`crates/rbms-config/src/schema.rs:314-322`)이다. 구조체에 `#[serde(default)]` 가 있고 `deny_unknown_fields` 는 `crates/`, `apps/` 어디에도 없다(`grep` 확인). 모르는 키는 무시된다는 것을 기존 테스트가 고정한다(`crates/rbms-config/src/tests.rs:542-543` `an_unknown_key_is_ignored_rather_than_failing_the_load`).

```ron
skin: (custom: { "<설정 폴더>/skin/rbms-default/play7.luaskin": (
    properties: { "Note Colors": 931, "Theme": 921 },
    filepaths: {},
    offsets: { 101: (x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: -128.0) },
) })
```

현재 설정 스키마는 6(`schema.rs:20`)이다. PLAY 메뉴 때문에 스키마가 오르지 않은 선례가 있다(`docs/play-options.md` 7절 머리). 기본 스킨 팩을 내보내면 `rbms-adjustments.ron`(버전 1, `adjustments.rs:34-37`)이 문서 이름, 옵션 이름과 항목 이름, 오프셋 이름과 축별 값으로 적힌다. 이 파일은 `version == 0` 또는 `version > 1` 이면 통째로 무시된다(`adjustments.rs:153-155`).

### 4.5 메뉴의 행 종류와 조작

| 행 | 종류(`SkinRowKind`, `skin_select.rs:281-295`) | 조작 |
| --- | --- | --- |
| 옵션 | `Property` | ← → 로 항목을 돌며 바꾼다(`stepped`, `skin_select.rs:1729`, 끝에서 한 바퀴). 바꾸면 문서를 다시 읽는다 |
| 파일 | `File` | 같음. 기본 스킨엔 없다. SKIN 메뉴에만 있다 |
| 위치·크기·각도·알파 오프셋 | `Offset(축)` | 숫자 행: 슬라이더(`number_row.rs:170`), ← → 한 칸, Shift 열 칸, Enter 직접 입력, 0 키 초기화. 문서를 다시 읽지 않는다 |
| 불투명도 | `Opacity` | 알파만 허용한 오프셋. 퍼센트 숫자 행(슬라이더는 0~100%, 직접 입력은 200%까지) |
| EXPORT | 별도 | 폴더 선택 후 복사 |

숫자 행은 `NumberSpec`(`crates/rbms-config/src/number.rs:22-38`: min, max, per_one, decimals, unit, signed, clock)으로 센다. `NumberEdit`(`number_row.rs:236`)은 숫자·부호·소수점만 받아서 16진수는 입력할 수 없다. SKIN 메뉴(F12)에는 묶음이 없는 평평한 목록이고 7키 기본 스킨은 52행이다(고정 4 + 옵션 11 + 오프셋 1 + 엔진 오프셋 15 + 불투명도 18 + RELOAD·RESET·EXPORT 3). PLAY 메뉴(F8)는 묶음별이고 옵션과 선언 오프셋은 LANE 묶음 끝에, 불투명도와 엔진 오프셋은 SKIN 묶음의 접이 둘에 있다(`play_menu/rows.rs:92-139`의 `Fold::Opacity`, `Fold::Place`, `Folds`). 접이를 늘리는 구조가 이미 있다.

### 4.6 반영 방식과 비용

| 값 | 반영 |
| --- | --- |
| 옵션, 파일, RELOAD, RESET | 문서를 다시 읽는다(`step_for`, `skin_select.rs:1101-1148`이 `stale` 에 올린다). 미리보기는 몇 프레임 어둡게 둔다(`skin_preview.rs` 의 "Waiting in the dark") |
| 오프셋, 불투명도 | 읽지 않는다. 저장값을 프레임마다 읽는다(`DocumentOffsets`, `skin_screen.rs:272`, `ScreenHost::offset`, `skin_host/mod.rs:914`) |

문서를 읽는 비용은 `target/release/rbms-cli skin-dump assets/skins/rbms-default --frames 0`(10-20 빌드)에서 play7 이 3.7ms, play14 4.4ms, select 5.9ms 였다(Lua 두 패스와 조립, 텍스처 디코드 제외). 슬라이더를 끄는 동안 매 프레임 다시 읽는 것은 시간상 가능하지만 미리보기가 계속 어두워지는 문제가 남는다. PLAY 메뉴의 미리보기가 같은 이유로 샘플 장면을 다시 시작하지 않게 만들어진 것이 그 증거다(`skin_preview.rs` 머리말의 "The play menu's preview is the exception"). 이 어두워짐의 정도는 실제 창에서 보지 못했다(조사하지 않은 것, 7절).

## 5. 엔진 능력

### 5.1 destination 색과 blend

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 색 | 키프레임마다 `r`,`g`,`b`,`a` 0~255 정수. 첫 키프레임은 형 기본값, 이후는 앞 키프레임에서 채운다. 보간은 채널마다 `round`(`lerp_channel`) | `model.rs:515-531`, `dst.rs:453-466` |
| 해석 | 보간한 색에 알파 오프셋을 더해 `Resolved.color` 로 나온다. 모든 오프셋의 알파가 차례로 더해지고 0~255 로 자른다 | `dst.rs:516-570`(색은 563-564) |
| 오프셋이 바꾸는 것 | `x += off.x - off.w/2`, `y += off.y - off.h/2`, `w += off.w`, `h += off.h`(relative 객체는 크기만), 각도 `angle_deg - off.r`, 알파 `a + off.a`. **RGB 는 바꾸지 못한다** | `dst.rs:472-480, 563-570` |
| blend | 0·1 알파, 2·3 가산(`SRC_ALPHA, ONE`), 4 곱(`ZERO, SRC_COLOR`), 9 목적지 반전. 그 밖은 알파 | `lib.rs:151-173` |
| 객체별 색 | image, number, text, graph, slider, judge 의 각 부분, gauge 노드, 마디선 모두 destination 색을 틴트로 곱한다 | `draw.rs`, `judge.rs:434`, `gauge.rs:589`, `notes.rs:1333` |
| 노트(`note` 객체) | 예외. 배치가 흰색·알파 블렌드·최근접으로 고정이고 destination 알파만 필드 전체 불투명도로 쓴다 | `notes.rs:131-133, 905-935` |

beatoraja 도 같다: `SkinObject.java:485, 515` 가 `color.a += off.a / 255.0f` 만 하고, `LaneRenderer` 는 노트를 그리기 전 `setColor(Color.WHITE)`, `setBlend(0)` 으로 초기화한다(`b3-play-objects.md:290`).

### 5.2 beatoraja 의 커스터마이즈 종류

원본은 `CustomOption`(property), `CustomFile`(filepath), `CustomOffset`(offset) 셋과 분류(`CustomCategory`)뿐이다(`SkinHeader.java:261, 312, 343, 385`). `CustomOffset` 은 `id`, `x`, `y`, `w`, `h`, `r`, `a`(각 축의 허용 여부)이고 값은 `SkinOffset{x,y,w,h,r,a}`(float)다(`SkinObject.java:774-781`). `x`, `y` 는 위치에 더하고, `w`, `h` 는 크기에 더하며(중심을 지키려고 위치는 `x - w/2`, `y - h/2` 만큼 움직인다), `r` 은 각도(도)에 더하고, `a` 는 알파(0~255 단위)에 더한다(식은 5.1절 표. 위치·크기 `SkinObject.java:404-432`, 알파 `485`·`515`, 각도 `538-541`). 색을 직접 받는 형식은 없고, 생태계의 관행은 filepath 로 이미지 변종을 고르게 하거나 알파 오프셋을 밝기 덮개에 거는 것이다(ModernChic 의 파츠 선택 13종과 밝기 행, `m6-misc-assets.md:499`, `docs/skin.md` 2.2절). R-BMS 의 지원은 세 종류 모두 있다(`SkinDef.property/filepath/offset`, `model.rs`). 알 수 없는 JSON 키는 원본이 무시하고(`JSONSkinLoader.java:99, 226`의 `setIgnoreUnknownFields(true)`) R-BMS 의 Lua 변환기도 "필드 이름과 정확히 같은 문자열 키만 읽고 나머지는 무시" 한다(`from_lua.rs` 머리말). 그래서 R-BMS 확장 키를 쓴 `.luaskin` 은 beatoraja 에서 열어도 무해하고 기본값으로 그려진다.

### 5.3 Lua 가 사용자 값을 읽는 경로

| 경로 | 읽는 때 | 값 | 한계 |
| --- | --- | --- | --- |
| `skin_config.option[이름]` | 본체 패스 | 선택한 항목의 op 번호 | 항목 수만큼의 유한한 색. 바꾸면 문서를 다시 읽는다 |
| `skin_config.offset[이름]` | 본체 패스(헤더 패스에서는 `skin_config == nil`) | 저장값 그대로 `{x,y,w,h,r,a}`(`env.rs:231-244, 383-410`, `loader.rs:884-899`) | 오프셋 변경은 문서를 다시 읽지 않아서 다음 로드까지 안 보인다. 축 라벨이 X·Y·W·H, 범위 ±2000 이라 색 입력으로 부적절 |
| dst 의 `offset`/`offsets` | 프레임마다 | 엔진이 x,y,w,h,각도,알파에 더함 | RGB 불가 |
| `main_state.offset(id)`, `main_state.option(op)` | 프레임마다(`main_state.rs:381-384, 881-888`) | 같은 저장값 | `draw`, `value`, `text`, `timer` 같은 함수값 필드에서만 호출되고 dst 색은 정수 필드라 함수를 받지 못한다 |
| 알파 오프셋 + 가산 합성 | 프레임마다 | 불투명도 행이 곧 채널 값 | 6.5절 D |

### 5.4 노트 색 입히기가 막히는 지점과 선례

노트는 `Field::put`(`notes.rs:911-950`)이 `tint: Color { a: alpha, ..BATCH_WHITE }` 로 그린다. 레인 정보는 `note.dst[레인]`(`Vec<Animation>`, `objects.rs:367`)에서 `animation_rect`(`notes.rs:697-699`)가 x, y, w, h 만 읽고, 같은 `Animation` 의 `r`,`g`,`b`,`a` 는 읽지 않는다. 그래서 기존 스킨이 레인 dst 에 색을 적어도 지금은 효과가 없다. ModernChic 의 `note.dst` 는 x, y, w, h 만 쓴다(`Play/lua/sp/notes.lua:34-60`)고 확인했다.

선례가 하나 있다. SCRATCH AUTO 레인은 스킨의 그림 대신 단색 초록(`AUTO_LANE_NOTE` = `30E050`, 몸통 알파 0xA0)을 `NoteImage::Solid` 로 칠한다(`notes.rs:141-152`). 곱셈 틴트를 쓰지 않은 이유가 주석에 있다: 곱셈은 채널마다라 기본 스킨의 빨간 턴테이블 노트가 검정이 된다. 사용자 색 입히기와 SCRATCH AUTO 초록이 겹치면 초록이 이겨야 한다(현재 동작 유지).

## 6. 설계 대안

### 6.0 비교표

| 안 | 한 줄 | 자유 RGB 부위 | 노트 | 반영 | 엔진 변경 | 기본 스킨 변경 | 다른 스킨 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A | 기본 스킨 전용. 새 색 행 종류 + Lua 가 본체 패스에서 dst 색에 적용 | 레인 면·구분선·마디선·판정선·커버·키빔·폭발·진행·바탕 | 프리셋 5세트 유지(자유 RGB 불가) | 문서를 다시 읽음 | 헤더 `color`, `skin_config.color`, 설정·메뉴·내보내기 | Lua 5개 모듈 | 영향 없음 |
| B | 엔진 수준 덮어쓰기. 설정의 색을 엔진이 노트에 칠함 | 노트와 레인 영역(note.dst 사각형) | 어떤 스킨이든 | 실시간 | 엔진 + 설정 + 메뉴(스킨과 무관한 PLAY 행) | 없음 | 모든 스킨(그림 모양 상실) |
| C | 엔진 확장(헤더 `color` 행 + dst/노트 `tint` 참조) + 기본 스킨 채택 | 위 전부, 노트 포함 | 흰 바탕 시트 x 레인 틴트 | 실시간 | A 의 것 + dst `tint`, 노트 `tint` | 이미지 생성기, Lua 5개 모듈 | 확장 키를 쓰는 스킨만 |
| D | 엔진 변경 없음. 순백 가산층 3겹의 알파를 불투명도 행으로 | 레인 면·구분선·판정선 등 검은 바탕 위의 것 | 불가 | 실시간 | 없음 | Lua, 불투명도 행 3N개 | 영향 없음 |
| E | 프리셋 확대. 기존 옵션 항목을 더 많이 | 고정된 몇 가지 색 | 프리셋 세트 | 문서를 다시 읽음 | 없음 | 시트 더 만들기, Lua | 영향 없음 |
| F | 파일 변종(filepath). 사용자가 이미지 파일을 고름 | 이미지로 만든 것 | 이미지 | 문서를 다시 읽음 | 없음 | filepath 행 선언 | 영향 없음 |

### 6.1 A 안 상세: 기본 스킨 전용(Lua 가 본체 패스에서 적용)

- 새 행 종류 `color`(R-BMS 확장): 헤더 `color = { { name = "White Lane", id = 120, def = "161616" }, ... }`. 저장은 `skin.custom[문서].colors` 에 새 맵을 둔다(`#[serde(default)]` 라 스키마 6 그대로).
- Lua 는 본체 패스에서 `skin_config.color[이름]`(새로 발행) 을 읽어 `s:rect(box, color)` 에 넘긴다. 값이 바뀌면 옵션처럼 문서를 다시 읽는다.
- 노트는 이 경로로 칠할 수 없다(5.4절). 노트 색은 Note Colors 5세트 그대로이거나, 시트에 색 묶음을 더 구워야 한다(E 안과 겹친다).
- 장점은 엔진 렌더러·dst 모델을 건드리지 않는 점, 단점은 슬라이더를 끌 때마다 문서를 다시 읽어 미리보기가 어두워지는 점(4.6절)과 노트가 빠지는 점이다.

### 6.2 B 안 상세: 엔진 수준 덮어쓰기

- 설정(`config.play` 또는 `skin.custom` 밖의 새 그룹)에 레인 종류별 색(흰, 검, 스크래치)을 두고, 엔진이 `note` 객체를 그릴 때 레인 종류별 색을 입힌다. 방식은 둘이다. 하나는 곱셈 틴트(흰 노트 그림에서만 맞고 색 있는 그림 위에서는 어두워짐), 하나는 SCRATCH AUTO 처럼 `NoteImage::Solid` 로 그림 대신 칠하는 것(모양·그림자 상실). 알파 마스크로 색을 입히는 셰이더(`docs/skin.md` 6절 후속, `gpu/mod.rs:58-88` 의 셰이더에 모드 추가)를 만들면 모양을 살리며 칠할 수 있다.
- 레인 면·구분선·키빔 같은 부위는 엔진이 식별할 수 없다(beatoraja 스킨 형식에서 그냥 이미지다). 할 수 있는 것은 `note.dst` 의 레인 사각형 위에 색 사각형을 덧그리는 것(스킨의 레인 그림을 가림)이다.
- UI 는 스킨 문서와 무관한 행이어야 한다(스킨이 선언한 행이 아니므로). PLAY 메뉴 LANE 묶음에 `NOTE COLOR WHITE/BLACK/SCRATCH` 같은 설정 행을 두고 `settings.ron` 의 `play` 그룹에 저장하는 형태가 된다(스킨별 값이 아니라 전역).
- 장점은 ModernChic 같은 외부 스킨에서도 듣는다는 점이다. 단점은 외부 스킨의 그림을 망가뜨리고 레인 종류 판정(7키 홀·짝, 9키 버튼)을 엔진이 해야 한다는 점, 스킨 제작자 의도와 충돌한다는 점, 스킨마다 값을 따로 가질 수 없다는 점이다.

### 6.3 C 안 상세: 엔진 확장 + 기본 스킨 채택(메인 추천)

색 행은 beatoraja 의 오프셋과 같은 id 방식으로 만든다. 값은 프레임마다 저장값에서 읽혀 문서를 다시 읽지 않는다.

| 층 | 변경 | 위치 |
| --- | --- | --- |
| 헤더 | `color` 목록: `{ name, id, def(16진 RRGGBB), category }`. `SkinDef.color: Vec<ColorDef>`(`offset` 옆) | `model.rs:554` 근처 |
| dst | `Destination.tint: Option<i32>`(색 id). `DestinationTrack.tint` 로 옮긴다 | `model.rs:473`, `loader/track.rs:249-264`, `dst.rs:303` |
| 해석 | `OffsetSource` 에 기본 구현이 있는 메서드 `color(id) -> Option<SkinColor>` 추가(기본 `None`, 기존 구현체는 그대로 컴파일). `resolve` 가 색 id 가 있고 값이 있으면 r,g,b 를 바꾸고 알파는 곱한 뒤 오프셋 알파를 더한다(`dst.rs:563-570` 자리) | `dst.rs:78, 516` |
| 노트 | `NoteSet.tint: Vec<i32>`(레인과 나란한 색 id, `size` 목록 선례). `NoteLane.tint` 로 옮기고 `Field::put` 이 `BATCH_WHITE` 에 곱한다. 지뢰·hidden·processed·`hcnreactive`(닳는 중)는 틴트에서 뺀다 | `objects.rs:367-370`, `notes.rs:474, 659, 911` |
| 저장 | `SkinCustomisation.colors: BTreeMap<i32, [u8; 4]>`. 내보내기는 이름 기준(`documents[이름].colors[이름] = "#RRGGBBAA"`)이고 `ADJUSTMENTS_VERSION` 은 1 그대로(없는 키는 이전 빌드가 무시) | `schema.rs:314`, `adjustments.rs:62` |
| Lua 발행 | `skin_config.color[이름] = {r,g,b,a}` 도 발행(기본값 계산용). 확장 키가 없는 beatoraja 에서는 `nil` 이라 `(skin_config.color or {})` 로 막는다 | `env.rs:231, 383` |
| 앱 읽기 | `DocumentOffsets`, `ScreenHost::offset` 처럼 `color` 를 구현 | `skin_screen.rs:272`, `skin_host/mod.rs:914` |
| 메뉴 | 새 `SkinRow::Color(i)`, `SkinRowKind::Color`. SKIN 메뉴는 옵션 행들 뒤, PLAY 메뉴는 새 접이 `COLOR`(또는 LANE 묶음 끝) | `skin_select.rs:281-309, 930-945`, `skin_menu.rs:716-743`, `play_menu/rows.rs:92-139` |
| 입력 | `HexEdit`(또는 `NumberEdit` 에 종류 추가): 16진 6자리·8자리, `#` 생략, Enter 확정, Esc 취소. 숫자 행(0~255)은 `NumberSpec::whole(0, 255, "")` 로 그대로 | `number_row.rs:236-300` |
| 기본 스킨 | 노트 시트를 순백 바탕으로 재생성(색 8종을 `note`, `long`, `held`, `hell`, `hell_held` 다섯 장으로 줄이고 지뢰·`hell_drain` 은 유지), Lua 가 `tint` 를 건다 | `tools/sections/play.py`, `lua/play/{layout,field,main}.lua`, `lua/theme.lua` |

색 행이 값이 없을 때(저장값 없음)는 dst 의 키프레임 색이 그대로 나온다. 그래서 헤더 `def` 는 메뉴가 기본 색을 보여 주기 위한 것이고, 기본 그리기는 지금과 같다(Theme·Accent·Note Colors 프리셋이 계속 기본 색을 정한다). 우선순위는 "사용자 색 행 > Theme/Accent/Note Colors 프리셋 > 제작자 기본값" 이고 0 키(초기화)가 저장값을 지워 프리셋으로 돌아가게 한다.

### 6.4 UI 사양(A·C 공통)

| 요소 | 사양 |
| --- | --- |
| 색 행(한 줄) | 라벨 `<부위> COLOR`, 값에 `#RRGGBB` 와 작은 색 견본 사각형. 값이 없으면 `DEFAULT` 를 뒤에 붙인다 |
| ← → | 문서가 선언한 프리셋 목록을 돈다(Accent 5색, `theme.tone` 7색, 회색 단계 0·22·38·60·128·184·242). 목록에 없는 색이면 `CUSTOM` |
| Enter | 그 자리에서 16진수를 친다(6자리 또는 8자리, `#` 생략 가능). Enter 확정, Esc 취소, 잘못된 입력은 `NOT A COLOR` 를 1.5초(`number_row.rs:80-81` 의 `NOT A NUMBER` 와 같은 방식) |
| 0 키 | 저장값을 지운다(프리셋으로 복귀) |
| 채널 행 | 색 행 아래 접이 `CHANNELS`(PLAY 메뉴는 접이, SKIN 메뉴는 평평한 행)에 R, G, B 숫자 행(0~255, 슬라이더 + 직접 입력). A 가 필요하면 A 행(0~255) |
| `A` 키(모든 화면에 적용) | 같은 이름의 색 행을 팩의 다른 문서에도 적용(`apply_to_peers`, `skin_select.rs:1241`) |
| 미리보기 | C 안은 다음 프레임 반영(오프셋과 같다). A 안은 문서를 다시 읽고 몇 프레임 어둡게 둔다 |

행 수(7K 기준, SKIN 메뉴는 현재 52행이라 한 줄 행이 유리하다):

| 구성 | 한 줄 행 | R·G·B 행까지 |
| --- | --- | --- |
| 종류 3(흰 건반, 검은 건반, 스크래치)로 레인 면과 노트 | 6 | 6 + 18 |
| 종류 3 + 레인 개별 8(7키 + 스크래치)을 레인 면에만 | 14 | 14 + 42 |
| 위 + 구분선, 마디선, 판정선, 키빔, 폭발, 커버, 화면 바탕 | 위 + 7 | 위 + 21 |

### 6.5 D 안 상세: 가산 3층 혼합(참고, 비추천)

블렌드 2(`SRC_ALPHA, ONE`)로 순수 빨강·초록·파랑 사각형을 겹쳐 그리고 각 층의 알파를 불투명도 행(오프셋)으로 받으면 검은 바탕 위에서 실시간으로 임의 RGB 가 나온다(층 알파가 곧 채널 값). 엔진 변경이 없고 기존 불투명도 행(슬라이더 0~100%, 직접 입력 200%)만 쓴다. 한계: 색마다 불투명도 행 3개(1% 걸음이라 채널 2.55 단위), 16진 입력 불가, 검은 바탕이 전제(레인 면 위의 가산은 그 아래 것과 더해진다), 노트 불가, 다른 부위와 겹치면 합산된다. 문서에 없는 사용법이고 의도가 읽히기 어려워 추천하지 않는다.

### 6.6 E·F 안 상세

E: 기존 `property` 행의 항목을 늘린다. 색 한 묶음당 시트 스프라이트가 늘고(노트 5종 x 색), 자유 색이 없다. F: filepath 행으로 이미지를 고르게 하는 beatoraja 관행이다. 기본 스킨은 자산 변종을 두지 않기로 했다(D4, `docs/plan/2026-10-11-default-skin-design.md` 1절). 색마다 PNG 를 만들어야 하고 HEX 입력이 없다.

### 6.7 영향(안 공통)

| 항목 | 영향 |
| --- | --- |
| 골든 테스트 | `crates/rbms-render/tests` 의 골든은 내장 화면과 합성 스킨 픽스처(`skin_render.rs`)라 기본값이 `None` 인 C 안에서는 그대로다. 기본 팩의 캡처 테스트(`stage/capture/play_rows.rs`)는 바탕 픽셀·레인 수·겹침을 단언할 뿐 노트 색을 못박지 않아서 그대로 통과해야 한다(`generate-images` 후 확인). 새 단위 테스트가 필요한 곳: `dst` 해석, `notes` 틴트(`skin_render/notes/tests.rs`), 로더 헤더, 설정 왕복, `skin_select` 행, 어댑터, 메뉴 키 |
| 이미지 생성기 `--check` | C·E 안은 `play.png`, `lua/atlas/play.lua` 를 다시 생성해 커밋해야 하고(`generate-images.py:117-139` 가 바이트 비교, 같은 입력이면 같은 바이트, 다른 OS·Pillow 판은 보장하지 않음) CI 에는 없는 로컬 게이트다. `crates/rbms-skin/tests/default_pack.rs` 의 시트·표 대조는 재생성 후 통과해야 한다. A 안은 이미지를 건드리지 않는다 |
| 스킨 덤프 | `cargo run -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120` 이 경고 0 이어야 한다 |
| 설정 호환 | 새 필드는 `#[serde(default)]` 라 이전 파일이 그대로 읽힌다. 새 파일을 이전 빌드가 읽으면 `colors` 를 무시하고 다시 쓸 때 잃는다(스키마 판은 안 올림) |
| 내보낸 팩 | `rbms-adjustments.ron` 버전 1 유지. 이전 빌드는 `colors` 를 무시한다 |
| beatoraja | 확장 헤더·dst 키는 무시되고 제작자 기본값으로 그려진다(`setIgnoreUnknownFields(true)`). Lua 는 `skin_config.color == nil` 을 막아야 한다 |
| 저장소 규칙 | 새 의존성 없음, Rust `//` 주석 금지(`//!`·`///` 영어만), 매직넘버 금지, `--locked`, 새 `#[allow]` 금지(HANDOFF 5절) |
| SCRATCH AUTO | 엔진 초록이 사용자 색보다 우선한다(현재 동작 유지) |

### 6.8 단계 제안(C 안 기준)

1. 엔진·데이터: `color` 헤더 행, dst `tint`, `OffsetSource::color`, 설정 `colors`, 내보내기, 로더 테스트. 렌더 변화 없음(값 없으면 그대로).
2. 메뉴: `SkinRow::Color`, 색 행·16진 입력·채널 행, SKIN·PLAY 메뉴, `A` 키. 기본 스킨 Lua 가 레인 면·구분선·판정선·커버 등에 `tint` 를 건다(노트 제외). 여기까지 이미지 변경 없음.
3. 노트: 노트 시트 순백 재생성, 노트 `tint` 목록, 렌더러 틴트와 제외 규칙, 캡처로 육안 확인.

## 7. 조사하지 않은 것과 위험

| 구분 | 내용 |
| --- | --- |
| 미조사 | 실제 GPU 창에서 문서를 다시 읽을 때 미리보기가 어두워지는 정도(코드와 문서로만 확인). 문서 읽기의 텍스처 디코드 시간. ModernChic 을 틴트 후보로 직접 그려 보지 않음 |
| 위험 | 노트 틴트가 곱셈이라 색 있는 그림(ModernChic 처럼 건반 종류별 노트 이미지를 따로 쓰는 스킨)에 쓰면 어두워진다. 그래서 확장은 opt-in(`tint` 키가 있는 레인만)이어야 하고 외부 스킨에는 모양을 살리는 틴트(알파 마스크 셰이더)가 따로 필요하다 |
| 위험 | 곱셈 틴트로는 HCN 누르는 중 가운데 레일의 하이라이트(색보다 밝음)를 만들 수 없다. 별도 흰 레일 스프라이트 겹치기나 단순화가 필요하다 |
| 위험 | 알파 오프셋은 더하는 방식이라 색 행에 A 를 두면 부위 불투명도 행과 합쳐져 중간값에서 어긋난다(4.2절). A 를 두지 않거나 곱셈으로 정의해야 한다 |
| 위험 | SKIN 메뉴는 묶음이 없는 52행의 평평한 목록이라 행을 더하면 길어진다. 접이가 필요하면 SKIN 메뉴에도 도입해야 한다 |
| 위험 | `settings.ron` 이 문서 절대 경로를 키로 쓰므로 색도 기기 간에 옮겨지지 않는다(`docs/skin.md` 6절 설정 행과 같은 한계) |

## 8. 결정 지점

1. 적용 범위. 선택지: 기본 스킨만(A) / 엔진 확장 + 기본 스킨 채택(C) / 모든 스킨에 강제 덮어쓰기(B). 메인 추천: C. 이유: 노트까지 자유 색이 되는 유일한 안이고 실시간이며, 확장 키는 beatoraja 에서 무시되어 호환을 해치지 않는다. 영향: `rbms-skin`, `rbms-render`, `rbms-config`, 앱 메뉴, 기본 스킨 시트·Lua.
2. 조절할 부위. 선택지: 최소(레인 면, 노트) / 표준(+구분선, 판정선, 키빔, 폭발, 커버, 마디선) / 전부(+화면 바탕, 진행 막대, 지뢰). 메인 추천: 표준. 이유: 요청의 "레인·노트" 와 눈에 띄는 부위를 덮고 행 수가 20 안쪽이다. 영향: Lua 와 헤더 선언만 늘고 엔진은 같다.
3. 레인 단위. 선택지: 종류 셋(흰, 검, 스크래치) / 레인별 개별 / 셋 기본 + 개별 접이. 메인 추천: 셋 기본 + 개별 접이는 후속. 이유: 7키 기준 6~8행이고 IIDX 식 사용에 맞는다. 9키는 버튼 색 역할 5종. 영향: 행 수(6.4절 표), Lua 의 색 id 수.
4. 색 입력 방식. 선택지: 프리셋만 / 16진 + 프리셋 / 16진 + R·G·B 슬라이더 + 프리셋. 메인 추천: 한 줄 색 행(견본 + 16진, ← → 프리셋)과 접이 안의 R·G·B 행. 이유: SKIN 메뉴가 평평한 52행이라 한 줄 행이 낫고 슬라이더는 필요한 사람만 연다. 영향: `HexEdit` 신설(숫자 편집기는 16진을 받지 않는다), 메뉴 행.
5. 반영 방식. 선택지: 실시간(저장값을 프레임마다 읽음) / 문서 다시 읽기. 메인 추천: 실시간. 이유: 슬라이더를 끌 때 미리보기가 어두워지지 않는다. 영향: dst `tint`, `OffsetSource::color`(A 안이면 없음).
6. 불투명도. 선택지: 기존 부위 불투명도 행만 / 색 행에 A 추가 / 레인 종류별 불투명도 행 추가. 메인 추천: 기존 행 유지 + 레인 면·노트는 레인 종류별 행만 후속 검토. 이유: 알파가 더하는 방식이라 두 곳에서 조절하면 중간값이 어긋난다. 영향: 오프셋 id 120~199, 노트는 색 알파 곱셈 정의 필요.
7. 노트 틴트의 대상. 선택지: 노트와 LN·HCN 몸통·끝 전부 / 노트·LN 만 / 일반 노트만. 메인 추천: 전부, 단 지뢰와 `hcnreactive` 는 제외. 이유: LN 만 다른 색이면 어색하고 지뢰·닳는 중은 의미색이다. 영향: `notes.rs` 제외 규칙, 시트 재생성 범위. HCN 누르는 중 하이라이트는 단순화 또는 흰 레일 겹치기.
8. 기존 옵션과의 관계. 선택지: 프리셋 항목 유지 + 색 행이 덮음 / 프리셋을 색 행으로 대체. 메인 추천: 유지하고 색 행이 우선. 이유: 저장된 설정(`Note Colors` 등)과 `A` 키 동작이 그대로 호환된다. 영향: Lua 의 우선순위 식과 0 키 의미.
9. 저장·내보내기 호환. 선택지: 설정 스키마 6·조절값 파일 버전 1 유지 / 둘 다 올림. 메인 추천: 유지(추가 필드). 이유: 이전 빌드가 새 키를 무시하고 선례(PLAY 메뉴)가 있다. 영향: 이전 빌드로 내려가면 색을 잃는다.
10. 기본 스킨 시트 재생성. 선택지: 노트 시트를 순백 + 회색 사다리로 다시 생성(항목 이름·op 는 유지, 그림자는 ≤15 어긋남 감수) / 구운 8색을 두고 새 색만 틴트(시트 증가). 메인 추천: 재생성. 이유: 시트가 작아지고 규칙이 하나이며 색을 곱셈으로 정확히 낸다(3절). 영향: `play.png`, `atlas/play.lua`, 생성기 `--check`, 캡처 육안 확인.
