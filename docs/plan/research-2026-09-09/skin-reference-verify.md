# skin-system 감사 보고서 반박 검증

검증자: 회의적 검증자. 대상: `scratchpad/research/skin-system.md` findings 17건.
검증 방법: 인용된 file:line 을 rbms / 레퍼런스 구현 양쪽에서 직접 열어 주장·수치 대조.

## 종합

- 총평: **수치 정확도가 매우 높다.** 968개 프로퍼티, 접두사별 13개 분류(287/273/151/74/43/40/30/22/16/16/9/5/2)는 전수 재카운트 결과 **한 개도 틀리지 않았고 합도 정확히 968**이다. LOC 수치, SkinConfig.Default 13개 중 .luaskin 5개, 커스텀 타이머/이벤트 범위, 와일드카드 랜덤 선택 로직도 전부 코드와 일치했다.
- 반박 성공(refuted): 0건. 정정 필요(partially): 5건. 확인(confirmed): 12건.
- 가장 중요한 정정은 **skin-17** 이다. "rbms 는 CPU 캔버스 기반이라 GPU 도입 여부를 결정해야 한다"는 전제가 사실과 다르다 — wgpu 백엔드는 **이미 주력 백엔드로 존재**한다(`apps/rbms-player/src/gpu.rs`, WGSL 셰이더·텍스처 샘플러 포함). 진짜 병목은 `Renderer` trait 이 `size/clear/fill_rect` **3개 메서드뿐**이라 텍스처 드로우 프리미티브 자체가 없다는 점이며, 이는 원 finding 보다 오히려 **더 심각**하다.

## finding 별 검증

| id | 판정 | 요지 |
|---|---|---|
| skin-01 | confirmed | 인용 라인 전부 일치 |
| skin-02 | confirmed | 인용 라인 전부 일치 |
| skin-03 | confirmed | .luaskin 5개, play7main.lua 849줄 실측 일치 |
| skin-04 | partially | 결론 맞음, 인용 라인 귀속 오류 + 프로퍼티 종수 6→8 |
| skin-05 | confirmed | acc 커브 4종·보간 10축 일치 |
| skin-06 | partially | 게이트 순서 맞음, dstloop==-1 동작 서술이 부정확 |
| skin-07 | confirmed | 968 및 13개 접두사 카운트 전수 재현 |
| skin-08 | confirmed | 산술 규칙 6종 일치 |
| skin-09 | confirmed | 10000~19999 / 1000~1999, isTimerWritableBySkin 일치 |
| skin-10 | confirmed | filemap prefix 치환 + Math.random 선택 일치 |
| skin-11 | partially | decide 12키 정확, select/result 목록에 누락 키 존재 |
| skin-12 | partially | CommandWord 실측 106종, 서브로더 6개→8개(패키지 3,195 LOC) |
| skin-13 | confirmed | 인용 LOC 전부 실측 일치 |
| skin-14 | confirmed | JsonSkin.Text 필드·font.rs outline/shadow 부재 확인 |
| skin-15 | partially | 슬롯 수 19→18, 그 외 일치 |
| skin-16 | confirmed | 이중 생성자·getOptionIds/getDrawConditions 일치 |
| skin-17 | partially(결론 유지, 근거·권고 정정) | wgpu 백엔드 이미 존재. 병목은 Renderer trait 3-메서드 |

### skin-04 (partially)
- 결론("순수 JSON 파서로 default play7.json 을 못 읽는다")은 **참**이다. 실측: `skin/default/play7.json` 에 Lua 식 draw 문자열 4개 존재 — `"draw":"gauge() >= 75"`, `"gauge() >= 50 and gauge() < 75"`, `"gauge() >= 25 and gauge() < 50"`, `"gauge() >= 0 and gauge() < 25"`.
- 정정 1: LuaScriptSerializer 가 등록된 타입은 6종이 아니라 **8종**이다 — Boolean/Integer/Float/String/Timer/FloatWriter 에 더해 `StringWriter`(JsonSkinSerializer.java:111) 와 `Event`(:112).
- 정정 2: 인용한 `:121-126` 은 LuaScriptSerializer 가 아니라 `DestinationOptionSerializer.read` 다(JsonSkinSerializer.java:115-130). "Factory 시도 → 실패 시 lua 컴파일" 동작 서술 자체는 그 코드에서 맞지만, 귀속 클래스가 틀렸다.

### skin-06 (partially)
- 4게이트 순서(draw 조건 → 타이머 → 루프 → starttime)는 코드와 일치한다: `SkinObject.java:588-595`(dstdraw AND) → `:350-357`(timer) → `:360-371`(loop) → `:372-375`(starttime).
- 정정: "dstloop==-1 이면 endtime 초과 시 미표시"는 직접적 서술이 아니다. 코드는 `time = -1` 로 **덮어쓸 뿐**이고(`:361-363`), 실제 미표시는 그다음 `starttime > time` 게이트가 성립할 때만 발생한다. `starttime <= -1` 인 스킨이면 표시가 유지된다. 이식 시 이 우회 경로를 그대로 재현하지 않으면 동작이 갈린다.

