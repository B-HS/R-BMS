# ADR: 기본 스킨 선곡 화면을 IIDX 식으로 바꿈(모노톤 결정의 부분 변경, 일반어 문구)

> 최종 갱신 2026-10-29 · 대응 커밋 66b9e63(상태 줄과 관계 줄만 2026-10-29 에 고쳤다)
> 상태: 일부 대체됨(메인 결정 3 의 BPM 문장만. 아래 관계 줄) · 나머지는 유효(질문 2~11 과 색 결정은 사용자 확인 2026-10-24 "18번까지 전부 추천안대로". 아래 "메인 결정"은 사용자 미확인) · 결정일 2026-10-24 · 결정자 사용자(요청과 결정 묶음)와 메인(세부, M5·M8·M10) · 커밋 `038481b`(색·배치 토큰), `b5f15df`(막대 등급·썸네일 확장 키, 선곡 보조 값), `d75b819`(분석·스킨 언어), `c0bebfa`(목록·머리·바닥), `90d57d1`(정보창), `eadde84`(확장 불린 `op` 로더 수리, 경로 이중 표시 정리)
> 부분 변경하는 결정: 2026-10-12 모노톤 결정([Lua 스킨 호환 결정](2026-10-09-lua-skin-compat-decisions.md) D18 의 무채색 방침과 기본 스킨 모노톤, 웨이브 14). 바뀌는 점은 아래 1절이다. 나머지 모노톤(바탕·면·선·글자)은 그대로다.
> 관계: [NOTES RADAR ADR](2026-10-24-adr-notes-radar.md), [랭킹 모드 ADR](2026-10-24-adr-ranking-mode.md), [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md).
> 일부 대체됨(2026-10-29): 메인 결정 3 의 "곡 머리의 큰 BPM(BPM 은 CHART DATA 에만)을 뺐다"는 [선곡 정보창 overflow 와 NPS ADR](2026-10-29-adr-select-info-overflow.md)이 되돌렸다(LEVEL·BPM 을 곡 머리 오른쪽으로 옮김). 이 파일의 문장은 고치지 않았다. 정보창의 좌표·칸 구성도 그 ADR 의 4절로 바뀌었다.

## 배경

- 사용자 요청(2026-10-24): "UI/UX 를 조금 더 꾸며봐봐. 또한 곡선택부분에 보면 곡 + 왼쪽에 스코어도 들어가있는데 이것도 구현하도록하고, 기타 화면에 있는것처럼 조금 더 iidx와 비슷하게끔 구현 진행해봐 현재 r-bms 에있는 내용도 표시 가능하게하고. rival 기능은 당장은 필요없는데 해당 UI 부분에 다른 수치를 넣는다던가 하면되겠지". 참고 화면은 IIDX 선곡(막대 왼쪽 등급, MUSIC DATA·SCORE DATA·RIVAL SCORE DATA 패널, NOTES RADAR).
- 조사 보고서 [2026-10-24-select-ui-iidx-survey.md](../reference/2026-10-24-select-ui-iidx-survey.md): 기본 스킨 선곡은 목록 + 곡 정보 영역이었고, 막대에는 램프와 레벨뿐이었으며, 정보창 값 일부(종류별 노트 수 350~353, 밀도 360~365, 주 BPM 92)는 빈칸이었다. 막대에 DJ 등급·썸네일을 올릴 `songlist` 키가 없었다.

## 결정

### 1) 색: 구조는 IIDX 식, 바탕은 모노톤 유지(질문 1, M5)

