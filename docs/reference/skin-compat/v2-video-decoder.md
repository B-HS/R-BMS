# R-BMS 스킨 동영상 디코더 선정 조사 (W7-1)

> 최종 갱신 2026-10-11 · 대응 단계: 웨이브 7B(동영상 source) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 7B 반영 사항 (2026-10-11)

`crates/rbms-video`(OpenH264 + `re_mp4`)와 스킨 동영상 source 재생을 넣은 뒤의 상태다.

- (W7-2) v2-video-decoder.md: 상단 '범위'·'남은 검증'과 §9 '대상 8개의 OpenH264 실제 디코드: 미실행' 행 — 실행 완료로 교체. 8개 전부 프레임 수 일치(600/601/921/750/2564/4229/7415/3604), 거부 샘플 0. 출력은 표시 순서이고 디코더가 crop 된 크기(1920x1080)를 돌려줌
- (W7-2) v2-video-decoder.md: §6 표와 §9 'CI 빌드 시간: 미확인' — rbms-video 클린 릴리스 빌드 real 8.0 s·user 20.5 s(Apple M5 Max, arm64 는 NEON .S 를 cc 가 조립, nasm 불필요). Linux·Windows 실측은 여전히 없음
- (W7-2) v2-video-decoder.md: §4.1 성능 행에 이 기기 실측 추가 — 디코드+색 변환 1080p 평균 5.9~7.8 ms·최대 12.8~24.6 ms, 720p 평균 2.4~3.3 ms·최대 5.5~8.1 ms. 디코드만 1080p 3.1~5.2 ms, 720p 1.3~1.9 ms
- (W7-2) v2-video-decoder.md: §4.2 re_mp4 행과 §9 'Mp4::read 메모리 사용 방식, raw_codec_config 반환 형식: 미확인' — 0.5.1 소스 확인 결과로 교체. Mp4::read 는 mdat 를 seek 로 건너뜀. raw_codec_config 는 avcC 원본. 위험: 컨테이너 안 크기 0 자식 박스에서 skip_box 가 제자리로 돌아가 무한 루프(moov.rs 등), AvcCBox 의 remainder 뺄셈 언더플로(avc1.rs:206), build_tracks 의 stsc/stco/stts/stss 인덱싱과 panic!(), hdlr·meta 의 박스 크기만큼 할당. 그래서 Mp4::read 를 쓰지 않음
- (W7-2) v2-video-decoder.md: §7.3 구현 구조 제안 표를 실제 구조로 고칠 것 — 디먹스 행: 자체 박스 검증기(crates/rbms-video/src/container.rs)가 moov 를 64 MiB 이하로 읽어 첫 H.264(avc1/avc3) 영상 트랙의 mvhd·tkhd·mdhd·hdlr·stsd(avcC 만)·stts·ctts·stss·stsc·stsz·stco/co64 만으로 moov 를 다시 조립하고 re_mp4 의 MoovBox::read_box 로 읽음. 샘플 표 전개는 mp4.rs 가 검사 산술로 수행(샘플 상한 2^22, 샘플 크기 상한 16 MiB, 파일 끝 넘는 샘플은 오류)
- (W7-2) v2-video-decoder.md: §7.3 디코드 호출·반복 재생 행 — Flush::NoFlush 로 디코드, 스트림 끝에서 flush_remaining() 의 프레임을 평면 복사로 보관해 하나씩 반환, 되감기는 Decoder 를 새로 만들고 SPS/PPS 를 다시 붙임. 입력 타임스탬프를 줄 수 없어 n번째 출력을 정렬된 합성 시각의 n번째에 대응시킴. 빈 패킷은 디코더에 넣지 않음
- (W7-2) v2-video-decoder.md: §7.3 워커 스레드·프레임 큐·버퍼 재사용 행 — sync_channel 이 아니라 Mutex+Condvar 한정 큐(기본 3프레임, DEFAULT_QUEUE_FRAMES), 소비자는 advance(시각)으로 가장 새 프레임만 취함, 따라잡을 때는 next_time_us 로 보여 줄 수 없는 프레임의 색 변환을 생략, 버퍼는 spare 목록으로 재사용, drop 시 join. '타이머 재시작' 행은 '장면 시계가 시작 시각보다 뒤로 가면 restart()' 로
- (W7-2) v2-video-decoder.md: §7.3 색 변환 행 — write_rgba8 을 쓰지 않고 자체 정수 고정소수 변환(color.rs)을 씀. 행렬은 SPS VUI → colr(nclx) → 높이 720 이상 BT.709 관례, 범위는 기본 제한
- (W7-2) v2-video-decoder.md: §1 표의 'Cargo.lock 에 cc 1.6.0' 근거 — Cargo.lock 은 .gitignore:3 으로 추적되지 않음을 덧붙일 것
- (W7-2) v2-video-decoder.md: §7.3 cargo 피처 행 — rbms-video 의 openh264(기본 켬), 워크스페이스 의존은 default-features = false, rbms-player 의 video(기본) = rbms-video/openh264, rbms-render 는 dev-dependency + 테스트용 video 피처. 끈 구성에서 open 은 VideoError::Disabled
- (리뷰 수정) v2-video-decoder.md: 상단에 '웨이브 7B 리뷰 수정 반영 사항' 추가. 본문 7.3 의 '반복 재생' 행에 있는 '탐색(seek)이 필요 없습니다'는 낡았습니다. 실제 구현은 VideoDecoder::seek 로 stss 키 샘플(합성 시각으로 안전성 확인, 탐색 시 IDR NAL 검증)부터 디코드하고, 시계가 여러 패스를 건너뛰면 워커가 pass_start 를 target.div_euclid(duration) * duration 으로 옮깁니다.
- (리뷰 수정) v2-video-decoder.md: 7.3 '프레임 큐 소비' 행에 추가. 시계가 화면 프레임보다 과거로 가면 큐와 화면 프레임을 버리고 그 시각에서 다시 찾습니다. 실측(릴리스): #default.mp4 20→12초 되감기 84 ms(수정 전 8.03초 정지), BGmovie01 +300초 점프 126 ms(수정 전 45.8초), +36,000초 102 ms, sample.mp4 1→8.2초 161 ms.
- (리뷰 수정) v2-video-decoder.md: 2절 표에 키 샘플 실측 추가. 8개 모두 stss 의 키가 전부 IDR 이고 합성 시각상 안전합니다. 최대 GOP 는 sample 250, sample2 91, BGmovie01 30, BGmovie02 30, #default 78, cyber 290, NOSTALGIC 169, travel 240 프레임입니다.
- (리뷰 수정) v2-video-decoder.md: 7.3 '디코드 호출' 행의 '프레임별 오류는 건너뛰고 계속합니다'를 구체화. 거부된 샘플은 표시 슬롯 하나를 넘기고 lost_samples 로 집계하며, 앱이 'is damaged' 경고를 한 번 냅니다. openh264 0.9.8 은 입력 타임스탬프를 받지 않아(decoder.rs 의 decode_with_options) 프레임과 샘플을 직접 대응시킬 수 없습니다. 실측: bitflip.mp4 는 패스당 215 샘플을 잃고 정지 구간이 9.409~16.617초이며, 손상 뒤 프레임은 2장 이릅니다.
- (리뷰 수정) v2-video-decoder.md: 7.3 '워커 스레드' 행에 추가. 워커가 패닉해도 Drop 가드가 종료를 알리고 사유 'the decoder panicked' 를 남겨 앱의 '경고 1회 후 미표시' 경로가 탑니다.
- (리뷰 수정) v2-video-decoder.md: 7.3 '트레이트' 행 갱신. VideoDecoder 는 info, next_time_us, next_frame, rewind, seek(기본 구현), lost_samples(기본 0) 입니다. NullDecoder 는 삭제됐고 피처를 끄면 open 이 Disabled 를 돌려줍니다. VideoInfo::frame_rate 도 삭제됐습니다.
- (리뷰 수정) v2-video-decoder.md: 9절 한계에 추가. stss 가 없는 파일은 첫 샘플만 시작점으로 봅니다. SPS 스케일링 리스트의 delta_scale 이 -128..=127 을 벗어나면 SequenceInfo::parse 가 None 을 돌려줍니다(디버그 빌드 오버플로 패닉 수정).

