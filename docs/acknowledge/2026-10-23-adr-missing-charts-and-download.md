# ADR: 표에 있고 라이브러리에 없는 곡 (흐린 막대, 안내 모달, 직접 zip 받기, 브라우저 열기, 다운로드 폴더)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 15·16 은 사용자 결정, `zip` 9.0.0·`typed-path` 0.12.3 고정 이유와 받기·풀기 상한 등 세부는 메인 결정이고 사용자 확인 2026-10-24(미확인 16 지금대로)) · 결정일 2026-10-23 · 결정자 사용자(결정 15·16) + 메인(세부) · 커밋 `9e39262`(`rbms-download` 크레이트. 제목은 마이그레이션 0012 이지만 웨이브 1 의 변경 전체가 들어 있다), `a6dc73c`(Windows 에서 압축 항목 이름 충돌 수리), `0ff2077`(없는 곡 막대), `6998d95`(다운로드 폴더·설정), `8d0a62a`(안내 모달·브라우저 열기), `5a0eacf`(받기·풀기·부분 스캔)
> 관계: [표 페이지 ADR](2026-10-14-adr-table-page-no-source-notice.md) 이 남기기로 한 곡 다운로드 링크(`url`·`url_diff`)를 이 기능이 쓴다. 폴더 구조는 [폴더 스택 ADR](2026-10-23-adr-select-folder-stack.md). 곡을 우리 쪽에 올리는 미러(결정 17)는 이 ADR 의 범위 밖이다.

## 배경

- 요구 5(아래 원문): 표에 있는데 곡이 없으면 그 곡을 흐리게(opacity) 보이고, 그 곡을 플레이하려 하면 모달로 곡을 내려받겠느냐고 안내하고, 곡 다운로드 폴더를 지정해 내려받게 한다.
- 표의 곡마다 `url`(곡 페이지)과 `url_diff`(차트 파일 링크)가 있다. 원본은 없는 곡 막대(`SongBar` 의 `existsSong` false)를 두고 누르면 두 링크를 곧장 브라우저에 넘긴다(`EventFactory.open_download_site`). Stella 표 2,350곡 가운데 `url_diff` 가 직접 zip 인 곡이 1,213곡(Stella 업로드 636 + 경로가 `.zip` 으로 끝나는 577)이다(조사 보고 기준).
- 표의 DL 은 차트 파일만(차분)일 때가 많아 소리와 그림은 곡 URL 에서 따로 받아 같은 폴더에 두어야 한다.

## 결정

### 1) 없는 곡 막대와 설정 (결정 16)

1. 표 레벨 폴더 안에, 라이브러리에 없는 곡이 가진 곡과 함께 흐린 막대(`SongBar` exists false)로 보인다. 대상은 표의 md5 가 라이브러리에 없는 항목이고, 한 레벨 안에서 같은 md5 는 한 번만 나온다. sha256 만 있는 항목은 가진 곡도 없는 곡도 아니라 보이지 않는다(기존 한계).
   - 기본 화면은 글자를 흐리게 그리고 램프 띠와 레벨 칸을 비우며, 세부 패널에 제목·아티스트·`NOT IN THE LIBRARY - ENTER FOR THE DOWNLOAD` 를 보인다. 스킨에는 `exists: false` 인 곡 막대(원본 막대 그림 4번)로 전달되고, 커서가 올라가면 곡 제목·아티스트가 표 항목에서 온다(`missing_chart_meta`).
   - 정체는 가진 곡과 같은 md5 라서 받은 뒤 그 줄이 가진 곡이 되어도 커서가 머문다.
   - 검색에는 나오지 않는다(검색은 라이브러리만 본다). 즐겨찾기 필터, 모드·레벨 범위 필터가 켜지면 빠지고, 클리어 필터는 기록이 없는 차트가 통과하는 값일 때만 남는다(기록·레벨·모드가 없는 차트로 본다). 정렬은 제목 순이면 가진 곡 사이에 제목으로 섞이고, 정렬 없음이면 가진 곡 뒤에 표 순서로, 그 밖이면 가진 곡 뒤에 제목 순으로 온다.
   - 레벨 막대의 `(n)` 과 루트·묶음 합계, 폴더 램프 분포는 가진 곡만 센다(M8).