- 바탕·면·선·글자의 모노톤은 그대로 둔다. **강조색이 허용되는 곳은 넷뿐이다**: 막대의 DJ 등급 배지(A 구리, AA 은, AAA 금, B~F 회색 단계), 램프 띠(기존 램프 색), SCORE TARGETS 의 메달(금·은·동. 코스 트로피 `TROPHY_COLORS` 와 같은 값), NOTES RADAR 의 노랑 반투명 다각형과 축 라벨 6색. 등급·메달 면과 글자의 대비는 모두 4.5:1 이상이다.
- 랭킹 강조(내 줄·라이벌 줄)는 강조색 허용 대상에 없어 새 색을 만들지 않았다: `theme.ranking.you` 는 `theme.color.accent`, `you_face` 는 `raised`, `rival` 은 `sub` 를 가리킨다.
- 토큰은 `theme.lua` 에 이름 있는 값으로만 더했다(기존 토큰 값은 바꾸지 않았다): `theme.rank`·`rank_ink`(f~aaa, 파일 순서가 DJ LEVEL 인덱스 0~7 과 같다), `rank_key[0..7]`·`rank_last`, `medal`·`medal_ink`·`medal_order`, `radar`(`area`, `side`, `radius`, `center`, `fill`, `fill_alpha`, `star`, `axis`, `axis_order`), `select_layout`(좌표 상수). 기존 `theme.rank_name` 은 AAA 가 1번이라 순서가 반대이므로 인덱스를 쓸 때 어느 표인지 구분한다. 램프 띠 색은 새로 만들지 않았다(`theme.lamp`·`theme.clear[id].color` 재사용).

### 2) 레이아웃(질문 2, 1920x1080)

지금 레이아웃을 IIDX 식으로 교체했다(두 레이아웃을 헤더 속성으로 공존시키지 않는다). 새 문서가 `rbms_extensions` 헤더를 유지한다.

| 영역 | 내용 |
| --- | --- |
| 곡 막대 안 | 등급 배지 0..56(`rank` 키, F~AAA 8조각), 램프 띠 56..64, 레벨 칸 64..128(`num32` 를 칸 가운데), 제목 144 부터(폴더·표 같은 일반 막대의 제목은 176), 썸네일 664..728(`thumb` 키, 128x128 을 64x64 로), 라벨은 656 이 오른쪽 끝. 코스 트로피는 배지 자리 가운데 |
| 선택 막대 | 틀은 1104 부터 시작해 위아래 4씩 커지고 2px 테두리와 8px 강조 띠. 가운데 칸만 16px 왼쪽으로 돌출한다. 4.4절 원안의 "x 1096 에서 더 왼쪽"은 chart_data 패널(352..1096)과 세로로 겹쳐 `selected.left = 1104` 로 바꿨다(패널과 8 떨어짐) |
| 스크롤 막대 | 트랙(x 1872, 폭 8)을 열린 폴더의 램프 분포(Rate 21040~21050 누적)로 칠한다(아래가 MAX, 위가 NO PLAY). 합(21051)이 부재이거나 0 이면 그리지 않는다. 위치 손잡이는 그 오른쪽(1884, 폭 4) |
| 머리 | PLAY STYLE 칩(키 모드 옵션 160~164: SP·DP·9KEYS, 폴더 위는 `-`), FREE PLAY/COURSE 칩(커서가 코스 막대인지 `GRADEBAR`. `OPTION_MODE_COURSE`(290)는 코스 진행 중에만 참이라 선곡에서는 항상 거짓이므로 쓰지 않았다), 시각 칩(정수 24·25 를 `%02d:%02d`, 부재면 `--:--`), 줄인 검색 상자(640..976), 정렬 라벨은 기존 SORT 단추 값 표시로 대신. 목록 위(1128, top 120)에 위치 경로(`STRING_DIRECTORY`, 끝의 ` > ` 만 지움) |
| 바닥 | 첫 줄을 키 이름 상자(ENTER·ESC·UP DOWN·H)와 라벨로, REPLAY·AUTOPLAY·PRACTICE·FAVORITE 옆에 RANKING 단추(이벤트 21002, 켜짐은 `op = { 21105 }`) |
| 정보창 왼쪽 | 곡 머리(제목·부제·아티스트, 전과 같다), 곡 그림 256 정사각(64, 352. 질문 11, 확대 팝업 없음. 그림 숨김 설정이면 커버 칸을 비운다), NOTES RADAR(64, 640) |
| CHART DATA(352, 352) | 난이도 탭 5칸(레벨 정수 21030+s, 있는 칸에만 난이도 색 밑줄 21115+s, 지금 차트의 칸은 옵션 151+s 로 채움, 클릭은 이벤트 21010+s), NOTES·LEVEL·BPM, 밀도 그래프(`judgegraph` type 0), 레인 막대(Rate 21000+l, 5·7키는 스크래치를 왼쪽에, 10·14키는 두 줄로 2P 스크래치를 오른쪽에), 오른쪽 열의 작은 글자 한 줄 LENGTH·TOTAL·KEYS·JUDGE. 레인 색은 모노톤(흰 건반 text, 검은 건반 sub, 스크래치 accent) |
| MY SCORE(352, 640) | SCORE, MISS COUNT, 큰 등급 배지, 램프 이름, 다음 등급까지(`NUMBER_DIFF_NEXTRANK`. "SCORE + 차이"가 닿는 n/9 구간으로 등급 이름을 정하고 목표가 최대 EX 이상이면 `MAX -n`), F~AAA 진행 막대(`RATE_SCORE` 110, 2/9~8/9 눈금), MAX COMBO·PLAY COUNT·CLEAR COUNT·RATE, 판정 5행(가로 띠) |
| SCORE TARGETS와 CLEAR RATE | AAA·AA·A 의 필요 EX = `ceil(TOTAL NOTES x 2 x k/9)`, k = 8·7·6(금·은·동), 달성하면 메달 면을 칠하고 못 하면 raised 면에 메달색 글자와 `+n`/`-n` 차이. CLEAR RATE = CLEAR COUNT / PLAY COUNT(소수 1자리). 기록 패널 아래 전체 폭 한 줄(`targets`, 원안의 패널 폭 312 에는 메달 셋과 CLEAR RATE 가 들어가지 않는다) |
| MY RECORDS(784, 640) | RIVAL 자리: BEST 1줄 + 최근 5판, 줄마다 램프·DJ LEVEL 배지·EX·MISS·날짜(앞 10글자). 랭킹 모드(불린 21105)에서는 같은 자리가 IR RANKING 6줄 |

