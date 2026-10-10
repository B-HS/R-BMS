# V1. 외부 스킨 첫 렌더 결과 — 나온 것과 빠진 것 (웨이브 2B)

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 3(공통 그리기 의미론과 결정 화면) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 3B 반영 사항 (2026-10-10)

스킨 텍스처 관리자(참조 source 만 로드, 화면 이탈 해제, 예산)와 Decide Stage(장면 수명 헬퍼, 백그라운드 차트 로드)를 넣은 뒤의 상태다.

- (W3-7) v1-first-render.md 상단: 결정 화면이 앱 호스트 + 실차트 + 장면 수명으로 캡처됐고 페이드까지 확인됐다는 기록 추가
- (W3-4) v1-first-render.md 상단: 웨이브 3B 에서 화면별 텍스처 수·RGBA 합계 실측(위 수치)과 앱 경로 캡처(select 3초 시점, result + 디버그 패널) 확인 사실 추가
- (리뷰 수정) v1-first-render.md 상단: 팩 캡처 하니스(stage/capture.rs draw_until_settled)가 '시각 0 에서 컴파일 → 5초 늙힘 → 한 프레임' 순서로 바뀌어 pack-select.png 가 도움말 오버레이 없이 나온다는 점과, 3A 의 pack-result.png(Info 패널 행 일부 누락)가 시계 역행의 산물이었다는 점 기록. 캡처 위치 wave3b-captures/final

### 리뷰어의 결정 화면 대조 (앱 경로, 실제 차트, 2026-10-10)

