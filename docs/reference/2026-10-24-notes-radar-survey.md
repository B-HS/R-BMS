# NOTES RADAR 조사 (2026-10-24)

조사만 했다. 저장소에서 쓴 파일은 이 문서뿐이고 코드·설정·DB 는 바꾸지 않았다. cargo·bun 빌드와 테스트, `rbms-player` 실행은 하지 않았다. 측정에는 이미 만들어져 있던 `target/release/rbms-cli`(2026-10-20 빌드)의 `chart-pattern` 만 읽기 전용으로 돌렸다(출력은 세션 scratch).

외부 사실은 출처 URL 을 붙였고 확인하지 못한 것은 "확인 못 함"으로 적었다. WebFetch 는 작은 모델의 요약이라 숫자는 원문 대조가 필요하다(6절).

## 1. 요구 해석

원문(`docs/PROCESS.md:24`, 요구 4): "곡선택에서, NOTES RADAR 구현도 가능하게해줘 CHORD, NOTES, PEAK, SCRATCH, SOF-LAN 이렇게해서 6각형 표시야". 참고 화면 `images/8.png`·`images/9.png`를 직접 읽고 관찰한 것:

| 항목 | 관찰 |
| --- | --- |
| 위치 | 선곡 화면 곡 정보 영역(곡 제목 오른쪽, 캐릭터 위, MUSIC DATA 패널의 오른쪽 위). 커서가 있는 곡(난이도 한 개)의 값이다 |
| 모양 | 정육각형. 흰 외곽선과 중심을 지나는 세 축선이 있고 안쪽 고리는 없다 |
| 축 배치(위에서 시계 방향) | NOTES(위), PEAK(오른쪽 위), SCRATCH(오른쪽 아래), SOF-LAN(아래), CHARGE(왼쪽 아래), CHORD(왼쪽 위). 사용자 설명과 같다 |
| 값 | 노란 반투명 다각형. 꼭짓점 거리가 값에 비례하고 바깥 육각형이 최댓값이다. 두 장 모두 SCRATCH 가 외곽 꼭짓점에 닿는다 |
| 별표 | 최댓값 축의 꼭짓점 옆에 노란 별이 붙는다(두 장 모두 SCRATCH). 별의 정확한 규칙(동률 처리)은 공식 문서에서 확인 못 함 |
| 라벨 | 축마다 색이 다르다(NOTES 분홍, PEAK 주황, SCRATCH 빨강 계열, CHARGE 청보라, CHORD 연두). 제목 "NOTES RADAR" 는 흰색 |

해석:

1. 곡(차트) 하나의 속성인 "곡 레이더"다. 플레이어의 성장 값인 PNR(플레이어 노트레이더)은 요구가 아니다. `iidx-data-parser` 가 가져오는 것은 PNR 이다(3.4절).
2. 계산 대상은 BMS 차트다. IIDX 의 값을 그대로 가져오는 것이 아니라 BMS 노트 데이터에서 6축을 계산한다.
3. 선곡에서 커서가 곡 막대에 있을 때 그린다. 폴더·코스 막대에는 값이 없다.
4. 기본 스킨이 그려야 하고, R-BMS 확장을 아는 외부 스킨도 값을 읽을 수 있으면 좋다("구현도 가능하게").
5. IIDX 와 같은 값이 나온다고 약속할 수 없다. 6축 중 3축의 입력·단위가 공개되어 있지 않다(3.3절).

## 2. 현재 R-BMS 상태

### 2.1 관련 코드 지도

| 영역 | 위치 | 지금 하는 일 |
| --- | --- | --- |
| 차트 모델 | `crates/rbms-model/src/lib.rs:69-117` | `LnKind`(Ln, Cn, Hcn, Undefined), `NoteKind`(Normal, LongStart, LongEnd, Mine), `TimeLine`(time_us, notes 레인별, bpm, stop_us, scroll) |
| 노트 수 정의 | `crates/rbms-model/src/lib.rs:165-192` | `counts_as_playable_note`: 일반·롱노트 머리는 1, 꼬리는 CN·HCN 일 때만, 지뢰 제외. 곡 DB·판정·결과가 같은 정의를 쓴다 |
| 모드 | `crates/rbms-model/src/mode.rs:1-60` | `BEAT_7K`(key 8, scratch [7]), `BEAT_5K`([5]), `BEAT_10K`([5,11]), `BEAT_14K`([7,15]), `POPN_9K`([]), `KEYBOARD_24K`. 24K 는 BMS 에서 검출되지 않는다 |
| 기존 밀도 분석 | `crates/rbms-chart/src/lib.rs:431-516` | `note_density`: 초당 칸(7범주) → peak·avg·end. beatoraja `SongInformation` 이식이다. 초 단위 칸이라 4초 창 최댓값은 못 준다 |
| 초 표 상한 | `crates/rbms-chart/src/lib.rs:23-31` | `MAX_TABLED_SECONDS` 86,400, `tabled_second` |
| 롱노트 종류 | `crates/rbms-chart/src/lib.rs:398-430` | `contains_undefined_long_note`, `resolve_long_note_flavour`(플레이어 LN MODE 로 종류를 정함) |
| 곡 상세 | `crates/rbms-library/src/lib.rs:47-110` | `ChartDetail`(notes, long_notes, duration, bpm 범위, 밀도 4값), `compute_chart_detail` 이 파일을 읽고 `to_model` 한 뒤 `note_density` |
| 스캔 | `crates/rbms-library/src/scan.rs:323-373`, `378-423` | 차트마다 `to_model` + `note_density` 를 하고 `DetailRow` 를 함께 만들어 512개씩 upsert(`scan.rs:36`, `276-300`) |
| 곡 DB | `crates/rbms-library/src/songdb/schema.sql`(`song_detail` 표), `songdb.rs:170-197`, `songdb/rows.rs:112-119`, `142-146` | 차트별 상세 캐시(duration, 밀도 4값, 밀도 칸 blob) |
| 스키마·재스캔 | `crates/rbms-library/src/songdb/schema.rs:10`, `31`, `75` | `SCHEMA_VERSION` 1, `PARSER_VERSION` 5. 올리면 다음 스캔이 변경 없는 차트까지 전부 다시 읽는다. `songdb` 에는 `ADD COLUMN` 장치가 없다(`rbms-store` 에는 `ADDED_COLUMNS` 가 있다: `scoredb/schema.rs:61-77`) |
| 선곡 상세 로드 | `apps/rbms-player/src/stage/select/mod.rs:643-662`, `apps/rbms-player/src/lib.rs:192` | 커서가 150 ms 머물면 `chart_detail` 호출 |
| 상세 읽기·캐시 | `apps/rbms-player/src/library.rs:112-155` | 캐시 행이 있으면 DB 에서, 없으면 `compute_chart_detail` 로 계산하고 `put_detail` |
| 스킨에 내는 값 | `apps/rbms-player/src/stage/select/list.rs:570-594`, `skin_host/chart.rs:107-186`, `257-300` | `chart_under_cursor` → `ChartMeta::with_detail` → 숫자 360~365·FLOAT 360·362·367 등 밀도 값 |
| 스킨 미리보기 | `apps/rbms-player/src/stage/select/skin_preview.rs:369` | 합성 `Density` 로 미리보기 |
| 확장 id | `apps/rbms-player/src/skin_host/extension.rs:33-82` | 이벤트 21000~21099, 옵션 21100~21199. 숫자·float·이미지 공간의 확장 id 는 아직 없다 |
| 숫자·float 문 | `crates/rbms-skin/src/lua/main_state.rs:858-870`, `crates/rbms-skin/src/property/names.rs:363` | Lua `number`·`float_number` 가 `reference_implements` 가 아닌 id 에 0 을 답한다. 확장 숫자·float id 는 이 문을 고쳐야 스킨에 닿는다 |
| 참조 이미지 | `crates/rbms-render/src/skin_render/refs.rs:44-103`, `object.rs:344-371` | 음수 destination id 가 이미지가 된다(스테이지 파일·배경·배너·검정·흰색). 확장 음수 id 를 더하는 자리다 |
| 그래프 그리기 | `crates/rbms-render/src/skin_render/graphs/mod.rs:1-30`, `pixmap.rs:73-190`, `hit_error.rs:166-183` | CPU `Pixmap` 에 칠해 텍스처로 올린다. 삼각형을 줄 단위로 칠하는 선례가 있다. 렌더러에는 선 도형이 없다(`graphs/mod.rs:8`) |
| 기본 스킨 | `assets/skins/rbms-default/lua/ui.lua:219-225`, `lua/select/main.lua:1-59`, `lua/props.lua:168-179` | `Builder:reference` 가 참조 이미지를 놓는다(`s:rect` 가 흰 참조 이미지를 쓴다). 확장 옵션 id 를 `props` 가 따로 적어 둔다 |

