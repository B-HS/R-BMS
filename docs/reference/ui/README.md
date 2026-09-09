# UI 레퍼런스 모음

rbms UI 재설계(곡선택/플레이/결과)의 시각 레퍼런스. 디자인 스펙은 [`../ui-design.md`](../ui-design.md).

## provided/ — 사용자 제공 (그라운드 트루스)
- `01-rbms-play-current-noteissue.png` — 현 rbms 플레이. **노트가 필드 상단서 팝인**(프레임 밖에서 나오는 느낌) → top 베젤로 해결.
- `02-rbms-play-current-nograph.png` — 현 rbms 플레이. **중앙 라이브 그래프 없음** → IIDX 페이스메이커 추가.
- `03-iidx-play-target.png` — **목표 플레이 화면**(IIDX SP): 좌 필드+게이지+턴테이블, 중앙 라이브 스코어 그래프(AAA/AA/A 밴드+TARGET/MYBEST), 우 BGA, 우하 판정카운트.
- `04-iidx-result-target.png` — **목표 결과 화면**(IIDX): 좌상 거대 DJ LEVEL(AAA) + 스코어 리포트(GREAT/GOOD/BAD/POOR·MAXCOMBO·TOTAL SCORE).
- `05-rbms-play-bezel-issue.png` — (해결됨) 노트 클리핑 전, 상단 베젤 블록이 어색하던 캡처. 근본 수정으로 제거.
- `06-reference-select-target.png` — **목표 곡선택**(레퍼런스 구현 modern chic): 좌 상세(커버+배너+INTERNET RANKING+CLEAR/FC rate), 하단 NOTES/TOTAL/TIME/PLAY/JUDGE 통계 + 판정분포 막대 + EX SCORE/DJ랭크 + **노트 밀도 그래프(PEAK/END/AVG density)**, 우 곡 바 리스트(KEY 수·LN타입·클리어램프). 곡선택 UI 고도화의 1차 타깃.

## fetched/ — 자동 수집 (18장, LR2/IIDX/레퍼런스 구현)
리서치 워크플로가 수집(`img_NN.*`). LR2 클라이언트 곡선택/플레이옵션/결과 스킨, IIDX 계열 스크린샷 다수. 일부 URL은 신뢰 불가해 유효 이미지(`file` 검증 통과분)만 보존.

> 메모: 검증 브라우징이 없는 리서치 에이전트는 일부 URL을 지어내므로, 본 폴더는 다운로드 성공·이미지 검증된 것만 담는다. 레이아웃 디테일의 1차 근거는 `provided/` 4장 + `ui-design.md`.