- 새 시트 조각은 등급 배지 8개(48x28, Inter Bold 로 구운 `rank_f`~`rank_aaa`, `tools/sections/select.py`의 `rank_badge`)뿐이다. 정보창의 나머지는 사각형·글자·숫자·참조 이미지·그래프로 그린다. 막대 태그 위치는 `BAR_TAG_LEFT = 72` 로 옮겨 램프 띠와 겹치지 않는다. `select.png`·`atlas/select.lua` 는 `theme.lua` 의 `rank`·`rank_ink` 에 의존하므로 `generate-images.py --check` 가 지킨다.
- 폴더·코스 막대의 정보(`info.lua` 의 곡 머리·폴더·코스 그리기)는 그대로 두고, 곡 머리 줄에 겹쳐 그리던 위치 경로는 목록 위로 모았다(경로가 두 번 보이던 것을 I7 이 정리).
- 내장(스킨 없는) 선곡 렌더러는 그대로다(질문 10a). 카테고리 아이콘 탭은 만들지 않았다(10c).

### 3) 문구는 일반어(M10, 질문 9)

| IIDX 표기 | 이 화면의 표기 |
| --- | --- |
| MUSIC DATA | CHART DATA |
| SCORE DATA | MY SCORE |
| SCORE CLASS | SCORE TARGETS |
| CLEAR VISUALIZATION | (표지 없이 스크롤 막대의 램프 누적) |
| RIVAL SCORE DATA | MY RECORDS(랭킹 모드에서는 IR RANKING) |
| CLEAR RATE, NOTES RADAR | 그대로 |

번역은 세 언어이고 패널 제목과 레이더 축 이름은 용어집대로 세 언어 모두 원어다([i18n ADR](2026-10-24-adr-ui-language.md)).

### 4) `songlist` 확장 키 `rank`·`thumb`(질문 3)

