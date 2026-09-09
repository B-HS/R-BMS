# beatmania IIDX 30~34 신규/변경 기능 조사 (웹 1차 조사)

조사일: 2026-09-09. 조사 시간 상한 8분 내 수행.
대상: 30 RESIDENT / 31 EPOLIS / 32 Pinky Crush / 33 Sparkle Shower / 34 ZINRAI

## 0. 버전 개요

| 버전 | 정식 타이틀 | 가동일 | 비고 |
|---|---|---|---|
| 30 | RESIDENT | 2022-10-19 | LIGHTNING MODEL 1080p(FHD) 대응 |
| 31 | EPOLIS | 2023-10-18 | 구형 기체는 GELDJ-JX 업그레이드 키트 필요 |
| 32 | Pinky Crush | 2024-10-09 | 게임머니 EPO → ARRO |
| 33 | Sparkle Shower | 2025-09-17 | DJ TRAINING 신설 |
| 34 | ZINRAI | 로케테스트 2026-06-26~28, 2026-07-18 (정식 가동일 **미확인**) | 로케테 기준 정보 |

출처: <https://iidx.org/compendium/feature_history>, <https://scrmbl.com/post/beatmania-iidx-34-zinrai-location-test>, <https://p.eagate.573.jp/game/2dx/34/topics/top/>

---

## 1. 질문 1 — 버전별 신규/변경 플레이 옵션

### 30 RESIDENT

| 항목 | 내용 | 출처 |
|---|---|---|
| 해상도 | LIGHTNING MODEL 네이티브 1080p(FHD) 대응. 구형 모니터용 HD 설정 병존 | iidx.org, namu 30 |
| FAST/SLOW | **스크래치와 건반의 FAST/SLOW 판정 표시를 분리**하는 옵션 추가 ("Added option for separated FAST/SLOW display for scratches and notes"). S-FAST 형태로 표시되며 표시 위치 조정 가능 | iidx.org, namu 30 |
| 녹색수/흰수 명칭 | 흰 숫자 = SUB-LENGTH, 녹색 숫자 = NOTES TIME 으로 공식 명칭 부여 | namu 30 |
| 실패 표시 | 폭사 시 "STAGE FAILED" → **"FAILED"** 로 문구 변경 | namu 30 |
| RANDOM/GAUGE/SUD+/HID+/LIFT/FLOATING | 이 버전의 신규·변경 사항 **미확인** (검색 범위 내 변경 언급 없음) | - |

### 31 EPOLIS

| 항목 | 내용 | 출처 |
|---|---|---|
| **옵션 삭제** | **H-RANDOM 폐지, EXPAND-JUDGE 폐지** | iidx.org, remywiki 검색 스니펫, namu 31 |
| 옵션 화면 | OPTION 화면 UI 리뉴얼. 조작 방식은 동일하되 설명이 직관적으로 변경, 가이드 UI 가 화면 하단으로 이동 | remywiki 검색 스니펫, namu 31 |
| HI-SPEED | 상세 옵션 화면에서 **클래식 하이스피드(구 방식) 적용 여부 토글** 가능 | namu 31 |
| LIFT | **LIFT 레인커버를 SUDDEN+ 와 독립적으로 설정** 가능. LIFT 전용 이미지 지정 가능 | iidx.org, namu 31 |
| 노트/빔 커스터마이즈 | 키빔 길이(매우 짧음/짧음/보통/김), 노트 두께(극세/세/보통/굵음) 추가 | iidx.org, namu 31 |
| 레인커버 디자인 | 프리미엄 레인커버(애니메이션 효과) 추가, LIFT 이미지 지정 | iidx.org, namu 31 |
| 페이스메이커 배경 | **애니메이션 페이스메이커 배경** 커스터마이즈 추가 | iidx.org |
| 서브화면 | 서브스크린(터치패널) 배경 커스터마이즈 추가. LM 터치스크린에서 플레이 커스터마이즈 조작 가능(플레이 중 조정 포함) | iidx.org, namu 31 |
| 키보드 배열 | 영문 키보드 배열 ABC / QWERTY 선택 | namu 31 |
| 리트라이 | LM 한정 **동일 RANDOM 배치 리트라이**(PASELI 유료, 1인 플레이·곡/난이도당 1회) | iidx.org, namu 31 |

### 32 Pinky Crush

| 항목 | 내용 | 출처 |
|---|---|---|
| BGA 표시 | 곡선택에서 **BGA 썸네일(정지 이미지) 미리보기** 표시, 플레이 설정에서 OFF 가능. 기존 ALL BEGINNER 프리뷰 대체 | iidx.org, bemaniwiki 32, namu 32 |
| 게이지 강제 폴더 | STEP UP 모드에 **HARD / EX-HARD 게이지 폴더** 추가(해당 게이지가 강제 적용) | iidx.org, namu 32 |
| RANDOM/판정/노트 스킨 | 이 버전의 신규·변경 사항 **미확인** | - |