### 구현 뒤 실측과 검증(리뷰어, 2026-10-11)

- 릴리스 앱 빌드: cargo build --release -p rbms-player 성공(50.6초, 바이너리 23.5 MB)
- 의존성: 추가된 직접 의존은 rbms-video 의 openh264 0.9.8 과 re_mp4 0.5.1 둘뿐이고 둘 다 optional. cargo search 기준 둘 다 최신. 전이 의존은 openh264-sys2, wide, bytemuck, (빌드) cc, find-msvc-tools, jobserver, libc, shlex, nasm-rs, log, walkdir, same-file, 그리고 re_mp4 쪽 byteorder, bytes, num-rational, num-bigint, num-integer, num-traits, autocfg, serde, serde_core, serde_derive, serde_json, itoa, memchr, zmij, thiserror, thiserror-impl, proc-macro2, quote, syn, unicode-ident
- 라이선스(cargo tree --format '{p} | {l}'): openh264 와 openh264-sys2 는 BSD-2-Clause. re_mp4, bytes, zmij 는 MIT. byteorder, memchr 는 Unlicense OR MIT. same-file, walkdir 는 Unlicense/MIT. bytemuck, wide 는 Zlib OR Apache-2.0 OR MIT. unicode-ident 는 (MIT OR Apache-2.0) AND Unicode-3.0. 나머지는 전부 MIT OR Apache-2.0. 모두 GPL-3.0-or-later 와 호환되고 카피레프트 충돌이나 비자유 라이선스는 없음
- 디코드(직접 실행, RBMS_SKIN_PACK=ModernChic, cargo test --release -p rbms-video every_movie_of_an_external_pack): sample 600, sample2 601, BGmovie01 921, BGmovie02 750, #default 2564, cyber 4229, NOSTALGIC 7415, travel 3604 프레임. 8개 모두 거부 샘플 0, 표시 시각 단조 증가, 통과. 자체 측정 바이너리로도 같은 프레임 수와 오류 0 을 확인했고 next_time_us 예고와 실제 시각 불일치 0건
- 릴리스 프레임당 디코드+색 변환 시간(같은 테스트, 300프레임): 1080p 30fps 는 평균 5.7~7.7 ms, 최대 12.3~16.0 ms(예산 33.3 ms). 720p 30fps 는 평균 2.5~3.3 ms, 최대 5.4~6.8 ms. 720p 60fps(NOSTALGIC)는 평균 2.4 ms, 최대 5.7 ms(예산 16.7 ms). 다른 작업이 돌던 때의 전체 길이 측정에서도 1080p 평균 8.3~10.9 ms, 최대 26.6 ms 로 예산 안
- 실시간 추종(VideoPlayer 를 4 ms 간격으로 실시간 구동): sample2(1080p30) 25초에 750 중 749프레임 표시, 지연 중앙값 2.9 ms·최대 8.7 ms, 20.03초의 반복 경계를 넘는 동안 표시 간격 최대 33,334 us. NOSTALGIC(720p60) 12초에 720 중 719프레임, 간격 최대 16,667 us. cyber(28.989fps) 8초에 231 중 231프레임. 역행 0건. 30fps 와 60fps 모두 실시간으로 따라감
- 견고성(실제 시도, sample.mp4 변형 711건을 시드 3개로 반복해 약 2,100건): 임의 위치 절단 30건, 최상위·중첩 박스 길이 0/1/7/8/0x7fffffff/0xffffffff, 64비트 길이, 표 entry_count 0/1/0xffffff/0x7fffffff/0xffffffff, stsz 모순 6종, timescale 0/1/0xffffffff, hdlr 를 soun 으로(영상 트랙 없음), 엔트리를 hvc1·mp4v 로(H.264 아님), avc1 과 avcC 비트·바이트 훼손 300건, 박스별 임의 바이트 훼손, mdat 비트 반전 1~65,536개, mdat 전체 난수·0·0xff, 0바이트 파일, PNG·텍스트·RIFF·0 채움·0xff 채움을 .mp4 로. 릴리스 빌드에서 패닉 0건, 전부 수 초 안에 반환, 스레드 1→1, RSS 증가는 약 30 MB 이내
- moov 가 앞에 있는 sample2.mp4 절단: 20, 33, 100, 5000, 11247 바이트는 박스 길이 오류. 11248~6,000,000 바이트는 'the file ends before its samples do'. 끝에서 186바이트만 자른 것은 601프레임 전부 디코드
- 매우 큰 해상도 SPS: 8192x8192 로 선언한 SPS 는 SequenceInfo 가 받아들이지만(FRAME_EDGE_LIMIT 8192) h264.rs 64~67행이 디코드된 프레임이 선언보다 작으면 버퍼를 늘리기 전에 오류를 냄. 선언만으로는 RGBA 버퍼가 할당되지 않음(코드 판독). 2^32 급 선언은 None
- 스레드·자원: 재생기 1개에 스레드 1→2, drop 뒤 1. 1080p 재생기 4개 동시에 스레드 1→5, RSS 311 MB, drop 뒤 1(ps -M 으로 OS 스레드 수를 직접 셈). 시계를 1.5초 세우면 디코드 0프레임에 CPU 0 ms. 디코더 되감기 400회에서 RSS 59~62 MB 로 일정. 1080p 25초 재생 뒤 RSS 106 MB, 720p 는 36~40 MB 로 영상 길이와 무관
- 수명 경로(코드 판독): 화면 이탈, 팩 교체, RELOAD 는 모두 SkinScreens::let_go(skin_screen.rs 723, 791, 931행)를 거치고 let_go 가 텍스처 해제 전에 MoviePlayers::let_go 로 재생기를 drop 해 join 함. 재컴파일은 adopt 가 이전 항목을 대체. 앱의 스레드 종료 테스트는 OS 스레드 수가 아니라 스크립트 디코더의 열린 개수(open_on)를 세지만, 디코더 drop 은 워커 안에서만 일어나고 VideoPlayer::drop 이 join 하므로 검증으로 유효

### 화면·화소 대조(리뷰어)