### skin-11 (partially)
- decide.json 최상위 12키(type,name,w,h,input,scene,fadeout,source,font,image,text,destination) — 실측 일치. 구현 순서 권고(decide 최소 수직 슬라이스)도 타당하다.
- 정정: 열거가 불완전하다. select.json 에는 보고서에 없는 `playerlamp`, `rivallamp` 가 있고(songlist 하위), result.json 에도 `value` 가 포함된다(보고서는 result 를 "+gaugegraph,judgegraph" 로만 서술). play7.json note 블록의 실제 슬롯은 note/lnend/lnstart/lnbody/lnactive/hcnend/hcnstart/hcnbody/hcnactive/hcndamage/hcnreactive/mine/hidden/processed 로, 보고서가 든 목록에 `hidden`/`processed` 가 빠져 있다.

### skin-12 (partially)
- 실측: `grep -rho 'new CommandWord("…"' skin/lr2/ | sort -u | wc -l` = **106**. "약 100"은 타당.
- 정정: 서브로더가 6개가 아니라 **CommandWord 를 쓰는 파일이 9개**(LR2SkinCSVLoader, LR2PlaySkinLoader, LR2SkinSelectSkinLoader, LR2FontLoader, LR2SkinLoader, LR2CourseResultSkinLoader, LR2SelectSkinLoader, LR2ResultSkinLoader, LR2SkinHeaderLoader)이고, 패키지 전체는 **3,195 LOC** 다. 즉 XL 산정은 오히려 **과소평가**에 가깝다.

### skin-15 (partially)
- 정정: NoteSet 의 텍스처 슬롯 String[] 필드는 19종이 아니라 **18종**이다(JsonSkin.java:339-356 을 1개씩 카운트: note, lnstart, lnend, lnbody, lnbodyActive, lnactive, hcnstart, hcnend, hcnbody, hcnactive, hcnbodyActive, hcndamage, hcnbodyMiss, hcnreactive, hcnbodyReactive, mine, hidden, processed).
- "rbms 에 hcnreactive 등 상태가 있는지 미확인" → 확인 결과: **없다.** rbms 는 `LnKind::{Ln,Cn,Hcn}`(crates/rbms-model/src/lib.rs:16) 로 종류만 구분하고, 판정은 head/release 2-판정 모델(crates/rbms-judge/src/matcher.rs:18-24, :245, :268)뿐이다. HCN 의 프레임 단위 active/damage/reactive 런타임 상태는 문서상 Phase 7 미구현(docs/PROCESS.md:176 "HCN 연속게이지·CN deferral·BSS는 Phase 7"). 따라서 매핑 표 작성 권고는 유효하되, 18슬롯 중 최소 6슬롯(hcnactive/hcnbodyActive/hcndamage/hcnbodyMiss/hcnreactive/hcnbodyReactive)은 **rbms 판정 엔진 확장이 선행되어야** 채워진다.

### skin-17 (partially — 결론은 유지, 근거·권고를 정정)
- 레퍼런스 구현 측 근거는 실측 일치: Skin.java:504-530(SkinObjectRenderer 의 `ShaderProgram[6] shaders`, blend, type(TYPE_NORMAL~TYPE_DISTANCE_FIELD 6종), clipBounds/scissors), SkinObject.java:36-44(dstblend "2:加算 9:反転", dstfilter "0 Nearest / 1 Linear"), :81-82(CENTERX/CENTERY 10종 룩업), StretchType.java 159 LOC.
- **정정 1**: "rbms 는 CPU 캔버스 기반"은 사실과 다르다. `apps/rbms-player/src/gpu.rs` 에 wgpu 백엔드가 이미 있고 WGSL 프래그먼트 셰이더(`textureSample`, gpu.rs:36-51)·BGA 텍스처 업로드(`write_texture`, gpu.rs:251)·인스턴스드 드로우(gpu.rs:64 주석)까지 동작한다. cpu.rs(223 LOC)는 **참조/테스트 백엔드**다(lib.rs:59-60 주석 "wgpu 백엔드와 CPU 참조 백엔드 둘 다 구현").
- **정정 2**: 따라서 권고의 "GPU 백엔드(wgpu) 도입 여부를 이 시점에 결정"은 이미 지난 결정이다. 실제 병목은 **`Renderer` trait 의 표면적**이다 — lib.rs:61-65 에 `size()`, `clear()`, `fill_rect()` **3개뿐**이며 임의 텍스처를 그리는 프리미티브가 아예 없다. 즉 blend/filter/center/stretch 이전에 `draw_textured_quad` 급 프리미티브부터 없다.
- 결과적으로 severity 는 high 가 아니라 **critical 에 가깝고**, effort 는 L 보다 크다(트레이트 확장이 rbms-render 전 호출부 3,158 LOC 에 파급). 권고는 "GPU 도입 결정"이 아니라 "Renderer trait 을 텍스처·블렌드·클립·회전중심을 받는 인터페이스로 확장하고 cpu.rs 참조 구현을 그에 맞춰 갱신" 이어야 한다.

