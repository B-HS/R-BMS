# 테스트 BMS 라이브러리 실측 (발광 BMS ★1)

최종 갱신 2026-10-15 · 대응 커밋 ea698a9

## 경로
`/Volumes/SSD/2026-07-31-backup/Documents/personally/1/` — 폴더 53개, 약 50곡(곡 DB 기준 727 차트, [테스트 부채](../quality-assurance/2026-10-14-test-debt.md) 3.18). 엔드투엔드 검증용. 이 문서의 아래 수치는 2026-10-15 에 이 경로에서 다시 센 값이다(아래 확장자 수와 모두 같았다).
이 폴더는 예전에 `/Users/hyunseokbyun/Documents/personally/1/` 에 있었고 그 경로는 지금 없다. 큰 라이브러리는 `/Volumes/SSD/bms`(같은 문서 3.18 이 센 11,336 차트)다. 둘 다 외장 SSD 라 마운트가 자주 빠진다. 마운트되어 있지 않으면 라이브러리가 필요한 검증(3.18, 3.38, 3.51, 3.56)은 하지 못하고 부채로 남는다.
(저장소에는 커밋 금지 — 저작권. `.gitignore`의 `/assets/songs` 처럼 외부 참조만.)

## 차트 포맷 분포 (확장자 카운트)
- `.bme` 483 (최다) · `.bms` 134 · `.pms` 57 · `.bml` 53 · `.bxs/.bxe/.bmx` 소수 · `.json` 53
- **첫 모듈 우선순위: BMS 패밀리(.bme/.bms/.bml/.pms)**. `.json`은 bmson이 아니라 `bx_song_data.json`(bxs/bmx 계열 메타) → bmson/bxs는 후순위.
- 한 폴더에 5K/7K/14K 차트 공존(예: `[onoken] FELYS`의 `5a-5.bms`/`7a-5.bme`/`_felys_14keys_ls.bme`) → 모드 프로파일(data-driven) 설계 정당화.

## 인코딩 — 파서 필수 처리
- BMS 텍스트는 **Shift-JIS**. 예: `yaku_e.bms` `#TITLE` 바이트 `96 F1 91 A9` = "約束".
- ASCII 전용 차트도 있음(Shift-JIS의 하위호환). → **Shift-JIS 디코드 + UTF-8 폴백(+BOM 감지)**. 크레이트: `encoding_rs`.
- 줄바꿈 **CRLF**, 매우 긴 라인 존재.

## 오디오 — 디코더/믹서 필수 처리
- **OGG Vorbis**(16,631개, 압도적): stereo 44100Hz. symphonia Vorbis 디코드.
- **WAV**(6,954개): **8-bit unsigned PCM이 다수**, 샘플레이트 **22050 / 24000 / 44100Hz 혼재**, mono/stereo 혼재.
  - 표본 200개: `8bit stereo 22050Hz` 130, `8bit stereo 44100Hz` 70.
- 결론: **로드 시 (a) 8/16/24/32-bit → f32 변환, (b) 디바이스 출력 레이트로 리샘플(rubato)** 을 일괄 수행해 믹서는 단일 포맷(f32, 디바이스 레이트)만 다루게 한다. (레퍼런스 구현 PCM.java가 같은 변환·trailing-silence 트림을 한 이유.)

## 데이터필드 관찰
- `#mmmCC:....` 형식. 예 `#00616:002B0000` = 마디 006, 채널 16(P1 스크래치), 2자리 base-36 오브젝트 ID × 4 = 4분할.
- 채널 04/07 = BGA(첫 모듈 범위 밖이나 파싱은 무시 처리), 01 = BGM.
- 마디별 가변 해상도: 데이터 문자열 길이/2 = 그 마디의 분할 수.

## BGA(범위 밖, 파싱만)
- `.bmp`(5,742)·`.png`(895) 이미지, `.mpg`(18) 비디오. 첫 모듈에선 `#BMPxx`/BGA 채널을 **파싱하되 렌더 안 함**.
