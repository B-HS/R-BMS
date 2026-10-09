# V1. 외부 스킨 첫 렌더 결과 — 나온 것과 빠진 것 (웨이브 2B)

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 2B · 기준 커밋: 웨이브 2B 커밋 · 색인과 갱신 규칙은 [README.md](README.md)

ModernChic 의 결정·결과·선곡·플레이 7키 문서를 맵 호스트 시나리오로 로드해 기존 스킨 렌더러로 1920x1080 CpuCanvas 에 그린 결과를 레이아웃 조사(`m2`·`m4`·`m5`)의 좌표와 대조한 기록이다. 웨이브 3~6 의 작업 근거로 쓴다. 해당 결함을 고친 단위는 이 문서의 항목에 "해소(단위, 날짜)"를 붙인다.

재현: `RBMS_SKIN_PACK=<ModernChic 폴더> RBMS_SKIN_CAPTURE_DIR=<출력 폴더> cargo test -p rbms-render --test skin_external -- --nocapture`. 캡처에는 제3자 스킨의 그림이 들어가므로 저장소에 넣지 않는다.

메인이 직접 연 캡처(2026-10-10): 결과 화면 3000ms 프레임은 랭크 AA·게이지 그래프·판정 표·FAST/SLOW·중앙 캐릭터·하단 곡 정보까지 그려진다. 선곡 화면 3000ms 프레임은 상하단 프레임·버튼 열·하단 3창·BPM·제목류가 그려지지만 오른쪽 곡 바 목록이 통째로 없고, 부제 텍스트가 크게 넘쳐 겹친다.

## 구현 단위의 요약

W2-9 완료: ModernChic 4개 화면(decide, result, musicselect, play7_hw)이 앱 없이 Lua 로드 → SkinScreen → 1920x1080 CpuCanvas → PNG 16장까지 패닉 없이 끝납니다. 상태: 완료, 소유 밖 파일 수정 없음.

1. 렌더 크레이트 변경은 필요 없었습니다. `SkinFrame.lua`(Option<&dyn LuaDrawEval>) 통로가 이미 있어, 테스트가 프레임마다 `skin.runtime().frame(&host, |bound| screen.draw(..lua: Some(bound)..))` 로 호스트를 묶습니다. `skin_render/mod.rs` 는 건드리지 않았습니다.
2. 함수 값이 실제로 평가됩니다. 같은 시각을 평가기 없이 다시 그리면 결과 화면은 67 → 33 객체(621,629 px 차이), 플레이 78 → 72(933,838 px), 선곡 106 → 92(29,644 px), 결정 28 → 27(7,636 px)입니다. 함수 실패 0, pcall 실패 0, 예산 초과 프레임 0.
3. 시나리오는 MapHost JSON 그대로이고(덤프 도구와 같은 형식), 테스트가 레퍼런스가 선언한 모든 option 을 "꺼짐"으로 채운 뒤 덮어씁니다(`op = {-2}` 같은 부정 조건이 성립하도록). 타이머는 "켜지는 시각"으로 적어 그 시각 이전 프레임에서는 꺼져 있습니다. 난이도는 색 확인을 위해 153(HYPER)을 켰습니다.
4. 텍스처는 그려지는 객체가 가리키는 source 만 읽습니다(모델을 따라가 `skin.sources` 를 거른 뒤 빌드). 화면당 3~23장, 2.8M~42.4M px, 한 프레임 0.28~1.36초(디버그 빌드), 전체 29초.
5. 화면 상태 요약: 결과 화면은 거의 다 나오고, 결정은 검정 띠·스테이지 파일·그래프만 빠지며, 선곡은 곡 목록 바가, 플레이는 노트가 통째로 빠집니다. 공통 원인 3개는 (가) 음수 id 참조 이미지(-100/-101/-102/-110/-111) 미구현, (나) `dst` 없는 최상위 destination(`songlist`, `notes`)이 키프레임 0개라 `prepare` 가 None 을 내 본체가 호출되지 않음, (다) 음수 폭 객체가 `Placement::region` 의 `fitted.w <= 0` 에서 버려짐입니다. 화면별 상세는 leftovers 에 좌표와 함께 적었습니다.

저장소 게이트용으로 팩 없이 도는 테스트 1개(시나리오 4개 파싱, 난이도 정확히 1개, 타이머 일정 검사)를 함께 넣었습니다. 캡처와 실행 로그는 scratchpad/wave2-captures/render/ 에 있습니다.

