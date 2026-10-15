# ADR: 사용자 폴더 — 전용 API, MY FOLDERS, 비공개

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(U4, Q3) + 메인(M3, M4)
> 세부 결정 표의 정본: [2026-10-14-web-decisions.md](2026-10-14-web-decisions.md). 설계 문서: `docs/plan/2026-10-14-web-user-folders.md`.

## 배경

사용자 지시(2026-10-14): 로그인한 사용자에게 사용자 폴더가 할당되고, R-BMS 는 로그인한 사용자의 정보를 불러올 때 곡 preset 폴더도 불러온다(기능은 table 과 같다).

## 결정

1. 폴더는 사용자당 여러 개, 이름을 붙여 웹에서 편집하고 R-BMS 는 읽기만 한다(U4).
2. 모델: `user_folder`, `user_folder_item` 두 테이블(마이그레이션 `web/drizzle/0001_user_folders.sql`). 곡 식별자는 md5 필수·sha256 선택. 한도: 폴더 50개, 폴더당 2,000곡, 전체 20,000곡, 이름 64자. 폴더 안의 묶음은 없다(M3).
3. 통로: 웹 편집은 세션 인증의 `/api/fe/me/folders…`(목록·만들기·순서·이름·삭제·곡 추가·제거·순서·표에서 가져오기·곡이 든 폴더 찾기·명시 저장용 통째 바꾸기). 클라이언트 읽기는 Bearer 토큰의 전용 통로 `/api/players/{id}/folders` 와 `/api/players/{id}/folders/{folderId}` 이며 본인만 읽는다(다른 사람이면 `FORBIDDEN`, `web/src/server/route/folder.ts`)(M4).
4. 클라이언트: 선곡 뿌리에 `MY FOLDERS` 한 줄 → 폴더 → 곡(`apps/rbms-player/src/user_folders.rs` 의 `USER_FOLDERS_LABEL`). 가지고 있지 않은 곡은 폴더 줄의 (가진 곡/전체)로만 보인다. 로그아웃하면 저장본을 지운다.
5. v1 은 전부 비공개(Q3 의 A). `is_public` 컬럼은 만들어 두되(기본 false, `web/src/server/db/schema.ts`) 쓰지 않는다.
6. 웹 화면: `/folders`, `/folders/[id]`(만들기·순서, 곡 검색과 표에서 가져오기, 명시 저장·미저장 경고·409 충돌 안내, 삭제 확인), 차트 페이지와 표 상세의 "내 폴더에 넣기".

## 이유

- 전용 API: 폴더는 비공개 데이터다. 표준 BMS 표 주소로 내주면 접근 통제가 주소의 비밀에만 기댄다.
- 비공개 v1: 공개·공유 주소는 권한·노출 설계가 더 필요하다. 사용자 A 선택.
- 명시 저장: 끌어 옮기기·여러 곡 편집 중 반쯤 저장된 상태를 만들지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 클라이언트가 폴더를 표준 표 주소(`header.json`·`data.json`)로 읽기 | 비공개 데이터를 주소 비밀에 기댄다 |
| 폴더 공개와 공유 주소(Q3 의 B) | 사용자 A 선택 |
| 사용자당 폴더 하나 | 사용자 결정 U4(여러 개) |

## 영향과 관련 파일

- 서버: `web/drizzle/0001_user_folders.sql`, `web/src/server/db/schema.ts`, `web/src/server/route/folder.ts`, `web/src/app/api/fe/me/folders/**`, `web/src/app/api/players/[playerId]/folders/**`.
- 웹 화면: `web/src/app/(app)/folders` 아래.
- 클라이언트: `apps/rbms-player/src/user_folders.rs`.
- 커밋: `d38b6fb`(API·0001), `711b518`(화면), `8b11a17`(클라이언트 MY FOLDERS). 운영 DB 에 0001 적용됨(2026-10-14).
- 남은 일(2차): 폴더 끌어 옮기기, 2,000곡 가상화.

## 날짜와 사용자 원문

2026-10-14. 지시 요지(`docs/history/2026-10-15-process-archive.md` 의 "현재 작업" 절 (나)): "로그인한 사용자에게 사용자 폴더가 할당된다. R-BMS 는 로그인한 사용자의 정보를 불러올 때 곡 preset 폴더도 불러온다(기능은 table 과 같다)." 결정 답: U4(사용자 결정), Q3 "여섯 개 모두 A(추천안)".