- 맞는 것 - 색: macOS 시스템 디코더(AVFoundation, 허용 오차 0 의 프레임 추출)로 8개 파일 15개 시점의 기준 프레임을 뽑아 rbms-video 출력과 화소 대조했습니다. 채널별 평균 절대차는 R 0.34~1.75, G 0.13~0.84, B 0.24~2.91(8비트)이고 프레임 평균 RGB 는 1 이내로 일치합니다. 색 행렬과 범위(전부 BT.709 제한 범위로 해석)가 맞습니다.
- 맞는 것 - 방향과 채널 순서: 같은 대조에서 상하 반전으로 가정하면 차이가 0.5~93.7, R/B 를 바꾸면 1.3~99.8 로 대부분 한두 자릿수 커집니다. 차이가 작은 경우는 좌우·상하 대칭에 가까운 어두운 장면입니다. 반전과 채널 뒤바뀜은 없습니다.
- 맞는 것 - 표시 순서와 드리프트: 8개 파일 모두 ctts 가 있는 High 프로파일(B 프레임)입니다. 기준 프레임과 가장 잘 맞는 우리 프레임 양옆으로 차이가 단조롭게 커져 재정렬 뒤 표시 순서가 맞습니다. 29.97fps 는 19.95초와 118초, 28.989fps 는 60초와 140초, 60fps 는 120초에서도 같은 프레임이라 누적 드리프트가 없습니다. 표시 간격은 전 구간 균일하고(예: 33,366~33,367 us) 마지막 프레임 뒤 간격도 한 프레임이라 반복 경계가 이어집니다.
- 맞는 것 - 영역: Read 로 연 review/render/decide_movie-3000.png 는 화면 전체 배경에 HUD 영상이 깔리고, 시스템 디코더의 2.9696초 프레임과 나침반 눈금(NW, N), 원형 화살표, 우측 사각 블록 위치가 같습니다. musicselect_movie-3000.png 는 선곡 UI 뒤 배경 전체에 주황 회로 영상이 있습니다. play7_nobga-8000.png 와 review/app/play-movie-play-09000ms.png 는 영상이 BGA 사각형 안에만 그려집니다.
- 맞는 것 - 프레임 변화: decide_movie-1500 은 눈금이 'NW N NE', -3000 은 'NW N' 입니다. 표시 프레임 시각은 렌더 단위에서 decide 0.000/1.468/2.970초, musicselect 0.000/1.500/3.000초, play7_nobga 0.000/6.000/8.000초였습니다. 앱 하니스에서는 decide-movie 0.968/1.969/2.970초(헤드리스와 GPU 동일), select-movie 3.0/6.0초, play-movie 1.733/4.0/7.5/13.5초였습니다. play7-fullcombo 는 91.4초로 영상 길이 85.47초를 넘겨 둘째 패스가 그려졌습니다.
- 맞는 것 - GPU 업로드: decide-movie 의 GPU 캡처와 헤드리스 캡처는 영상만 보이는 y 0~199 구간에서 화소가 완전히 같습니다. 그 아래 약 107만 화소 차이(최대 채널 차 96)는 웨이브 7A 의 decide-fnt-1500ms 헤드리스 대 GPU 차이(1,066,999화소, 같은 영역)와 같은 기존 차이입니다.
- 맞는 것 - 기존 캡처 불변: 렌더 단위 캡처에서 동영상과 무관한 50장 중 49장이 웨이브 7A final 과 바이트 단위로 같습니다. result-750.png 한 장은 1화소(채널 차 2)가 다른데, 릴리스에서 영상 피처를 끈 캡처와 켠 캡처가 바이트 동일이고 디버그에서 영상 피처를 끈 캡처는 7A 와 동일하므로 원인은 빌드 프로파일(디버그 대 릴리스)입니다. 앱 하니스 캡처는 60장이 7A 와 바이트 동일합니다.
- 틀린 것(영상 변경과 무관하게 달라진 것): result-*, course-result-* 16장은 228x26 영역만 다르고 내용은 날짜 글자(2026/10/10 → 2026/10/11)입니다. pack-select.png 는 헤더 줄무늬와 움직이는 점 등 10,940화소가 다른데, 같은 트리를 두 번 돌린 결과끼리도 39,978화소(GPU 는 16,445화소)가 달라 실행 간 비결정입니다.
- 틀린 것(의도된 변화): play-sample, play5/7/10/14, play7-*, pack-play7 등 기존 플레이 캡처 12장은 BGA 영역이 7A 와 다릅니다. BGA 없는 곡에서 팩의 범용 영상이 이제 그려지기 때문이고 구현 보고에 적힌 내용과 일치합니다.
- 관찰 - 고정 오프셋: 편집 목록(edts)이 없는 #default.mp4 와 NOSTALGIC.mp4 는 시스템 디코더보다 한두 프레임 이르게 표시됩니다(#default 는 8.0초와 13.5초에서 우리 쪽 7.967초, 13.467초 프레임이 기준과 일치. NOSTALGIC 은 약 21 ms). 가장 이른 표시 시각을 0 으로 정규화하는 설계 때문이고 시간이 지나도 일정해 결함으로 분류하지 않았습니다. edts 가 있는 6개는 정확히 같은 프레임입니다.
- 확인 못 한 것: 창에 띄운 실제 앱에서 연속 재생이 눈으로 매끄러운지는 보지 못했습니다(정지 캡처와 표시 프레임 시각 수열로만 판단). Linux, Windows, nasm 없는 x86_64 빌드와 그 디코드 속도는 확인하지 못했습니다. 창 최소화·가림 때 프레임 루프가 멈춰 장면 시계가 건너뛰는지도 확인하지 못했습니다. 색은 채널 평균 수준으로만 대조했고 시스템 디코더와 화소 단위로 같지는 않습니다(크로마 업샘플링 차이).


- 조사일: 2026-10-10
- 범위: 문서 조사만 수행했습니다. 실험 빌드와 실제 디코드 실행은 하지 않았습니다(저장소·Cargo 레지스트리 무변경).
- 결론: 1순위 `openh264`(소스 동봉 정적 빌드) + `re_mp4` 디먹서, 2순위 FFmpeg 정적 링크(`ffmpeg-next`).
- 남은 검증: 대상 mp4 8개의 실제 디코드 성공 여부는 문서상 지원 범위 대조까지만 했고, 실행 검증은 W7-2 첫 단계에서 필요합니다.

## 1. 프로젝트 제약 (직접 확인한 사실)

| 항목 | 확인한 내용 | 근거 파일 |
| --- | --- | --- |
| 배포 형태 | "It is a single native binary with no JVM and no runtime dependencies." | `README.md` 11행 |
| 라이선스 | `license = "GPL-3.0-or-later"` | `Cargo.toml` |
| 툴체인 | `rust-version = "1.95"`, `edition = "2024"`, `rust-toolchain.toml` 채널 1.95.0 | `Cargo.toml`, `rust-toolchain.toml` |
| lint | `unsafe_code = "deny"` (워크스페이스 전체) | `Cargo.toml` |
| CI | `ubuntu-latest`, `macos-latest`, `windows-latest` 에서 `cargo build --workspace`, `cargo test --workspace`. Linux 만 apt 설치 단계가 있음. lint 잡은 `clippy --all-features` | `.github/workflows/ci.yml` |
| 릴리스 | macOS 유니버설(`aarch64-apple-darwin` + `x86_64-apple-darwin` 를 `lipo`), Windows x64. Linux 릴리스 산출물은 없음 | `.github/workflows/release.yml` |
| 이미 쓰는 네이티브 빌드 | `rusqlite`(bundled), `mlua`(vendored) 가 `cc` 로 C 소스를 컴파일. `Cargo.lock` 에 `cc 1.6.0` | `Cargo.toml`, `crates/rbms-skin/Cargo.toml`, `Cargo.lock` |
| 오디오 디코더 | `symphonia 0.5`(잠금 0.5.5), `isomp4` 피처 사용 | `crates/rbms-audio/Cargo.toml`, `Cargo.lock` |
| 그래픽 | `wgpu 29.0.3`(잠금 29.0.4), `image 0.25.10` | `apps/rbms-player/Cargo.toml` |
| 프레임 업로드 통로 | `Renderer::register_texture(key, rgba, width, height)` 가 같은 key 재등록 시 픽셀을 교체하며, 주석이 동영상 배경 갱신 용도를 명시 | `crates/rbms-render/src/lib.rs` 326~340행 |
| 기존 스레드 패턴 | `std::thread::spawn` + `std::sync::mpsc` | `apps/rbms-player/src/library.rs`, `dialog.rs` |
| 계획 문서 | W7-1 디코더 선정, W7-2 "디코더 트레이트와 구현, 반복 재생, 프레임 업로드", 위험 항목 "기능 플래그 뒤에 둔다" | `docs/plan/2026-10-09-lua-skin-compat.md` 156, 376~377, 429행 |
| 레퍼런스 동작 | beatoraja 는 `SkinSourceMovie` 가 `FFmpegProcessor.play(time, true)` 로 반복 재생하고, 반복 시 `grabber.restart()` 를 호출. FFmpeg 는 javacv jar 로 동봉 | `beatoraja/src/.../skin/SkinSourceMovie.java`, `video/FFmpegProcessor.java`, `build.xml` |

## 2. 대상 동영상 8개 실측

방법: 이 기기에 `ffprobe`, `ffmpeg`, `mediainfo`, `MP4Box` 가 없어 파이썬 표준 라이브러리로 mp4 박스(`ftyp`, `mvhd`, `tkhd`, `mdhd`, `hdlr`, `stsd`/`avcC`, `stts`, `ctts`, `stss`, `stsz`)를 읽고 `avcC` 안의 SPS·PPS 비트를 직접 파싱했습니다. 파일은 읽기만 했습니다.