저장소에 "radar" 가 나오는 곳은 `docs/PROCESS.md`(이번 요구)와 `docs/reference/ui-design.md:26`, `220` 뿐이다. 후자는 2026-09 문서로 "radar(펜타곤) 는 불가 → 5축 막대로 대체" 라고 적었다. 지금은 `Pixmap` 으로 다각형을 칠할 수 있으므로 이 서술은 낡았다(7절 문서 단위에서 고친다).

### 2.2 실측: 이 Mac 의 라이브러리

`/Volumes/SSD/bms`(11,336 차트)에 `target/release/rbms-cli chart-pattern ... --mode beat-7k` 를 돌렸다.

| 항목 | 값 |
| --- | --- |
| 읽기 | 11,336 차트를 약 5.0초(벽시계), CPU 17.4초(약 4코어)에 읽고 모델을 만들고 JSON 으로 썼다. 차트당 CPU 약 1.5 ms(JSON 쓰기 포함) |
| 7키 | 7,903 차트(다른 모드 3,433, 실패 0) |
| 롱노트 머리가 있는 차트 | 2,319(29.3%). 이 중 2,317 은 롱노트 종류가 문서에 없다(`has_undefined_ln`) → 종류를 플레이 때 정하므로 레이더는 종류와 무관해야 한다 |
| 스크래치 레인 롱노트 머리가 있는 차트 | 1,243(15.7%). BSS 에 해당 |
| 속도 변화 | 박자 변화가 있는 차트 1,946(24.6%), 스톱 있는 차트 309(3.9%), `#SCROLL` 변화는 0 |
| 지뢰 | 19 차트 |
| 동시 노트 묶음 크기(앞 2,000 차트) | 1개 982,383, 2개 565,952, 3개 286,202, 4개 126,687, 5개 46,327, 6개 13,499, 7개 4,534, 8개 1,434 |

이 값은 4.2절의 "이 라이브러리에 적용했을 때" 표에서 다시 쓴다. 측정 스크립트는 scratch 에 있고 저장소에 넣지 않았다(알고리즘은 4.1절 글로 재현할 수 있다).

## 3. 원본(beatoraja)과 IIDX 기준

### 3.1 beatoraja

beatoraja 소스(`/Users/hyunseokbyun/development/beatoraja/src`)에 "radar" 라는 문자열은 없다(`grep -rIli radar` 결과 0). 레이더가 있다면 재료가 되는 값만 있다.

| 값 | 위치 | 설명 |
| --- | --- | --- |
| 노트 수 4종 `n`, `ln`, `s`, `ls` | `bms/player/beatoraja/song/SongInformation.java:28-40`, `86-92` | 키 일반·키 롱·스크래치 일반·스크래치 롱 |
| 평균·최대·종반 밀도 | `SongInformation.java:150-176` | 초 단위 칸의 노트 수. peak 는 가장 많은 한 초, 롱노트 몸통도 지나가는 초마다 칸에 더한다(`:121`, `:133`) |
| 초별 분포 7범주 | `SongInformation.java:108-148`, `252-265` | 36진 문자열로 저장한다 |
| 박자 변화 `speedchange` | `SongInformation.java:179-197` | `BPM x scroll` 의 변화 지점과 스톱(속도 0) 목록 |
| 주 BPM `mainbpm` | `SongInformation.java:179-206` | BPM 별 노트 수가 가장 많은 값 |
| 레인별 노트 수 `lanenotes` | `SongInformation.java:129-138` | 레인마다 [일반, 롱, 지뢰] |
| 계산 시점 | `song/SongInformationAccessor.java:136-148` | 곡 DB 갱신(스캔) 때 `new SongInformation(model)` 로 모아 `information` 표에 저장한다. 커서 이동 때 계산하지 않는다 |
| 스킨 id | `skin/SkinProperty.java:510-515`, `598-600` | `NUMBER_DENSITY_PEAK/END/AVERAGE`(360~365), `FLOAT_CHART_*DENSITY`. 레이더 id 는 없다 |

결론: 외부 beatoraja 스킨은 레이더 값을 읽을 수단이 없다. R-BMS 가 값을 내려면 확장 id 가 필요하다(4.5절). 동시치기(CHORD)는 beatoraja 가 모으지 않는 값이다.

### 3.2 공식 정의

공식 도움말은 6항목을 이렇게만 정의한다. 계산식·0~200 범위·상한은 공식에 없다.

| 축 | 공식 문구 |
| --- | --- |
| NOTES | 楽曲の長さに対する総ノーツ数の多さ |
| PEAK | ノーツ数の瞬間密度 |
| SCRATCH | 楽曲の長さに対するスクラッチの多さ |
| CHARGE | チャージノートやバックスピンスクラッチの難しさ |
| CHORD | 同時押しの難しさ |
| SOF-LAN | BPM変化に対するノーツの難しさ |

출처: KONAMI 보도자료(2022-10-19, IIDX 30 RESIDENT) https://www.konami.com/amusement/corporate/ja/topics/20221019IIDX/ , 공식 도움말 IIDX 32 https://p.eagate.573.jp/game/2dx/32/howto/play/game_screen.html 및 IIDX 28 https://p.eagate.573.jp/game/2dx/28/howto/play/game_screen.html (둘 다 6항목을 같은 문구로 정의, 육각형·별표는 설명하지 않음). 도입 시점은 한 출처만 "IIDX27 부터"라고 적었다(https://yuisin.com/bemani/radar_iidxddr.html, 해당 글은 계산식을 주지 않는다. 다른 출처로 교차 확인 못 함).

### 3.3 커뮤니티 역산(출처별)