2. 설정 `library.show_missing_charts`(기본 켬)와 행 `MISSING CHARTS`(SETTINGS 의 SELECT 탭 SONG LIST 구역). 켜짐이 기본인 것은 요구가 직접 요청했기 때문이고, "저장값이 없으면 화소가 변하지 않는다"는 이전 원칙의 예외다(M7). 이 값은 기호라 계정 설정 묶음과 함께 동기화된다.
3. **다운로드 폴더**: `library.download_folder`(기본 없음)와 행 `DOWNLOAD FOLDER`(같은 탭의 DOWNLOADS 구역, 폴더 고르기, 0 키로 없음). 곡 폴더 목록에 자동으로 들어가 곡 폴더와 같이 스캔된다(`LibraryOptions::scan_roots`, 곡 폴더와 같은 경로면 한 번). 곡 폴더 화면에는 곡 폴더들 뒤, `+ ADD FOLDER` 앞에 `DOWNLOADS  <경로>` 행이 보이고 거기서는 지울 수 없다(안내 `the download folder is changed in SETTINGS > SELECT`). 이 값은 기기 고유라 설정 묶음 올리기에서 빼고 내려받을 때도 이 기기의 값을 둔다. 기본값은 두지 않고(M9) 첫 받기에서 폴더를 고르게 한다(고르기 창 제목 `Select download folder`).

### 2) 안내 모달 (결정 15, 영어 UI: M12)

- 없는 곡 막대에서 재생·오토·연습·리플레이 이벤트나 Enter 가 모달을 연다(원본은 모드와 상관없이 다운로드 사이트를 연다). 모달은 레코드 모달처럼 키(컨트롤러 키 포함)와 마우스를 먼저 받는다. 열 때 디스크와 네트워크를 읽지 않는다.
- 문구: 제목 `NOT IN THE LIBRARY`, 안내 두 줄 `THE TABLE'S DL IS THE CHART FILE ONLY (A DIFFERENCE CHART).` / `THE SONG ITSELF - ITS SOUNDS AND IMAGES - HAS TO BE GOT FROM THE SONG URL AND PUT BESIDE THE CHART.`, 키 안내 `LEFT RIGHT CHOOSE   ENTER PRESS   ESC CLOSE`. 곡 제목·아티스트·`표 이름 레벨 제목`도 보인다.
- 버튼 줄: `GET CHART`(링크가 직접 zip 이고 다른 받기가 없을 때만 켜짐, 받는 중에는 `CANCEL` 로 바뀌고 풀기가 끝나면 꺼짐), `OPEN SONG URL`, `OPEN CHART URL`(각각 브라우저에 넘겨도 되는 주소일 때만 켜짐), `CLOSE`. 꺼진 버튼은 흐리게 그리고 커서가 건너뛴다. 받는 중 줄은 `DOWNLOADING 1.2 MB / 3.4 MB`(크기를 모르면 받은 양만), `EXTRACTING`, `SCANNING`, 끝나면 결과 문구, 다른 곡을 받는 중이면 `ANOTHER DOWNLOAD IS RUNNING`.
- 사용자 요구의 "확인으로 받겠냐" 는 예·아니오 확인창이 아니라 이 버튼 줄이다(`GET CHART` 를 누르면 받는다).

### 3) 어떤 링크를 앱이 받나 (결정 15)

- 경로(쿼리 제외)만 본다(`classify_link`): http·https 가 아니면 `Unusable`, 호스트 `stellabms.xyz` 의 `/upload/…` 이거나 경로가 대소문자 무시 `.zip` 으로 끝나면 `DirectZip`, 그 밖은 `Browser`. `page.html?file=a.zip` 은 페이지다.
- rar·7z·lzh, 단일 `.bms` 같은 차트 파일 링크(M11), HTML·클라우드 페이지는 앱이 받지 않고 브라우저로 넘긴다. 이 링크에서는 `GET CHART` 가 꺼진다.

### 4) 받기·풀기와 안전 장치 (새 크레이트 `rbms-download`)