## 놓친 항목 (missed)

| # | 항목 | 근거 |
|---|---|---|
| M1 | 기본 스킨 JSON 이 **엄격 JSON 이 아니다**(libgdx Json 의 관대 파싱). serde_json 은 `play7.json` 을 파싱 실패한다 — 실측: `json.loads` 가 line 16 col 3 에서 JSONDecodeError. 원인은 배열 끝 trailing comma. | <reference>/skin/default/play7.json:10-16 (`{"name":"2P","op":921},` 뒤 `]}`). 대응: json5/serde-hjson 계열 또는 관대 파서 필요 — 보고서 skin-02 의 "serde 구조체 1:1 미러링" 만으로는 로드 불가. |
| M2 | 스킨의 **Lua 실행이 샌드박스 대상**이다. `LuaSkinLoader.sandboxed(skinPath)` 가 스킨 디렉터리를 루트로 잡아 파일 접근을 제한한다 — mlua 채택 시 동일한 샌드박스 경계를 설계해야 한다(임의 스킨 = 임의 코드 실행). | skin/lua/LuaSkinLoader.java:36-43 (`sandboxed`, `new LuaSkinLoader(skinRoot)`), :30-38 (`SkinLuaAccessor(false, sandboxRoot)`). 보고서 skin-03/skin-04 는 보안 축을 전혀 다루지 않는다. |
| M3 | `Renderer` trait 이 **텍스처 프리미티브 자체를 갖지 않는다**(size/clear/fill_rect 3개). 스킨의 image/note/gauge 전부가 텍스처 드로우인데, 이 3-메서드 표면이 skin-01~17 전체의 실질 선행 조건이다. | /Users/gkn/R-BMS/crates/rbms-render/src/lib.rs:61-65. |
| M4 | rbms 에 **이미 wgpu 백엔드가 있다**(WGSL 셰이더·샘플러·텍스처 업로드). 보고서가 이를 놓쳐 skin-17 권고를 잘못된 결정 지점으로 유도한다. | apps/rbms-player/src/gpu.rs:36-51(shader), :178-209(bga_tex/bind group), :251(write_texture). |
| M5 | JSON 스킨의 **`scene`/`input`/`fadeout`/`playstart`/`close` 최상위 타이밍 필드**를 보고서가 언급하지 않는다. 화면 전환 수명(로드→표시→페이드아웃)이 이 값들로 정해지므로 decide 수직 슬라이스에서 바로 필요하다. | skin/default/decide.json 최상위 키 실측(type,name,w,h,input,scene,fadeout,…), play7.json 의 playstart/close. |
| M6 | 스킨 **커스텀 프로퍼티(`property`) 시스템**이 누락됐다. play7.json 최상위 `property` 배열이 사용자 선택 옵션(예: "Play Side" → op 920/921)을 정의하고, 그 op 가 destination 조건으로 소비된다. SkinConfig.Property 로 저장·복원되므로 스킨 모델과 설정 저장이 함께 설계돼야 한다. | skin/default/play7.json:12-20; skin/lua/LuaSkinLoader.java:64(`header.setSkinConfigProperty(property)`). |
| M7 | `filepath`(사용자 파일 선택) 와 skin-10 의 와일드카드가 **한 기능의 양면**인데 보고서는 skin-10 에서 filemap 만 언급하고, 그 filemap 을 채우는 최상위 `filepath` 선언(그리고 UI 노출)을 별도 항목으로 다루지 않았다. | skin/default/play7.json 최상위 `filepath` 키; skin/SkinLoader.java:95-105(filemap 소비). |
| M8 | rbms 의 현행 `SkinConfig` 는 이미 **RON 외부 로드 가능한 데이터 스킨**이다(`#[derive(Deserialize)]` + `#[serde(default)]`). 보고서는 이를 "스칼라 상수 묶음"으로만 서술해, 폴백 스킨으로 격하할 때 기존 RON 호환을 깨지 않아야 한다는 제약을 놓쳤다. | /Users/gkn/R-BMS/crates/rbms-render/src/skin.rs:9-11(`#[derive(Debug, Clone, Deserialize)] #[serde(default)]`), :6-8(독스트링 "loadable from a RON file"). |

## 미조사 범위

- `SkinTextBitmap`/`SkinTextFont`/`SkinTextImage` 본문(총 1,838 LOC) 통독 미수행 — skin-14 는 JsonSkin.Text 필드 정의와 rbms font.rs grep 만으로 검증.
- LR2 CSV 전처리기(`#IF`/`#SETOPTION`) 동작 검증 미수행 — CommandWord 개수만 실측.
- `SkinNoteDistributionGraph`/`PomyuCharaLoader` 등 그래프·캐릭터 계열 미검증(skin-13 의 LOC 추정치에만 반영).
- rbms `apps/rbms-player/src/gpu.rs` 전문 통독 미수행 — 텍스처/셰이더 존재 여부만 grep 확인.
