# V1. 외부 스킨 첫 렌더 결과 — 나온 것과 빠진 것 (웨이브 2B)

> 최종 갱신 2026-10-11 · 대응 단계: 웨이브 7B(동영상 source) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 7B 반영 사항 (2026-10-11)

`crates/rbms-video`(OpenH264 + `re_mp4`)와 스킨 동영상 source 재생을 넣은 뒤의 상태다.

- (W7-2) v1-first-render.md: 상단 반영 사항에 동영상 결과 추가 — 결정·선곡의 '背景の種類 = 動画' 와 플레이의 범용 BGA(#default.mp4)가 그려짐. 캡처 이름 decide_movie-*, musicselect_movie-*, play7_nobga-*(렌더), decide-movie-*, select-movie-*, play-movie-*(앱)


## 웨이브 7A 반영 사항 (2026-10-11)

비트맵 폰트(.fnt) 로더와 표준·distance field 그리기를 넣은 뒤의 상태다.


### 리뷰어의 비트맵 폰트 화면 대조 (2026-10-11)

- 맞는 것 — 画像フォント 를 끈 캡처는 웨이브 6 과 동일: decide/musicselect/musicselect_slide/course/result/result_menu2/play7_hw/play7_bga/play7_cover/play7_judge/play7_visualizers/play5·10·14_hw 40장이 wave6-captures/final 과 cmp 로 바이트 단위 일치
- 맞는 것 — Decide(decide_fnt-1500 대 decide-1500, type 1 거리장): 표 이름·장르·제목·아티스트·Tips 가 TTF 와 같은 가운데 정렬 위치와 같은 높이에 나옴. 제목 크롭에서 글리프가 선명하고 난이도색의 어두운 외곽선이 사방에 둘러짐(outlineWidth=1). Tips 줄은 outlineWidth 가 없어 외곽선 없이 나오며 가나·한자가 폰트 글리프로 그려짐. 페이지 뒤섞임·깨진 글리프 없음
- 맞는 것 — Select(musicselect_fnt-3000 대 musicselect-3000): 오른쪽 정렬인 제목·장르·아티스트·디렉터리(type 1)가 TTF 와 같은 오른쪽 끝에 맞고 어두운 외곽선이 나옴. 곡 바 텍스트(bartext.fnt, type 0)는 왼쪽 끝과 윗선이 같고 넘치는 줄('Second Track -a long title…')이 TTF 와 마찬가지로 상자 밖으로 이어짐. 바 글자 폭은 TTF 보다 조금 넓음(다른 폰트 파일)
- 맞는 것 — Result(result_fnt-3000 대 result-3000, type 0): 날짜와 Player 줄이 같은 가운데 위치. 하단 제목 줄은 SHRINK 로 상자 안에 들어가며 TTF 보다 큼(비트맵은 size=30 기준, TTF 는 dst h 기준이라는 원본 규칙)
- 맞는 것 — Play(play7_hw_fnt-2000 대 play7_hw-2000, type 0): 로딩 대제목·장르·아티스트·상단 제목이 같은 위치. fnt 쪽에 그림자가 없는 것은 스킨이 비트맵 폰트 분기에서 shadowOffset 을 주지 않기 때문(Play/lua/require/textproperty.lua 확인)
- 맞는 것 — 앱 경로 GPU 캡처: decide-fnt-1500ms.gpu.png(폰트 페이지 11장, 경고 0)과 field-font-1920x1080.gpu.png 가 헤드리스 캡처와 같은 모양. 자작 픽스처는 흰 본체·빨간 외곽선·우하단 파란 그림자가 보임
- 맞는 것 — 쓰이는 페이지만 올라감: Decide 는 두 폰트 합계 18장 중 11장만 텍스처가 됨(앱 캡처 로그)
- 틀린 것 — 화면 캡처에서 원본 규칙과 어긋나는 위치·크기·정렬·글리프는 찾지 못함
- 확인 못 한 것 — 한글 등 폰트에 없는 글자의 폴백: ModernChic 폰트에는 한글(U+AC00)이 없고 가나는 있음. 캡처 시나리오의 문자열이 전부 라틴·일본어라 폴백이 화면에 나오는 장면이 없음. 유닛 테스트(a_character_the_font_lacks_…, field_tests 의 폴백 건)가 통과한 것만 확인
- 확인 못 한 것 — type 0 그림자 패스: 비트맵 폰트 분기에서 그림자를 쓰는 텍스트가 Play 의 18px 기호('~', '%', '/')뿐이라 캡처로 판별하지 못함. 유닛 테스트 a_shadow_is_the_line_at_half_brightness_moved_right_and_down 통과만 확인
- 확인 못 한 것 — 실제 GPU 창에서의 동작: 화면 진입 직후와 선곡 스크롤 중 텍스트가 비는 시간, 16 MiB 페이지 여러 장이 한 프레임에 업로드될 때의 끊김
- 확인 못 한 것 — decide-fnt-1500ms 의 GPU·헤드리스 차이 245픽셀(최대 96/255): 레벨 숫자 이미지 가장자리라는 구현 보고를 픽셀 단위로 재확인하지 않음. Metal 외 백엔드(Vulkan·DX12·GL)의 거리장 셰이더


## 웨이브 6 반영 사항 (2026-10-11)

note·judge·커버·bga·비주얼라이저 재작성, 플레이 상태기계(PRELOAD → READY → PLAY → FAILED/FINISHED), 플레이 타이머 드라이버와 오프셋 1~5, 호스트 군집 C·D, 스킨 경로의 내장 레이아웃 의존 제거와 구 어댑터(`state.rs`)·구 드라이버(`screen.rs`) 삭제를 한 뒤의 상태다.

- (W6-2) v1-first-render.md 상단: '플레이의 판정·커버는 내장 필드 지오메트리에 기댄다'는 문장에 'W6-2 뒤 판정·커버는 문서와 오프셋만으로 그려지고 캡처 좌표가 m2 §10 과 일치' 추가
- (W6-1) v1-first-render.md 플레이 항목: '노트 세로 지오메트리가 내장 Skin 기준'을 삭제하고 '웨이브 6: 문서 기준 지오메트리, 7/5/14/10키 렌더 단위 캡처 확인. 앱 경로에서 노트가 흐르는 프레임은 W6-3 뒤 확인'을 추가.
- (W6-6) v1-first-render.md 또는 웨이브 6 반영 사항: ModernChic 7키 BGA 영역(606,290,1280x720)의 본+레이어+미스 캡처(play7_bga-{4000,5000,5500,6000})가 m2 §7.2 좌표와 맞는다는 기록을 추가
- (W6-3) v1-first-render.md 상단: 웨이브 6 플레이 캡처 16장의 목록과 경로, 하니스가 장면·곡 시계를 직접 세는 방식(PlayClocks) 추가

### 리뷰어의 플레이 화면 대조 (앱 경로, 실제 차트 오토플레이, 2026-10-11)

"틀린 것" 중 DP 좌우 판정 콤보, 플레이 로딩 창의 종류별 노트 수는 같은 웨이브의 리뷰 수정 단위가 고쳤다. 선곡 화면의 종류별 노트 수(350~353) 빈칸은 원천 데이터가 없어 남았다. 메인이 수정 뒤 `play7-play-06500ms`(노트·LN·키 빔·봄·판정 GREAT 53·게이지 35.5·점수·BPM·스코어 그래프·노트 그래프)를 직접 확인했다.

- 맞는 것(7키 play7-play-06500ms, BPM150·hispeed 2.0·차트 6500ms): 레인 틀 x 51..570, 판정선 화면 y 841..853(문서 y 227, h 12). 다음 노트(마디 4의 0.2 지점)가 판정선 위 약 234px(계산 0.1375×1706=234.6), 그 다음이 약 570px(계산 575.8), 스크래치 빨간 노트 화면 y 약 107(계산 853-746). 간격 337px = 0.2마디×(853×2.0)=341 로 하이스피드와 일치
- 맞는 것(BPM 75 구간 play7-play-21000ms): 같은 음표 간격이 336px 로 유지(비 CONSTANT 는 section 거리 기준). LN 은 누르는 중 노란 본체가 판정선까지 이어지고 시작 캡이 판정선에 고정
- 맞는 것(판정): 'GREAT 53' 문자 x 132..347(셀 126..353 = 181-55×2/2), 숫자 365..413·420..470(셀 363/418), 화면 y 616..700(문서 380..464). 3자리 'GREAT 233' 은 문자가 27.5px 더 왼쪽. play7_judge 캡처에서 GD 4자리, BD 는 숫자 없음·이동 없음, 600ms 뒤 소멸
- 맞는 것(수치 대 엔진): 6500ms 콤보 53·EX 0106·PG 0053·SCORE 010392(=150000×53/1020+50000×53/1020 각각 절삭)·게이지 35.5(엔진 35.6 의 소수 절삭)·TL 01:25. 21000ms 콤보 233·EX 0466·SCORE 045685·게이지 88.5·목표 대비 +52. 풀콤보 200000·EX 2040·MAX COMBO 01020·게이지 100.0·TL 00:05
- 맞는 것(5/10키 점수): 5키 46/149노트에 030872, 10키 92/298노트에 030872 는 원본 ScoreDataProperty.java:70-73 의 BEAT_5K/10K 식(100000×PG/총노트) 그대로. W6-3 이 의심한 '절반 점수'는 결함이 아님
- 맞는 것(영역): 게이지 바 (25,119,400x35)·숫자 x 429, 점수 (820,222), BPM 최소/현재/최대 75/150/300 (1077/1191/1333,232), 최대 콤보 (1520,222), BGA 사각형 x 606..1886·y 70..790, 스코어 그래프 (596,4,771x200) 막대 3종과 AAA 라벨, 노트 그래프 (1402,71,472x107) 커서, 판정 카운터 x 274/374/474, 진행 바 마커가 시간에 따라 아래로 이동
- 맞는 것(로딩·READY·종료): 로딩 창 (121,349)-(500,660) = 문서 (121,420,380x311), 로딩 바 062%(8개 중 5개), 스테이지파일·제목·장르·아티스트. READY 500ms 에 레인 안쪽이 절반(y 약 420)까지 펼쳐지고 GOOD LUCK. 폐점은 셔터가 닫힌 뒤 STAGE FAILED 와 START/SELECT 재시작 안내, 메시지 창 y 381..698. 페이드 250/500ms 에 화면 전체 절반 어두움
- 맞는 것(DP·5키): 14키 좌 스크래치 381..489·건반 492~, 우 건반 1026~·스크래치 1431..1540, 판정이 좌우에 하나씩, 게이지 중앙 100.0, SCORE 066009(=49507+16502). 10키는 안 쓰는 2레인(783..895, 1026..1138)에 AUTO PLAY LANE 커버. 5키는 x 456..570 에 5키 커버
- 맞는 것(렌더 하니스): play7_cover-5120 에서 레인커버 y 0..170, 히든 커버 580..681, 리프트 커버 682..852, 판정이 리프트만큼 위(448..525). play7_bga 에서 재생 전 검정, 본 그림 위 레이어(검정 투명), 미스 구간은 미스 그림만. play7_visualizers 에서 타이밍 눈금자 x 1402..1874 와 선·EMA
- 맞는 것(원본 특이 동작 재현): READY 동안(차트 시각 0) 노트가 하나도 보이지 않음 — LaneRenderer.java:399 의 0.0×0/0 = NaN 으로 순회가 끝나는 동작 그대로. R-BMS 는 판정 타이밍을 노트 필드에 더하지 않으므로 항상 이렇게 보임
- 틀린 것: play14-play-09000ms 의 좌 (430..825, 620..697)·우 (1075..1467, 620..697) 판정 콤보가 둘 다 134, play10 도 둘 다 92. 원본은 영역마다 그 판정 시점의 콤보라 같은 프레임 화음에서는 서로 다른 수가 나옴(findings 의 프레임 최종 콤보 항목)
- 틀린 것(웨이브 5 대비 회귀): 선곡 계열 16장 (1831,895)-(1903,1053) 의 NOTE/SC/LN/BSS 수가 '0000' 에서 빈칸, 코스 결과 menu2 (223,565)-(361,599) 의 TOTAL 이 빈칸. 플레이 로딩 창의 같은 4개 수치(x 228..300·y 527..595 부근)도 앱 캡처에서 빈칸(렌더 하니스는 시나리오 값 1320/180/112/12 표시)
- 틀린 것(캡처에 안 드러나는 것): 리프트·커버·히든을 켠 상태, GR/GD/BD 판정, 실제 BGA 그림, 사용자 오프셋이 0 이 아닌 경우, 1920x1080 이 아닌 창은 앱 경로 캡처가 없어 화면으로 대조하지 못함. 렌더 하니스 캡처로만 확인
- 아직 없는 것(뒤 웨이브 범위): 비트맵 폰트(.fnt), 동영상 BGA·동영상 배경, 9키 스킨 문서(ModernChic 9키는 내장 폴백), GPU 실창과 실제 오디오 시계에서의 확인
- 아직 없는 것(데이터 원천 부재): 미스 레이어(모델이 채널 06 을 본 그림에 합침), BACKBMP -101·배너 -102, 종류별 노트 수 350~353, 코스 TOTAL 368, 키별 판정 이미지 인덱스 500~519·1510~1699, bgaExpand 설정, 숫자키 1~4 재생 배속(ModernChic 오토플레이 안내문 x 847..1646·y 83..189 은 표시되지만 조작 없음), PMS 4분 박자 확대와 dst2 낙하 입력값

### 성능 실측(리뷰어, 릴리스 빌드, 헤드리스)

- 성능(--release, 스크래치 크레이트에 skin_external 사본+무연산 렌더러, 400프레임 평균): ModernChic 7키 화면 노트 약 512개 = prepare 19us(최악 94us) + draw 명령 생성 43us(최악 327us), 쿼드 730개, Lua 23회/3us. 14키 화면 노트 약 1024개 = prepare 21us + draw 45us, 쿼드 1270개, Lua 32회/5us. 합계 약 0.07ms 로 16.7ms 예산의 0.4%, 6.9ms 예산의 1%. GPU 래스터 시간은 미측정(헤드리스)
- 성능 보조: RunTrace 가 매 프레임·매 입력마다 도는 note_marks 전수 순회는 23040노트 차트에서 평균 36us(최악 41us, 동등 루프 측정). 프레임당 할당은 작은 Vec 몇 개(draw_note 의 Vec<Lane>, lanes_down·lane_keys, RecentHits::errors 가 비주얼라이저당 Vec 2개)뿐이고 타임라인·노트 복사는 없음(LaneNotes 는 빌림, ChartOverview::meta/series 도 빌림)

### 리뷰 수정 뒤에도 남은 것

- 선곡 화면의 종류별 노트 수(350~353)는 여전히 빈칸입니다. rbms-library 의 songdb 에 열을 추가하고(SCHEMA_VERSION 1 → 2 마이그레이션), PARSER_VERSION 을 올려 사용자 라이브러리를 전체 재스캔해야 하므로 이 단위에서 하지 않았습니다. 스키마 변경과 재스캔 비용에 대한 결정이 필요합니다
- 결정·결과 화면의 밀도(360~365)와 메인 BPM(92)도 같은 성격입니다. 원본은 선곡에서 고른 곡이면 SongInformation 값을 보여 주는데 ChartOverview 는 값을 주지 않습니다. 발견 목록 밖이라 손대지 않았습니다
- 이번에 넣은 종류별 노트 수는 ChartOverview.meta() 를 쓰는 결정·플레이·결과 화면 모두에 나옵니다. 원본은 코스 스테이지와 직접 실행에서 이 값이 빈칸이므로 그 두 경로에서는 원본과 다릅니다(코스 결과 화면은 자체 ChartMeta 를 써서 영향 없음)
- 발견 2(한 번 절삭)는 전용 테스트가 없습니다. 테스트에서 장면 시계를 마이크로초 단위로 고정할 수 없어 식 변경만 했습니다
- rbms-skin/src/property/mod.rs 의 UNMAPPED_* 문서 주석과 사용처 없는 UNMAPPED_IMAGE_INDEX 는 리뷰 제안대로 웨이브 8 문서 정리로 남겼습니다
- 사용자 오프셋 환산(발견 10)은 1:1 이 아닌 창의 캡처가 없어 단위 테스트로만 확인했습니다. 1280x720 창에 1920x1080 스킨을 그리는 앱 캡처는 없습니다
- 연습 모드에서 원본은 bga 객체 자리에 연습용 대체 화면을 그리는데, 스킨 장면의 연습 런은 BGA 를 그대로 그립니다. 발견 목록 밖이라 그대로 두었습니다
- 발견 1의 결정(스킨 장면에도 적용, CONSTANT 는 2000ms/하이스피드, 옵션 400 은 false)을 docs/acknowledge 와 사양 §4.5 에 기록해야 합니다. 이 단위는 docs 를 고치지 않습니다
- 발견 14의 연습 재시작 이벤트는 '쓸 계획 없음'으로 보고 삭제했습니다. 스킨 안 연습 UI(STATE_PRACTICE)를 뒤 웨이브에 넣을 계획이면 되살려야 합니다


## 웨이브 5 반영 사항 (2026-10-10)

songlist 재작성, 선곡 상태 모델, 선곡 입력 키 표와 장면 수명, 패널 1~3 과 옵션 이벤트, 호스트 군집 F·H(선곡)·E, 슬라이더 쓰기와 편집 텍스트, 선곡 사운드, 스킨 위 시스템 오버레이를 넣은 뒤의 상태다.

- (W5-1) v1-first-render.md 상단(웨이브 5 반영 사항 신설): '아직 없는 것' 의 '선곡 화면의 곡 바 내용' 해소(W5-1, 렌더 단위 캡처 musicselect-*·musicselect_slide-*, 캡처 위치 wave5-captures/W5-1). 재현 절에 선곡 시나리오의 막대 22개(종류 8, 램프 1~10, 난이도 0~5, 라벨 LN/CN/HCN/MINE/RANDOM, 폴더 분포)와 musicselect_slide 샷 추가
- (W5-5) v1-first-render.md 또는 r3 §8.6 캡처 주의: draw_until_compiled 는 매 프레임 시계를 scene_us 로 고정하므로 timer_observe_boolean 계열 타이머가 그 시각에 시작해 ModernChic 의 helpScene(전체 화면 act)이 켜진 채 클릭을 가로챈다. 상호작용 캡처는 시작 시각에서 컴파일한 뒤 age_skin_scene 으로 시간을 보내는 draw_until_settled 방식을 써야 한다
- (W5-2) v1-first-render.md 상단: 선곡 화면 항목을 '곡 바에 제목·레벨·램프·라벨·폴더 그래프가 나온다(캡처 select-songs/root/table/courses). 하단 정보창 숫자와 스크롤 보간은 W5-4·W5-3a'로 갱신
- (리뷰 수정) v1-first-render.md 상단 '웨이브 5 반영 사항': 앱 경로 선곡 캡처에서 BPM 프레임·숫자, 7KEYS 모드 표시, BGA 인디케이터, BPM 연동 캐릭터(단일 BPM), 좌하단 수치, 스코어창(기록 유무), 코스 곡명 5행, 기본 정렬의 SORT 버튼이 나온다는 것과, 아직 없는 것(노트 종류 4칸, 변속 차트의 캐릭터)을 적습니다

### 리뷰어의 선곡 화면 대조 (앱 경로, 실제 라이브러리, 2026-10-10)

아래 "틀린 것" 4항목(곡 메타 미공급, 코스 곡명, 리플레이 슬롯 오버레이, 기본 정렬의 SORT 버튼)은 같은 웨이브의 리뷰 수정 단위가 고쳤다. 메인이 수정 뒤 `select-panel0`(곡 바 제목·레벨·램프·라벨, BPM, 최고 기록)과 `select-panel1`(옵션 패널) 캡처를 직접 열어 확인했다.

- 맞는 것 — 곡 바: 슬롯 3~12 가 보이고 비중앙 x=1160, 70 간격입니다. 중앙(liston)은 x=1125, 화면 y 505~575(문서 y=505)이고 selectmusic-frame 은 (1112,499,990,82) 입니다. 1300ms 캡처에서 중앙부터 오른쪽에서 미끄러져 들어옵니다
- 맞는 것 — 바 종류: 곡 인덱스 0, 폴더 1(파랑), 표·표 레벨 2(녹색), 완전한 코스 3(분홍), 곡이 빠진 코스 4(보라, '?')
- 맞는 것 — 제목: 곡은 bar+130, 폴더·표는 bar+50, y+15, 왼쪽 정렬. 스캔 직후 곡은 신규 색(255,255,0), 불완전 코스는 (200,200,200). 긴 제목은 줄이지 않고 스크롤바 프레임 뒤로 넘어갑니다
- 맞는 것 — 레벨·램프·라벨: 레벨은 bar+30 에 난이도별 시트(2 녹, 9 황, 12 적, 25 insane, 5 회색), 2자리 가운데 정렬. 램프는 bar-8 에 곡마다 다른 색, 폴더는 NOPLAY. LN 은 bar-90(1070), RANDOM 은 bar+525(1650), MINE 은 bar+640(1800, 프레임에 일부 가림). 렌더 크레이트 캡처에서는 금 트로피(bar+37)와 CN·HCN 라벨도 확인했습니다
- 맞는 것 — 폴더 분포 그래프: bar+40, y+5, 650x10. ALL SONGS(13곡 중 무플레이 3)는 1200~1700 만 채색되고 나머지는 어둡습니다. 표 자체에는 없고 표 레벨(LV 1, LV 2)은 서로 다릅니다
- 맞는 것 — 스크롤바: 프레임 x=1860, 램프 x=1857. index/total x 600 대로 곡 목록 4/13 은 화면 y 약 347~392, 루트 0 은 162~207, 코스 1/2 는 462~507 입니다
- 맞는 것 — 곡 정보 텍스트: directory (990,715) 오른쪽 정렬 'ALL SONGS >', genre y=660, title (1000,570,h70) 오른쪽 정렬 황록색, artist y=520
- 맞는 것 — 참조 이미지와 폴더 표시: 스테이지 파일 (80,406,480,360) = 화면 y 314~674, 배너 (700,785,300,80) = 화면 y 215~295. 폴더 바에서 folder-totalsongs (870,498) 0013, 클리어 현황표 (83,240,917,150) 의 11칸이 분포와 일치하고 값이 0 인 칸은 그려지지 않습니다
- 맞는 것 — 패널: 패널 1 은 op-title (533,58), op-info (487,866), 랜덤 (353,204), 게이지 (659,299), HS 고정 (1006,323), DP (819,162), DP 전용 커버 (1013,157,627,698), 타깃 21행(x=40)이고 선택기는 NORMAL / NORMAL / MAIN BPM / OFF / RATE AAA. 패널 2 는 assist-menu (651,201) 에 REGUL SPEED·LEGACY NOTE·EXPAND JUDGE 가 OFF. 패널 3 은 subop-menu (651,216) 에 GAS NONE, BGA ON, NDTA OFF, +0ms, 시스템 버튼 7행
- 맞는 것 — 버튼: LN TYPE 은 LN, KEY TYPE 은 ALL, SORT TYPE 은 曲名(정렬이 Title 일 때), TEXT 는 no-text, OFFLINE (486,880), FAV/INV (832,880, 곡 바에서만)
- 맞는 것 — 시스템 오버레이: 단축키 안내 x 300~1620 / y 142~938, 필터 x 48~924 / y 90~426, 랭킹 x 48~972 / y 90~990, 기록 모달, 검색 상자 x 24~744 / y 804~858, F1 옵션 패널, 코스 표시 띠, WELCOME·NO RESULTS 카드(막대 0개일 때 songlist 미표시). 연결 점은 좌상단 x 6~21, 디버그 패널은 y 216 부터라 상단 프레임·HELP·버튼 열을 가리지 않습니다. 토스트는 우하단 분석창 위쪽을 일시적으로 덮습니다
- 틀린 것 — 곡 바 위에서 BPM 프레임 (835,442,180,50) 과 BPM 숫자 (650,442) 가 없습니다. 모드 표시 (1527,213,306,39), BGA/NO_BGA 인디케이터 (659,880,168,40), BPM 연동 캐릭터 (1700,265) 도 없습니다. 좌하단 창의 NOTES (150,177), TOTAL (150,146), TIME (150,115), JUDGE (150,53) 와 우하단 분석창 수치는 전부 0 입니다. 같은 문서를 MapHost 로 그린 musicselect_slide-3000.png 에는 180 BPM, 7KEYS, BGA, NOTES 1624, TIME 02:14 가 나오므로 렌더가 아니라 앱 연결 문제입니다
- 틀린 것 — 코스 바에서 course-frame (30,410,1005,345) 의 곡명 5행 (584, y 685/633/581/529/477) 이 전부 빈칸입니다. 완전한 코스(2곡 보유)와 불완전한 코스 모두 같습니다
- 틀린 것 — REPLAY 1 의 선택 오버레이 (1433,1004,53,46) 가 리플레이 없는 곡, 폴더, 코스, 빈 목록에서 항상 켜져 있습니다. MapHost 캡처에서는 4칸 모두 회색입니다
- 틀린 것 — 기본 설정(정렬 Default)에서 SORT 버튼 (1795,956,104,91) 이 그려지지 않습니다(overlay-first-run.png, overlay-no-results.png). 다른 캡처는 하니스가 정렬을 Title 로 바꿔 둔 상태입니다
- 확인 못 한 것 — 최고 기록 창의 수치. 캡처 하니스가 scoredb 를 채우지 않아 기록이 있는 곡(overlay-modal.png 의 EASY, EX 640/1000)도 스코어창은 NO PLAY / 0000 으로 나옵니다. 값 계산은 skin_host/select/tests.rs 단위 테스트로만 확인했습니다
- 아직 없는 것 — 노트 분포·BPM 그래프 (1366,97,415,100), 비트맵 폰트 제목(지금은 대체 글꼴), 동영상 배경, IR 랭킹 목록·라이벌(qco 랭킹, rivalview, 사이드메뉴 랭킹·램프 패널 값), 스킨 설정 화면
- 아직 없는 것 — 설정 원천이 없는 표시: 패널 3 의 NOTES DISPLAY TIME 수치와 HISPEED AUTO ADJUST·CONSTANT 값, 패널 2 의 BPM GUIDE·JUDGE AREA·MARK NOTE·NO MINE 선택기, 자동 저장 조건
- 아직 없는 것 — 헤드리스 캡처는 내장 폰트만 써서 한글 글리프를 확인하지 못했습니다. GPU 창에서의 표시와 조작감(키 반복, 휠, 패드, IME)은 사용자 확인 절차로 남습니다

### 리뷰 수정 뒤에도 남은 것

- 결정 필요: 패널이 닫혀 있을 때 F(기본 7키 배치의 건반 6)는 즐겨찾기와 NEXT_REPLAY 를 둘 다 실행합니다. 리플레이가 2개 이상인 곡에서만 슬롯이 넘어가고 OPTION_CHANGE 음이 납니다. 키 표가 누름 이벤트가 아니라 키 상태를 프레임마다 읽기 때문에, 한쪽만 남기려면 그 인덱스를 표에서 가리는 규칙이 필요합니다
- NUMBER_MAINBPM(92): BPM 이 하나인 차트만 답합니다. 변속 차트는 값 없음이라 BPM 연동 캐릭터가 나오지 않습니다. rbms-library 의 ChartDetail 에 주 BPM 이 있어야 합니다
- 노트 종류별 수(350~353): 우하단 NOTE/SC/LN/BSS 4칸은 여전히 0000 입니다. ChartDetail 이 스크래치·롱스크래치를 나누지 않습니다
- 선곡의 옵션 190~195(스테이지 파일·배너·BACKBMP): 원본은 마지막에 로드한 차트의 그림 유무를 읽습니다. 커서 곡 값으로 틀리게 답하지 않도록 곡 바에서는 비워 두어 구 어댑터가 답합니다. ModernChic 선곡은 이 옵션을 쓰지 않습니다
- 발견 10 의 나머지: 폴더 분포 집계는 기록이 있는 곡마다 조회가 2번(best_score, best_clear_for_md5)입니다. 하나로 합치려면 rbms-store 의 램프 규칙을 앱에 복제해야 해서 두었습니다. 이제 스킨이 그릴 때만 돕니다
- 발견 7 의 나머지: SelectShown 을 만들 때 막대 제목·폴더 경로·코스 스테이지 문자열 복제는 여전히 매 프레임입니다
- 발견 11 의 나머지: SelectTimers 상태(AppShared, SkinScene 의 skin_select_timers)는 W6-8 에서 구 어댑터와 함께 지울 대상입니다. docs/PROCESS.md 에 미룬 사실을 적어야 합니다
- 발견 8 은 테스트가 없습니다. 선곡 테스트에 시스템 사운드를 관찰하는 수단이 없어 코드 변경(print_selection 호출 제거)만 했습니다
- 키 안내(guide.rs)의 건반 역할 표에 REPLAY 와 NEXT_REPLAY, 숫자 4 가 없습니다
- 14키 곡에서 패널 1 의 DP 커버가 걷히는지, 리플레이가 있는 곡의 REPLAY 슬롯 표시는 캡처하지 않았습니다(하니스에 14키 차트와 리플레이 기록이 없음)
- GPU 창에서의 조작감(화살표 슬라이드, 유지 반복, 휠, 패드, IME)과 한글 글리프는 사용자 확인 절차로 남습니다


## 웨이브 4 반영 사항 (2026-10-10)

스킨 입력 디스패치와 이벤트 실행기, gauge·gaugegraph·timingdistributiongraph·judgegraph type 1·2, 플레이 엔진 기록 확장, Result·CourseResult Stage 의 장면 수명, 호스트 군집 B·G·H·E 일부, 스킨 사운드 버스를 넣은 뒤의 상태다.

- (W4-2) v1-first-render.md 상단: 결과 화면 '아직 없는 것' 중 게이지 2001, gaugegraph, 메뉴2 의 그래프 3종(timingdistributiongraph, fsGraph, notesGraph)이 렌더 단위 캡처에서 나옴(W4-2). 재현 절에 result_menu2·course 샷과 Click(mainMenu act 호출) 추가, 캡처 수 17 → 25
- (리뷰 수정) v1-first-render.md 상단(웨이브 4 반영 사항 신설): 앱 경로 단곡 결과의 EXSCORE·레이트·콤보·클리어 타입·UPDATE·판정별 SLOW·preClearType·STDDEV/AVERAGE 가 공급됨, 코스 실패 게이지 바 0 칸, 캡처 위치 wave4-captures/final

### 리뷰어의 결과·코스 결과 화면 대조 (앱 경로, 실제 플레이 결과, 2026-10-10)

아래 "틀린 것" 항목은 같은 웨이브의 리뷰 수정 단위가 고쳤다(단곡 결과의 점수 스냅샷 연결, 코스 게이지 바, IR 상태 줄). 메인이 수정 뒤 `result-clear-3000ms` 캡처를 직접 열어 EXSCORE 00008·100.00%·+8·COMBO 00004·MAX CLEAR·AAA·UPDATE 배지를 확인했다.

- 확인한 캡처: /private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/41ddb2d1-97ab-47c9-943d-b6bc19d0d388/scratchpad/wave4-captures/review 의 앱 경로 result-{clear,fail}-*, course-result-{clear,fail}-* 20장과 렌더 단위 result-*, result_menu2-*, course-* 12장, result-menu-after 를 전부 Read 로 열었습니다(일부는 4장 묶음 시트). 화면 y = 1080 - (y + h)
- 맞는 것 — 배치: mainInfo 줄 y 6~56(3초마다 문구 전환), 그래프 틀 x 35~700·y 65~421, 게이지 그래프 x 40~694·y 70~305, 게이지 바 x 56~455·y 371~405, 랭크 큰 글자 x 95~495·y 112~280, Info 버튼 x 46~280·y 436~479, 판정 행 PERFECT~MISS(y 629~1010), 중앙 키 모드·난이도 라벨 y 982~1022, 날짜·플레이어 y 75~145(코스는 15~85), chartBtn y 6~70(코스에는 없음), 하단 바 y 1030~1080 이 m5 §9.3 표와 일치합니다
- 맞는 것 — 게이지: 5키 NORMAL(보더 75)에서 100% 일 때 청록 37칸(x 56~351) + 적색 13칸(x 352~455)으로 칸 선택식과 일치합니다. 0ms 에 선단 1칸, 1000ms 이후 전부 점등, 선단 뒤 소등 칸(랜덤 애니메이션)이 보입니다. 숫자는 100.0% / 2.0%
- 맞는 것 — 게이지 그래프: 보더 아래 녹색·위 적색 계단선, 1000ms 에 약 2/3 드러남, 3000ms 에 전체. 코스는 3곡이 이어 붙고 곡 경계 흰 세로선(클리어 x 247·465, 실패 x 293·561)과 미도달 곡의 0 바닥선이 보입니다
- 맞는 것 — 타이밍 분포(x 51~683, y 911~998): 5키 NORMAL 판정 폭 띠 PG ±15(0,0,136), GR ±37(0,136,0), GD ±75(136,136,0), BD ±112(136,0,0), 그 밖 검정. 오토플레이는 중앙 열에 막대(12,249,12)와 평균·편차선(170,170,170)이 겹치고, 무입력 실패는 평균선이 -120ms(x 114~115)에 있습니다
- 맞는 것 — 메뉴 1/2 전환: Info 클릭과 좌우 방향키로 바뀌고, 메뉴 2 에 JUDGE NORMAL, TOTAL/Ref, notesGraph(y 651~739)·bpm 선, fsGraph(y 781~869), 타이밍 그래프가 나옵니다. 코스 메뉴 2 는 Course 틀과 1st~3rd 곡명, 4th·5th 빈 칸
- 맞는 것 — 연출: 0ms 캐릭터 2배 → 정상 크기, 1000ms 시작 애니메이션(선 y 380·698, 반투명 띠, 빗금 y 455~625), 페이드 프레임의 검정 + 'Go to the next stage!!!' / 'Do not give up!!!'(x 500~1445, y 527~562). 클리어·실패 배경과 캐릭터 전환
- 맞는 것 — 코스 결과 수치: 클리어는 EXSCORE 00024, 100.00%, +24, PERFECT 00012, CLEAR, AAA, 12 Notes, UPDATE 배지. 실패는 EXSCORE 00008, 33.33%, 랭크 D, FAILED, POOR 00004 와 SLOW 0004, FAST/SLOW 막대가 SLOW 쪽으로 가득, 'ラストまであと 4 Notes'
- 맞는 것 — 렌더 단위 캡처(시나리오 호스트): 값이 전부 공급되면 판정 표·자릿수·정렬, judgesGraph, 1500ms 의 CLEAR 글자(y 380~710), UPDATE 배지까지 m5 대로 나옵니다. 렌더 자체에는 틀린 곳을 찾지 못했습니다
- 틀린 것 — 단곡 결과(앱 경로)의 점수 값: EXSCORE (x 257~397, y 499~535) 00000(기대 00008), 레이트 0.00%(기대 100.00%), COMBO (y 564~600) 00000(기대 00004), diff '+0'(첫 플레이면 +8 / 콤보 diff 는 빈칸), 클리어 타입 (x 296~530, y 436~479) NOPLAY(기대 클리어 램프, 실패는 FAILED), UPDATE 배지 없음
- 틀린 것 — 단곡 결과의 시작 애니메이션: 1000~2000ms 에 preClearType 글자(0,370,1920,330)가 나오지 않습니다(클리어 타입 인덱스가 0 이라 빈 행)
- 틀린 것 — 단곡 실패의 판정 표: POOR 00004 인데 SLOW 칸(x 430~532, y 889~925)이 0000(기대 0004), FAST/SLOW 막대와 합계도 0. 메뉴 2 의 STDDEV·AVERAGE 가 00.00(그래프의 평균선은 -120ms)
- 틀린 것 — 코스 실패의 게이지 바: 선단 1칸 점등(x 56~63). 원본은 미도달 곡이 있으면 0 칸
- 틀린 것 — 스킨 위 IR 상태 줄: x 1730~1905, y 14~38 에 'IR: OFF' / 'IR: SKIPPED (autoplay)' 가 항상 보이고 페이드 검정 위에도 남습니다
- 아직 없는 것(뒤 웨이브): 선곡 화면의 곡 바 내용·패널·이벤트 동작(스킨 버튼 위 클릭은 소비되지만 아무 일도 하지 않음), 플레이 화면의 노트 지오메트리·판정·상태기계, 비트맵 폰트, 동영상 배경, IR 랭킹 목록(380~399)·클리어 분포·IR 메뉴 내용, 표 이름·표 레벨 문자열, 결과 화면 사운드 루프 설정, 편집 텍스트 입력, customEvents, 코스 이전 기록·목표, GPU 실창 확인


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