| 구분 | 값 |
| --- | --- |
| 받기 상한(M13) | 512 MiB, 무응답 30초(클라이언트 timeout, 읽기 대기마다), 전체 15분(64 KiB 조각마다 남은 시간과 취소를 확인), 리다이렉트 10회, 최종 주소도 http·https |
| 받은 파일 검사 | 처음 4바이트가 zip 머리(`PK\x03\x04`, 빈 보관 `PK\x05\x06`)가 아니면 거절(HTML 응답 포함). 알린 크기(`Content-Length`)가 상한을 넘으면 읽기 전에 거절. 받은 바이트로도 센다. 실패하면 파일을 남기지 않는다 |
| 풀기 상한(M13) | 항목 10,000개, 항목 하나 1 GiB, 전체 2 GiB. 쓴 바이트로 센다(머리에 적힌 크기를 믿지 않는다). 목록 읽기에도 항목당 1 KiB + 1 MiB 의 읽기 예산을 둔다 |
| 이름·경로 | 항목 이름은 UTF-8 이거나, 아니면 Shift_JIS 로 읽는다(일본어 압축본의 `#WAV` 이름과 맞추려고). 조각마다 빈 이름·`.`·`..`·제어 문자·Windows 금지 문자(`<>:"/\|?*`)·끝의 공백·점·Windows 장치 이름이면 그 항목을 건너뛴다. `/` 와 `\` 둘 다 구분자다. 심볼릭 링크 항목은 건너뛴다 |
| 쓰는 파일 | 차트(`bms bme bml pms bmson`), 소리, 그림, 영상, `txt` 확장자만 쓴다. 실행 파일·스크립트는 쓰지 않는다 |
| 덮어쓰기 | 없다(`create_new`). 이미 있는 이름은 쓰기 전에 확인해 건너뛴다(Windows 에서 폴더가 막은 이름이 접근 거부로 나오는 차이 때문, `a6dc73c`). 풀린 폴더는 `settle` 로 다운로드 폴더 안에 `제목 [md5 앞 8자]` 로 옮기고, 있으면 ` (2)`, ` (3)`… |
| 폴더 이름 | 제목에서 금지 문자를 `_` 로, 앞뒤 공백·점 제거, 64자·200바이트 자르기, 끝에 ` [md5 앞 8자]`. 비거나 장치 이름이면 md5 앞 8자만 |
| 차트가 없는 압축 | 풀린 폴더를 지운다(M20). 압축에 차트는 있지만 이 md5 가 없으면 폴더는 두고 알린다(M19) |

- 작업은 한 번에 하나다. 다운로드 폴더 안의 `.rbms-partial` 을 비우고 `archive.zip` 을 받아 `unpacked` 로 푼 뒤 최종 폴더로 옮기며, 어떤 경우에도 일이 끝나면 `.rbms-partial` 은 지워진다.
- 끝난 일은 그 폴더만 부분 스캔한다(`ScanRequest::partial`). 부분 스캔은 곡 DB 의 파서 버전을 기록하지 않아 다른 폴더에 필요한 전체 재스캔이 끝난 것으로 취급되지 않는다. 모든 스캔이 `LIBRARY_WRITE` 잠금을 지나므로 전체 스캔과 부분 스캔이 곡 DB 를 동시에 쓰지 않는다. 스캔이 시작할 때의 라이브러리 세대가 끝날 때와 다르면 읽은 것을 받아들이지 않고 같은 폴더를 다시 스캔한다. 스캔 뒤 표를 다시 맞추고, 선곡이 열린 폴더와 커서를 지킨 채 목록을 받아들인다. 플레이 중에 끝난 일은 선곡으로 돌아왔을 때 받아들인다(플레이 중 프레임 스레드에서 라이브러리를 바꾸지 않는다).
- 결과 알림: `downloaded: 제목 (n chart(s))`, `the archive had n chart(s) but not this one`, `download failed: 이유`, `download cancelled`. 시작할 수 없는 이유는 `another download is running`, `the chart's link is not a zip archive`, `no download folder is chosen`.

### 5) 브라우저 열기 (결정 15)