- 이 플레이어 전용 키(beatoraja 는 무시): `rank = { dst x 8 }`(인덱스 0~7 = F, E, D, C, B, A, AA, AAA, 각 항목의 `id` 는 일반 `image` id), `thumb = dst 하나`(`id` 는 아무것도 가리키지 않는다. 그림 전체를 소스로 `stretch` 대로 맞추고 색·알파는 dst 의 것). 막대 왼쪽 위 모서리 기준 상대 좌표이고 램프처럼 "마지막에 놓인 자리"에 그린다. 모델 `SongList.{rank, thumb}` 는 `#[serde(default)]` 라 JSON 스킨도 읽는다. 로더는 같은 필드 목록을 네 곳(`from_lua.rs`, `script.rs`, `branch.rs`, `loader.rs`)에서 따라가므로 함께 고쳤다.
- 그리기 순서: 램프 → 썸네일 → 등급 → 레벨. 새 부품은 prepare 도 원본 부품이 모두 끝난 뒤 맨 끝에 묻어, 두 키가 없는 외부 스킨은 그리기와 Lua 호출 순서가 예전과 같다.
- 배지는 라이브러리에 있는 곡 막대 중 보조 판이 아닌 기록이 하나라도 있는 막대에만 그린다. 폴더·코스·없는 곡·기록 없는 곡은 그리지 않는다. `SongBar.rank: Option<u8>`.
- 썸네일은 곡 막대에만 붙고 첫 프레임에는 없다가 작업자가 읽은 뒤 나타난다. 작업자(`stage/select/thumbs.rs`): 커서 위 11개·커서·아래 12개(텍스처 풀 24 와 같은 24개)를 가까운 막대부터 읽고, 스테이지 그림을 먼저 시도해 없거나 깨졌으면 배너를 쓰며, 가운데 정사각을 잘라 `imageops::resize(Triangle)` 로 128x128 로 줄이고(디코드 원본은 작업자 안에서 버림) 최근 48장 LRU 로 보관한다. 작업자는 늘 가장 새 목록만 처리해 지나간 요청을 건너뛴다. 새 의존성은 없다(`image` 0.25 의 `imageops` 는 지금 기능 설정으로 컴파일된다). 막대 캐시가 텍스처가 바뀔 때마다 다시 만들어지지 않도록 `SongBar` 가 아니라 `SongBars.thumbs: &[(usize, TextureId)]` 로 싣는다.

### 5) 내 기록 6줄과 형제 차트(질문 5, S3)

- 형제 차트: 현재 목록(`select_items`)에 있는 차트 중 같은 폴더·제목·모드이고 `#DIFFICULTY` 1~5 가 슬롯. 목록에 없는 차트를 슬롯에 넣으면 이벤트가 이동시킬 수 없어 막았다. 같은 슬롯을 두 차트가 주장하면 커서 차트가, 그다음은 목록에서 먼저 나온 차트가 가진다. 이동은 클릭과 같은 효과음.
- 폴더 램프 분포는 열린 폴더에서 디스크에 있는 곡 막대만 램프 id 별로 센다.
- MY RECORDS: 0행(BEST)은 점수 DB `best()`(정보창 SCORE·램프·MISS 와 같은 출처), 날짜는 점수 로그에서 같은 EX 를 처음 낸 보조 없는 판. 1~5행은 점수 로그의 최근 5판(최신순, 보조 판 포함, 읽기 한도 1000판). MISS 는 도달하지 못한 노트를 더한 값이라 `ScoreBook` 이 아니라 로그를 읽는다. 기록을 깰 수 있는 판이 한 번도 없으면 BEST 행이 없다. 날짜는 로컬 시각.
- IR 상위(질문 6a)는 랭킹 모드(21105)일 때만 같은 자리에 보인다(M8).

## 메인 결정(사용자 미확인)

1. **막대 등급과 내장 선곡 행의 DJ LEVEL 은 정보창 RANK 와 같은 기준(보조 판 제외, `best_ex_for_md5`)이다**: 보조 판으로만 최고를 낸 곡은 내장 행 등급이 낮아지거나 사라진다. 판정·점수 의미는 바뀌지 않는다. 지금 R-BMS 대비 바뀜(`best_score` 가 보조 판을 뺀다).
2. 형제 차트를 라이브러리 전체가 아니라 현재 목록으로 한정했다.
3. 정보창에서 DIFFICULTY 이름 칸, 판정 5행의 비율 막대, 곡 머리의 큰 BPM(BPM 은 CHART DATA 에만)을 뺐고, 기록 줄 날짜는 앞 10글자만 보인다.
4. `selected.left = 1104`, `targets` 줄을 전체 폭으로, 머리 칩의 좌표(4.4절 원안에 좌표가 없었다), 바닥 안내 줄의 토큰은 만들지 않음.
5. 읽는 썸네일 범위를 커서 위 11·아래 12 로 고정(슬롯이 24개를 넘는 외부 스킨은 끝쪽 막대에 그림이 없다. 외부 스킨은 키를 모르므로 영향 없음).

