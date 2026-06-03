# 2026-05-31 — 판정 검증 + 조정 수치 + 설정 탭 + 스킨/레인장식/가로 게이지

사용자: "판정·total 제대로 계산되나? 너무 빡세다" → 검증 후 수치 조정·UI 개선.

## 판정/게이지 검증 결과 (충실히 포팅됨)
- 7K NOTE 판정 윈도우 = beatoraja `JudgeProperty.SEVENKEYS` **정확 일치** (PG±20 / GR±60 / GD±150 / BD-280~220 / MS-150~500 µs @judgerank100). LN END도 일치.
- `#RANK 0~4 → judgerank [25,50,75,100,125]%` = beatoraja `JudgeWindowRule.NORMAL.judgerank`와 **정확 일치**. 즉 #RANK 2(NORMAL)=75%=PGREAT±15ms는 beatoraja와 동일(버그 아님).
- 게이지/TOTAL: `deltas * total/notes`(그루브) 공식 일치. press 매칭/자동미스(BAD 늦은경계 이후) 정확, 조기미스 없음.
- 결론: "빡센" 건 NORMAL 윈도우가 원래 타이트한 것 + 입력/표시 지연 미보정. → JUDGE OFFSET으로 캘리브레이션.

## 구현
- **조정 수치**(settings.ron 영속): `JUDGE WIDTH`(judge_rate 50~200%, `Player::set_judge_rate`가 rank judgerank×rate로 윈도우 재스케일 — 넓히면 관대), `TOTAL`(override; AUTO면 #TOTAL, 없으면 `default_total`=7.605n/(0.01n+6.5) max260), 기존 `JUDGE OFFSET`.
- **설정 탭**: `SETTING_TABS` PLAY/GAUGE/JUDGE/DISPLAY/INPUT, `Tab`키 전환, set_tab+set_sel(탭 내). setting_line/adjust_setting은 글로벌 인덱스.
- **스킨 일반/와이드**: `assets/skins/{normal,wide}.ron` + `include_str!` 임베드 + `SKIN` 설정(NORMAL↔WIDE), `--skin`은 외부 오버라이드. wide=넓은 필드(0.42)+작은 우측 BGA.
- **레인 outline + 구분선**: `SkinConfig.outline_color/alpha`·`divider_color/alpha`(alpha=0=off, opacity 겸용). `draw_field_decor`가 빔↔판정선 사이(노트 아래)에 내부 구분선 + 테두리.
- **가로 게이지**: HUD 게이지를 좌측 세로 → **레인 아래 가로**(0%왼~100%오른, 80% 흰 틱, % 숫자) IIDX/beatoraja식.

## 적대적 리뷰 — 2결함
1. (MED) `judge_rate>100`이면 넓힌 BAD 늦은경계(예 -420ms)를 고정 `MJUDGE_START`(-290ms) 후보 게이트가 잘라, head+290~420ms 사이 "눌러도 무반응·미스도 아직" 림보. → press 게이트를 활성 윈도우(`gate_late=w.bd.0`, `gate_early=w.ms.1`)에서 유도(미스 sweep과 동일 소스). 회귀테스트(350ms 늦은 press=BAD) 추가. (beatoraja도 mjudgestart/end를 스케일된 테이블에서 산출 → 일치)
2. (LOW) DP(10K/14K) 스크래치 2개가 한쪽 끝에 묶임(IIDX는 양 바깥) — cosmetic, 좌표/판정 정상. 한계 문서화.

## 오토 캘리브레이션 (판정 오프셋) — beatoraja 확인 후 포팅
사용자 "오토 캘리브레이션 판정라인 가능?" → beatoraja `JudgeManager.java:718-725` 확인: `isNotesDisplayTimingAutoAdjust`가 정확판정(judge≤2)+|mfast|≤150ms마다 `judgetiming -= round(mfast/30000)` 실시간 보정. 포팅: `AUTO CAL` 설정(JUDGE 탭). 인터랙티브 정확판정의 timing delta 평균을 누적 → 결과 시 `calibrated_offset`(offset_ms += round(mean_us/1000), clamp±200)로 보정·저장(min 20히트).
- 적대적 리뷰 1결함(MED): beatoraja식 런-중-실시간 보정은 ① 같은 런 초/후반 판정 불일치 ② 리플레이가 최종 offset만 저장→재생 desync. → **런 중 offset 고정, 평균 오차로 다음 런 보정**(per-run)으로 재설계. 부호/수렴은 정확(발산 없음) 확인됨. 1런 수렴 테스트.

## 검증
**92 테스트**(judge_rate +2, 오토캘 +2), 빌드 무경고. 렌더 PNG로 외곽선·구분선·가로게이지 확인, wide.ron 파싱 OK. 적대적 리뷰 3회(judge 게이트·오토캘 포함).