- 주소는 `browsable_url` 을 통과한 것만 넘긴다: http·https 와 호스트가 있고 공백·제어 문자가 없고 2,048자 이하이며 정규화한 결과가 `http` 로 시작한다(옵션으로 읽히지 않게).
- 셸을 거치지 않고 열기 프로그램을 따로 실행한다: macOS `open <주소>`, Windows `rundll32 url.dll,FileProtocolHandler <주소>`(M10: `cmd /C start` 는 `&`·`|`·`^`·`%` 를 해석하는데 곡 URL 은 그런 글자가 많다), 그 밖은 `xdg-open <주소>`. 프레임 스레드가 기다리지 않도록 짧은 스레드가 실행하고 끝까지 기다려 프로세스를 남기지 않는다. 실패는 `could not open the browser: 이유` 로 알린다.

### 메인 결정(설계서 select-tree-download 8절, 사용자 확인 2026-10-24)

| id | 결정 | 이유 |
| --- | --- | --- |
| M7 | 없는 곡 막대 설정 행을 두고 기본 켬 | 요구가 직접 요청했다("요구로 확인"). 원본 `showNoSongExistingBar = true` |
| M8 | 레벨 폴더 수는 가진 곡 수 | 지금 화소·시험 유지 |
| M9 | 다운로드 폴더 기본값은 없음, 첫 받기 때 고르기 | 사용자 디스크 위치를 앱이 정하지 않는다. 요구 5 "폴더 지정과 함께" |
| M10 | Windows 는 `rundll32 url.dll,FileProtocolHandler` | 셸 해석 없음 |
| M11 | 단일 차트 링크(`.bms` 등 123곡)는 브라우저로 | 결정 15 "직접 zip 링크만" |
| M12 | 모달 문구는 영어 | 앱 UI 전체가 영어 |
| M13 | 받기·풀기 상한은 위 표의 값 | 차분 zip 은 수십 KB, 직접 zip 링크 일부는 곡 패키지 |
| M19·M20 | 압축에 그 md5 차트가 없으면 폴더를 남기고 알림 / 차트가 하나도 없으면 폴더를 지움 | 같은 곡의 다른 판일 수 있다 / 쓸모없는 파일 방지 |

### 6) 의존성: `zip` 9.0.0, `typed-path` 0.12.3 (메인 결정, 사용자 확인 2026-10-24)

- 사용자가 승인한 것은 "새 의존성 `zip`"이다(결정 15). 버전은 **crates.io 에 게시된 지 14일이 지난 판만 쓴다**는 규칙으로 정했다. 이 규칙은 이 ADR 을 쓰라고 한 작업 지시가 전한 것이고, 저장소의 규칙 문서에서 출처를 찾지 못했다.
- 2026-10-23 에 crates.io API 로 확인한 게시일: `zip` 9.0.0 은 2026-10-09 이고, 9.0.1 은 10-10, 9.0.5 는 10-17 이다. `typed-path` 0.12.3 은 2026-02-11 이고 0.12.4 와 0.13.0 은 2026-10-20 이다. `zip` 9.0.x 는 `typed-path ^0.12` 를 요구하므로, 14일이 지난 가장 새 조합이 `zip` 9.0.0 과 `typed-path` 0.12.3 이다. 설계서가 적은 9.0.5·0.12.4(게시 6일·3일)를 쓰지 않았다.
- 명세: `zip = { version = "9.0.0", default-features = false, features = ["deflate-flate2-zlib-rs"] }`(작업 공간). 기본 기능(aes·bzip2·lzma·zstd·ppmd·deflate64·time)은 쓰지 않는다. 라이선스 MIT, `typed-path` 는 MIT OR Apache-2.0 이고 `THIRD-PARTY-NOTICES.md` 에 올랐다. 새로 `Cargo.lock` 에 들어온 외부 크레이트는 이 둘뿐이다(`rbms-download` 는 작업 공간 안 크레이트).
- 고정은 `Cargo.lock`(추적, 결정 D12)이 한다. `Cargo.toml` 의 `"9.0.0"` 은 `^9.0.0` 이라 `cargo update` 가 더 새 9.0.x 로 올릴 수 있다. 올릴 때는 게시 14일이 지난 판인지 같은 방식으로 다시 확인한다.
- 같은 `zip` 을 `rbms-cli` 의 course-snapshot(공식 코스 zip 읽기)과 mirror-pack(곡 미러용 zip 쓰기)도 쓴다. 결정 15 의 승인 문구는 앱의 받기·풀기를 두고 한 것이라 쓰기까지 이른 사용은 사용자 확인 항목이다(설계 교차 검토 F7 D-1).