| 출처 | 형태 | 내용 | 신뢰도 |
| --- | --- | --- | --- |
| (A) UkiyoAmeto/iidx33-notesradar `docs/algorithm.md`(IIDX 33·34 바이너리 역공학 문서) https://github.com/UkiyoAmeto/iidx33-notesradar , 원문 https://raw.githubusercontent.com/UkiyoAmeto/iidx33-notesradar/main/docs/algorithm.md | 6개 원시 지표를 축마다 4구간 꺾은선으로 0~2.0(표시 0~200)에 옮긴다. 상한 200 은 `min` 이 아니라 마지막 구간의 끝점이다 | 구간표(아래). 게임이 런타임에 원시 지표를 계산한다는 설명 | 정규화 구조와 구간표: 높음(코드에서 읽은 값이라고 문서가 밝힘, PEAK 만 커뮤니티 측정과 교차). 입력 정의: 낮음(문서가 "커뮤니티 결론·디버그 라벨"이라고 밝힘) |
| (B) よねくん(@yonexun) 정리 https://posfie.com/@yonexun/p/f968ULu (쪽 1 NOTES·PEAK, `?page=2` SOF-LAN, `?page=3` CHARGE·CHORD) | 트윗 모음(2019, 2022-11) | NOTES 는 SP 에서 평균 밀도에 선형, PEAK 는 약 4초 창의 최대 노트 수, SOF-LAN·CHARGE·CHORD 는 가중치 가설 | 중간(피팅 결과, 일부 "미검증" 명시) |
| (C) 공식 도움말·보도자료 | 정의 | 3.2절 | 정의만 확실 |
| (D) 블로그(https://shigetsu-mizyu.blogspot.com/2024/02/iidx.html), NotesRadarBot 시트 | 곡별 값 | 곡별 값은 게임에 표시되지 않아 한 곡씩 역산해 시트로 모음. 본문 표는 이미지·링크라 읽지 못함 | 값 자체는 쓸 수 있으나 차트 데이터가 없어 검증 불가 |

(A)의 구간표(SP / DP, 원시 지표 x 가 p0 이하면 `r = x / p0`, p0~p1 은 `1.0 + 0.5 (x - p0) / (p1 - p0)`, p1~p2 는 `1.5 + 0.25 (x - p1) / (p2 - p1)`, p2~p3 는 `1.75 + 0.25 (x - p2) / (p3 - p2)`, 표시값 = r x 100, p3 이상은 200):

| 축 | SP p0 / p1 / p2 / p3 | DP p0 / p1 / p2 / p3 | (A)의 원시 지표 설명 |
| --- | --- | --- | --- |
| NOTES | 10.2 / 15.3 / 17.7 / 20.5 | 10.4 / 15.4 / 17.9 / 20.0 | 총 노트 수 / 연주 시간(초당 노트). 연주 시간의 정의는 명시 없음 |
| PEAK | 70 / 108 / 125 / 136 | 70 / 113 / 122 / 170 | 약 4초 창 안의 최대 노트 수. 창 이동 방식·세는 대상은 명시 없음 |
| SCRATCH | 1.4 / 2.6 / 3.9 / 6.2 | 1.4 / 2.6 / 3.9 / 7.0 | 초당 스크래치. BSS 포함 여부는 명시 없음 |
| SOF-LAN | 440 / 1000 / 2000 / 3800 | 440 / 1000 / 2000 / 4000 | "BPM 변화 관련 양". 단위·정의 없음 |
| CHARGE | 160 / 310 / 440 / 900 | 145 / 280 / 400 / 750 | CN/초라고만 적음. 가중·단위 설명 없음 |
| CHORD | 500 / 1010 / 1200 / 1450 | 500 / 1000 / 1200 / 1500 | 초당 동시 누름이라고만 적음. 동시 누름의 정의 없음 |

교차 확인: PEAK 의 SP 값은 (B)의 별개 출처와 일치한다. (B) 2022-11-15 の よねくん: PEAK100 = 70개, PEAK150 = 70+38 = 108, PEAK200 = 108+28 = 136, DP 는 70·113·(175 에서 125)·170(200 은 "?" 표시). theta 의 관찰도 같은 지점(100, 150, DP 만 175)에서 함수가 바뀐다고 한다. NOTES·SCRATCH·SOF-LAN·CHARGE·CHORD 의 구간표는 (A) 한 출처뿐이다. (A) §2.4 의 NOTES 역산은 자기 sqlite 데이터로 3개 행을 맞춘 것이라 오차 지표가 없다.

(B)의 가설(IIDX 30, 모두 본인이 "일부 미검증"이라 밝힘):

| 축 | 가설 | 비고 |
| --- | --- | --- |
| NOTES | 평균 밀도(총 노트 ÷ 시간), SP 에서 0~200 선형. 시간 정의가 풀리지 않아 흩어짐 | 쪽 1, 2022-11-18 |
| PEAK | 약 4초 창의 최대 노트 수. 같은 BPM 한 마디 차트 둘이 15 차이가 나서 창 경계는 모름(theta) | 쪽 1, 2022-11-15 |
| SOF-LAN | BPM 변화 지점마다 (주변 노트 수 x BPM 변화량)을 누적. 변화 전 노트는 안 센다. 변화 후 2초까지를 시간에 따라 선형으로 줄어드는 가중으로 센다. 다음 변화가 오면 앞 창은 거기서 끝난다(이중 계산 없음). 변화율 = abs(BPM 차) ÷ 작은 쪽 BPM 이고 보정은 로그에 거의 선형, 상한 1.26·하한 0.01 로 "어느 정도" 맞음. SP 에서 100 근처에서 함수가 갈라지는 듯함 | 쪽 2, 2022-11-22~26. 9곡이 거의 일치(약 0.1% 오차) |
| CHARGE | 점수 방식. CN 한 개 = 3, CN 시작과 동시에 있는 다른 노트 하나마다 +3, CN 도중(끝과 겹침 포함)에 지나가는 다른 노트 하나마다 +4. 같은 시작의 CN n 개는 `3n²`(4개·5개로 확인). CN 길이·개수보다 주변 노트 수가 중요하다. LEGACY 를 켜면 CHARGE 는 0 점이다 | 쪽 3, 2022-11-18. 끝과 겹침은 "검증 못 함". BSS 는 논의 없음 |
| CHORD | 동시 누름 n개(스크래치 포함)의 가중치가 `X_n = X_(n-1) + 0.05 n + 0.1`, 즉 n=2..8 에서 0.2, 0.45, 0.75, 1.10, 1.50, 1.95, 2.45 (`(n-1)(n+6)/40`). 텍스타지 데이터는 노트의 미세한 어긋남을 반영하지 않아 오차가 남는다 | 쪽 3, 2022-10-26(YAA)·2022-11-17~18. 미검증 |
| 보조 | 플레이어 레이더는 곡 값 x 스코어 레이트(EX ÷ (노트 수 x 2))로 오른다. AUTO SCRATCH 는 SCRATCH 0, LEGACY 는 CHARGE 0(IIDX 27~29 시절) | https://posfie.com/@yonexun/p/wFcCKL7 (2019). R-BMS 요구와는 무관(PNR) |

(A)와 (B)가 어긋나는 곳: (A)는 CHORD·CHARGE 를 "초당 개수"로 요약하고 (B)는 가중 합이다. 같은 단위로 맞출 수 있는지 확인 못 함. 이 라이브러리에서 해 본 결과는 4.2절.

### 3.4 `iidx-data-parser` 판단

`/Users/hyunseokbyun/development/iidx-data-parser`(읽기만, 수정 없음).

| 확인한 것 | 결과 |
| --- | --- |
| `README.md` | e-amusement 의 DJ DATA 에서 플레이어의 NOTES RADAR 6값(`status.html`, `music/notesradar.html`)을 모은다 |
| `docs/DATA_SCHEMA.md:25-35`, `106-110` | `notesRadar.values`(NOTES·CHORD·PEAK·CHARGE·SCRATCH·SOF-LAN, 플레이어 합계 하나). 차트 목록의 항목(`charts[]`)에는 레이더 값이 없고 곡명·난이도·램프·EX 만 있다 |
| `docs/PARSING-NOTES.md:27-31`, `70` | `json/notesradar.html` 응답 스키마를 확정하지 못해 DOM 만 읽는다 |
| `tests/fixtures/status-logged-in.html:61-75` | 합성 값(128.45, 131.22, ...) |
| `src/` | 차트 노트 데이터를 다루는 코드 없음 |

판단: 차트별 레이더 값도 차트 노트 데이터도 없다. 이 저장소로는 계산식 검증이 불가능하다. 쓸 수 있는 것은 6축 이름·e-amusement 표기 순서(NOTES, CHORD, PEAK, CHARGE, SCRATCH, SOF-LAN, `src/shared/domain.ts:6`)뿐이다. 화면의 시계 방향 순서(NOTES, PEAK, SCRATCH, SOF-LAN, CHARGE, CHORD)와 다르므로 id 순서는 화면 순서로 정한다(4.5절).

### 3.5 검증에 쓸 수 있는 자료(모두 확인 못 함 또는 접근 못 함)

- NotesRadarBot(@NotesRadarBot)의 곡별 값 시트: 검색 결과에서만 확인(https://x.com/notesradarbot , https://iidx.org/external_resources). 시트 본문은 열지 못했다.
- `Glebsin/iidx2bms`·`krrcream/iidx2bms`(IIDX 33 차트를 BMS 로 바꾸는 도구)는 검색 결과의 제목만 보았다. 사용자가 이런 BMS 를 갖고 있다면 (곡별 값 시트) x (변환 BMS) 로 NOTES·PEAK·SCRATCH 의 검증 세트를 만들 수 있다. 사용자 라이브러리에 있는지는 확인 못 함.
- 기타: https://hiroblo-net.com/2023/01/05/bmskill1/ 은 DNS 실패로 읽지 못했다. SOUND VOLTEX 의 Effect Radar 문서(https://voltexes.com/effect-radar/)는 계산식이 없다.

## 4. 설계안

### 4.1 BMS 대응(축별 추천)

공통 정의:

- 누름 이벤트 = `counts_as_playable_note` 를 만족하는 노트(일반, 롱노트 머리, 선언된 CN·HCN 의 꼬리). 롱노트 종류가 `Undefined` 이면 머리만 센다. 레이더는 종류와 무관해야 하므로 CHARGE 도 머리 기준으로 센다.
- 연주 시간 `D` = 첫 누름 이벤트와 마지막 누름 이벤트 사이(초). 하한 1초(`RADAR_MIN_SPAN_US`)로 0 나눗셈과 장난 차트를 막는다. 이 라이브러리 7,903 차트에서 "마지막 노트 시각"과 "첫~마지막"의 NOTES 중앙값은 14.41 대 14.66 으로 2% 차이라 어느 쪽이든 영향이 작다(측정).
- 지뢰는 세지 않는다(IIDX 에 없음, `counts_as_playable_note` 와 같다).

| 축 | 원시 지표(추천) | BMS 에서 맞닥뜨리는 것 | 상태 |
| --- | --- | --- | --- |
| NOTES | 누름 이벤트 수 / D | 5키·7키·DP 모두 같은 식. DP 는 DP 구간표 | 단위 확실(초당 노트) |
| PEAK | 각 누름 이벤트 시각 t 에서 `[t, t + 4초)` 의 누름 이벤트 수의 최댓값(두 포인터 O(n)) | 초 칸(`note_density`)으로는 4초 창을 못 만든다. 롱노트 몸통은 세지 않는다(beatoraja 의 peak 는 몸통을 센다. 다르다) | 창 길이는 (A)·(B) 일치. 이동 단위는 확인 못 함 |
| SCRATCH | 스크래치 레인(`Mode.scratch`)의 누름 이벤트 수 / D. BSS(스크래치 레인 롱노트)도 머리 하나로 센다 | 5키 스크래치 [5], 7키 [7], 10키 [5,11], 14키 [7,15]. 9키(POPN)는 스크래치가 없다 | BSS 포함은 확인 못 함 |
| CHARGE | (B)의 점수 방식: 롱노트 하나마다 `3 + 3 x (같은 시각에 시작하는 다른 노트 수) + 4 x (롱노트가 쥐어진 동안 지나가는 다른 레인 노트 수)`의 합 / D. 스크래치 레인 롱노트도 같은 식(BSS) | LN·CN·HCN 구분 없이 머리 기준. HCN 의 추가 난도는 반영하지 않는다(근거 없음) | 가설. 스케일 불명(6절) |
| CHORD | 같은 `time_us` 의 누름 이벤트가 n(>=2)개인 행마다 `(n-1)(n+6)/40` 을 더한 합 / D. 스크래치 포함, DP 는 양손 합산 | 7키 + 스크래치 전체 동시 8개가 최대 | 가설. 스케일 불명 |
| SOF-LAN | 속도 `S(t) = bpm x scroll`(스톱 중 0, beatoraja `speedchange` 와 같은 정의)이 바뀌는 지점마다 `변화 후 2초 안의 노트들의 선형 감쇠 가중 합 x clamp(abs(dS) / min(S전, S후), 0.01, 1.26)` 을 더한다. 다음 변화에서 창을 닫는다. 변화 전 노트는 세지 않는다 | `TimeLine.bpm`, `stop_us`, `scroll`(`rbms-model/src/lib.rs:94-114`)에서 얻는다. 스톱은 S 가 0 이 되는 변화라 상한 1.26 이 걸린다(IIDX 가 스톱을 같게 다루는지는 확인 못 함). 이 라이브러리에서 변화가 있는 7키 차트는 약 26% | 가설. 단위·스케일 불명 |

모드별:

| 모드 | 처리 |
| --- | --- |
| `BEAT_7K`, `BEAT_5K` | SP 구간표 |
| `BEAT_14K`, `BEAT_10K` | DP 구간표(양쪽 레인 모두 센다) |
| `POPN_9K`, `KEYBOARD_24K` | 레이더 없음(값 없음, 스킨은 그리지 않음). IIDX 레이더가 정의된 모드가 아니다 |

### 4.2 정규화

- 원시 지표를 DB 에 저장하고 읽을 때 표시값(0~200)으로 바꾼다. 구간표를 고쳐도 DB 를 다시 만들 필요가 없다.
- 구간표는 (A)의 표를 그대로 상수로 둔다(새 의존성·매직넘버 없음, 이름 있는 상수 배열과 `///` 영어 문서). CHARGE·CHORD·SOF-LAN 은 가설 정의의 스케일을 맞추는 이름 있는 상수 하나씩(`RADAR_CHARGE_SCALE` 등)을 둔다.
- 표시값은 200 으로 자른다(IIDX 와 같다). 자르지 않은 원시 값은 DB 에 남는다.
- 별표 축은 스킨(Lua)이 여섯 float 를 비교해 정한다. 엔진에 별표 로직을 넣지 않는다.

이 라이브러리(7,903개 7키 차트)에 적용했을 때(측정, 헤더 `#PLAYLEVEL` 의 `12` 인 1,665개는 BMS 난이도 체계가 섞여 있어 IIDX 12 와 같지 않다):

| 지표(정의) | 전체 | `#PLAYLEVEL` 12 |
| --- | --- | --- |
| NOTES 표시값(첫~마지막 시간) | 중앙 143.7, 200 도달 19.5% | p10 112.8, 중앙 153.4, p90 200 |
| PEAK 표시값 | 200 도달 22.4% | p10 115.8, 중앙 146.1, p90 200 |
| SCRATCH 표시값 | p90 96.8, 200 도달 0.9% | p10 10.9, 중앙 38.6, p90 100.0 |
| CHORD, 같은 시각 2개 이상인 행 수 x 100 / D | 중앙 81.8, 200 도달 0.1% | p10 56.8, 중앙 90.0, p90 114.6 |
| CHORD, 가중 `(n-1)(n+6)/40` 합 x 1000 / D | 중앙 197.1(대부분 200 에 붙음) | 중앙 200 |
| CHARGE, 롱노트 머리가 있는 2,319개의 `점수/D` | p50 2.6, p90 105.3, p98 288.4 (단위 없는 원점수) | 측정 안 함 |
| SOF-LAN, 변화가 있는 2,038개의 원시 합 | p50 11.3, p90 60.6, p98 668.6 (단위 없음) | 측정 안 함 |

분모 주의(측정 때 쓴 것): NOTES·CHORD 가중 합·CHARGE 점수는 "첫 누름~마지막 누름" 시간, SCRATCH 와 행 수 방식 CHORD 는 "0초~마지막 노트" 시간이다. SOF-LAN 은 시간으로 나누지 않은 합이고 PEAK 는 분모가 없다. 두 시간 정의의 차이는 NOTES 중앙값에서 2% 였다.

읽는 법: NOTES·PEAK·SCRATCH 는 공개 구간표를 그대로 써도 값이 IIDX 처럼 중간 영역에 몰려 쓸 만해 보인다(레벨 12 의 NOTES·PEAK 중앙값 약 150). CHORD 는 가중치를 구간표 단위(x1000)로 맞추면 레벨 12 의 절반이 200 에 붙어 스케일이 약 3배 틀리고, 단순 행 수 x 100 은 중간 영역에 들어온다. 즉 CHORD·CHARGE·SOF-LAN 의 스케일은 눈으로 맞출 수 없고 IIDX 값과의 대응을 확인할 자료가 없다. 이 표는 "공개 구간이 BMS 에서 의미 있는 범위를 낸다"는 정황이지 검증이 아니다.

### 4.3 계산·저장 위치

| 안 | 비용 | 장점 | 단점 |
| --- | --- | --- | --- |
| A. 스캔 때 계산해 DB 에 저장(새 차트·바뀐 차트) + 저장된 행이 없는 차트는 커서가 머물 때 한 번 계산해 저장 (추천) | 스캔에서 이미 `to_model` 을 하므로 추가 비용은 노트·타임라인 한 번 훑기(마이크로초 단위, 추정). 차트당 파일 읽기+파싱은 측정상 CPU 약 1.5 ms | 커서 이동은 SQLite 한 줄 읽기. 외장 드라이브가 빠져도 값이 보인다. `PARSER_VERSION` 을 올리지 않아 전체 재스캔이 없다. 폴더·정렬용 값으로도 쓸 수 있다 | 새 표 하나와 읽기·쓰기 코드가 늘고, 값이 없는 행을 처리하는 경로가 하나 더 생긴다 |
| B. `PARSER_VERSION` 6 으로 올려 전체 재스캔 | 11,336 차트 기준 읽기·모델 구축은 측정상 약 5초(벽시계)이고 md5·sha256·밀도·SQLite 쓰기가 더해진다(합계는 측정 안 함). 더 큰 라이브러리는 비례 | 한 번의 스캔으로 모든 행이 채워진다. 코드 경로가 단순 | 사용자가 첫 실행 때 전체 재스캔을 겪는다. 레이더가 곡 값이 아닌 구조 변화였다면 정당하지만 지금은 저장된 값이 바뀌지 않는다 |
| C. 커서 이동 때만 계산(DB 없음) | 150 ms 머문 뒤 파일 읽기+파싱+계산. 차트당 수 ms | 구현 가장 단순 | 파일이 안 읽히는 곡(드라이브 분리)은 값이 없다. 같은 곡을 계속 다시 파싱한다. 폴더·정렬에 못 쓴다 |

추천 A 의 상세:

- 새 표 `song_radar`(`path` 기본키, `song(path)` 참조 `ON DELETE CASCADE`, `version`, 모드에 레이더가 없음을 뜻하는 `applies`, 원시 6열). `CREATE TABLE IF NOT EXISTS` 로 추가한다. `SCHEMA_VERSION` 1 을 그대로 두면 추가 표만 늘어 이전 빌드도 같은 DB 를 연다(`ADD COLUMN` 장치가 `songdb` 에 없어 `song_detail` 에 열을 더하는 길은 피한다).
- `RADAR_VERSION` 상수를 따로 둔다. 정의가 바뀌면 `version` 이 다른 행을 커서가 머물 때 다시 계산한다(전체 재스캔 없이).
- 스캔(`scan.rs:323-423`)은 `DetailRow` 를 만들 때 레이더도 계산해 같은 배치로 쓴다. 레이더가 없는 모드(9키·24키)는 `applies = 0` 행을 남겨 커서가 머물 때마다 다시 계산하지 않는다.
- `chart_detail`(`library.rs:112-128`)은 상세 행이 있어도 레이더 행이 없거나 버전이 다르면 `compute_chart_detail` 로 돌아가 상세와 레이더를 함께 저장한다.

### 4.4 그리기

기본 스킨(Lua)은 선곡 전체를 그린다. 다각형 채우기는 기존 스킨 객체(이미지·숫자·그래프)로 만들 수 없다(그래프는 4방향 막대이고, 사각형을 회전·확대하는 것만으로는 꼭짓점이 값에 따라 움직이는 삼각형을 만들 수 없다). 두 길이 있다.

| 안 | 내용 | 평가 |
| --- | --- | --- |
| A. 엔진이 칠한 참조 이미지(추천) | 음수 destination id 로 놓는 새 확장 참조 이미지(`ReferenceImage::NotesRadar`). 렌더러가 프레임이 가져온 6개 표시값으로 256x256 `Pixmap` 에 육각형 다각형(흰색 불투명)을 칠해 텍스처로 올리고 destination 의 색·알파가 노란 반투명으로 입힌다. 격자(외곽선·세 축선)·라벨·별표는 스킨이 그린다 | `graphs` 의 `Pixmap`·`Layer` 와 `fill_triangle` 선례가 있다. 값이 바뀔 때만(커서가 다른 차트로 갈 때) 다시 칠하므로 프레임 비용이 없다. 스킨이 색과 위치를 정한다. beatoraja 는 모르는 음수 id 를 "그리지 않는 이미지"로 처리하므로(`refs.rs:3-8`) 겸용 스킨이 안전하다 |
| B. 스킨이 막대 6개로 흉내 | 회전한 `graph` 막대 6개(스포크형 레이더) | IIDX 모양이 아니다(채워진 다각형이 아님). 요구 모양 불일치 |
| C. 내장(비스킨) 선곡 화면에 `fill_rect` 줄 채우기 | `rbms-render/src/select.rs` 의 DENSITY 패널 옆 | 스킨이 못 읽힐 때만 쓰이는 화면이다. 선택 사항 |

A 의 칠하기: 중심 (128,128), 꼭짓점 k 는 위에서 시계 방향 60도씩, 반지름 = 표시값/200 x R(R = 124). 볼록 육각형이라 줄 단위 가장자리 교차로 채우고 가장자리는 세로 4배 샘플로 앤티앨리어싱한다. 새 의존성 없음.

### 4.5 외부 스킨용 값 노출

beatoraja 에 해당 값이 없으므로 확장 id 를 쓴다.

기존 사용 번호(`apps/rbms-player/src/skin_host/extension.rs:33-82`):

| 종류 | 사용 중 | 남은 번호 |
| --- | --- | --- |
| 이벤트(21000~21099) | 21000, 21001 | 21002~21099(98개) |
| 옵션(21100~21199) | 21100~21104, 21110~21114, 21120~21124, 21130~21134, 21140~21148(29개) | 21105~21109, 21115~21119, 21125~21129, 21135~21139, 21149~21199(71개) |
| 숫자·float·이미지·문자열 | 없음. 번호 공간이 서로 별개이므로(`rbms-skin/src/property/host.rs:111-112` 설명) 옵션 번호와 겹치지 않는다 | 해당 공간은 전부 비어 있다 |

추천 배정:

| 번호 | 공간 | 뜻 |
| --- | --- | --- |
| 21150~21155 | float | 표시값 0~200: 21150 NOTES, 21151 PEAK, 21152 SCRATCH, 21153 SOF-LAN, 21154 CHARGE, 21155 CHORD(위에서 시계 방향 = 화면 순서). 값이 없으면 `FLOAT_ABSENT` |
| 21150~21155 | 숫자 | 같은 값을 정수(버림)로. 값이 없으면 `INTEGER_ABSENT` |
| 21160 | 참조 이미지(destination id `-21160`) | 육각형 다각형 |

필요한 작은 코어 변경: Lua `main_state.number`·`float_number` 가 `reference_implements`(`names.rs:363`, `main_state.rs:858-870`)에 걸려 확장 id 에 0 을 답한다. 확장 숫자·float 대역(21150~21199)을 "구현됨"으로 인정하는 상수와 검사를 `rbms-skin` 에 둔다. 상수는 렌더러와 앱이 함께 보므로 앱 크레이트의 `extension.rs` 가 아니라 `rbms-skin` 에 두고 `extension.rs` 가 가리킨다. `rbms_extensions` 헤더 선언과는 무관하게 모든 스킨에 답한다(기존 확장 id 와 같은 규칙, `docs/skin.md` 1.4절).

### 4.6 바뀌는 파일 목록(예상)

| 구분 | 파일 | 변경 |
| --- | --- | --- |
| 새 파일 | `crates/rbms-chart/src/radar.rs` (+ `radar/tests.rs`) | 원시 지표·구간표·정규화 |
| 새 파일 | `crates/rbms-render/src/skin_render/graphs/radar.rs` | 육각형 칠하기 |
| 새 파일 | `assets/skins/rbms-default/lua/select/radar.lua` | 격자·라벨·값·별표 배치 |
| 수정 | `crates/rbms-chart/src/lib.rs` | 모듈·재수출 |
| 수정 | `crates/rbms-library/src/lib.rs`, `scan.rs`, `songdb.rs`, `songdb/rows.rs`, `songdb/schema.sql` | `ChartDetail`·`DetailRow` 에 레이더, 새 표, 스캔에서 계산 |
| 수정 | `apps/rbms-player/src/library.rs` | 읽기·없으면 계산 |
| 수정 | `apps/rbms-player/src/skin_host/chart.rs`, `extension.rs`, `stage/select/skin_preview.rs`, `stage/select/list.rs`(필요 시) | `ChartMeta` 에 레이더, float·숫자 읽기, 미리보기 합성값 |
| 수정 | `crates/rbms-skin/src/property/names.rs`(+ 새 상수 위치), `lua/main_state.rs` | 확장 숫자·float 대역 인정, 참조 이미지 상수 |
| 수정 | `crates/rbms-render/src/skin_render/refs.rs`, `object.rs`, `frame.rs`(프레임 시리즈), `graphs/mod.rs` | 참조 이미지 종류 추가, 프레임이 6값을 나름 |
| 수정(선택) | `crates/rbms-render/src/select.rs` | 내장(비스킨) 선곡 화면 육각형(5절 질문 6) |
| 수정 | `assets/skins/rbms-default/lua/props.lua`, `lua/select/main.lua`, `lua/theme.lua`, `tools/generate-images.py`(+ 생성되는 `image/select.png`) | id 미러, 한 줄 호출, 색, 격자 스프라이트 |
| 수정(선택) | `apps/rbms-cli/src/main.rs` 주변 | 차트 요약에 레이더 출력, 분포 점검용 |
| 문서 | `docs/skin.md` 1.4절, `docs/architecture.md`, `docs/crates.md`, `docs/reference/ui-design.md:26`·`220`, ADR, QA 점검표, `docs/PROCESS.md` | 7절 문서 단위 |

### 4.7 새 id·설정·DB·API 변경 여부

| 항목 | 변경 |
| --- | --- |
| 새 스킨 id | 있음. float 21150~21155, 숫자 21150~21155, 참조 이미지 21160(4.5절). 이벤트·옵션 id 는 쓰지 않는다. 램프 id 0~10·와이어 램프 이름 변경 없음 |
| 설정 새 필드 | 없음(`SETTING_COUNT` 97 그대로, 설정 스키마 6 그대로). 레이더를 켜고 끄는 것은 스킨이 정한다 |
| 클라이언트 DB | `songdb` 에 새 표 `song_radar` 하나(`SCHEMA_VERSION` 1 유지, `PARSER_VERSION` 5 유지) |
| 점수 DB·웹 DB·IR API | 없음. 서버·웹·`contract/client-requests` 변경 없음. 점수·판정 의미 변경 없음 |
| 사용자 확인 필요한 변경 | 없음(판정·점수 의미를 바꾸지 않는다) |

### 4.8 테스트 방법

| 대상 | 방법 |
| --- | --- |
| 원시 지표 | `rbms_parser::parse` + `to_model` 로 만든 합성 차트(`crates/rbms-chart/src/tests.rs` 방식)로 손으로 센 기대값: 1초 간격 16분음표 한 줄의 NOTES·PEAK, 동시 2·3·8개 행의 CHORD 가중, 롱노트 하나(+ 동시 노트, 몸통 안 노트)의 CHARGE 점수, BPM 변화 한 번(+ 스톱·`#SCROLL`)의 SOF-LAN, 스크래치 롱노트(BSS) |
| 경계 | 롱노트 `Undefined` 와 `Cn`·`Hcn` 이 같은 값, 노트 0·1개·시간 0, 9키·24키 `None`, DP 합산, 장난 차트(분당 수만 BPM)가 200 으로 자르고 NaN·무한대가 없음 |
| 정규화 | 구간 끝점(p0, p1, p2, p3)이 정확히 100·150·175·200, 단조 증가, 0 이하 0, p3 초과 200, SP·DP 표가 다름 |
| DB | 왕복, 모드 없음 행, `version` 불일치 시 재계산, 곡 삭제 시 `ON DELETE CASCADE`, 상세 행은 있고 레이더 행이 없는 옛 DB 에서 커서가 머물 때 채워짐 |
| 스킨 호스트 | `ChartMeta` 의 float·숫자 읽기, 값 없음 센티널, `main_state.float_number(21150)` 이 0 이 아닌 값을 냄(코어 문 수리 검증), 화면에 차트가 없을 때 `FLOAT_ABSENT` |
| 렌더 | CPU 캔버스 골든(`rbms-render/src/golden.rs`)으로 값 [200,0,0,0,0,0] 과 [200]x6 의 몇 픽셀(안·밖·꼭짓점), 값이 같으면 다시 칠하지 않음 |
| 기본 스킨 | `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 선곡 캡처 시험(`HANDOFF.md` 7절의 `RBMS_SKIN_CAPTURE_DIR` 시험)으로 육각형을 눈으로 확인 |
| 실데이터 | `rbms-cli <chart>` 요약의 레이더 출력으로 알려진 곡 몇 개의 값이 직관과 맞는지, `chart-pattern` 덤프로 라이브러리 분포(4.2절) 재측정 |
| IIDX 대조(선택) | 3.5절의 시트와 변환 BMS 가 있을 때 NOTES·PEAK·SCRATCH 를 대조 |

### 4.9 대안과 기각 이유

- 원본 IIDX 값을 외부에서 가져온다: 곡별 값이 게임에 표시되지 않고 BMS 곡에는 대응 값이 없다.
- 구간표를 이 라이브러리 분포로 새로 만든다: IIDX 와 닮은 모양이라는 목적에서 멀어지고 사용자마다 다르다. 3축(CHARGE·CHORD·SOF-LAN)은 부득이 임시 스케일이 필요해 5절 질문 1 로 둔다.
- 레이더를 서버가 계산한다: 클라이언트가 이미 차트를 읽고 곡 DB 에 저장하는 구조이고 웹에는 차트 파일이 없다.

## 5. 사용자 결정이 필요한 질문

"전부 추천대로" 한마디로 답할 수 있게 적었다. 6개다.

1. CHARGE·CHORD·SOF-LAN 의 정의와 스케일: **A안(추천: 공개된 커뮤니티 가중치 가설로 계산하고 IIDX 값과 다를 수 있음을 ADR·문서에 적는다. 단위가 불명이라 임시 스케일을 이름 있는 상수로 두고 실제 화면을 보고 조정)** / B안(단순 개수: 초당 롱노트 머리 수·초당 동시 누름 행 수 + 공개 구간표 그대로) / C안(이 3축은 일단 0으로 두고 NOTES·PEAK·SCRATCH 3축만 의미 있게 표시). 추천 이유: A 가 IIDX 의 상대 순위에 가장 가깝고 B 는 스케일 정황은 좋지만 정의가 단순하다. 어느 안이든 3축은 IIDX 와 같은 값이 아니다.
2. 계산·저장 시점: **A안(추천: 스캔 때 계산해 새 표에 저장 + 값이 없는 행은 커서가 머물 때 채움, 전체 재스캔 없음)** / B안(`PARSER_VERSION` 6 으로 전체 재스캔) / C안(커서 이동 때만 계산).
3. 모드 범위: **A안(추천: 5키·7키는 SP 구간, 10키·14키는 DP 구간, 9키·24키는 레이더 없음)** / B안(9키도 SP 구간으로 계산, 스크래치 축은 0).
4. 200 초과 처리: **A안(추천: 표시값을 200 으로 자르고 원시 값은 DB 에 둠, IIDX 와 같다)** / B안(숫자는 200 초과를 보이고 다각형만 자름. 이 경우 스킨 숫자 id 가 200 을 넘을 수 있다).
5. 외부 스킨 노출: **A안(추천: float·숫자 21150~21155 와 참조 이미지 21160. `rbms-skin` 의 확장 숫자·float 대역 인정을 함께 고침. 기본 스킨도 같은 문을 지나므로 어느 안이든 이 수리가 필요하다)** / B안(문자열 id 로 값을 내보내고 스킨이 숫자로 되돌림. `main_state.text` 는 이 문에 걸리지 않지만 우회라 비추) / C안(옵션 id 로 구간 단계만 노출, 비추).
6. 내장(비스킨) 선곡 화면 육각형: **A안(추천: 이번엔 안 한다, 스킨이 못 읽힐 때의 보조 화면이고 요구는 곡선택 스킨)** / B안(`fill_rect` 줄 채우기로 함께 구현).

위치·크기·색(곡 정보 영역 어디에 놓을지)은 요구 5(선곡 UI, N7)와 한 번에 정한다. 이 문서의 질문이 아니다.

## 6. 위험과 확인 못 한 것

위험:

1. 구간표 한 출처: NOTES·SCRATCH·SOF-LAN·CHARGE·CHORD 의 구간표는 IIDX 33·34 바이너리 역공학 문서 1건뿐이다(그 저장소는 상한을 푸는 모드이고 이 조사는 수학적 구조만 읽었다). PEAK 의 SP 값만 별개 커뮤니티 측정(70·108·136)과 일치한다. 문서 스스로 입력 정의는 커뮤니티 결론·디버그 라벨이라고 밝힌다. 이 구간표 숫자는 WebFetch 요약(작은 모델)으로 읽었으므로 구현 전에 원문(`docs/algorithm.md` 2.1절)과 한 번 대조해야 한다.
2. 입력 정의 불명: NOTES 의 연주 시간, PEAK 창 이동 방식, SCRATCH 의 BSS 포함, SOF-LAN 단위, CHARGE·CHORD 의 스케일. 3.3절 (A)와 (B)가 서로 다른 요약을 준다.
3. 스케일 오차: CHORD 가중 합을 구간표 단위(x1000)로 두면 이 라이브러리에서 레벨 12 의 절반이 200 에 붙는다(4.2절). 가설 3축의 값은 IIDX 와 수십 % 달라도 이상하지 않다.
4. 검증 자료 없음: `iidx-data-parser` 는 플레이어 레이더 6값만 갖고 차트별 값·노트 데이터가 없다(3.4절). 차트별 값 시트와 변환 BMS 쌍이 없어 정확도 검증을 하지 못했다.
5. 난이도 체계: 4.2절 표의 "레벨 12"는 BMS 헤더 `#PLAYLEVEL` 의 12 로 IIDX 레벨 12 와 같지 않다. "IIDX 처럼 중간에 몰린다"는 관찰은 정황이다.
6. 스톱·`#SCROLL`: IIDX 의 SOF-LAN 이 스톱·스크롤을 어떻게 다루는지 확인 못 함. 이 라이브러리의 7키 차트에는 `#SCROLL` 변화가 0 이라 `scroll` 항의 실데이터 점검도 못 했다.
7. 곡 DB 낡은 행: 새 표가 없는 옛 DB 의 모든 차트는 처음 커서가 머물 때 한 번씩 파일을 읽는다. 외장 드라이브가 빠지면 값이 비어 보인다(스킨은 센티널로 그리지 않음). 알려진 한계로 문서화한다.
8. 같은 레이어 충돌: 선곡 UI(N7), 랭킹 보기(N5), 다국어(N3)와 `lua/select/*`, `skin_host/extension.rs`, `skin_preview.rs` 를 함께 만진다. 7절의 소유 규칙으로 막는다.
9. `#RANDOM` 분기가 있는 차트는 파서의 결정적 시드로 읽은 한 갈래의 값이다. 기존 밀도와 같은 한계다(`docs/crates.md` rbms-parser).

확인 못 한 것:

- IIDX 의 별표(최대 축) 규칙, 동률 처리.
- IIDX 의 연주 시간·PEAK 창 이동·스톱 처리·BSS·HCN 가중.
- 곡별 레이더 값 시트(NotesRadarBot, 블로그 표 본문)와 `bm2dx.com/IIDX/notes_radar/` 표, `togetter` 정리의 `ryomitaka` 글. 접근하지 못했거나 본문이 이미지·링크였다.
- https://hiroblo-net.com/2023/01/05/bmskill1/ (DNS 실패).
- 도입 시점 IIDX27 은 한 출처.
- 이 라이브러리 지표는 Python 근사(작은 스크립트)로 `chart-pattern` 덤프를 읽은 값이라 Rust 구현과 같은 숫자가 나온다는 보장은 없다. cargo 빌드·테스트와 실제 창 확인은 하지 않았다.
- 계산 시간의 레이더 추가분은 측정하지 않았다(차트 읽기·모델 구축 약 1.5 ms/차트 CPU 가 측정값이고 노트 훑기는 그보다 훨씬 작다는 추정).

## 7. 구현 단위 분해

크레이트 경계로 나눈 worktree 병렬 단위다. 의존: R0 → (R1, R4 병렬) → R2 → R3 → R5, R6·R7 은 R1 뒤. 단위마다 새 worktree 에서 시작하고 메인이 병합·게이트·커밋한다(`HANDOFF.md` 5절).

| 단위 | 내용 | 파일 소유권(수정 가능) | 의존 | 예상 변경 |
| --- | --- | --- | --- | --- |
| R0 | 확장 숫자·float 대역과 참조 이미지 id 상수, `reference_implements` 문 수리(Lua `number`·`float_number`) | `crates/rbms-skin/src/property/{names,mod}.rs`, `crates/rbms-skin/src/lua/main_state.rs`(+ 그 시험) | 없음 | 약 80줄 + 시험 80줄 |
| R1 | 원시 지표·구간표·정규화·모드 처리 | `crates/rbms-chart/src/{radar.rs,radar/tests.rs,lib.rs}` | 없음(공개 API 모양은 메인이 먼저 고정) | 약 350줄 + 시험 350줄 |
| R4 | 육각형 칠하기와 참조 이미지·프레임 시리즈 | `crates/rbms-render/src/skin_render/{refs.rs,object.rs,frame.rs,graphs/mod.rs,graphs/radar.rs}`(+ 시험, 선택 시 `select.rs`) | R0(상수) | 약 330줄 + 시험 200줄(선택 +120줄) |
| R2 | `song_radar` 표, 스캔에서 계산, `ChartDetail`·`DetailRow` 확장 | `crates/rbms-library/src/{lib.rs,scan.rs,songdb.rs,songdb/rows.rs,songdb/schema.sql}`(+ 시험) | R1 | 약 250줄 + 시험 150줄 |
| R3 | 앱 연결: 상세 읽기·없으면 계산, `ChartMeta`·float·숫자, 확장 id 목록, 미리보기 합성값 | `apps/rbms-player/src/{library.rs,skin_host/chart.rs,skin_host/extension.rs,stage/select/skin_preview.rs}`(+ 시험) | R0, R2 | 약 200줄 + 시험 150줄 |
| R6 | CLI: 차트 요약에 레이더(원시+표시) 출력, 라이브러리 분포 점검(선택) | `apps/rbms-cli/src/main.rs` 와 그 모듈 | R1 | 약 40줄(선택 +150줄) |
| R5 | 기본 스킨: 격자 스프라이트, `radar.lua`, 값·별표, id 미러, 캡처 확인 | `assets/skins/rbms-default/{lua/select/radar.lua,lua/select/main.lua,lua/props.lua,lua/theme.lua,tools/generate-images.py,image/select.png}` | R3, R4 | 약 200줄 Lua + 생성 이미지 |
| R7 | 문서: `skin.md` 1.4절 표, `architecture.md`, `crates.md`, `ui-design.md` 낡은 서술, ADR, QA 점검표, PROCESS·HANDOFF | `docs/**` | 모든 단위 끝 | 약 200줄 |

병렬 가능 묶음:

1. 웨이브 1: R0 와 R1 을 동시에 시작한다. R0 가 끝나면 R1 이 끝나기를 기다리지 않고 R4 를 시작한다.
2. 웨이브 2: R2 와 R6 을 동시에 시작한다(R1 이 끝난 뒤).
3. 웨이브 3: R3(R0·R2 가 끝난 뒤).
4. 웨이브 4: R5(R3·R4 뒤)와 R7.

충돌 규칙: `lua/select/main.lua`·`lua/theme.lua`·`skin_preview.rs`·`extension.rs` 는 N5·N7 도 만진다. R5 는 `radar.lua` 새 파일과 `main.lua` 한 줄 호출만 소유하고 육각형을 놓을 위치·크기는 N7 의 선곡 레이아웃 결정이 정한 값(상수 하나)을 받는다. 확장 id 는 이 문서가 float·숫자 21150~21155 와 참조 이미지 21160 만 쓰고 이벤트 21002~21099·옵션 21105 이후는 N5·N7 이 쓸 수 있도록 남긴다.

게이트(각 단위 끝, 최소 단일 검증): R1·R2·R3·R0 `cargo test -p <크레이트> --locked` 와 `cargo fmt --all --check`, R4 `cargo test -p rbms-render --locked`, R5 `skin-dump --strict` 와 캡처 시험 1회, 통합 후 클리피 한 번(`HANDOFF.md` 7절 게이트). 실기 점검(QA)은 R7 에서 항목으로 적는다: 쓸 만한 곡 5~10곡의 육각형을 눈으로 확인, 스크래치 곡·롱노트 곡·BPM 변화 곡·9키 곡·DP 곡 각 1곡.
