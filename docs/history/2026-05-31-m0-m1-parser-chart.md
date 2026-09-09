# 2026-05-31 — M0(파서) + M1(타이밍/레인) 구현·검증

## 한 일
- Cargo workspace 구성: `rbms-model`(순수 타입) ← `rbms-parser` ← `rbms-chart`, `rbms-judge`(스텁), `apps/rbms-cli`(검증 도구).
- **rbms-parser (M0)**: BMS 패밀리 lexical 파서.
  - base36/62 변환(`base.rs`), 채널은 base36 고정.
  - 인코딩: UTF-8 BOM/valid-UTF-8 우선, 아니면 Shift-JIS(`encoding_rs`).
  - 헤더(#TITLE/#ARTIST/#BPM/#PLAYLEVEL/#RANK/#TOTAL/#LNOBJ/#LNTYPE/#WAVxx/#BMPxx/#BPMxx/#STOPxx/#SCROLLxx/#BASE …), 데이터필드 `#mmmCC:....`(마디 가변 해상도, "00" skip), 마디길이 #xxx02.
  - 제어흐름 `#RANDOM/#SETRANDOM/#IF/#ELSEIF/#ELSE/#ENDIF/#ENDRANDOM`(스택 기반, 시드 결정적 LCG).
  - MD5 + SHA-256(raw 바이트).
- **rbms-chart (M1)**: `to_model(src, mode) -> Model`.
  - 마디→절대 µs 적분: `time = prev.time + prev.stop + 240_000_000*(section-prev.section)/prev.bpm`.
  - section = 누적 rate(마디길이 반영), STOP = `240e6*S/(192*bpm)`, BPM(03 hex inline / 08 ref), SCROLL(SC), BPM/scroll 상속(exp_ 사이드 벡터).
  - 채널→레인(모드 프로파일 + **key clamp**: 테이블은 P1+P2 전체, key가 실제 레인 한정), LN(채널 51-59 토글 / #LNOBJ 변환), mine(D1-E9), hidden(31-49), BGM(01).

## 검증 결과
- 단위 테스트 **27개 통과** (parser 11 + chart 14 + model 2).
  - 타이밍: 고정 BPM 절대시각, 마디 인덱스, 반마디 위치, STOP 1마디 시프트, BPM 03/08 변화 — 전부 손계산과 일치.
  - 매핑: 7K 채널→레인(11→0,13→2,18→5,19→6,16→7), DP/PMS P2 채널 무시(panic 없음).
  - 해시: MD5/SHA-256 known-answer("abc").
- **실제 라이브러리(발광 ★1, `/Documents/personally/1/`) 727곡(.bms/.bme/.bml/.pms)**:
  - 파싱 **panic 0 / 727**.
  - **MD5 byte-exact 일치 727 / 727** → 레퍼런스 구현 점수·IR·리플레이 키 호환의 근거 확보.
  - Shift-JIS 타이틀 정확("約束 -HappyHyperStarmiX-", "Parousia[α]", 전각공백 보존).

## 발견·수정한 버그
- `Mode::lane_of_raw`가 18칸 테이블의 P2 레인(8-15)을 7K(key=8)에서도 반환 → DP/PMS 차트에서 `notes[14]` 인덱스 초과 **panic 149건**. → `lane < key` clamp로 수정(모드 확장성 설계와 일치). 회귀 테스트 추가.

## 알려진 후속(이번 범위 밖)
- 모드 자동 감지(현재 CLI는 BEAT_7K 고정 — DP/PMS는 부분 매핑). 파일 사용 채널+파일명 힌트로 감지.
- #LNTYPE 2(연속형) LN, CN/HCN 구분(#LNMODE), mine 데미지 정확값('ZZ' 즉사).
- bmson/.bxs.
- 적대적 리뷰(`wfgr2tk4o`) 결과 반영.

## 다음 (M2~)
- M2 오디오(cpal+symphonia+rubato, RT voice pool, SAMPLES_PLAYED 클럭), M3 렌더(wgpu+winit, scroll), M4 autoplay, 이후 M5 입력·M6 판정.