## 이유

- 요구 5 와 결정 15·16: 없는 곡을 보이고, 안내하고, 가능한 것만 앱이 받는다. 원본처럼 곧장 브라우저로 보내지 않는 것은 DL 이 차분이라는 사실을 먼저 말하기 위해서다.
- 직접 zip 만 받는 이유: rar·7z·HTML 은 해제 크레이트나 페이지 해석이 더 필요하고, 임의 사이트 페이지를 앱이 대신 읽으면 안전 장치가 늘어난다. 브라우저가 이미 하는 일은 브라우저에 맡긴다.
- 안전 장치가 많은 이유: 받는 것은 외부 사이트가 준 압축 파일이다. 압축 폭탄(상한), 경로 이탈(조각 검사), 덮어쓰기(`create_new`), 실행 파일(확장자 허용 목록), 무한 응답(총 시간·크기)을 각각 막는다. 보안 규칙(`security.md`)의 "검증되지 않은 값을 파일 경로에 흘리지 않는다"를 따른다.
- 부분 스캔은 큰 라이브러리를 매번 다시 읽지 않기 위함이고, 파서 버전을 기록하지 않는 것은 부분 스캔이 다른 폴더의 재파싱 필요를 지우지 않게 하려는 것이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 원본처럼 두 링크를 곧장 브라우저에 | DL 이 차분이라는 안내를 못 한다(결정 15) |
| rar·7z·lzh 해제, 단일 `.bms` 저장 | 결정 15 가 직접 zip 만 정했다 |
| 우리 서버 미러에서 받기 | 결정 17 은 준비만 한다(이 ADR 의 범위 밖) |
| 설정 폴더 옆에 다운로드 폴더 자동 생성 | 사용자 디스크 위치를 앱이 정하지 않는다(M9) |
| Windows `cmd /C start "" <주소>` | cmd 가 `&`·`%` 를 해석한다(M10) |
| 받기·풀기를 프레임 스레드에서 | 플레이 렉 기준 위반([렉 기준 ADR](2026-10-14-adr-play-hitch-criteria.md)) |
| `zip` 9.0.5 / `typed-path` 0.12.4 | 게시 14일이 지나지 않았다 |
| 막대 수에 없는 곡을 포함 `(가진/전체)` | 지금 화소·시험이 바뀐다(M8) |

## 영향과 관련 파일