## 화면별 상세(나온 것 / 빠진 것 / 잘못 나온 것)

- [공통·W3-2] 음수 id 참조 이미지가 전부 'object "-110" is not a kind this build draws' 로 빠집니다. 화면별 개수: decide -100 x1, -110 x2 | result -110 x5 | musicselect -100 x1, -102 x1, -110 x4, -111 x11 | play7_hw -100 x1, -101 x1, -110 x10, -111 x1
- [공통·W5-1/W6-1] dst 없는 최상위 destination 은 키프레임 0개로 조립되고 `dst::resolve` 의 `frames.len().checked_sub(1)?` 에서 None 이 되어 본체 draw 가 호출되지 않습니다. 해당: musicselect `{id="songlist"}`(Select/lua/songlist.lua:245), play7_hw `{id="notes", offset=..}`(Play/lua/sp/notes.lua:243)와 id "def" 1개. 테스트는 9행짜리 곡 목록과 2마디 차트를 FrameExtra 로 넘겼지만 그려진 것은 0입니다
- [공통·W3-2] 음수 폭/높이 destination 은 `Placement::region` 의 `fitted.w <= 0.0 || fitted.h <= 0.0` 에서 버려집니다(뒤집기 미구현). 해당 destination 수: result 1, musicselect 2, play7_hw 7. 눈에 보이는 결과: result gra_fastRate (673,427,-246,26) 파랑 막대 없음, play 스코어 막대 now/best/target (1192,133/80/27,-415x51) 없음
- [공통·W3-3] 텍스트는 위치·정렬·색은 맞게 나오지만 그림자(결정 title 4px, artist 2px), overflow(선곡 subtitle overflow=1 이 줄지 않고 1080px 폭으로 지나감), wrapping 이 없습니다. 일본어는 스킨 TTF 로 나옵니다
- [공통] 그래프 데이터 통로: FrameExtra 가 화면당 한 종류라 decide(None)·musicselect(Select)·play(Play)의 judgegraph/bpmgraph 는 빈 칸입니다. decide (460,30,1000,150), musicselect (1366,97,415,100), play (1402,71,472,107)
- [decide·웨이브 3] 나온 것: bg 전체, lockon 모서리(500ms 에 500x500 반투명 확대 상태, 1500ms 에 난이도 색 50x50 이 (627,743)/(1243,743)/(627,288)/(1243,288)), tablename (960,815) 주황, genre (960,610) 흰색, title (960,490,h90) 주황 가운데 정렬, artist (960,430) 흰색, HYPER 라벨 (840,370,240,45), 레벨 '12' (890,270,71x93 2자리), tips (960,220) 회색. t=0 은 텍스트 a=0 이라 배경과 모서리만. 빠진 것: 검정 띠 -110 (0,200,1920,680,a=200) → 배경이 어두워지지 않음, 스테이지 파일 -100, 노트 분포·BPM 그래프, timer 2 페이드아웃의 BLACK(3000ms 프레임은 1500ms 와 픽셀 동일)
- [result·웨이브 4] 나온 것: bgClear, ring, 캐릭터(t=0 에 2배 → 200ms 에 정상), mainInfo 3초 순환 문구, mainGraphFrame, 랭크 AA 이미지 (95,800), infoFrame 의 NORMAL·1624 Notes, numGauge 86.4%, mainJudgeFrame 메뉴1 전체(EXSCORE 02890, 88.97%, +142, COMBO 00812 +22, PG~MS 값과 early/late 열), Info/CLEAR/AA 버튼 줄, lampGreen 8개, diffFrame 의 7keys·HYPER, 날짜·플레이어 텍스트, chartBtn, 하단 텍스트, 버튼 3개(0,0)/(1800,0)/(1860,0), prepare 연출(500ms 흰 선 2개, 1500ms 띠와 큰 CLEAR 글자, 3000ms 소멸), scoreUpdate UPDATE(3000ms). timer_observe_boolean 기반 메뉴1 타이머가 동작합니다. 빠진 것: 하단 BLACK 바 (0,0,1920,50) → 하단 텍스트가 캐릭터 발 위에 겹침, 게이지 2001 (56,674,400,35)이 검은 빈 칸(gauge.rs 가 결과 화면에서 그리지 않음), gra_fastRate. 잘못 나온 것: grooveGaugeGraph (40,774,655,236)가 R-BMS 자체 양식(자홍 배경 + 빨간 계단선 + 파란 띠)으로 영역을 불투명하게 덮어 같은 사각형의 judgesGraph 가 보이지 않음, fsGraph 는 'type 2 → type 0' 경고와 함께 대체 그림
- [musicselect·웨이브 5] 나온 것: 시작 연출(0ms 검정, 500ms 'welcome to beatoraja world!!!' (325,513)), 배경, main-top/bottom, main-title, skinname, 'Ver 4.6', keyinfo, 검색창, 인디케이터 OFFLINE/BGA/FAV, 버튼 열 7종과 라벨, 키 안내 캐러셀(문구 교체 확인), directory/genre/title(216,255,0)/fullArtist(함수 값) 오른쪽 정렬 기준 x=1000, BPM 프레임과 180, 7KEYS 모드 표시 (1527,213), selectmusic-frame (1112,499,990,82), 스크롤바와 램프(rate 0.25 위치), BPM 연동 캐릭터 (1700,265), 사이드메뉴 아이콘 4개, 하단 3창의 수치 전부(NOTES 1624, TOTAL 320, 판정 막대 5개, EX 2748, 84.60%, 랭크 A, CLEAR, 노트 종류 4개, 밀도 3개), subtitle 스크롤과 main-bg 띠. 빠진 것: 곡 목록 바 전체(1125..2085 영역이 배경만), stagefile -100, banner -102, 분석창 BLACK (1366,97,415,100)과 그래프, slow/fast 의 fastRate (1147,80,-180,34). 1914개 중 그려진 것은 약 100개인데 나머지는 패널·사이드메뉴·폴더·코스 등 조건이 꺼진 객체입니다
- [play7_hw·웨이브 6] 나온 것: 배경, 제목바·TL 01:58·제목 텍스트 (1345,1033), 정보 프레임, BGA 슬롯 (606,290,1280x720)에 대역 이미지, 점수 026800·MAX COMBO 00096·BPM 180, 이펙터·램프(6000ms 에 점등), 진행바, 키 베드·스크래치·키 커버(0ms 화살표), 게이지 프레임·NORMAL GAUGE·36.4, 레인과 READY 타이머에 맞춰 자라는 레인 안쪽 선(4000ms 절반, 6000ms 전체), 판정선·글로, 로딩 창 (121,420,380x311)과 PLEASE WAIT → GOOD LUCK!!!(option 80 → 81 전환), 2000ms 의 BGA 위 곡 정보, 레벨·판정 카운터 3열, 스코어 그래프 프레임·등급선·라벨·0426·+12/-8, 노트/타이밍 그래프 프레임, 6000ms 타이밍 비주얼라이저 선. 빠진 것: 노트와 마디선 전체, BGA·레인·게이지 로드 마스크 등 -110 10개, 스코어 막대 3개, 노트 분포 그래프, 'lamp' 2개(정의 없는 id). 잘못 나온 것: 게이지 바 (25,119,400,35)가 50칸 구분 없이 3색 띠(분홍/진보라/진빨강)로 보임 → W6 에서 노드 시트 대조 필요
- 웨이브 3 이후 이 테스트에 FADEOUT 중간 시각(결정 3500~3900ms)과 결과 메뉴2, 선곡 폴더 바 시나리오를 추가하면 대조 범위가 넓어집니다. 지금은 지시대로 화면당 4시각만 둡니다
- play 시나리오의 READY 타이머(40)는 PLAY 뒤에도 켜진 채입니다. 시나리오 형식이 '켜지는 시각'만 표현해서이며, 끄는 시각이 필요해지면 테스트의 Switch 표처럼 타이머용 표를 추가해야 합니다

