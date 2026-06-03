# 2026-05-31 (세션 C) — 백엔드 IR-슈퍼셋 설계 + GitHub Actions CI/CD

**문서 단계**(백엔드/FE 구현 X — 설계·CI 설정만). 원본/인터넷 대조 후 `docs/backend/`·`.github/workflows/`·`docs/ci-release.md` 작성.

## 1. 조사 (정본 대조)
- **beatoraja IR**(`src/bms/player/beatoraja/ir/`): `IRConnection`(register/login/rivals/tables/getPlayData·sendPlayData/course/URL) · `IRScoreData`(**판정 early/late 분리** epg/lpg…ems/lms·option비트필드·seed·gauge·assist·deviceType·judgeAlgorithm·rule·skin·avgjudge, EX=(epg+lpg)*2+egr+lgr) · `IRChartData/IRCourseData/IRTableData/IRAccount/ClearType(0-10)`. IR은 플러그인(리플렉션) — 실 HTTP 프로토콜은 외부.
- **LR2IR**(naktazdim/lr2irscraper 등): `getrankingxml.cgi?id=&songmd5=`(md5·XML·clear 1-5·name/id/notes/combo/pg/gr/minbp) · `search.cgi?mode=ranking&bmsmd5=` · 코스=차트해시 결합.
- 기존 `docs/reference/ir-api.md` + `crates/rbms-ir`. 백엔드 컨벤션 `~/.claude/convention/backend.md`(Bun/Hono/Drizzle/Zod/better-auth).

## 2. docs/backend (6문서)
- `README`(인덱스·스택·결정) · `PRD`(목표/FR-18까지·호환매트릭스·로드맵) · `api-spec`(전 엔드포인트) · `data-model`(Drizzle/MySQL) · `endpoint-tasks`(Route→Service→ServiceDb 체크) · `compatibility`(LR2IR/beatoraja 필드매핑+출처).
- 도메인: auth·players·charts(ranking md5·sha256)·scores(early/late)·courses·tables·**replays(µs)**·**settings 동기화**·**integrity**·**FE(web)**·system·LR2IR 어댑터.

## 3. 사용자 추가 요구 반영 (문서)
- **공평 평가**: 제출이 *플레이 전체 옵션*(judge_rate·total·autoplay·assist·hispeed·lift·cover·constant·offset·seed·gauge·random·rule·skin·lntype…) 보존 → 서버가 `ranked` 산출(autoplay·assist·judge폭·total·미상빌드 → unranked+flags).
- **µs 리플레이**: 입력 이벤트 `{t_us(µs),lane,press}` 무손실 → 재시뮬·핵분석(등간격·반응속도·동시성).
- **빌드 무결성**: 모든 제출에 `client_build_sha256`+platform → `client_build` allowlist(릴리스 CI 산출물 해시 등록)·`submission_audit` 로깅 → 변조 탐지.
- **FE 대비**: search·leaderboards·activity·recent·stats·리플레이뷰어 + envelope·CORS·세션쿠키(better-auth).

## 4. GitHub Actions
- `ci.yml`: **dev/prod** push·PR → ubuntu·macos·windows `build`+`test`(Linux ALSA/X11/GTK dev libs), fmt/clippy informational.
- `release.yml`: **prod push=patch 자동 bump** / 수동 bump(dispatch) / 태그(`v*`) → `Cargo.toml [workspace.package] version` perl 교체·커밋·태그 → **macOS 유니버설(lipo+ad-hoc codesign)**·**Windows x64** 빌드 → 아카이브+SHA-256 → `softprops/action-gh-release`.
- 검증: YAML 파싱 OK(ruby)·버전 bump perl이 워크스페이스 version만 교체 확인.

## 5. 라이선스 / 서명 (사용자 질의)
- **클라(rbms)=GPL-3.0 고정**(beatoraja GPL 포팅·파생, 클린룸 아님 → MIT 불가). **백엔드/FE=HTTP 별 작품이라 MIT 가능**(별 저장소). 폰트 Inter(OFL)·deps 호환.
- **서명**: 무서명 하드차단 아님(경고+우회). **macOS arm64는 ad-hoc 서명 없으면 실행 불가** → 워크플로 `codesign -s -` 추가(lipo 후). 공증(Apple)·Authenticode(Win)는 무경고용 P3.

## 6. 후속
- 백엔드 구현(Bun/Hono/Drizzle) · `rbms-ir` 슈퍼셋 확장(early/late·전체옵션·빌드해시·µs·login) · FE 프로젝트(MIT) · 클라 런타임 자기-빌드해시 산출 · GitHub 원격+dev/prod 브랜치 설정 · LICENSE(GPL-3.0) 파일 추가. → `ROADMAP.md`.