### 33 Sparkle Shower

| 항목 | 내용 | 출처 |
|---|---|---|
| **JUDGE 위치 조정** | 판정 문자 표시 위치 조정 범위가 기존 **+1~+3 → -2~+10** 으로 확장. 기존 설정 변환은 +1→+3 / +2→+6 / +3→+9 | iidx.org, bemaniwiki 33, namu 33 |
| 레인 밝기 | "레인 표시물 밝기 조정" — 노트·소절선·키빔·폭발·판정문자(·풀콤보 이펙트)의 밝기를 **1.0~0.9(90~100%) 범위로 일괄 조정** | iidx.org, bemaniwiki 33, namu 33 |
| **CN/HCN 스킨** | **차지노트 디자인을 일반 노트와 별도로 설정 가능**(보유 디자인 선택 + 두께 조정) | iidx.org, bemaniwiki 33, namu 33 |
| 집중 모드 | 집중(コンセントレーション) 모드 커스터마이즈, 배경 밝기 저하 가능, 홈 버튼으로 배경만 보기 | iidx.org, namu 33 |
| 카드 엔트리 배경 | 엔트리 카드 배경 이미지 추가 + 투명도 조정 | iidx.org, namu 33 |
| LM 서브화면 | 터치스크린에 "어시스트" 창 추가 — 보면 정보·곡 추천 표시 | iidx.org |

### 34 ZINRAI (로케테스트 기준)

| 항목 | 내용 | 출처 |
|---|---|---|
| 녹색수 리셋 | 게임 중 START 버튼 2회 눌러 **BPM 변화 후 노트 속도(녹색수) 리셋** 가능. 옵션에서 ON/OFF | remywiki 검색 스니펫 |
| **LANE COVER ADJUST** | BPM 변화 시 **SUD+ 가 슬라이드**해 HI-SPEED 값과 녹색수를 맞춤 | bemaniwiki 34, remywiki 스니펫 |
| **SPEED ADJUST** | BPM 변화 시 **HI-SPEED 값이 조정**되어 SUD+ 위치·녹색수에 맞춤 (10.0 상한 초과 가능) | bemaniwiki 34 |
| **LANE SPEED ADJUST** | HI-SPEED 와 녹색수가 같아질 때까지 SUDDEN+ 위치를 변경 | remywiki 스니펫 |
| SUD+/LIFT 고정 | 위 어시스트 사용 시 "SUD+およびLIFTの位置が完全に固定され、動かすことも外すこともできなくなる" (위치 완전 고정, 이동·해제 불가) | bemaniwiki 34 |
| BPM 변화 예고 | "BPM変化の直前に変化の内容が表示" — 상승은 파랑, 하강은 빨강 표시 | bemaniwiki 34 |
| 표시 영역 정보 상시 표시 | 레인커버 수치를 상시 표시하는 옵션 | bemaniwiki 34 |
| 기체 조명 | LIGHTNING 모델 조명이 보라색·기존보다 어두운 톤 | bemaniwiki 34 |

---

## 2. 질문 2 — 스코어 그래프/PACEMAKER/타깃, 결과화면, 곡선택 화면

### 스코어 그래프 / PACEMAKER / 타깃

| 버전 | 변경 | 출처 |
|---|---|---|
| 30 | 페이스메이커 그래프 강화 — 기존 가로 방향 단색에서 **원통형 그라데이션** 표현으로 변경 | namu 30 |
| 31 | **애니메이션 페이스메이커 배경** 커스터마이즈 추가 | iidx.org |
| 32 | 페이스메이커 그래프·비교 표시 리파인, **라이벌 로테이트** 및 비교 스코어 메커니즘 변경. 서브라이벌(최대 10명) 스코어가 메인 라이벌 부족 시 스코어 창에 표시 | bemaniwiki 32, namu 32 |
| 33 | 그루브 게이지 그래프에 "演奏しやすい譜面に変更しました"(연주하기 쉬운 보면으로 변경됨) 표기 추가 | bemaniwiki 33 |
| 34 | **스코어 클래스** — 상위 10위/100위/500위 스코어를 표시(플레이어명 없이 점수만) | bemaniwiki 34 |
| 타깃 종류 자체의 신규 추가 | 30~34 전 범위에서 **미확인** | - |

### 결과 화면