| 파일 | 코덱·프로파일 | 레벨 | 해상도 | fps | 길이 | 프레임 | 키프레임 | 비트레이트 | 오디오 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `Decide/bg/movie/sample.mp4` | H.264 High(100) | 4.0 | 1920x1080 | 29.97 | 20.02초 | 600 | 3 | 2452 kbps | 없음(tmcd 타임코드 트랙 1개) |
| `Decide/bg/movie/sample2.mp4` | H.264 High(100) | 4.0 | 1920x1080 | 30 | 20.03초 | 601 | 7 | 4995 kbps | AAC 2ch 48 kHz |
| `Select/bg/movie/BGmovie01.mp4` | H.264 High(100) | 4.2 | 1920x1080 | 30 | 30.70초 | 921 | 31 | 5524 kbps | 없음 |
| `Select/bg/movie/BGmovie02.mp4` | H.264 High(100) | 4.2 | 1920x1080 | 30 | 25.00초 | 750 | 26 | 7989 kbps | 없음 |
| `Play/parts/common/BGA/movie/#default.mp4` | H.264 High(100) | 3.1 | 1280x720 | 30 | 85.47초 | 2564 | 33 | 1475 kbps | AAC 2ch 48 kHz |
| `Play/parts/common/BGA/movie/cyber.mp4` | H.264 High(100) | 3.1 | 1280x720 | 28.989 | 145.88초 | 4229 | 60 | 2465 kbps | 없음 |
| `Play/parts/common/BGA/movie/NOSTALGIC.mp4` | H.264 High(100) | 3.2 | 1280x720 | 60 | 123.58초 | 7415 | 44 | 2998 kbps | AAC 2ch 48 kHz |
| `Play/parts/common/BGA/movie/travel.mp4` | H.264 High(100) | 3.1 | 1280x720 | 29.97 | 120.25초 | 3604 | 26 | 1574 kbps | 없음 |

8개 공통 특성과 디코더 요구 사항:

| 특성 | 값 | 디코더에 요구되는 것 |
| --- | --- | --- |
| 비트 깊이·크로마 | 8비트 4:2:0 | 기본 |
| 주사 방식 | 전부 `frame_mbs_only_flag = 1` (프로그레시브) | 인터레이스 지원 불필요 |
| 엔트로피 코딩 | 전부 CABAC | CABAC 필수 |
| B 프레임 | 전부 `ctts` 보유(합성 시각 재정렬 있음) | B 슬라이스 디코드와 표시 순서 재정렬 필수 |
| 8x8 변환 | `sample.mp4`, `cyber.mp4`, `travel.mp4` 는 `transform_8x8_mode_flag = 1`, 나머지 5개는 0 | High 프로파일 8x8 CABAC 잔차 필수(3개) |
| 가중 예측 | `weighted_pred_flag` 1 이 6개, `weighted_bipred_idc` 2(암시적)가 6개 | 명시적·암시적 가중 예측 |
| 참조 프레임 | 2~5 | 다중 참조 |
| 슬라이스 그룹 | 1 (FMO 없음) | 불필요 |
| 최대 샘플 크기 | 44~107 KB | OpenH264 의 1 MB 제한 이내 |
| 컨테이너 | 비분할 mp4(`mvex` 없음), `moov` 가 끝에 있는 파일 5개, 편집 목록(`elst`) 있음, 회전 행렬 없음 | 샘플 테이블 기반 디먹스. 스트리밍 불필요 |
| 프레임 타이밍 | 전부 고정 프레임률(`stts` 항목 1개) | 단순 |
| 1080p 코딩 크기 | 1920x1088, 하단 8줄 crop | crop 반영 필요 |

Baseline 전용 디코더는 8개 모두를 디코드하지 못합니다. High 프로파일·CABAC·B 프레임이 필수입니다.

## 3. 후보 평가 요약

| 후보 | 최신 버전·시기 | 라이선스 | GPL-3.0-or-later 호환 | 최종 사용자 런타임 의존 | 대상 8개 | 판정 |
| --- | --- | --- | --- | --- | --- | --- |
| OpenH264 (`openh264` + `openh264-sys2`, `source` 피처) | 0.9.8, 2026-08-08 | BSD-2-Clause(래퍼·코어 모두) | 호환 | 없음(정적 링크) | 문서상 지원 범위 안. 실행 미검증 | 1순위 |
| FFmpeg 정적 링크 (`ffmpeg-next` + `ffmpeg-sys-next`) | 9.0.0, 2026-08-05 / sys 9.0.1, 2026-10-09 | 래퍼 WTFPL, FFmpeg 는 LGPL-2.1+ 기본 | 호환 | 정적이면 없음, 시스템 링크면 필요 | 지원 | 2순위 |
| FFmpeg (`rsmpeg` + `rusty_ffmpeg`) | 0.18.0+ffmpeg.8.0, 2025-08-24 | MIT + FFmpeg LGPL | 호환 | 위와 같음 | 지원 | 대안(FFmpeg 를 직접 빌드해 주지 않음) |
| FFmpeg (`ffmpeg-the-third`) | 6.0.0+ffmpeg-9.0, 2026-08-09 | WTFPL + FFmpeg LGPL | 호환 | 위와 같음 | 지원 | 대안 |
| FFmpeg CLI (`ffmpeg-sidecar`) | 2.6.0, 2026-09-30 | MIT | 호환 | ffmpeg 실행 파일 필요(런타임 다운로드 또는 동봉) | 지원 | 제외 |
| OS 기본 디코더 직접 바인딩 (`objc2-video-toolbox`/`objc2-av-foundation`, `windows`, Linux 대안) | 0.3.2(2025-10-04), `windows` 0.62 | Zlib/Apache/MIT, MIT/Apache | 호환 | macOS·Windows 없음, Linux 는 드라이버 의존 | macOS·Windows 는 지원 범위 안, Linux 미확정 | 제외(구현량, `unsafe`, Linux 공백) |
| `gpu-video` (Vulkan Video, 구 `vk-video`) | 0.4.0, 2026-05-12 | MIT | 호환 | GPU·드라이버 의존 | 미확인 | 제외(macOS 미지원) |
| `waterkit-codec` | 0.1.4, 2026-09-17 | Apache/MIT 로 추정(미확인) | 미확인 | OS 디코더 의존 | 미확인 | 제외(초기 단계) |
| GStreamer (`gstreamer`) | 0.25.4, 2026-09-21 | MIT OR Apache-2.0 (GStreamer 본체 LGPL) | 호환 | GStreamer 런타임과 플러그인 설치 필요 | 플러그인 있으면 지원 | 제외 |
| 순수 Rust `rust_h264` | 0.4.0, 2026-04-20 | MIT OR Apache-2.0 | 호환 | 없음 | README 상 Baseline/Main/High 지원 주장. 미검증 | 실험 후보 |
| 순수 Rust `rusty_h264-decoder` | 0.16.0, 2026-09-07 | BSD-2-Clause | 호환 | 없음 | README 가 "High-profile 8x8 CABAC residual" 미지원 명시. 3개 불가 | 제외(현재) |

## 4. 후보별 상세

