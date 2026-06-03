# 2026-05-31 — M6 판정 + M4 autoplay 통합

## M6 — rbms-judge (순수)
- `windows.rs`: `JudgeWindows`(7K NOTE µs 테이블: PG±20·GR±60·GD±150·BD[-280,+220]·MS[-150,+500]ms), `(late,early)` 쌍, `dmtime=note−press`(>0=FAST). `scaled(judgerank%)`(PG~BD 스케일, MS 고정), `rank_to_judgerank`(#RANK 0–4→{25,50,75,100,125}). `judge(dmtime)->Option<Judge>`.
- `matcher.rs`: `JudgeEngine` — 레인별 시간정렬 노트+커서, `press`(스캔범위 [-290k,+500k]에서 최근접 미판정 매칭→윈도우 분류), `update`(BAD-late 경과 노트 MISS 스윕), combo/max_combo/counts[6]/ex_score(PG2+GR1). `from_model`(Normal+LN헤드만).
- 검증: 9 단위테스트(윈도우 분류·FAST부호·rank스케일·최근접·미스·콤보·노매치).

## M4 — rbms-play (통합, 순수: model/chart/judge만 의존)
- `simulate_autoplay(model)`: 모든 노트를 정확 시각에 press → JudgeEngine 반환(검증용).
- `Player`: 키음 스케줄러(bg=채널01 + 노트), `update(now, play_fn)`(due 키음 emit + autoplay 판정 + 미스 스윕), `press(lane, now, play_fn)`(최근접 노트 키음 + 판정). 오디오/렌더 비의존(play_fn 클로저) → 테스트 용이.
- 검증: 3 단위테스트 + **실차트 autoplay**: FELYS 812·Parousia ANOTHER 2024·約束 566 노트 전부 PGREAT·풀콤보·EX만점·0미스.

## 누적
- 워크스페이스 테스트 **56 통과**. M0~M4·M6 헤드리스 완결.
- 남음: wgpu+winit 실시간 윈도우(오디오 클럭→매 프레임 playfield+입력→판정), M5 실입력 배선.