| 버전 | 변경 | 출처 |
|---|---|---|
| 30 | 결과 화면 UI 재정리, FAST/SLOW·페이스메이커 관련 텍스트에 **신규 시스템 폰트** 적용. **노트 레이더 값 실시간 확인** 가능(항목별 10곡 계산 대상곡 열람) | namu 30 |
| 31 | Play Share UI 갱신 — 레벨·노트 수·**FAST/SLOW 그래프** 포함. 단위인정 결과 화면에 곡명 추가 | namu 31 |
| 32 | DJ LEVEL 에 따라 캐릭터 표시 분기(AAA~A: エレキ/シア/鉄火, B 이하 없음, FAILED 시 우는 토끼). Play Record 를 **YouTube 로 직접 업로드**(대부분의 곡, LM 한정) | bemaniwiki 32, iidx.org |
| 33 | 결과 화면 레이아웃을 Tricoro 계열 디자인으로 개선, HI-SPEED·차지노트 설명 명확화. DJ LEVEL 별 캐릭터(AAA: アヤハ/リィナ 3색) 표시 | namu 33, bemaniwiki 33 |
| 34 | CLEAR DATA 에 **HARD CLEAR / EX-HARD CLEAR 레이트 추가**(풀콤보 레이트와 함께 표기) | bemaniwiki 34, remywiki 스니펫 |
| 판정 분포(PGREAT/GREAT…) 상세 표시·MAX- 표기 변경 | 30~34 범위에서 **미확인** | - |

### 곡선택 화면

| 버전 | 변경 | 출처 |
|---|---|---|
| 30 | **스크롤바를 통한 클리어 램프 체크리스트** 표시. 폴더 끝까지 남은 칸 수 표시. **노트 레이더 전용 폴더** 신설. 모드/곡선택 화면에 이미지 캐릭터 일러스트 랜덤 등장. 등록 라이벌 최대 6명으로 증가 | namu 30, remywiki 스니펫 |
| 31 | **곡 필터 기능 — 커스텀 폴더 최대 5개**. "첫 방문자 추천" 폴더 신설(e-amusement pass 없는 초심자용 한정 곡 폴더). OTHERS 폴더를 0~9 번호별 서브폴더로 분리 | namu 31, remywiki 스니펫 |
| 32 | **카테고리 기능**으로 폴더 종류(초심자/버전/레벨 등) 필터링. 단, 공식 사이트의 카테고리 커스터마이즈 기능은 폐지. **BGA 썸네일 프리뷰**. 오리지널 필터에 "譜面難易度"(보면 난이도 범위)·"ノーツレーダー"·"ライバル" 추가. 현행 버전 곡명은 파란 폰트 + 외곽선 | bemaniwiki 32, iidx.org, namu 32 |
| 33 | **악곡 레코멘드 기능** — 최근 플레이 기반으로 아티스트/장르/플레이 경향별 추천 폴더 생성(2곡째부터 표시), 난이도 변경 용이. 플레이 전 노트 수·노트 내역·레이더 값 확인 가능 | bemaniwiki 33, namu 33 |
| 34 | **DJ LEVEL DISPLAY** — 곡명 왼쪽에 난이도와 나란히 **자기 최고 DJ LEVEL 을 색상 구분 표시**. 譜面情報表示(넘버패드 [9] 또는 서브모니터)로 시간대별 노트 수·건반/스크래치 내역·노트 레이더·클리어율/풀콤보율 상시 표시 | bemaniwiki 34, remywiki 스니펫 |

---

## 3. 질문 3 — 모드 요약 (1줄)

| 버전 | 모드 |
|---|---|
| 30 | **ARENA**: 온라인 대전 모드 운용 지속(namu 30). |
| 30 | **BPL BATTLE**: 통상 모드에서 표시되며 텐키 5로 토글(namu 30). |
| 30 | **STEP UP**: 과제곡 난이도 범위를 조정 가능하도록 개선(namu 30). |
| 30 | **WEEKLY RANKING**: 2022-11-16 추가, 매주 수요일 12시 곡 교체·상위 스코어 시 별 획득(remywiki 스니펫). |
| 30 | **메달/장식 시스템**: TRAN 메달을 대체하는 장식(achievement) 시스템 도입(iidx.org, namu 30). |
| 31 | **커스터마이즈 폴더**: 플레이어 취향 기반 추천 필터 폴더가 시리즈 최초 도입(remywiki 스니펫). |
| 32 | **BATTLE 통합**: ARENA 와 BPL BATTLE 을 하나의 BATTLE 모드로 통합, 모드 선택 후 매칭 형식 선택(bemaniwiki 32, namu 32). |
| 32 | **FREE / FREE PLUS / HAZARD 폐지**, PREMIUM FREE 로 일원화(비PASELI J리전용 4분 고정 크레딧 플레이 추가)(iidx.org, bemaniwiki 32). |
| 32 | **EX 단위인정 폐지**(bemaniwiki 32). |
| 32 | **My Activity**(LM): 일/주 단위 활동 기록·목표 설정·퍼크, 본인/라이벌 플레이 타임라인(namu 32). |
| 32 | **모드 순서 변경**: STANDARD → STEP UP → 단위인정 → PREMIUM FREE(namu 32). |
| 33 | **DJ TRAINING**: 신설 트레이닝 폴더·랭크(WHITE~BLACK 7단계), 폴더당 50포인트 달성 시 상위 폴더 해금(iidx.org, bemaniwiki 33, namu 33). |
| 33 | **STEP UP**: 스토리형(Cupro/과일 축제 8스테이지, 단계별 포인트 상승) 구성(namu 33). |
| 33 | **ARENA**: 에피소드 단위 선곡·경쟁 스코어링의 온라인 대전(namu 33). |
| 33 | **Extra Stage 조건 완화**(32 시점): 레벨 6 이상 어시스트 클리어 이상으로 단순화(iidx.org, 32 항목). |
| 34 | 모드 구성 변경 사항 **미확인**(로케테스트 정보만 공개, 1인 플레이 한정 시연). |