### 4.1 OpenH264 계열

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 크레이트 | `openh264` 0.9.8 (2026-08-08), 누적 142만·최근 91만 다운로드, MSRV 1.85, 0.9.5 는 yank | https://crates.io/api/v1/crates/openh264 |
| 피처 | `default = ["source"]`. `source`: "Uses the bundled OpenH264 source; works out of the box (default)." `libloading`: "You'll need to provide Cisco's prebuilt library." | https://docs.rs/openh264/latest/openh264/ |
| 빌드 방식 | `build.rs` 가 `cc` 로 동봉 C++ 소스(`upstream/codec/{common,processing,decoder,encoder}`)를 정적 라이브러리로 컴파일. 네트워크 접근 없음. bindgen·libclang 불필요 | https://raw.githubusercontent.com/ralfbiedert/openh264-rs/master/openh264-sys2/build.rs |
| nasm | x86·x86_64 SIMD 에만 사용하는 선택 사항. 없으면 "Failed to compile NASM files, not using any assembly." 를 출력하고 C++ 만으로 계속. `OPENH264_NO_ASM` 으로 끌 수 있음. ARM·AArch64 는 Windows 외 대상에서 `.S`(NEON)를 `cc` 로 조립하므로 nasm 불필요 | 같은 build.rs |
| 플랫폼 | 컴파일·단위 테스트: x86_64 Windows(MSVC·GNU), Linux, macOS, i686. 컴파일만: aarch64 macOS·Linux·Windows, armv7. WASM 미지원 | https://docs.rs/openh264/latest/openh264/ |
| 동봉 OpenH264 | `upstream/VERSION` 이 커밋 `4a2615fac570c6ca1ed4f157b9fdab9466edfd80`("decoder: bounds-check CAVLC I_PCM copy before reading bitstream (#3968)")을 가리킴. 대응 릴리스 태그는 미확인. `RELEASES` 의 최신 항목은 v2.6.0 | https://github.com/cisco/openh264/commit/4a2615fac570c6ca1ed4f157b9fdab9466edfd80 , https://raw.githubusercontent.com/cisco/openh264/master/RELEASES |
| 디코더 지원 범위 | 업스트림 README 는 "Constrained Baseline Profile up to Level 5.2" 만 적고 있으나, `RELEASES` 는 v1.5.0 "Decoder support of 'Constrained High Profile' of H.264", v2.0.0 "B-frame decoding support for Main and High Profile with two test cases", v2.2.0·v2.3.0 B 프레임 수정을 기록 | https://raw.githubusercontent.com/cisco/openh264/master/README.md , RELEASES |
| 소스상 수용 조건 | `ParseSps` 가 `PRO_BASELINE`, `PRO_MAIN`, `PRO_EXTENDED`, `PRO_HIGH`, 스케일러블 프로파일을 수용. `chroma_format_idc <= 1`, 8비트만, `frame_mbs_only_flag = 0`(인터레이스)은 `ERR_INFO_UNSUPPORTED_MBAFF`, 레벨은 5.2 까지, FMO 는 map type 0·1 만. `decoder_core.cpp` 에 `CreateImplicitWeightTable`(`uiWeightedBipredIdc == 2`)와 `ParsePredWeightedTable` 존재 | https://raw.githubusercontent.com/cisco/openh264/master/codec/decoder/core/src/au_parser.cpp , https://raw.githubusercontent.com/cisco/openh264/master/codec/decoder/core/src/decoder_core.cpp |
| 알려진 제한 | "Decoder errors when compressed frame size exceeds 1MB", "Single thread for all slices". 다중 스레드 디코드는 v2.1.0 에서 실험적 추가이고 크레이트의 `DecoderConfig::num_threads` 는 `unsafe` 이며 "will probably segfault the decoder" | 업스트림 README, https://docs.rs/openh264/latest/openh264/decoder/struct.DecoderConfig.html |
| 대상 8개 대조 | 전부 High(100)·8비트 4:2:0·프로그레시브·CABAC·슬라이스 그룹 1·레벨 4.2 이하·최대 샘플 107 KB 로 위 수용 조건 안. 실제 디코드 실행은 미검증 | 2절 실측 |
| B 프레임 mp4 사례 | 이슈 #50: B 프레임이 있는 mp4(Big Buck Bunny)에서 오류가 났고 원인은 디코드마다 암시적으로 flush 하던 동작. 2024-12-12 에 명시적 flush 방식으로 수정 완료. 현재 mp4 예제는 `Flush::NoFlush` 로 디코드하고 끝에서 `flush_remaining()` 호출 | https://github.com/ralfbiedert/openh264-rs/issues/50 , https://raw.githubusercontent.com/ralfbiedert/openh264-rs/master/openh264/examples/mp4/main.rs |
| API 안전성 | `Decoder::new`, `decode`, `flush_remaining`, `DecodedYUV::write_rgba8`, `YUVSource` 의 `y`/`u`/`v`/`strides` 는 safe. R-BMS 쪽 `unsafe` 불필요 | https://docs.rs/openh264/latest/openh264/decoder/struct.Decoder.html , https://docs.rs/openh264/latest/openh264/decoder/struct.DecodedYUV.html |
| 입력 형식 | Annex B(시작 코드). mp4 샘플은 길이 접두(AVCC)이므로 변환과 SPS·PPS 선행 삽입이 필요. 크레이트 API 가 아니라 예제 디렉터리의 `Mp4BitstreamConverter` 에 구현돼 있어 직접 작성해야 함 | 위 예제 |
| 성능(크레이트 README, Ryzen 9 7950X3D, 단일 스레드) | 1920x1080 한 프레임 디코드 5.70 ms(asm 없음), 2.80 ms(nasm 있음). YUV→RGB 1920x1080 1.36 ms(`target-cpu=native`) | https://docs.rs/openh264/latest/openh264/ |
| 유지보수 | 2026 년에 0.9.2~0.9.8 릴리스. 보안 갱신은 업스트림 소스를 당겨오는 방식 | crates.io API, README FAQ |
| 코덱 범위 | H.264 만. mpg(MPEG-1/2), wmv, avi(다양한 코덱), webm(VP8/VP9) BGA 는 덮지 못함 | - |

### 4.2 mp4 디먹서

| 크레이트 | 버전·시기 | 라이선스 | 영상 샘플 제공 | 비고 | 근거 |
| --- | --- | --- | --- | --- | --- |
| `re_mp4` | 0.5.1, 2026-07-08, MSRV 1.92, 누적 228만 | MIT | 제공. `Sample` 에 `offset`, `size`, `decode_timestamp`, `composition_timestamp`, `duration`, `is_sync`. `Track::raw_codec_config` | `mp4` 크레이트의 포크(rerun). 의존: byteorder, bytes, num-rational, serde, serde_json, thiserror. `Mp4::read<R: Read + Seek>(reader, size)` 가 파일 전체를 메모리에 올리는지는 미확인. `raw_codec_config` 가 `avcC` 원본인지도 미확인 | https://crates.io/api/v1/crates/re_mp4 , https://docs.rs/re_mp4/latest/re_mp4/struct.Sample.html , https://docs.rs/re_mp4/latest/re_mp4/struct.Mp4.html |
| `mp4` | 0.14.0, 2023-08-01, 누적 1420만 | MIT | 제공(`Mp4Reader::read_sample`) | openh264-rs 예제가 사용. 3년간 릴리스 없음, 열린 PR 26개 | https://crates.io/api/v1/crates/mp4 , https://github.com/alfg/mp4-rust |
| `mp4parse` | 0.17.0, 2023-05-29 | MPL-2.0 | 샘플 단위 인덱스 제공 여부 미확인 | Mozilla. 3년간 릴리스 없음 | https://crates.io/api/v1/crates/mp4parse |
| `symphonia` isomp4 | 잠금 0.5.5, 최신 0.6.1(2026-08-13) | MPL-2.0 | 제공하지 않음 | `stsd.rs` 가 오디오 샘플 엔트리만 파싱하고 나머지는 `SampleEntry::Other`("Potentially video, subtitles, etc."). 0.5.5 와 master 모두 같음 | https://raw.githubusercontent.com/pdeljanov/Symphonia/v0.5.5/symphonia-format-isomp4/src/atoms/stsd.rs , https://raw.githubusercontent.com/pdeljanov/Symphonia/master/symphonia-format-isomp4/src/atoms/stsd.rs |
| 자체 최소 파서 | - | - | - | 대상 파일은 비분할 mp4 라 `stsd`/`avcC`, `stsz`, `stsc`, `stco`/`co64`, `stts`, `ctts`, `stss` 만 읽으면 됨. 의존성은 줄지만 직접 유지해야 함 | 2절 실측 |

이미 쓰는 symphonia 로는 영상 샘플을 얻을 수 없으므로 디먹서 의존성이 하나 더 필요합니다.

