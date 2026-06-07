# 2026-06-07 — 첫 실행 / 온보딩 / 스캔 UI (.dmg 배포 대비)

## 배경
`.dmg` 더블클릭 배포로 가면 터미널이 없다. 기존엔 (1) 인자·기억된 폴더·리플레이가 전부 없으면 `usage` 출력 후 `std::process::exit(1)` → .dmg 첫 실행은 그냥 꺼지고 폴더 고를 화면에 도달조차 못 함(닭-달걀), (2) 초기 라이브러리 스캔이 `App::new`에서 **동기 블로킹** → 큰 라이브러리는 창도 뜨기 전에 수 초 freeze(진행 UI 없음). 사용자 지적: "곡 읽고·폴더 읽고 하는 초기 실행 UI가 있어야".

사용자 결정(AskUserQuestion): 첫 실행 = **빈 곡선택 + '폴더 추가' 안내 CTA**, 범위 = **풀 온보딩**.

## 구현
- **무인자 진입**(`main()`): `None => remembered.unwrap_or_default()` — 빈 문자열(bare launch)로 GUI 진입(종료 안 함). `App::new`가 빈 경로를 bare launch로 처리.
- **초기 스캔 백그라운드화**(`App::new`): 차트/리플레이 → Play; **폴더 없음(첫 실행) → Stage::Select(빈, 온보딩)**; 그 외 → **백그라운드 스레드**가 `scan_folders`+`fetch_and_match` → `ScanOutcome`를 `scan_rx`로 전달, 시작은 Stage::Loading. `songs`/`table_names`/`table_levels`는 빈 채로 시작해 `apply_scan`이 채움(`frame()`이 매 프레임 `scan_rx` 폴링). → 큰 라이브러리도 창 즉시 표시 + SCANNING 애니메이션.
- **스캔 진행 곡 수**: `scan_folders`/`scan_folder`에 `&AtomicUsize` 카운터 추가(곡당 증가), App `scan_count: Arc<AtomicUsize>`를 SCANNING 화면에 "{n} charts found"로 표시. `rescan_all_folders`도 리셋·전달.
- **빈 상태 온보딩 CTA**: `SelectView.empty_hint: Option<(&'static str,&'static str)>`(렌더가 rows 0일 때 중앙 표시). `build_select_view`가 상태별 계산 — 검색 중 → "NO RESULTS", 폴더 없음 → "WELCOME TO rbms / Add your music folder — press O or click FOLDERS below", 그 외 → "NO CHARTS". 헤드리스 렌더로 시각 확인.
- **Esc-중-스캔 수정**: Stage::Loading에서 Esc는 `to_select_or_exit`(songs 비어 종료) 대신 **Select로 취소 복귀**(반쯤 로드된 audio/player 정리). 초기 스캔 중 Esc로 앱이 꺼지던 회귀 방지.

## 검증
- 빌드 무경고, `cargo test --workspace` **889 통과**.
- 헤드리스 렌더(`CpuCanvas`)로 첫 실행 화면 PNG 확인: "WELCOME TO rbms" + 폴더추가 안내 + 하단 FOLDERS(O) 버튼.
- **집중 적대 리뷰 1회**: 1건 MEDIUM(Esc-중-스캔 종료 회귀) 반영. 나머지(디렉토리 인자 스캔 유지·bad path·`--table` 레이스 없음·이중 적용 없음·빈 테이블 폴백·empty_hint borrow·직접 차트/리플레이 실행 무영향) 정상 확인.

## 잔여
- **`.dmg`/`.app` 패키징 + 무인자 진입 + macOS 공증(notarize)**은 빌드/릴리스 측(`release.yml`) 작업으로 별도(사용자가 .dmg+공증 선택). 이번 작업은 그 전제가 되는 **런타임 첫 실행 UX**.
- 초기 스캔은 백그라운드지만 차트당 헤더 파싱은 여전히 비용 — 매우 큰 라이브러리는 SCANNING이 길 수 있음(진행은 곡 수로 가시화됨).