## 이유

- IIDX 처럼 곡 목록에서 바로 내 기록·난이도·레이더를 보는 것이 요청이고, 사용자는 현재 R-BMS 에 있는 내용도 표시하라고 했다.
- 강조색 넷으로 한정하면 2026-10-12 모노톤 결정의 취지(색이 의미를 가진 곳에만)를 지킨다.
- 일반어 이름은 IIDX 의 고유 표기를 피하고(M10) 사용자가 정한 NOTES RADAR 만 원어를 유지한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 두 레이아웃을 헤더 속성으로 공존(질문 2) | Lua·시트 두 배 |
| 막대 확장 없이 정보창에만 등급 표시(질문 3) | 요청이 막대 왼쪽의 스코어 |
| 전체를 IIDX 색(채도 높은 패널)으로 | 2026-10-12 모노톤 결정을 통째로 뒤집는다. 부분 변경만 사용자 확인 |
| IR 상위 6명을 항상 표시(질문 6a) | 서버 요청 증가 |
| 선곡 `op` 대신 함수로 확장 불린을 읽는 우회 유지 | 로더를 고쳐 `op` 를 쓴다(확장 id ADR) |
| 썸네일을 막대 캐시(`SongBar`)에 심음 | 텍스처가 바뀔 때마다 목록 재구성 |
| 내장 선곡 렌더러에도 배지·썸네일(Q10a) | 범위 밖 |

## 영향과 관련 파일

- 기본 스킨: `assets/skins/rbms-default/lua/{theme,i18n}.lua`, `lua/select/{list,controls,main,info,radar,records}.lua`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua`, `apps/rbms-cli/scenarios/{base,select}.json`(D1·D2 가 읽는 새 id 값과 언어 불린).
- 엔진: `crates/rbms-skin/src/{model/objects,loader/*}`, `crates/rbms-render/src/skin_render/songlist*`, `apps/rbms-player/src/stage/select/{thumbs,browse_facts,scene,list,analysis}.rs`, `skin_host/select/browse.rs`.
- 외부 스킨: `rank`·`thumb` 키가 없어 영향이 없다. 선곡에서 원본 id(350~353, 92, 순위 슬롯)가 값을 내는 변화는 [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md).
- 알려진 위험(테스트 부채 3.117·3.118·3.122): 썸네일 디코드와 프레임 비용 미측정(외장 SSD 지연, 휠을 빠르게 돌릴 때 p99), 형제 탐색·램프 계수가 목록 길이만큼(O(N)) 돌아 10만 차트 규모의 평평한 목록에서 이동마다 수 ms 일 수 있음, 한 폴더에 같은 제목 여러 곡이 든 묶음 팩에서 형제 판단 오류, 선곡 Lua 프레임이 이전 0.23 ms 에서 지금 영어 0.63 ms(예산 초과 0), 랭킹 모드 오버레이가 IR RANKING 패널을 덮음, 확장 불린을 `op` 로 쓰는 객체가 선곡 상태가 비어 있는 첫 프레임에는 거짓, 한 프레임에 옵션 둘이 바뀌면 옵션 패널의 INFORMATION 이 마지막 것만(기존).
- 문서: [play-options.md](../play-options.md) 17절, [skin.md](../skin.md), [theme.md](../theme.md), [ui-design.md](../reference/ui-design.md).

## 날짜와 사용자 원문

2026-10-24.

- 사용자 요청: "5. UI/UX 를 조금 더 꾸며봐봐. 또한 곡선택부분에 보면 곡 + 왼쪽에 스코어도 들어가있는데 이것도 구현하도록하고, 기타 화면에 있는것처럼 조금 더 iidx와 비슷하게끔 구현 진행해봐 현재 r-bms 에있는 내용도 표시 가능하게하고. rival 기능은 당장은 필요없는데 해당 UI 부분에 다른 수치를 넣는다던가 하면되겠지"
- 사용자 결정: "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로"(선곡 UI 질문 2~11 의 추천안과 색 결정, RIVAL 자리는 내 기록 6줄).