### 4.3 FFmpeg 바인딩 계열

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| `ffmpeg-next` | 9.0.0 (2026-08-05), 누적 776만, WTFPL, safe 래퍼 | https://crates.io/api/v1/crates/ffmpeg-next |
| `ffmpeg-sys-next` | 9.0.1 (2026-10-09), 누적 806만, WTFPL | https://crates.io/api/v1/crates/ffmpeg-sys-next |
| 시스템 링크 절차 | Linux: `apt install -y clang libavcodec-dev libavformat-dev libavutil-dev pkg-config`. macOS: `brew install pkg-config ffmpeg`. Windows MSVC: LLVM 설치 후 `LIBCLANG_PATH`, FFmpeg 를 설치해 `FFMPEG_DIR` 지정, 실행 시 FFmpeg `bin` 을 `PATH` 에 추가 | https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building |
| 탐색 순서(`build` 미사용) | `FFMPEG_DIR` → vcpkg(MSVC 한정) → pkg-config. bindgen 이 매 빌드 실행되므로 libclang 필요 | https://raw.githubusercontent.com/zmwangx/rust-ffmpeg-sys/master/build.rs |
| `build` 피처 | 빌드 중 `https://github.com/FFmpeg/FFmpeg` 의 `release/<major>.<minor>` 브랜치를 얕게 clone(네트워크 필요) → `configure` → `make -j` → `make install`. 항상 `--enable-static --disable-shared`. Windows 는 `sh` 가 PATH 에 있어야 하고 MSVC 대상은 `--toolchain=msvc` | 같은 build.rs |
| `build` 의 CPU 종속 | 네이티브 빌드는 기본으로 `-march=native -mtune=native` 를 넘김. `build-portable` 피처(9.0.0 부터)나 `FFMPEG_MARCH` 로 끌 수 있음. 끄지 않으면 CI 러너 CPU 전용 바이너리가 배포됨 | 같은 build.rs, https://docs.rs/crate/ez-ffmpeg/latest/source/docs/INSTALL.md |
| Windows MSVC 소스 빌드 | FFmpeg 공식 문서: MSYS2 와 NASM 필요, `msys2_shell.cmd -use-full-path` 에서 `./configure --toolchain=msvc` 후 `make`. GitHub 러너에는 MSYS2 가 `C:\msys64` 에 있으나 PATH 밖, NASM·make 는 목록에 없음 | https://ffmpeg.org/platform.html , https://raw.githubusercontent.com/actions/runner-images/main/images/windows/Windows2025-Readme.md |
| Windows 정적 대안 | `vcpkg install ffmpeg:x64-windows-static-md` + `static` 피처. 링크 시 누락 시스템 라이브러리(`bcrypt`, `mfplat` 등)를 앱 `build.rs` 에서 추가해야 할 수 있음. 러너에 vcpkg 사전 설치(`C:\vcpkg`) | https://docs.rs/crate/ez-ffmpeg/latest/source/docs/INSTALL.md , 러너 이미지 README |
| macOS 유니버설 | arm64 와 x86_64 용 FFmpeg 를 각각 정적으로 빌드해야 함. build.rs 가 교차 컴파일 플래그(`--enable-cross-compile`, `--arch`, `--target-os`)를 넣지만 실제 동작은 미확인 | 같은 build.rs |
| 빌드 시간 | 공개된 수치를 찾지 못함(미확인). FFmpeg 전체를 구성 축소 없이 빌드하므로 OpenH264 보다 크게 늘 것으로 보이며, 포럼에 "drastically slows building" 이라는 사용자 언급이 있음 | https://users.rust-lang.org/t/how-can-i-configure-or-use-correctly-ffmpeg-next-with-windows/134829 |
| 라이선스 | "FFmpeg is licensed under the GNU Lesser General Public License (LGPL) version 2.1 or later." `--enable-gpl` 을 쓰면 GPL. H.264 등 내장 디코더는 `--enable-gpl` 없이 사용 가능. FFmpeg 의 LGPL 준수 점검표는 동적 링크를 권장("Use dynamic linking") | https://ffmpeg.org/legal.html |
| 특허 | "Does FFmpeg use patented algorithms?" → "We do not know, we are not lawyers so we are not qualified to answer this." | https://ffmpeg.org/legal.html |
| 코덱 범위 | H.264 전 프로파일과 MPEG-1/2, WMV, VP8/VP9 등 BGA 형식 전반을 한 디코더로 처리. beatoraja 와 같은 기반 | - |
| `rsmpeg` / `rusty_ffmpeg` | FFmpeg 를 직접 빌드하지 않음. 미리 빌드한 라이브러리를 `FFMPEG_PKG_CONFIG_PATH`, `FFMPEG_LIBS_DIR`, vcpkg 로 연결. MIT, MSRV 1.81 | https://crates.io/api/v1/crates/rsmpeg , https://raw.githubusercontent.com/CCExtractor/rusty_ffmpeg/master/README.md |
| `ffmpeg-the-third` | 6.0.0+ffmpeg-9.0 (2026-08-09), WTFPL, MSRV 1.80, `build`·`static` 피처 있음 | https://crates.io/api/v1/crates/ffmpeg-the-third |
| `video-rs` | 0.12.0 (2026-09-09), MIT OR Apache-2.0, FFmpeg 기반 고수준 래퍼. 빌드 요구는 FFmpeg 계열과 같음 | https://crates.io/api/v1/crates/video-rs |
| `ffmpeg-sidecar` | ffmpeg 실행 파일을 자식 프로세스로 띄워 파이프로 프레임 수신. `auto_download()` 로 런타임 다운로드(압축 100 MB 미만) | https://raw.githubusercontent.com/nathanbabcock/ffmpeg-sidecar/main/README.md |
| 참고 사례 | Rerun 뷰어는 H.264 를 별도 설치한 FFmpeg 실행 파일로 디코드하며 "intentionally does not come bundled with FFmpeg to avoid licensing issues" | https://rerun.io/docs/reference/video |

### 4.4 운영체제 기본 디코더

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 3개 OS 를 한 API 로 덮는 성숙한 크레이트 | 찾지 못함. `waterkit-codec` 0.1.4 가 VideoToolbox·Media Foundation·VA-API 를 묶지만 0.1 단계이고 H.264 디코드의 OS 별 지원·디먹서·라이선스가 README 에 명시돼 있지 않음. `moq-video` 는 설계 문서가 인코드 중심 | https://docs.rs/crate/waterkit-codec/latest/source/README.md , https://docs.rs/crate/moq-video/latest/source/DESIGN-native-codecs.md |
| macOS | `objc2-video-toolbox` 0.3.2, `objc2-av-foundation` 0.3.2 (2025-10-04). 프레임워크 바인딩이며 고수준 디코더 API 가 아님 | https://crates.io/api/v1/crates/objc2-video-toolbox , https://crates.io/api/v1/crates/objc2-av-foundation |
| Windows | Media Foundation H.264 디코더: "Baseline, Main, and High profiles, up to level 5.1", 입력은 Annex B. mp4 는 MPEG-4 File Source 가 처리. `windows` 0.62.2 가 이미 `Cargo.lock` 에 있음 | https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-decoder , https://learn.microsoft.com/en-us/windows/win32/medfound/supported-media-formats-in-media-foundation |
| Windows N 에디션 | "media-related technologies" 가 빠져 있고 Media Feature Pack 으로 복원 | https://support.microsoft.com/en-us/topic/media-feature-pack-list-for-windows-n-editions-c1c6fffa-d052-8338-7a79-a4bb980a700a |
| Linux | OS 표준 디코더 없음. `cros-codecs` 0.0.6 (2025-06-18, VA-API, BSD-3-Clause)나 `gpu-video` 는 GPU·드라이버에 의존 | https://crates.io/api/v1/crates/cros-codecs |
| `gpu-video` | 0.4.0 (2026-05-12), MIT, Vulkan Video, `wgpu ^29` 연동, H.264 디코드 지원. Linux·Windows 만 언급하고 macOS 없음. 디먹서 없음 | https://docs.rs/gpu-video/latest/gpu_video/ |
| R-BMS 관점의 비용 | FFI 호출에 `unsafe` 가 필요해 `unsafe_code = "deny"` 와 충돌. 백엔드를 OS 별로 3벌 작성하고 Linux 는 소프트웨어 대안을 따로 둬야 함 | `Cargo.toml` |