- 새 크레이트 `crates/rbms-download`(`classify_link`, `browsable_url`, `download_to`/`download_with`, `extract_zip`, `settle`, `safe_folder_name`, 상한 상수), 앱 `apps/rbms-player/src/{download,open_url}.rs`, `stage/select/download_modal.rs`, `stage/select/{list,mod,events,skinned,overlay}.rs`, `crates/rbms-render/src/select.rs`(흐린 행·세부 패널), `crates/rbms-table/src/lib.rs`(`missing_levels_in`), `crates/rbms-library/src/scan.rs`(`ScanRequest::partial`, `stamps_parser_version`), `apps/rbms-player/src/library.rs`(`LIBRARY_WRITE`), `crates/rbms-config/src/{schema,settings}.rs`(설정 둘, `scan_roots`), `apps/rbms-player/src/{dialog,ir_sync}.rs`·`stage/{folders,settings,loading}.rs`, 계약 `contract/client-requests/settings-blob.json`(`download_folder: None,`·`show_missing_charts: true,` 가 설정 본문에 더해져 재생성).
- 테스트: `crates/rbms-download/src/tests/*`(상한·리다이렉트 10회와 11번째·HTML 응답·취소·경로 이탈·Shift_JIS 이름·Windows 장치 이름·이름 충돌), `stage/select/missing_tests.rs`, `stage/select/download_modal_tests.rs`, `download/tests.rs`(`a_finished_download_scans_only_its_folder_and_brings_the_bar_back`, `an_archive_without_this_chart_says_so_and_keeps_the_bar_missing`, `an_archive_without_any_chart_leaves_nothing`, `a_second_download_is_refused_while_one_runs`, `a_refresh_whose_library_changed_meanwhile_is_run_again`, `a_download_that_ends_during_play_is_taken_on_the_browser`), `loading.rs`(`a_rescan_walks_the_download_folder_too`). 앱의 시험은 외부 네트워크와 브라우저·폴더 고르기 창을 건드리지 않는다(손을 넣지 않은 시험은 실패하게 해 두었다). 크레이트의 받기 시험은 같은 기계의 루프백 서버만 쓴다.
- 설계서와 다른 점(구현과 다름, 설계서 `docs/plan/2026-10-23-select-tree-download.md` 3.6·3.8·3.10절): `zip` 9.0.5·`typed-path` 0.12.4 가 아니라 9.0.0·0.12.3 이다. 작업 폴더는 `.rbms-partial/{md5}/` 가 아니라 `.rbms-partial` 아래 고정 이름 `archive.zip`·`unpacked` 다(작업이 하나뿐이라서). `download_folder` 와 `show_missing_charts` 는 설정 파일에 늘 적힌다(설계는 `None` 미기록). 설정에서 다운로드 폴더를 바꾸면 선곡이 그 폴더를 부분 스캔하는 경로가 더해졌다.
- 들어감: 수리 커밋 `45d09fd` 가 셋을 고쳤다. (1) 받기 대기 정지: 다시 띄운 부분 스캔이 받은 차트를 기다리는 상태(`awaited`)를 잊어 받기가 막히던 것을 고쳤다(라이브러리 세대가 달라 다시 돌릴 때도 대기를 이어받는다). (2) 표 사본 덮어쓰기 경쟁: 부분 스캔이 표 세대(`table_generation`)까지 기록해 두고, 도는 사이 표 사본이 바뀌었으면 결과를 받아들이지 않고 다시 돈다. (3) 작업장 스캔 제외: 스캔이 `.rbms-partial`(`rbms_library::scan::DOWNLOAD_WORK_FOLDER`)이라는 이름의 폴더를 걸어 들어가지 않는다. 설계서 9절 3번이 적은 "풀기 중 전체 스캔이 반쯤 푼 차트를 잠시 DB 에 넣을 수 있다"는 위험이 (3)으로 해소됐다. 시험은 `download/tests.rs`·`scan/tests.rs`·`library/tests.rs` 에 더해졌고, 테스트가 시스템 브라우저 열기를 부르면 즉시 실패하게 했다(`open_url.rs`).
- 남은 위험·확인하지 못한 것: 차분만 받은 곡은 소리가 없어 키음 없이 재생될 수 있다(사용자가 곡 URL 에서 받은 파일을 같은 폴더에 넣어야 한다. 모달 문구로만 막는다). Windows `rundll32` 와 Linux `xdg-open` 의 실제 동작은 CI 에 브라우저가 없어 확인하지 못했다(실기 항목). 큰 압축을 푼 직후 스캔 중 선곡 커서 이동에서 곡 DB 읽기가 잠깐 바쁠 수 있다(테스트 부채 후보). 링크 분류의 곡 수(1,213 등)는 조사 보고의 관찰값이다. 실제 Stella 압축을 받아 본 실기는 하지 않았다.

## 날짜와 사용자 원문

2026-10-23.

- 요구 5(원문): "stellabms 에는 DL 도 지원하는데 곡 다운로드 폴더를 폴더 지정과함께 곡이 없는경우 해당 곡은 opacity 처리 + opacity 처리된 곡을 플레이하려하면 모달 + 확인으로 곡을 다운로드 받겠냐등의 안내문구를 적어서 다운로드 가능하게 만들어"
- 결정 회신(원문): "13 - 16 추천대로". 결정 15·16 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): 직접 zip 링크만 앱이 받아 다운로드 폴더에 풀고(새 의존성 `zip` 승인) rar·7z·HTML 은 브라우저로, 모달에 "차분만, 음원은 곡 URL" 안내 / 표 레벨 안 흐린 없는 곡 막대 + 설정의 전용 다운로드 폴더(곡 폴더 목록에 자동 포함).
- `zip` 9.0.0·`typed-path` 0.12.3 으로 고정한 것은 사용자에게 따로 묻지 않았다. 14일 규칙이 사용자의 지시인지와 그 원문은 저장소에서 확인하지 못했다(이 ADR 을 쓰라는 작업 지시가 전한 규칙을 따랐다).