- 맞는 것 — 0 ms(decide-0000ms.png): 배경만 보이고 텍스트·검정 띠 없음. lockon 0번 세트의 시작 사각(500x500, a=30)이 모서리에 옅게 보임. 그래프 영역 (460,900)~(1460,1050) 에 배경 텍스처(0.8 검정, 10노트 단위 띠 (17,17,0), 0초 보조선 회색 63)와 BPM 주선(녹색, y 974~975)이 있음
- 맞는 것 — 500 ms(decide-0500ms.png): 검정 띠 화면 y 200~880(스킨 좌표 0,200,1920,680). 장르 윗선 430, 제목 윗선 500(그림자 포함), 아티스트 610, 카테고리 라벨 665~710, 레벨 숫자 717~810(중앙 x 960), tips 835. m5 §9.1 표·3A 실측과 일치
- 맞는 것 — 난이도 색: 샘플 차트(#DIFFICULTY 없음)는 unknown 으로 제목·라벨·레벨이 (195,195,195) 이고(픽셀 (960,540), (652,290) 실측) 라벨은 UNKNOWN. 자작 차트(#DIFFICULTY 4)는 제목·ANOTHER 라벨·레벨 12 가 빨강
- 맞는 것 — 1500 ms(decide-1500ms.png): lockon 모서리 최종 위치 (627,287)~(1293,792), 50x50, 난이도 색. 3000 ms 와 3500 ms 는 1500 ms 와 바이트 단위로 같음(FADEOUT 499 ms 까지 BLACK 높이 0 인 키프레임과 일치)
- 맞는 것 — 스테이지 파일(decide-rich-1500ms.png): (640,300)~(1280,780) 4:3 사각이 a=200 띠 아래로 어둡게 비침
- 맞는 것 — 그래프(decide-rich-1500ms.png): 노트 분포 칩이 초 단위 열로 쌓이고 10초마다 보조선. BPM 선은 주 150 녹색, 최저 75 파랑(아래), 최고 300 빨강(위), 전환 세로선 회색. 샘플 차트(4초 길이)는 2·3초 열에 칩 1개씩(회색 204)이고 원본 SkinNoteDistributionGraph·SkinBPMGraph 의 계산식과 맞음
- 맞는 것 — 페이드: 3750 ms 는 검정이 y 270~808 로 가운데서 열리고 GET READY 가 (598,516) 에 반투명. 3950 ms 는 검정 y 55~1025, GET READY 거의 불투명. tips 는 검정 위로 남고 제목은 가려짐(z 순서 일치)
- 맞는 것 — 렌더 단위 캡처 17장은 웨이브 3A 와 바이트 단위로 동일. 텍스처 관리자가 그리기 결과를 바꾸지 않음
- 맞는 것 — pack-result.png: Info 패널의 EXSCORE~MISS 행이 전부 그려짐(3A 캡처에서는 일부만 나왔음). pack-play7.png 는 3A 와 파일 크기 동일
- 틀린 것 — pack-select.png·pack-select.gpu.png: 선곡 화면 전체가 도움말 오버레이로 덮임. 스킨이나 렌더 결함이 아니라 캡처 하니스가 스킨 첫 평가 직후 0 ms 를 찍는 문제(findings 의 capture.rs 항목). 같은 팩을 올바른 순서로 찍은 textures-select.png 는 정상이고 좌측 메뉴 아이콘까지 나옴
- 아직 없는 것 — 결정 화면: -101(BACKBMP)·-102(배너) 미공급, 테이블명·테이블 레벨은 TableLevel 보기에서 고를 때만 채워짐(캡처에서는 빈 값), 코스 stage 표시는 캡처로 확인하지 못함
- 아직 없는 것 — 스킨이 읽히는 동안 Decide 는 검정 화면(내장 레이아웃 없음). Select·Play·Result 는 그동안 내장 화면을 그렸다가 스킨으로 바뀜
- 아직 없는 것 — 플레이 화면은 스킨이 오는 동안 곡은 진행되고 스킨 시계만 멈춤. 그 사이 켜진 플레이 타이머는 한 시각으로 모임(PRELOAD 대기는 웨이브 6)
- 아직 없는 것 — 실제 GPU 창에서의 선곡 → 결정 → 플레이 흐름과 사운드 확인(사용자 절차). 이번 검토는 헤드리스·오프스크린 캡처와 코드 추적까지


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-0) v1-first-render.md: '[공통] 그래프 데이터 통로' 항목 — 통로 해소(W3-0, 2026-10-10): 화면과 무관하게 series 를 채울 수 있음. 데이터 공급은 미완이라 캡처는 그대로
- (W3-0) v1-first-render.md: '[공통·W3-2] 음수 id 참조 이미지' 항목과 실행 기록의 객체 수 — 경고 문구가 사라지고 객체로 조립됨(decide 66→69, result 189→194, musicselect 1897→1914, play7_hw 274→287 객체). 그려지는 객체 수와 픽셀은 동일. build warnings 는 decide 0, result 1, musicselect 1, play7_hw 2
- (W3-1a) v1-first-render.md: '[공통·W5-1/W6-1] dst 없는 최상위 destination' 해소(W3-1a, 2026-10-10) — songlist·notes·judge(def)가 그려짐. 근거는 SkinBar/SkinNote/SkinJudge 생성자의 자체 키프레임
- (W3-1a) v1-first-render.md: '[musicselect·웨이브 5] 빠진 것: 곡 목록 바 전체' 일부 해소(W3-1a) — 바 이미지는 x 1125~, 선택 바 y 505~575 에 나옴. 바 안 제목·레벨·램프·트로피는 여전히 없음(W5-1). 그린 객체 수 99/104/110/107
- (W3-1a) v1-first-render.md: '[play7_hw·웨이브 6] 빠진 것: 노트와 마디선 전체' 일부 해소(W3-1a) — 노트는 레인 열(스크래치 x 54~160)에 나옴. 마디선과 NOTES_1P 오프셋 적용은 여전히 없음(W6-1). 그린 객체 수 78/86/73/79
- (W3-1a) v1-first-render.md: '구현 단위의 요약' 5번의 공통 원인 (나) 해소 표시, '위험과 주의'의 '함수 타이머는 평가된 프레임 시각에 켜집니다' 항목에 '평가는 이제 prepare 단계에서만 일어남' 추가
- (W3-1b) v1-first-render.md: 캡처 수치의 객체 수(결정 69, 결과 195, 선곡 1914, 플레이 286~288)가 정리 후 수치(34, 159~160, 1911, 270)로 바뀌었음을 갱신. 빠진 것 목록의 원인 (가)~(라)는 이 단위에서 해소한 항목이 아님
- (W3-6) v1-first-render.md: [decide·웨이브 3] 앱 경로 결정 화면의 난이도 색·모서리 색·라벨·레벨·장르·제목(FULLTITLE)·아티스트가 호스트 군집으로 나옴 — 해소(W3-6, 2026-10-10, 앱 경로 캡처 확인). 테이블 이름 텍스트(960,815)는 앱에 곡→표 조회가 없어 아직 빠짐
- (W3-6) v1-first-render.md: [result·웨이브 4] '날짜·플레이어 텍스트'는 렌더 단위 캡처에서만 나오던 것이 앱 경로에서도 나옴 — 해소(W3-6). 단 날짜는 UTC 라 로컬과 하루 어긋날 수 있음(위험 항목)
- (W3-2) v1-first-render.md: '[공통·W3-2] 음수 id 참조 이미지' 해소(W3-2, 2026-10-10). '[공통·W3-2] 음수 폭/높이 destination' 해소(W3-2, 2026-10-10)
- (W3-2) v1-first-render.md [decide]: 빠진 것 중 '검정 띠 -110', '스테이지 파일 -100', 'timer 2 페이드아웃의 BLACK' 해소. 테스트에 3750ms 시각이 추가되어 페이드아웃 중간 프레임이 찍힘
- (W3-2) v1-first-render.md [result]: '하단 BLACK 바', 'gra_fastRate' 해소. [musicselect]: 'stagefile -100', 'banner -102', '분석창 BLACK', 'fastRate (1147,80,-180,34)' 해소. [play7_hw]: '-110 10개', '스코어 막대 3개' 해소
- (W3-2) v1-first-render.md 재현 절: 프레임이 tests/skin/external/images/{stagefile,backbmp,banner}.png(자작)을 참조 이미지로 넘기고, 시나리오는 191/193(스테이지 파일·배너 있음)을 켠다고 추가. play destination 수 268 → 269
- (W3-5) v1-first-render.md [공통] 그래프 데이터 통로: 해소(W3-5, 2026-10-10)
- (W3-5) v1-first-render.md [decide·웨이브 3]: '노트 분포·BPM 그래프' 빠진 것 해소(W3-5). 캡처에서 (460,30,1000,150)에 나옴
- (W3-5) v1-first-render.md [musicselect]·[play7_hw·웨이브 6]: '노트 분포 그래프' 빠진 것 중 judgegraph/bpmgraph 해소(W3-5). 플레이 커서와 BPM 선도 나옴. 결과 Info 메뉴 2 의 세 그래프는 클릭 전환 후 확인 필요
- (W3-3) v1-first-render.md: [공통·W3-3] 항목 해소(W3-3, 2026-10-10) — 그림자·overflow·wrapping 구현. 같은 항목의 '선곡 subtitle overflow=1 이 줄지 않고 1080px 폭으로 지나감'은 정정 필요: 그 부제는 dst 폭 1920 보다 짧아 원래 줄지 않는 것이 맞고, 실제 결함은 굵기(black 대신 medium)와 세로 기준(줄 상자라 약 40px 아래)이었음
- (W3-3) v1-first-render.md 상단 '메인이 직접 연 캡처' 문단: '부제 텍스트가 크게 넘쳐 겹친다'에 해소 표시 — 부제는 띠(화면 y 314..674) 안 y 410~540 에 들어가고, 제목과의 겹침은 원본 좌표 그대로임
- (W3-3) v1-first-render.md [decide·웨이브 3] 항목: 텍스트 4종의 실측 좌표 추가(title 잉크 x 301..1620 y 499..585, 그림자 4px, 전부 중심 x 960) 와 '제목·tablename 이 black 굵기로 나옴'
- (리뷰 수정) v1-first-render.md: 웨이브 3A 리뷰의 '앱 경로에서 이미지 인덱스 ref 를 쓰는 image/imageset 누락'(선곡 LN·KEY·SORT·FAV/INV, 결과 chartBtn·useOption·clearType) 해소. 군집이 채워질 때까지 구 어댑터가 첫 세트(UNMAPPED_IMAGE_INDEX = 0)를 답한다고 적습니다
- (리뷰 수정) v1-first-render.md: '스킨 시계(NUMBER 21~26)가 UTC' 해소. rbms_skin::lua::local_time 이 os.date 와 같은 C 라이브러리 달력으로 로컬 시각을 읽고 새 의존성은 없습니다. 미해결 목록에 올리지 않습니다