### 4.5 GStreamer

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 크레이트 | `gstreamer` 0.25.4 (2026-09-21), MIT OR Apache-2.0, MSRV 1.92 | https://crates.io/api/v1/crates/gstreamer |
| 설치 요구 | "you need to have at least GStreamer 1.14 and gst-plugins-base 1.14 installed". Linux 는 배포판 패키지, macOS 는 공식 `.pkg`(runtime·devel) 두 개, Windows 는 `.msi`(runtime·devel) 두 개와 PATH 설정. 정적 링크·동봉 안내 없음 | https://gstreamer.pages.freedesktop.org/gstreamer-rs/stable/latest/docs/gstreamer/ |
| 판정 | 최종 사용자가 GStreamer 런타임과 H.264 플러그인을 설치해야 하므로 README 의 "no runtime dependencies" 와 맞지 않음. CI 3개 OS 모두 설치 단계 필요 | - |

### 4.6 순수 Rust H.264 디코더

| 크레이트 | 버전·시기 | 지원 범위(README) | 대상 8개 | 근거 |
| --- | --- | --- | --- | --- |
| `rust_h264` | 0.4.0, 2026-04-20(첫 공개 2026-04-05, 버전 4개), MIT OR Apache-2.0, 누적 3.4만 | "Supports Baseline, Main, and High profiles (8-bit 4:2:0)", CAVLC·CABAC, B 프레임, 가중 예측, MBAFF. "174 byte-exact tests against FFmpeg, verified up to 1080p". Annex B·AVCC 입력, `OrderedDecoder` 가 표시 순서 출력 | 주장 범위로는 가능. 8x8 변환 언급 없음, 성능 수치·`unsafe` 사용 미확인, 4월 이후 릴리스 없음 | https://crates.io/api/v1/crates/rust_h264 , https://github.com/roticv/rust_h264 |
| `rusty_h264-decoder` | 0.16.0, 2026-09-07(첫 공개 2026-06-27, 약 10주간 버전 17개), BSD-2-Clause, MSRV 1.80 | Baseline + B 슬라이스, High 는 CAVLC 한정, CABAC 는 Main. "Not yet: CABAC `I_PCM` (errors gracefully), High-profile 8×8 CABAC residual, and full JVT-suite conformance." ffmpeg 대비 약 1.6~1.8배 느림(자체 측정) | `sample.mp4`, `cyber.mp4`, `travel.mp4`(CABAC + 8x8 변환)는 미지원 범위. 나머지 5개는 미확인 | https://docs.rs/crate/rusty_h264-decoder/latest/source/README.md , https://docs.rs/crate/rusty_h264/latest/source/README.md |
| `oxideav-h264` | 0.1.8, 2026-09-01 | "no codec functionality yet" | 불가 | https://crates.io/api/v1/crates?q=h264%20decoder&per_page=40&sort=recent-downloads |
| `h264-decoder` | 0.2.4, 2024-08-11 | 미확인 | 미확인 | 같은 검색 결과 |

순수 Rust 디코더는 2026 년에 등장했으나 모두 공개 6개월 이내이고 독립 검증 자료가 없습니다. 지금 주 디코더로 쓰기에는 근거가 부족하고, 트레이트 뒤의 교체 후보로 두는 것이 적절합니다.

## 5. 라이선스와 특허 (사실만)

| 항목 | 사실 | 근거 |
| --- | --- | --- |
| BSD-2-Clause(FreeBSD) | FSF: "It is a lax, permissive non-copyleft free software license, compatible with the GNU GPL." | https://www.gnu.org/licenses/license-list.en.html |
| MIT(Expat) | FSF: GPL 호환 | 같은 페이지 |
| LGPL-2.1 | FSF: "It is compatible with GPLv2 and GPLv3." | 같은 페이지 |
| MPL-2.0 | FSF: 3.3절로 간접 호환 | 같은 페이지 |
| WTFPL v2 | FSF: GPL 호환(권장하지는 않음) | 같은 페이지 |
| Apache-2.0 | FSF: GPLv3 와 호환 | 같은 페이지 |
| BSD-2 조건 | OpenH264 를 정적 링크한 바이너리를 배포하면 저작권 고지를 배포물에 포함해야 함. 현재 `release.yml` 은 `LICENSE` 와 `README.md` 만 동봉 | BSD-2-Clause 조문, `.github/workflows/release.yml` |
| OpenH264 소스 빌드와 특허료 | Cisco FAQ: 소스에서 빌드한 경우 Cisco 가 MPEG LA 비용을 부담하는가 → "No. Cisco is only covering the licensing fees for its own binary module" | https://www.openh264.org/faq.html |
| Cisco 바이너리 조건 | 바이너리가 최종 사용자 기기로 별도 다운로드돼야 하고 사전에 제3자 소프트웨어에 통합되면 안 됨, 사용자가 켜고 끌 수 있어야 함, "OpenH264 Video Codec provided by Cisco Systems, Inc." 고지 표시 | https://www.openh264.org/BINARY_LICENSE.txt |
| 크레이트의 대응 경로 | `libloading` 피처로 Cisco 사전 빌드 라이브러리를 런타임에 로드 가능 | https://docs.rs/openh264/latest/openh264/ |
| AVC 라이선스 요약(Via LA) | 디코더·인코더 단위 기준 연 1~100,000 단위 $0.00, 100,001~5,000,000 단위 $0.20. 특허 만료일은 페이지에 없음 | https://via-la.com/licensing-programs/avc-h-264/ |
| FFmpeg | 특허 사용 여부에 대해 "We do not know" | https://ffmpeg.org/legal.html |

법률 판단은 이 보고서의 범위가 아닙니다.

## 6. CI·릴리스 영향

| 항목 | OpenH264 (`source`) | FFmpeg 정적 링크 |
| --- | --- | --- |
| 필수 추가 설치 | 없음. C++ 컴파일러만 필요(3개 러너 모두 보유) | libclang(bindgen) 필수. Windows: vcpkg 정적 트리플렛 또는 MSYS2 + NASM + make. Linux·macOS: nasm/yasm, make, 네트워크(`build` 피처) |
| 선택 추가 설치 | x86_64 SIMD 를 켜려면 nasm. 러너 이미지 README(Ubuntu 24.04, Windows 2025, macOS 15 arm64)에 NASM 이 없음 | - |
| macOS 유니버설 | arm64 는 NEON `.S` 를 `cc` 가 조립, x86_64 슬라이스는 nasm 이 없으면 C++ 경로 | 아키텍처별 FFmpeg 정적 빌드 2회 |
| 빌드 시간 증가 | 미확인(실측 필요). 동봉 C++ 소스 4개 묶음 컴파일. `Swatinem/rust-cache` 가 `target` 을 캐시 | 미확인(실측 필요). FFmpeg 전체 빌드 |
| 바이너리 휴대성 | 문제 없음 | `build-portable` 을 켜지 않으면 `-march=native` |
| lint 잡 | `clippy --all-features` 이므로 피처를 켠 상태로 항상 빌드됨 | 같음. lint 잡에도 FFmpeg 준비 단계가 필요 |
| 근거 | https://raw.githubusercontent.com/ralfbiedert/openh264-rs/master/openh264-sys2/build.rs , https://raw.githubusercontent.com/actions/runner-images/main/images/ubuntu/Ubuntu2404-Readme.md , https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-15-arm64-Readme.md | https://raw.githubusercontent.com/zmwangx/rust-ffmpeg-sys/master/build.rs , https://ffmpeg.org/platform.html |

## 7. 추천

### 7.1 1순위: `openh264` 0.9.8 (`source` 피처) + `re_mp4` 0.5.1