## 위험과 주의

- 테스트가 MapHost 에 레퍼런스 선언 option 전부를 false 로 채웁니다. 덤프 CLI(W2-6)가 같은 JSON 을 그대로 MapHost 로 읽으면 목록에 없는 option 은 None 이라 부정 조건(`op = {-2}`)이 거짓이 됩니다. 두 도구의 그려지는 객체 수가 달라 보일 수 있으니 통합 때 기본값 규칙을 한쪽으로 맞추는 것이 좋습니다
- 함수 타이머(timer_observe_boolean)는 평가된 프레임 시각에 켜집니다. 캡처는 0/500/1500/3000ms 네 프레임만 평가하므로 장면 도중에 참이 되는 조건은 실제보다 늦게 켜진 것으로 찍힙니다. 이번 네 화면의 관찰 대상은 시작부터 참인 조건이라 영향이 보이지 않았습니다
- 결과 화면은 스킨이 os.date 로 오늘 날짜를 그리므로 캡처가 날마다 달라집니다. 골든으로 쓰지 않았습니다
- 팩이 있을 때 테스트는 디버그 빌드로 약 30초, RGBA 로 화면당 최대 약 170 MiB(선곡 42.4M px)와 그 복사본을 씁니다. CI 에는 팩이 없어 영향이 없습니다
- 참조 source 선별은 테스트가 모델을 따라가 계산합니다(top-level destination id, note/gauge/judge/songlist 가 나열한 id, imageset). 렌더러의 빌드 규칙이 바뀌면 이 계산과 어긋날 수 있고, 그때는 'has no usable source' 경고로 드러납니다. 이번 실행에서 그 경고는 0건입니다. W3-4 텍스처 관리자가 들어오면 이 선별은 지우는 것이 맞습니다
- 플레이 프레임은 내장 `Skin::default_for(BEAT_7K, 1920, 1080)` 지오메트리를 넘깁니다. 노트가 아예 그려지지 않아 이 지오메트리가 맞는지는 이번에 확인하지 못했습니다
- 다른 단위가 crates/rbms-skin/src/lua/* 와 loader/lua_skin.rs 를 동시에 고치고 있습니다. 제 실행 시점에는 컴파일과 로드가 정상이었고 재시도는 필요 없었습니다. 통합 뒤 수치(객체 수, 함수 수)는 한 번 다시 확인해 주세요

## 실행 기록

- RBMS_SKIN_PACK=/Users/hyunseokbyun/Downloads/ModernChic RBMS_SKIN_CAPTURE_DIR=<scratchpad>/wave2-captures/render cargo test -p rbms-render --test skin_external -- --nocapture → exit 0, 'test result: ok. 2 passed; 0 failed', 29.05s. PNG 16장 생성(decide/result/musicselect-{0,500,1500,3000}, play7_hw-{0,2000,4000,6000}). 전체 출력은 <scratchpad>/wave2-captures/render/run.log
- 실행 출력(화면별): decide 66 객체/69 destination, 이미지 3장 2,759,600 px(최대 1920x1080), 빌드 167ms, 프레임 281~559ms, 그린 객체 5/20/28/28 | result 189/194, 12장 33,515,456 px(최대 3920x3800), 빌드 1361ms, 프레임 841~1033ms, 66/67/69/67 | musicselect 1897/1914(함수 게이트 1384, 함수 타이머 420, 함수 1817개), 11장 42,422,200 px(최대 3200x3200), 빌드 1615ms, 프레임 616~1357ms, 98/103/109/106 | play7_hw 274/289, source 27개 중 23개 참조·23장 34,934,894 px(최대 5190x2571), 빌드 1518ms, 프레임 1046~1121ms, 77/85/72/78. 네 화면 모두 load warnings 0, lua function failures 0, pcall 실패 0, frames over budget 0
- PNG 16장을 전부 Read 로 직접 열어 확인했습니다(시나리오 수정 뒤 바뀐 musicselect-1500/3000, play7_hw-0/2000/4000/6000 은 최종본을 다시 열었습니다). 음수 폭 누락은 result-3000 의 SLOW/FAST 영역과 play7_hw-6000 의 스코어 그래프를 sips 로 잘라 확대해 확인: 빨강 SLOW 막대만 있고 파랑 FAST 막대 없음, now/best/target 막대 없음
- env -u RBMS_SKIN_PACK -u RBMS_SKIN_CAPTURE_DIR cargo test -p rbms-render → 실패 0: lib 285, golden_play 6, golden_result 8, golden_select 4, golden_shell 6, primitives 34, skin_external 2, skin_render 23(골든 포함) 전부 ok
- cargo clippy -p rbms-render --all-targets --all-features -- -D warnings → Finished, 경고 0
- rustfmt --edition 2024 --check crates/rbms-render/tests/skin_external.rs → 통과. 새 파일에 // 주석, #[allow], unsafe, todo!/unimplemented! 없음(grep 확인)
- ModernChic 무변경: 실행 전후 `find . -type f -exec stat -f '%N %z %m'` 목록(574개 파일) diff 가 비어 있음, 폴더 수 179 동일, `find . -newer <실행 전 목록>` 결과 없음. 테스트 자체도 실행 전후 스냅숏(경로·크기·mtime)을 assert 로 비교. 스킨이 쓴 것은 임시 오버레이의 History/information.txt 하나이며 테스트가 끝에 지움
- /Users/hyunseokbyun/development/beatoraja: git status --short 출력 없음(읽지도 쓰지도 않음)
