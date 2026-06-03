# 테스트 BMS 라이브러리 실측 (발광 BMS ★1)

## 경로
`/Users/hyunseokbyun/Documents/personally/1/` — 약 50곡. 엔드투엔드 검증용.
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
- 결론: **로드 시 (a) 8/16/24/32-bit → f32 변환, (b) 디바이스 출력 레이트로 리샘플(rubato)** 을 일괄 수행해 믹서는 단일 포맷(f32, 디바이스 레이트)만 다루게 한다. (beatoraja PCM.java가 같은 변환·trailing-silence 트림을 한 이유.)

## 데이터필드 관찰
- `#mmmCC:....` 형식. 예 `#00616:002B0000` = 마디 006, 채널 16(P1 스크래치), 2자리 base-36 오브젝트 ID × 4 = 4분할.
- 채널 04/07 = BGA(첫 모듈 범위 밖이나 파싱은 무시 처리), 01 = BGM.
- 마디별 가변 해상도: 데이터 문자열 길이/2 = 그 마디의 분할 수.

## BGA(범위 밖, 파싱만)
- `.bmp`(5,742)·`.png`(895) 이미지, `.mpg`(18) 비디오. 첫 모듈에선 `#BMPxx`/BGA 채널을 **파싱하되 렌더 안 함**.