### 리뷰어의 화면 대조 (2026-10-10, 캡처를 직접 열어 조사 문서 좌표와 대조)

- 결정(review/decide-0·500·1500·3000·3750) 맞는 것: bg 전체, 검정 띠 (0,200,1920,680) a=200 → 화면 y 200~880 전폭, 스테이지 파일 (640,300,640,480) → x 640~1280·y 300~780 이 띠 아래에서 어둡게, 500ms 는 1x1 시작점이라 안 보임, lockon 모서리 500ms 확대 상태(세트 0 이 약 x 157~544·y 72~459)와 1500ms 의 (627,743)/(1243,743)/(627,288)/(1243,288) 50x50 HYPER 색, tablename 윗변 y 230·genre 430·title 500(잉크 약 498~590, 중심 x 962)·artist 610·tips 835 전부 x=960 가운데 정렬, 난이도 색 (255,192,0) 한 벌만, HYPER 라벨 (840,370,240,45), 레벨 12 가 x 890~1032·y 717~810, 그래프 두 개 (460,30,1000,150) → y 900~1050(0ms 는 바탕과 BPM 선만, 500ms 부터 칩), BPM 선 180 초록 중앙·90 파랑 1/3·240 빨강·정지 자홍, 3750ms 페이드 BLACK y 270~810 과 GET READY (598,514,730,50) 반투명
- 결정 틀린 것: 없음. 아직 없는 것: 앱 경로의 스테이지 파일·그래프 데이터 공급과 Decide Stage(W3-7), bg 동영상 source(웨이브 7, 앱 로그에 sample.mp4 디코드 경고)
- 결정 앱 경로(review-app/pack-decide.png, .gpu 동일 배치): 곡 없는 하니스라 UNKNOWN 라벨 (840,370)·레벨 0(align 2 로 x 928~995 중앙)·회색 모서리·tips·검정 띠가 좌표대로 나옴. 제목류와 그래프는 데이터가 없어 비어 있음(W3-7)
- 결과(review/result-0·500·1500·3000) 맞는 것: mainInfo (35,1024,665,50), mainGraphFrame (35,659,665,356), judgesGraph 칩 (40,774,655,236), 랭크 AA (95,800), infoFrame·NORMAL·1624 Notes, numGauge 86 / .4, mainJudgeFrame (35,70,665,582), EXSCORE 02890·88.97%·+142, COMBO 00812 +22, PG~MS 3열, UPDATE (570,545/480,107,36) 3000ms, gra_slowRate 빨강 x 427~568 과 gra_fastRate (673,427,-246,26) 파랑이 673 에서 왼쪽으로, 0174/0180, lampGreen 8개, diffFrame·7keys·HYPER, 날짜 윗변 y 75·플레이어 115, chartBtn (795,1010,330,64), 하단 BLACK (0,0,1920,50) 위 bottomResult, 버튼 3개, 0ms 캐릭터 2배
- 결과 틀린 것: 렌더 캡처에는 없음. 앱 캡처는 findings 1·2(chartBtn·useOption·clearType 누락, 날짜 하루 전). 아직 없는 것(뒤 웨이브): 게이지 2001 (56,674,400,35) 빈 칸과 grooveGaugeGraph 자체 양식(W4-2), Info 메뉴 2 의 그래프 3종과 클릭(W4-1), 앱 경로의 판정 표·랭크·램프·라벨(W4-5)
- 선곡(review/musicselect-0·500·1500·3000) 맞는 것: 시작 연출(0ms 검정, 500ms welcome 문구 (325,513)), stagefile (80,406,480,360) → x 80~560·y 314~674, banner (700,785,300,80), main-top/title/skinname/Ver 4.6/keyinfo, 검색창, OFFLINE/BGA/FAV, 버튼 열과 LN·ALL·曲名, 캐러셀, directory 오른쪽 끝 990·genre/title/fullArtist 1000 에 윗변 340/390/440/530, BPM 프레임과 180, 7KEYS (1527,213), selectmusic-frame y 499~581, 스크롤 램프 y 약 318~350, 하단 3창 수치 전부, bar-fastRate (1147,80,-180,34) 파랑이 빨강과 맞닿음, 분석창 BLACK (1366,97,415,100) 위 분포·BPM 선, 캐릭터 (1700,265), 사이드 아이콘 4개, subtitle 스크롤(3000ms x=768 시작)
- 선곡 틀린 것: 렌더 캡처에는 없음. 앱 캡처는 findings 1(LN·KEY·SORT 버튼, FAV/INV 누락). 아직 없는 것(W5): 곡 바 안의 제목·레벨·램프·트로피와 바 종류별 그림(지금은 바 이미지와 선택 프레임만), 검색 텍스트 편집(원본은 ref 30 에 writer 가 있으면 빈 문자열도 그림), 폴더 클리어 현황 수치, 패널·IR
- 플레이(review/play7_hw-0·2000·4000·6000) 맞는 것: -110 마스크 검정, 스코어 막대 now/best/target 이 1192 에서 왼쪽으로, 분포·BPM 그래프 (1402,71,472,107)와 흰 커서, -100 FIT_WIDTH_TRIMMED 가 BGA 위에 희미하게, 로딩 창과 PLEASE WAIT, 노트가 레인 열에 표시. 아직 없는 것(W6): 노트 세로 지오메트리·마디선, 게이지 50칸, 판정, 'lamp' 2개, BGA 위 제목이 직전 가산 blend 를 물려받아 밝게 번짐(원본 규칙대로이나 실기 대조 전), 1280x720 래퍼로 인한 텍스트 흐림
- 구현 단위 서술과 실제 이미지 대조: W3-0~W3-6 의 좌표 서술은 다시 찍은 캡처와 일치합니다. 다른 곳은 W3-1a 의 '폴백으로 문서가 전과 같이 보인다'는 주석(findings 1)과 W3-1b 의 None 유지 서술(findings 7)입니다


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