---

## 4. 미확인 항목 (추측으로 채우지 않음)

- 30~34 각 버전의 RANDOM 계열(MIRROR/R-RANDOM/S-RANDOM 등) 및 ASSIST(AUTO SCRATCH/LEGACY NOTE 등) 신규·삭제 여부 — EPOLIS 의 H-RANDOM 폐지 외에는 확인되지 않음.
- GAUGE 종류 자체(ASSISTED EASY/EASY/NORMAL/HARD/EX-HARD)의 신규 추가 — 32의 STEP UP 게이지 폴더 외 변경 미확인.
- 판정 분포(PGREAT/GREAT/GOOD/BAD/POOR) 표시 형식, MAX- 표기, 게이지 추이 그래프 상세 사양 변화.
- 폭발(BOMB) 스킨 커스터마이즈의 버전별 최초 도입 시점 — 33의 밝기 일괄 조정에 "폭발" 포함은 확인, 스킨 자체 선택 도입 시점은 미확인.
- 34 ZINRAI 의 정식 가동일, 최종 사양(로케테스트 사양은 제품판과 다를 수 있음).
- remywiki 각 버전 페이지 본문은 WebFetch 403(봇 차단)으로 직접 통독 실패 — 검색 스니펫으로만 인용. bemaniwiki/namu 로 교차 확인한 항목만 표에 기재.

---

## 5. 출처 URL

- iidx.org 기능 변경 이력: <https://iidx.org/compendium/feature_history>
- RemyWiki AC_RESIDENT: <https://remywiki.com/AC_RESIDENT> (403, 검색 스니펫 인용)
- RemyWiki AC_EPOLIS: <https://remywiki.com/AC_EPOLIS> (403, 검색 스니펫 인용)
- RemyWiki AC_Pinky_Crush: <https://remywiki.com/AC_Pinky_Crush>
- RemyWiki AC_Sparkle_Shower: <https://remywiki.com/AC_Sparkle_Shower>
- RemyWiki AC_ZINRAI: <https://remywiki.com/AC_ZINRAI> (403, 검색 스니펫 인용)
- BEMANIwiki 34 ZINRAI: <https://bemaniwiki.com/?beatmania+IIDX+34+ZINRAI>
- BEMANIwiki 33 Sparkle Shower: <https://bemaniwiki.com/?beatmania+IIDX+33+Sparkle+Shower>
- BEMANIwiki 32 Pinky Crush: <https://bemaniwiki.com/?beatmania+IIDX+32+Pinky+Crush>
- NamuWiki(en) 30 RESIDENT: <https://en.namu.wiki/w/beatmania%20IIDX%2030%20RESIDENT>
- NamuWiki(en) 31 EPOLIS: <https://en.namu.wiki/w/beatmania%20IIDX%2031%20EPOLIS>
- NamuWiki(en) 32 Pinky Crush: <https://en.namu.wiki/w/beatmania%20IIDX%2032%20Pinky%20Crush>
- NamuWiki(en) 33 Sparkle Shower: <https://en.namu.wiki/w/beatmania%20IIDX%2033%20Sparkle%20Shower>
- 공식 e-amusement 34 ZINRAI 토픽(로케테스트): <https://p.eagate.573.jp/game/2dx/34/topics/top/>
- 공식 KONAMI 32 Pinky Crush 제품 페이지: <https://www.konami.com/arcadegames/products/am_bmiidx32/>
- 공식 e-amusement 32 Pinky Crush: <https://p.eagate.573.jp/game/2dx/32/>
- ZINRAI 로케테스트 보도: <https://scrmbl.com/post/beatmania-iidx-34-zinrai-location-test>
- ZINRAI 발표 보도(電ファミ): <https://news.denfaminicogamer.jp/news/2606173b>