1. 런타임 의존이 없습니다. 정적 링크라 README 의 "no runtime dependencies" 를 그대로 유지합니다.
2. CI 변경이 가장 작습니다. `rusqlite`(bundled), `mlua`(vendored)와 같은 `cc` 빌드이고 필수 설치 단계가 없습니다.
3. 대상 8개의 특성(High, CABAC, B 프레임, 8x8 변환, 암시적 가중 예측, 프로그레시브, 레벨 4.2 이하)이 업스트림 릴리스 노트와 소스의 수용 조건 안에 있습니다.
4. API 가 safe 라 `unsafe_code = "deny"` 를 건드리지 않습니다.
5. BSD-2-Clause 로 GPL-3.0-or-later 와 호환됩니다.
6. 한계: H.264 전용이라 mpg·wmv·avi·webm BGA 는 덮지 못하고, 인터레이스 H.264 는 디코드하지 못합니다. 단일 스레드 디코드입니다.

### 7.2 2순위: FFmpeg 정적 링크 (`ffmpeg-next` 9.x)

1. BGA 동영상 형식 전반을 같은 디코더로 덮어야 한다고 결정될 때의 선택입니다.
2. 비용: 3개 OS 각각의 정적 빌드 절차(Windows vcpkg 정적 트리플렛, macOS 아키텍처별 2회, libclang), CI 시간 증가, lint 잡 준비 단계.
3. 시스템 링크(동적)는 최종 사용자에게 FFmpeg 설치를 요구하므로 채택하지 않습니다.

### 7.3 구현 구조 제안 (W7-2)

| 주제 | 제안 |
| --- | --- |
| 크레이트 배치 | 새 크레이트 `crates/rbms-video` 에 디코더 트레이트와 구현을 둡니다. C++ 빌드를 한 크레이트에 격리하고 `rbms-render` 는 프레임 업로드 통로만 유지합니다 |
| 트레이트 | `VideoDecoder`: 영상 정보(폭, 높이, 프레임 간격, 길이) 조회, 다음 프레임 디코드(표시 시각 + RGBA 버퍼), 처음으로 되감기. 구현체 교체(OpenH264 → 순수 Rust 또는 FFmpeg)가 이 경계에서 끝나게 합니다 |
| cargo 피처 | `rbms-video` 에 `openh264` 피처(기본 켬), `rbms-player` 가 이를 전달. 끄면 프레임을 내지 않는 구현만 남고, 계획 문서의 "프레임이 없으면 그리지 않는다" 계약으로 동작합니다. CI 가 깨질 때 피처만 꺼서 우회할 수 있습니다 |
| 디먹스 | `re_mp4` 로 첫 H.264 트랙의 샘플 표(오프셋, 크기, 합성 시각, 키프레임 여부)를 읽고, 샘플은 파일에서 오프셋으로 읽습니다. 오디오·타임코드 트랙은 무시합니다 |
| 비트스트림 변환 | 샘플의 길이 접두 NAL 을 시작 코드로 바꾸고, 첫 샘플과 되감기 직후에 `avcC` 의 SPS·PPS 를 앞에 붙입니다(openh264-rs mp4 예제의 `Mp4BitstreamConverter` 와 같은 역할) |
| 디코드 호출 | `Flush::NoFlush` 로 디코드하고 스트림 끝에서 `flush_remaining()` 을 호출합니다(이슈 #50 의 수정 방식). 프레임별 오류는 건너뛰고 계속합니다 |
| 워커 스레드 | 재생 중인 동영상마다 `std::thread` 하나. 기존 코드와 같은 `std::sync::mpsc` 를 쓰되 용량 2~4 의 `sync_channel` 로 역압을 겁니다. 수신 측이 사라지면 워커가 끝납니다 |
| 프레임 큐 소비 | 렌더 스레드는 매 프레임 스킨 시각과 비교해 표시 시각이 지난 프레임을 꺼내고 가장 최신 것만 `Renderer::register_texture` 로 같은 key 에 올립니다. 큐가 비면 이전 텍스처를 유지하고 렌더를 막지 않습니다 |
| 반복 재생 | 워커가 마지막 샘플 뒤에 flush 하고 샘플 0(IDR)부터 다시 디코드하며 표시 시각에 길이를 누적합니다. 탐색(seek)이 필요 없습니다. beatoraja 의 `LOOP` 명령 → `grabber.restart()` 와 같은 동작입니다 |
| 타이머 재시작 | 장면 재진입이나 타이머 리셋 시 재시작 명령을 보내 큐를 비우고 샘플 0 부터 다시 시작합니다 |
| 버퍼 재사용 | 1080p RGBA 한 프레임이 약 8.3 MB 이므로 렌더 스레드가 쓴 버퍼를 반환 채널로 워커에 돌려줘 프레임마다 할당하지 않습니다. 큐 3개 기준 동영상당 약 25 MB |
| 색 변환 | 처음에는 `write_rgba8` 로 RGBA 를 만들어 기존 업로드 통로를 그대로 씁니다. 병목이 실측되면 Y·U·V 평면 업로드와 셰이더 변환으로 바꿉니다 |
| 첫 검증 | 구현 첫 단계에서 대상 8개를 끝까지 디코드해 프레임 수(600, 601, 921, 750, 2564, 4229, 7415, 3604)와 오류 0 을 확인합니다. 하나라도 실패하면 2순위로 전환을 논의합니다 |
| 배포물 고지 | OpenH264 의 BSD-2 고지를 릴리스 압축 파일에 포함합니다 |

## 8. 사용자 결정이 필요한 사항

1. H.264 특허·배포: 소스 빌드 OpenH264 는 Cisco 의 특허료 부담 대상이 아닙니다. 소스 빌드 동봉을 그대로 진행할지, Cisco 바이너리 런타임 로드(`libloading`, 런타임 다운로드와 켜고 끄기 UI 필요)로 갈지, OS 디코더로 갈지 정해야 합니다.
2. CI 에 nasm 설치 단계를 추가할지: x86_64 에서 디코드 속도가 약 2배 차이 납니다(크레이트 README 수치). 추가하지 않으면 Windows·Linux·macOS x86_64 슬라이스는 C++ 경로로 동작합니다.
3. CI 빌드 시간 증가 허용 범위: 수치는 미확인이라 W7-2 에서 실측한 뒤 판단이 필요합니다.
4. 동영상 피처의 기본값: 기본 켬으로 배포할지, 릴리스 빌드에서만 켤지.
5. BGA 동영상 범위: mpg·wmv·avi·webm 까지 같은 디코더로 덮어야 한다면 FFmpeg(2순위)가 필요합니다. 스킨 mp4 는 OpenH264, BGA 는 나중에 별도 결정으로 나눌지 정해야 합니다.
6. 디먹서: `re_mp4` 의존성을 추가할지, 비분할 mp4 전용 최소 파서를 직접 둘지.
7. 인터레이스 H.264 mp4 미지원을 수용할지(대상 8개에는 없음).
8. Linux 릴리스 산출물 계획: 현재 `release.yml` 은 macOS·Windows 만 만듭니다. FFmpeg 를 택할 경우 Linux 정적 빌드 절차가 추가로 필요합니다.
9. 배포물의 제3자 라이선스 고지 파일 추가 여부.

## 9. 조사 한계

| 항목 | 상태 |
| --- | --- |
| 대상 8개의 OpenH264 실제 디코드 | 미실행(실험 빌드 금지). 문서·소스상 수용 조건 대조만 수행 |
| OpenH264·FFmpeg 추가 시 CI 빌드 시간 | 미확인 |
| `re_mp4` 의 `Mp4::read` 메모리 사용 방식, `raw_codec_config` 반환 형식 | 미확인 |
| `ffmpeg-sys-next` `build` 피처의 Windows MSVC·macOS 교차 빌드 실제 성공 여부 | 미확인 |
| `rust_h264` 의 성능·8x8 변환 지원·`unsafe` 사용 | 미확인 |
| `openh264` 동봉 소스의 정확한 릴리스 태그 | 미확인(커밋 해시만 확인) |
| 근거 수집 방식 | crates.io API, docs.rs, 저장소 원문을 WebFetch 로 읽었고, 요약 모델을 거친 인용이므로 구현 전 핵심 수치(버전, 피처명)는 `cargo` 가 해석하는 실제 메타데이터로 다시 확인이 필요합니다 |
