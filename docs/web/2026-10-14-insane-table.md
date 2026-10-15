# 발광 난이도표(発狂BMS難易度表) 스냅샷과 사본·기본 등록 설계

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9

> 작성: 2026-10-14, 웨이브 17A 작업 A3. 조사와 설계이며 웹 소스(`web/src`)와 Rust 소스는 고치지 않았다.
> **구현 결과(웨이브 17B~18, 2026-10-14): 이 문서의 추천안이 모두 그대로 채택·구현됐다.** 6절의 결정은 [../acknowledge/2026-10-14-web-decisions.md](../acknowledge/2026-10-14-web-decisions.md)(U2·U3·M1·M2·M5·Q1) 와 [../acknowledge/2026-10-14-adr-default-insane-table.md](../acknowledge/2026-10-14-adr-default-insane-table.md) 가 정본이다: 사본 주소 `https://bms.hyuns.uk/tables/insane1`(슬래시 없음, noindex. 출처·삭제 요청 문구는 2026-10-14 에 뺐다: [../acknowledge/2026-10-14-adr-table-page-no-source-notice.md](../acknowledge/2026-10-14-adr-table-page-no-source-notice.md)), 표는 DB 가 아니라 파일을 직접 서빙(7.2 의 A안), 클라이언트는 `seeded_default_tables` 표식과 실행 파일에 내장한 스냅샷으로 기본 등록(7.4 의 B안, 8b11a17). 서빙 코드: `web/src/server/service/domain/table-snapshot/`, `web/src/server/route/table-snapshot.ts`, 라우트 `web/src/app/tables/[tableId]/{header,data}.json`, 화면 `web/src/app/(app)/tables/insane1/page.tsx`. 응답 규칙은 [../backend/api-spec.md](../backend/api-spec.md) 7.1, 라우트는 [architecture.md](./architecture.md) 1-3. 본문은 조사 당시의 기록이다.
> **이후 바뀐 것(2026-10-14~15, 사용자 지시 "제발 다른사이트의 흔적좀 지워놔")**: 6절의 "출처 세 곳·삭제 요청 문구"와 header 의 `mirror_of` 는 모두 없앴다(4a0872d·ea34002). 스냅샷 정규화도 바뀌어 0절의 "정규화는 값을 바꾸지 않는다"는 더 이상 맞지 않는다: `web/scripts/snapshot-table.ts` 가 header 의 `original_url` 을 버리고, 곡의 `comment` 를 비우고, 내려받기 주소(`url`·`url_diff`·`name_diff`) 밖의 글자 필드에서 웹 주소를 지운다(4fda17c·dd5518e. 내려받기 주소는 사용자 정정으로 남겼다). 지금 `source.json` 은 2026-10-15 에 다시 받은 기록이고 `normalization` 에 이 규칙을 적는다. 서빙하는 필드는 [../backend/api-spec.md](../backend/api-spec.md) 7.1 이 정본이다.
> 만든 것: `web/data/tables/insane1/{header.json,data.json,source.json}`, `web/scripts/snapshot-table.ts`, 이 문서.
> 관찰 시각: 2026-10-14 02:38~02:42 UTC. 모든 수치는 그 시각 `https://darksabun.club/table/archive/insane1/` 의 응답이다.

## 0. 요약

- 원본은 표준 BMS 표 3단(HTML 의 `bmstable` 메타 -> `header.json` -> `data.json`)이고, 곡 1035개, 레벨 26개(`1`~`25` 와 `???`), 전 곡 md5 32자리 소문자 16진, **sha256 은 한 곡도 없다**.
- 스냅샷은 저장소에 커밋한다. 원본 바이트의 SHA-256 은 `source.json` 에 있고, 정규화(LF, 곡당 한 줄)는 값을 바꾸지 않는다(파싱 결과가 원본과 같음을 확인).
- 재배포를 금지하는 문구는 원본 페이지·header·사이트 첫 화면·내비게이션·푸터·robots.txt 어디에도 **없다**. 허용하는 문구도 없다(라이선스·관리자 표기가 없음). 저장은 사용자 결정대로 진행했고, 허락 확인은 결정 사항으로 올린다(§6).
- 사본 주소는 **`https://bms.hyuns.uk/tables/insane1`(끝 슬래시 없음)** 를 추천한다. 운영 사이트가 `/x/` 를 `/x` 로 308 리다이렉트하는 것을 확인했고(§7.1), 클라이언트는 상대 주소를 리다이렉트 후 주소 기준으로 풀기 때문에 슬래시를 정본으로 삼으면 깨진다.
- R-BMS 기본 등록은 **"등록한 적 있음" 표식**을 설정에 두는 방식을 추천한다(§7.4). 새 설치와 기존 사용자 모두 한 번만 넣고, 지우면 다시 넣지 않는다. 같은 표 판별은 주소 정규화 + 별칭 목록으로 한다.

## 1. 형식과 클라이언트가 읽는 필드

근거 파일: `crates/rbms-table/src/{html.rs,dto.rs,fetch.rs,saved.rs,lib.rs}`, `apps/rbms-player/src/tablesrc.rs`.

### 1.1 표준 3단

1. 표 페이지(HTML). `<meta name="bmstable" content="header.json">` 하나가 header 주소를 준다.
2. `header.json`(JSON 객체): `name`, `symbol`, `data_url`, `level_order`, 선택적으로 `course`, `tag`, `last_update`, `original_url`, `grade`(표마다 다름).
3. `data.json`(JSON 배열): 곡 객체. 표준 필드 `md5`, `sha256`, `level`, `title`, `artist`, `url`, `url_diff`, `comment`.

### 1.2 클라이언트(`DifficultyTable::fetch`)의 규칙

| 단계 | 읽는 것 | 허용하는 변형 |
|---|---|---|
| 첫 응답 | 첫 비공백 바이트(BOM 제거 후)로 모양 판정: `[` 이면 body, `{` 이면 header, 그 외는 페이지(`fetch.rs` `shape_of`) | 사용자가 등록한 주소가 페이지, header, body 중 무엇이든 됨. body 이면 같은 폴더의 `header.json` 을 있으면 받음(실패해도 무시) |
| 페이지 | `<meta>` 태그 중 `name` 이 `bmstable`(대소문자 무시)인 첫 태그의 `content`(`html.rs`) | 속성 순서, 대소문자, 큰따옴표/작은따옴표/무따옴표, `/>`, 줄바꿈, 주석 처리된 태그 무시, `&amp;` 를 `&` 로. HTML 전체를 훑으므로 head 밖(body)에 있어도 찾음 |
| 주소 해석 | `join_url(base, rel)` = `url::Url::join`(RFC 3986). base 는 **리다이렉트가 끝난 뒤의 최종 주소**(`response.url()`) | 상대 경로, 루트 상대(`/abs`), 절대 URL, `..` 모두 됨. body 주소는 페이지가 아니라 **header 의 최종 주소** 기준 |
| header | `name`, `symbol`(없으면 `*`), `data_url`, `level_order`, `course`(원문 그대로 보관만 함) (`dto.rs`) | 값이 숫자/불리언이어도 문자열로 바꿈. `data_url` 은 문자열 하나 또는 목록(여러 body 합산). `level_order` 가 없거나 비면 본문에서 숫자 오름차순 -> 비숫자 순으로 유도 |
| header 가 안 읽는 것 | `last_update`, `original_url`, `tag`, `grade` 등 | 있어도 무시되고 **저장본에도 남지 않는다** |
| body | 배열의 각 항목. 객체가 아니거나 `md5`·`sha256` 둘 다 없으면 건너뛰고 `skipped` 로 셈 | 필드는 `md5`, `sha256`, `level`, `title`, `artist`, `url`, `url_diff` 만 읽음. `comment`, `name_diff`, `lr2_bmsid` 등은 무시되고 저장본에도 없음 |
| 곡 맞춤 | `match_levels_in` 은 **md5 만** 쓴다(소문자화, trim). sha256 만 있는 항목은 보관되지만 맞춰지지 않음 | 이 표는 전 곡 md5 가 있어 영향 없음 |
| 레벨 묶음 | `by_level`: `level_order` 순서, 곡이 없는 레벨은 제외, `level_order` 에 없는 레벨은 첫 등장 순으로 뒤에 붙임 | 이 표는 header 의 26개와 본문 레벨이 정확히 일치 |
| 인코딩 | UTF-8(BOM 제거). UTF-8 이 아니면 Shift_JIS 로 정상 해독될 때만 그것으로, 아니면 손실 변환 | 이 표는 UTF-8 |
| 한도 | 응답 한 건 최대 64 MiB, 무응답 15초, 한 표 전체 30초, User-Agent `rbms-table`, 2xx 가 아니면 실패 | `Content-Type`, 캐시 헤더는 안 봄. 조건부 요청 안 함 |

저장본(`<설정 폴더>/tables/<주소 해시>.json`, `SavedTable::json_of`)에는 `name`, `symbol`, `level_order`, `entries`(위 7개 필드), `course`, `skipped`, `source`(등록한 주소 그대로), `fetched_at` 이 들어간다. 파일 이름은 주소 문자열의 해시(`url_hash`)라서 **등록 주소의 문자열이 바뀌면 다른 표**가 된다.

## 2. 받은 기록

| 대상 | 요청 주소 | 상태 | 크기 | Content-Type | Last-Modified | ETag | 인코딩 | 원본 바이트 SHA-256 |
|---|---|---|---|---|---|---|---|---|
| 페이지 | `https://darksabun.club/table/archive/insane1/` | 200(리다이렉트 없음) | 4171 B | `text/html; charset=utf-8` | Tue, 04 Aug 2026 13:11:48 GMT | 없음 | UTF-8, CRLF | `c9e17227e5e53bc88d3f65ed0c404a5acee1a7d36d5106d91fe2848327b8a1f1` (요청마다 바뀜, 아래 참고) |
| header | `.../insane1/header.json` | 200 | 500 B | `application/json` | Wed, 04 Mar 2026 10:52:44 GMT | `"69a80efc-1f4"` | UTF-8, BOM 없음, CRLF | `ef94c77b2f8bb03574cbf242aa217697ced16d6cd3646eb3c778e67e2a78d561` |
| data | `.../insane1/data.json` | 200 | 473105 B | `application/json` | Wed, 04 Mar 2026 10:52:44 GMT | `"69a80efc-73811"` | UTF-8, BOM 없음, CRLF(11387줄), 2칸 들여쓰기 | `5e9902fe59d9d9de9143ae9cc6fa47b8dc50b1937e90584d17984d64e72e7f6c` |

- 서버는 Cloudflare(`server: cloudflare`, HTTP/2). 재시도는 필요 없었다.
- 페이지 HTML 에는 Cloudflare 가 요청마다 끼워 넣는 스크립트(`window.__CF$cv$params={r:'<요청 고유 id>',...}`)가 있어 **바이트가 요청마다 다르다**(curl 받은 해시 `c9e1...`, 스크립트가 받은 해시 `4bb6bc9d...`, 크기는 둘 다 4171 B). 그래서 HTML 은 저장하지 않고, 쓰는 것은 메타 내용(`header.json`)뿐이다.
- 스크립트가 Bun `fetch` 로 받은 header·data 의 SHA-256 은 curl 로 받은 값과 같다. ETag 는 약한 형태(`W/"..."`)로 보였는데 압축 전송에 따른 Cloudflare 의 변환으로 보이며(추측), 내용 해시는 같다.
- 페이지 `<head>` 에는 `<meta name="ROBOTS" content="NOINDEX, NOFOLLOW" />` 와 `<meta name="bmstable" content="header.json" />` 가 있다. 제목(`<title>`)은 비어 있고 JS(`convert_table.js`)가 `header.name + " - DARKSABUN"` 으로 채운다. 즉 사람이 보는 표는 JS 로 `data.json` 을 읽어 그린다.
- 요청 수(darksabun.club): 표 3개 파일(curl) + 권리 확인용 보조 파일 6개(`footer.html`, `convert_table.js`, `table.js`, `robots.txt`, 첫 화면, `navbar.html`) + 스냅샷 스크립트 실행 1회(3건) + `--check` 1회(3건) = 15건. 지시는 "필요한 파일만 한 번씩" 이었는데 스크립트로 저장하고 `--check` 를 보이는 과정에서 표 3개 파일이 3번씩 받아졌다. 병렬 요청 없음. 부하는 없었지만 지시와 다르므로 기록한다.
- 운영 사이트(bms.hyuns.uk)에는 GET 4건만 보냈다: `/api/tables`, `/table/insane1/`, `/table/insane1`, `/tables`.

## 3. 분석

### 3.1 header

```json
{
  "data_url": "data.json",
  "last_update": "2017.02.05",
  "level_order": ["1", "2", "...", "25", "???"],
  "name": "発狂BMS難易度表",
  "original_url": "http://nekokan.dyndns.info/~lobsak/genocide/insane.html",
  "symbol": "★"
}
```

- 이름 `発狂BMS難易度表`, 기호 `★`(U+2605). `course`, `grade`, `tag` 없음. `data_url` 은 문자열 하나(상대 경로 `data.json`).
- `last_update` 가 2017.02.05, 경로가 `archive` 이므로 더 갱신되지 않는 보관본으로 보인다(경로명에서의 추정). `original_url` 은 `http://nekokan.dyndns.info/~lobsak/genocide/insane.html`, 원 표 페이지의 주소다(이 주소는 받지 않았다).
- 클라이언트는 `original_url`, `last_update` 를 읽지 않는다.

### 3.2 data.json

| 항목 | 값 |
|---|---|
| 곡 수 | 1035 |
| 최상위 | JSON 배열, 항목 전부 객체, 건너뛰어지는 항목 0 |
| 필드(전 곡 동일, 키 순서도 동일: 알파벳 순) | `artist`, `comment`, `level`, `lr2_bmsid`, `md5`, `name_diff`, `title`, `url`, `url_diff` — **9개, 전부 문자열** |
| `sha256` | 필드 자체가 없음(0/1035) |
| 정렬 | md5 오름차순으로 정렬되어 있음(레벨별 정렬 아님) |
| md5 | 1035/1035 가 `^[0-9a-f]{32}$`. 대문자 0, 중복 0(같은 md5 가 여러 레벨에 있는 경우 없음) |
| 같은 제목 | 6개 제목이 서로 다른 md5 로 2~3번 나옴(예: `Little "Sister" Bitch` 레벨 2, 10, 22). md5 가 다르니 별개의 차트 |
| `lr2_bmsid` | 전부 숫자 문자열(13~240706), 중복 0. 표준 필드가 아니라 클라이언트는 무시 |
| 값이 비지 않은 곡 수 | `title` 1035, `level` 1035, `md5` 1035, `lr2_bmsid` 1035, `artist` 1034, `url` 1033, `url_diff` 825, `name_diff` 782, `comment`(공백 제외) 790 |
| 빈 `artist` | 1곡(`27d747cb8db9a924e0addacb5f70e698` `The SunLight [U]`) |
| 빈 `url` | 2곡(`6210734d...` `gravitronicⅠ -ANOTHER-`, `e8836d17...` `Utopiangela (Cartoon Channel 203)`) |
| `url_diff` 와 `name_diff` | 861곡에서 같음, 43곡은 `url_diff` 만 채워짐, 210곡은 둘 다 빔. `name_diff` 만 채워진 곡은 없음 |
| 공백·탭 | `url_diff`/`name_diff` 중 2곡이 앞뒤 공백(`'http://ibizatrance.nomaki.jp/index.html '`, `'\thttp://t.co/pIFkeWSSfe'`). 이 중 탭으로 시작하는 값은 `http` 로 시작하지 않아 링크로 쓰려면 trim 이 필요 |
| `comment` | 792곡에 내용, 최대 270자. 줄바꿈(`\n`) 125곳과 탭 2곳 포함(정상 내용). 345곡의 comment 에 URL 문자열이 있음 |
| 제목·아티스트 문자 | 609곡에 비ASCII. 가나가 있는 제목 231, 한자가 있는 제목 251, ASCII 만인 제목 674. 전각 물결 `～`, 전각 공백 U+3000, `☆`, `★`, `×`, `ω` 등 기호. BMP 밖 문자 없음, 제어 문자·U+FFFD·사설 영역 문자 없음 |
| `<` `>` 포함 | 12곡의 title/artist/comment 에 있음(`アズール <ANOTHER>` 등) -> 웹은 반드시 이스케이프 출력(React 기본)하고 `dangerouslySetInnerHTML` 을 쓰지 않는다 |
| 최장 | 제목 66자, 아티스트 95자, 곡 한 줄(JSON) 706자 |

### 3.3 레벨

`level_order` 는 `1`..`25`, `???` 로 26개이고, 본문에도 정확히 이 26개만 나온다(순서에 없는 레벨 0, 곡이 없는 레벨 0).

| 레벨 | 곡 | 레벨 | 곡 | 레벨 | 곡 |
|---|---|---|---|---|---|
| 1 | 53 | 10 | 64 | 19 | 36 |
| 2 | 56 | 11 | 45 | 20 | 37 |
| 3 | 53 | 12 | 58 | 21 | 37 |
| 4 | 52 | 13 | 46 | 22 | 26 |
| 5 | 59 | 14 | 43 | 23 | 28 |
| 6 | 46 | 15 | 35 | 24 | 11 |
| 7 | 60 | 16 | 31 | 25 | 2 |
| 8 | 47 | 17 | 30 | `???` | 5 |
| 9 | 46 | 18 | 29 | 합계 | 1035 |

`???` 의 5곡: `la noche (end of mix)`, `Merry Christmas Mr.DO`, `gravitronicⅠ -ANOTHER-`, `Milly Barll`, `Into the sunset [ANOTHER]`.

### 3.4 URL 분포 (`url`, `url_diff`, `name_diff` 세 필드 합산, 죽은 링크는 확인하지 않음)

- 값이 있는 URL 은 총 2640개. 스킴은 `http` 2625(+탭으로 시작하는 1), `https` 14. `javascript:`·`data:` 없음. 호스트 189종.
- 상위 호스트: `absolute.pv.land.to` 435, `manbow.nothing.sh` 388, `nekokan.dyndns.info` 203, `skydrive.live.com` 164, `www.geocities.jp` 140, `akred.web.fc2.com` 92, `web.archive.org` 82, `www.dropbox.com` 69, `sabosabun.tank.jp` 46, `cid-ce1dd07774d515b4.skydrive.live.com` 42.
- `url`(곡 소개/이벤트 페이지) 필드만 보면 134종 호스트: `manbow.nothing.sh` 388, `skydrive.live.com` 89, `web.archive.org` 76, `nekomimi.name` 21, `www.geocities.jp` 20.
- 대부분 오래된 `http://` 호스트이므로 웹 페이지에서 링크는 `target="_blank" rel="noopener noreferrer nofollow"` 로, 스킴은 `http`/`https` 만 허용하고 trim 한 뒤 출력한다. 링크가 죽어 있을 수 있음을 UI 가 가정해야 한다.

## 4. 저장 구성과 정규화 규칙

```
web/data/tables/insane1/
  header.json    467 B     정규화한 header
  data.json      400,653 B 정규화한 body (1035줄 + 괄호 2줄)
  source.json    2,526 B   출처와 해시
web/scripts/snapshot-table.ts
```

### 4.1 정규화

- UTF-8, BOM 없음, 줄바꿈 LF, 마지막에 개행 하나. (저장소 `.gitattributes` 의 `* text=auto eol=lf` 와 일치. 원본의 CRLF 는 값이 아니라 줄바꿈이므로 바꿔도 뜻이 같다.)
- 키 순서와 곡 순서를 그대로 유지, 값은 한 글자도 바꾸지 않는다. 문자열은 `JSON.stringify` 로 쓰므로 비ASCII 는 원문 그대로 UTF-8 이다. 정수처럼 보이는 키가 있는 객체는 JS 가 순서를 바꿀 수 있으나 이 표에는 없다.
- `header.json`: 2칸 들여쓰기(500 B -> 467 B).
- `data.json`: **곡마다 한 줄**. 원본의 2칸 들여쓰기(곡당 11줄, 473,105 B)와 비교하면 크기가 15% 작고(400,653 B, gzip 115,252 B), git diff 에서 곡 하나의 변경이 한 줄 변경으로 보인다. 필드 단위 가독성은 줄지만 이 표는 곡 단위로 diff 하는 편이 의미 있고(md5 로 정렬되어 있어 변경이 흩어지지 않음), 줄이 706자 이하라 리뷰 도구에서도 문제가 없다.
- 확인한 것: 원본 JSON 과 스냅샷 JSON 을 파이썬으로 파싱해 비교하면 데이터·header 모두 같고, 모든 곡의 키 순서도 같다(`==`, 키 순서 비교 모두 참).

### 4.2 `source.json`

원본 주소 셋(페이지, header, data), `bmstable` 메타 내용(`header.json`), 받은 시각(`2026-10-14T02:41:43.771Z`), 각 요청의 상태·최종 주소·Content-Type·Last-Modified·ETag·바이트 수, 원본 바이트 SHA-256(header·data), 정규화 후 파일의 SHA-256, 곡 수 1035, 레벨 26, 유효 md5 1035, 만든 방법(명령과 정규화 규칙)이 있다. 페이지 HTML 의 해시는 `sha256_volatile` 이라는 이름으로 두었고 비교에 쓰지 않는다.

정규화 후 파일 해시:

| 파일 | SHA-256 |
|---|---|
| `header.json` | `0b98f3ff52698a1bcff00705c953275bc64361ad94272327c318758e0a4dda8d` |
| `data.json` | `7c606a33e254d62bdfb1ce861089d1328e7ae44267073f043dfefb8b77a5fc5e` |

### 4.3 스크립트 `web/scripts/snapshot-table.ts`

```
bun scripts/snapshot-table.ts <표 주소> <출력 폴더>           # 받아서 세 파일을 쓴다
bun scripts/snapshot-table.ts <표 주소> <출력 폴더> --check   # 다시 받아 저장본과 비교, 쓰지 않는다
```

- 표 주소는 페이지 또는 header.json 주소. 페이지면 `bmstable` 메타(주석 제거, 속성 순서·대소문자·따옴표 변형 허용)를 읽고 `new URL(rel, 최종 주소)` 로 header 를 푼다. `data_url` 이 목록이면 순서대로 받아 합친다. body 주소를 직접 주면 거절한다.
- UTF-8 만 받는다(`fatal` 해독). Shift_JIS 표는 이 스크립트 범위 밖이다. 응답 한 건 64 MiB, 요청 30초 제한.
- `--check` 종료 코드: 0 같음, 1 다름(어떤 파일이 다른지 출력), 2 실패. 저장본이 `source.json` 의 해시와 맞는지(손으로 고치지 않았는지)도 같이 본다. 원본 바이트의 해시가 아니라 **정규화 결과**를 비교하므로 줄바꿈·들여쓰기만 바뀐 경우는 "같음"이다.
- 컨벤션: arrow function, 주석 없음, `any` 없음, 매직넘버 상수화(`REQUEST_TIMEOUT_MS` 등), Zod 로 외부 JSON 검증. 확인: `prettier --check`(저장소 설정) 통과, `tsc --noEmit --strict` 통과, `eslint`(저장소 설정) 통과.
- 실행 결과: 쓰기 `wrote 1035 entries across 26 levels`, `--check` -> `same: 1035 entries, header and data match the origin after normalization`, 종료 코드 0. 음성 검증: 로컬 서버(127.0.0.1)에 레벨 하나를 바꾼 사본을 놓고 `--check` 하면 `the data differs from the origin` 종료 코드 1, 같은 서버로 쓰기를 하면 레벨 27개로 정상 생성(`META CONTENT=... NAME=...` 처럼 순서·대소문자가 다른 태그도 읽음).
- `data_url` 은 저장본에서 원본 그대로 `data.json`(상대 경로)이다. 사본 주소로 바꾸는 일은 응답 시점에 웹이 한다(§7.3).

### 4.4 이 구성이 다음 웨이브에 요구하는 것

- (완료: d38b6fb 가 `web/.prettierignore` 에 `data` 를 추가했다. 아래는 당시 요구) `web/.prettierignore` 에 `data` 를 추가해야 한다. 현재 파일은 `.next node_modules bun.lock drizzle public` 이라서 `bun run format` 이 `data.json` 의 한 줄 형식을 펼쳐 버리고, `format:check` 와 `--check` 가 어긋난다. (이 워크플로는 기존 파일을 고치지 않아 건드리지 않았다. 확인: `prettier --check` 를 `data/` 에 돌리지는 않았고 `.prettierignore` 내용으로 판단했다.)
- `tsconfig.json` 의 `include` 가 `**/*.ts` 라서 `scripts/snapshot-table.ts` 도 `bun run typecheck` 와 `eslint` 대상이 된다. 단독으로는 통과했고 전체 `bun run typecheck` 는 돌리지 않았다.
- JSON 을 `import` 해서 쓰려면 `resolveJsonModule`(이미 true)이면 된다. 400 KB 라 번들·함수 크기에 문제 없다. 파일은 `web/` 안이므로 Vercel 루트 디렉터리 `web/` 에 포함된다.
- 갱신 절차: `bun scripts/snapshot-table.ts <주소> data/tables/insane1` -> `git diff` 로 곡 단위 변경 확인 -> 커밋. 보관본이라 변경이 없을 것으로 보지만 `--check` 를 CI 에 넣을 필요는 없다(원본 사이트가 내려가면 CI 가 깨진다).

## 5. 출처와 권리

### 5.1 원본에 적힌 것 (그대로)

| 위치 | 문구 |
|---|---|
| header `name` | `発狂BMS難易度表` |
| header `original_url` | `http://nekokan.dyndns.info/~lobsak/genocide/insane.html` |
| header `last_update` | `2017.02.05` |
| 페이지 `<meta name="ROBOTS">` | `NOINDEX, NOFOLLOW` |
| 페이지 상단 | 링크 `DARKSABUN`, 제목은 JS 가 `発狂BMS難易度表 - DARKSABUN` 으로 채우고, 큰 제목(`#bigTableTitle`)은 `original_url` 로 링크됨, `Last Update : 2017.02.05` |
| `footer.html` | `DARKSABUN` 한 단어 |
| `robots.txt` | `User-agent: *` / `Allow: /` / `Sitemap: https://darksabun.club/sitemap.xml` |
| 사이트 첫 화면 텍스트, `navbar.html` 텍스트 | 이벤트·표 목록만(예: `Main Tables`, `Mirror`, `Table List`). 라이선스·저작권·재배포·관리자 표기 없음 |

- **재배포를 금지하거나 허락을 요구하는 문구는 찾지 못했다.** 반대로 라이선스(CC 등), 관리자 이름, 재배포 허가 문구도 없다. 따라서 "허락 문구가 있어서 저장한다"가 아니라 "금지 문구가 없고 사용자가 저장하기로 정했다"가 근거다.
- 표의 내용은 곡 제목·아티스트·md5·레벨·외부 링크이고, 레벨 판단(난이도 평가)과 모음이 창작 요소다. `original_url` 이 가리키는 `~lobsak/genocide` 경로가 원 작성자 쪽 주소로 보이나, 작성자를 단정할 근거(페이지에 이름 표기)는 확인하지 못했다.
- `NOINDEX, NOFOLLOW` 는 검색엔진 색인에 대한 지시이지 재배포 금지 문구가 아니다. 다만 원 운영자가 검색 노출을 원치 않는다는 신호로 읽을 수 있어, 사본 페이지도 색인에서 제외하는 편이 안전하다(§7.2, 결정 3).

### 5.2 사본 페이지에 넣을 출처 표기(영어, 제안)

> This difficulty table (発狂BMS難易度表 / Insane BMS Difficulty Table) is a static archive copy of the table published at https://darksabun.club/table/archive/insane1/ (snapshot of 2026-10-14, 1035 charts, last updated by the original on 2017.02.05). The table was originally published at http://nekokan.dyndns.info/~lobsak/genocide/insane.html. All charts, levels and links belong to their original authors and the table's original compilers; this copy is provided unchanged for use with BMS players, and will be removed on request. The data is not modified.

- 각 곡의 `url`·`url_diff` 링크는 곡 작성자의 것이므로 "링크만 제공, 파일은 호스팅하지 않음"을 한 줄 덧붙인다.
- 푸터 또는 표 페이지 상단에 출처 링크 둘(darksabun 보관본, `original_url`)을 항상 보이게 한다.
- header 응답에도 `mirror_of`(darksabun 주소) 같은 선택 필드를 넣을 수 있다. 알려지지 않은 필드는 R-BMS 와 일반 도구가 무시한다(§1.2).

## 6. 결정 필요

1. 원 운영자에게 저장·재배포 허락을 확인할지 — **A안(추천: 저장 계속 + 출처·삭제 요청 문구를 넣고, 가능하면 darksabun 운영자에게 공개 전 연락)** / B안(사용자 결정대로 연락 없이 진행) / C안(사본을 만들지 않고 원본 주소를 기본 등록)
2. 사본 URL 정본 — **`/tables/insane1`(추천: 기존 `/tables`, `/tables/[tableId]` 와 같은 복수형, 정적 세그먼트가 동적 세그먼트보다 우선)** / `/table/insane1`(사용자 예시)
3. 사본 페이지의 색인 — **`noindex`(추천: 원본이 NOINDEX 이므로 따름)** / 색인 허용
4. 클라이언트 기본 등록 방식 — §7.4 **B안(표식 + 별칭, 추천)** / A안(새 설치만) / C안(앱에 사본을 내장)

## 7. 웹과 클라이언트의 사용 설계 제안

### 7.1 주소 설계와 트레일링 슬래시 (관찰)

운영 사이트에서 확인한 것(GET 만):

- `GET https://bms.hyuns.uk/api/tables` -> 200, 본문 `[]`(운영 DB 에 IR 난이도표 없음).
- `GET https://bms.hyuns.uk/table/insane1/` -> **308, `location: /table/insane1`**(본문 `Redirecting...`). 즉 Next.js 기본 동작으로 끝 슬래시가 제거된다(`web/next.config.ts` 에 `trailingSlash` 없음).
- 클라이언트는 페이지를 받은 뒤 `bmstable` 의 `content` 를 **리다이렉트 후 최종 주소**에 대해 푼다. 사본 페이지 메타가 darksabun 처럼 상대 경로 `header.json` 이면 최종 주소가 `/tables/insane1` 이므로 `https://bms.hyuns.uk/tables/header.json` 으로 풀려 404 가 난다(`join_url` 은 `url` 크레이트 RFC 3986: 마지막 세그먼트를 대체).
- 따라서 두 가지를 같이 한다.
  1. 정본 주소를 슬래시 없이 `https://bms.hyuns.uk/tables/insane1` 로 하고 클라이언트 기본 등록에도 그 문자열을 쓴다.
  2. 메타 내용을 **루트 상대 또는 절대 경로**로 한다: `<meta name="bmstable" content="/tables/insane1/header.json">`. `join_url("https://x.club/t/insane1/header.json", "/abs/data.json")` 가 `https://x.club/abs/data.json` 이 되는 단위 테스트가 `crates/rbms-table/src/lib.rs` 에 이미 있다. 슬래시가 있든 없든 같은 결과가 된다.

경로 구성(제안):

| 경로 | 내용 | 비고 |
|---|---|---|
| `/tables/insane1` | 사람이 보는 페이지 + `bmstable` 메타 | 기존 `/tables/[tableId]`(DB 의 IR 표)보다 정적 세그먼트가 우선. `(app)` 레이아웃 안 |
| `/tables/insane1/header.json` | header 응답 | Route Handler |
| `/tables/insane1/data.json` | body 응답 | Route Handler |

- Next 16 에서 메타 태그는 `metadata.other` 로 낼 수 있으나(확인하지 못함, 구현 시 `node_modules/next/dist/docs/` 에서 확인), **페이지를 정적으로 만들어** 메타가 초기 HTML 의 `<head>` 에 있게 해야 한다. 스트리밍 메타데이터(동적 페이지)는 `<body>` 로 나올 수 있다. R-BMS 는 문서 전체를 훑어 찾으므로 괜찮지만 다른 도구는 `<head>` 만 볼 수 있다. 수용 확인: 배포 뒤 `curl -s https://bms.hyuns.uk/tables/insane1 | grep bmstable`.
- 기존 `/api/tables`(IR 계약, `capabilities.tables`)와 섞지 않는다. 이 둘은 폴더·sha256 구조의 별도 계약이다.

### 7.2 DB 에 넣을지, 파일에서 바로 읽을지

| 선택지 | 내용 | 평가 |
|---|---|---|
| **A. 파일 직접(추천)** | `web/data/tables/insane1/*.json` 을 정적 import, 모듈 로드 때 Zod 로 한 번 검증, 응답·페이지가 그 객체를 씀 | 배포 = 데이터. 마이그레이션·시드·관리자 호출 불필요. 원본이 내려가도 동작. 표준 형식 그대로 서빙 가능 |
| B. `difficulty_table`/`table_folder`/`table_chart` 에 넣기 | `POST /api/tables`(admin)로 시드 | **막힘**: `table_chart.chart_sha256` 가 `char(64) NOT NULL` 인데 이 표는 sha256 이 없고, 서버 `chart` 테이블(`sha256` PK, `md5` 유니크)에 없는 곡은 sha256 을 알 수 없다. `tableService.upsert` 가 `sha256.length > 0` 만 남기므로(`table.service.ts`) 곡 대부분이 버려진다. 또 IR 응답 `/api/tables` 에 노출되면 sha256 없는 항목이 생긴다 |
| C. 파일 + DB 보강 | A 로 표를 서빙하고, 화면에서 곡별 서버 데이터(기록 수, 점수 있음)를 `chart.md5`(유니크 인덱스 `chart_md5_uniq`)로 `inArray` 조회해 붙임 | 좋은 UX(곡 행에서 `/charts/<md5>` 로 이동, 기록 유무 표시). `resolveChartSha256Cached(hash)` 가 md5 도 받는다(`read-cache.ts`). 1035개 `IN` 조회는 한 쿼리로 가능. **A 위에 얹는 단계적 개선으로 추천** |

- 곡 -> 표 레벨 역색인(md5 -> 레벨)을 모듈 로드 때 `Map` 으로 만들어 두면 차트 상세 화면에 "발광 ★N" 배지를 붙이기 쉽다(1035건이라 비용 무시).
- `/tables` 목록 화면은 지금 DB 표만 보여 준다(운영은 비어 있음). 번들된 표를 같은 목록에 합쳐 보여 줄지는 UI 결정이다(§6 에는 올리지 않았다. 구현 웨이브에서 정한다).

### 7.3 응답 규칙 (header, data)

- `header.json`: 저장본을 읽어 `data_url` 만 **요청 origin 기준 절대 URL**(`${origin}/tables/insane1/data.json`)로 바꿔 돌려준다. 상대 경로 `data.json` 도 R-BMS 에서는 동작하지만(header 최종 주소 기준으로 풂), 절대 URL 이 다른 도구에서 더 안전하다. 프리뷰 배포에서도 맞도록 origin 은 요청에서 얻는다. 나머지 필드(`name`, `symbol`, `level_order`, `last_update`, `original_url`)는 그대로 둔다. 선택: `mirror_of` 추가.
- `data.json`: 저장본 배열 그대로(`comment`, `name_diff`, `lr2_bmsid` 도 유지: 다른 도구가 `name_diff` 를 쓴다).
- 헤더: `Content-Type: application/json; charset=utf-8`, `Access-Control-Allow-Origin: *`(브라우저 기반 도구용. 운영 사이트는 이미 정적 응답에 `access-control-allow-origin: *` 를 주는 것을 관찰했으나 Route Handler 응답에서는 명시한다), `ETag` = `source.json` 의 정규화 파일 SHA-256 앞 부분 등 안정 값.
- 캐시: 내용은 배포 단위로만 바뀐다. 제안 `Cache-Control: public, max-age=3600, s-maxage=86400, stale-while-revalidate=604800`. R-BMS 클라이언트는 캐시·조건부 요청을 쓰지 않으므로(저장본 + 수동 재조회) 영향이 없고, 브라우저·CDN 용이다. Vercel 이 배포마다 캐시를 새로 쓰는지는 이 조사에서 확인하지 못했다. 안전하게 짧은 `s-maxage` 를 쓰려면 `3600` 으로 낮춘다.
- 페이지 `/tables/insane1` 은 정적 프리렌더(`cacheComponents: true` 환경에서 동적 데이터 접근 없음). 페이지의 표 데이터는 TanStack Query 로 `data.json` 을 다시 받지 않고 서버 컴포넌트가 props 로 내려도 된다(구현 판단).
- 출력 안전: 제목·아티스트·comment 는 일반 텍스트로만 렌더(12곡에 `<`/`>`). 링크는 §3.4 규칙.

### 7.4 R-BMS 의 기본 등록

현재 구조(읽은 코드):

- 표 목록은 설정 `Config.library.tables: Vec<TableSource>`(`crates/rbms-config/src/schema.rs`, `TableSource { name, location }`) 에 저장된다. 기본값은 빈 목록(`LibraryOptions::default`).
- 같은 표인지는 **`location` 문자열의 완전 일치**로만 본다(`stage/tables.rs` `add_source`, `lib.rs` 의 `--table` 처리). 정규화 없음.
- 표는 설정 폴더 `tables/<주소 해시>.json` 에 저장본으로 남고, 목록에서 지우면 저장본도 지운다(`forget_table`).
- 시작 때 저장본이 없는 주소 표는 `NotFetched` 가 되고 곡 목록이 뜬 뒤 한 번 받아 온다(`start_first_fetches`). 실패하면 그 실행에서는 `Failed` 로 남고 사용자가 R 로 다시 받는다.
- `--table <주소>` 실행 인자(`lib.rs`)는 목록에 없으면 추가한다. 앞으로 deep link 로 쓸 수 있다.
- 서버 설정 동기화(`ir_sync.rs` `keep_local`)는 `library.tables` 를 **로컬 값으로 유지**하고 업로드에서도 기본값으로 비운다. 즉 표 목록은 기기별이고 계정으로 가지 않는다. (사용자 폴더 기능이 "읽기 전용 표 같은 동작"이면 이 경로와 별개로 서버에서 오는 폴더가 된다. 그것은 A1/A2 의 몫이다.)

기본 등록 선택지:

| 안 | 방법 | 장점 | 단점 |
|---|---|---|---|
| A | `LibraryOptions::default().tables` 에 사본 항목을 넣음 | 코드 몇 줄 | **새 설치(설정 파일 없음 또는 파싱 실패)에만** 들어감. v0.1.0 사용자는 못 받음. 파싱 실패로 기본값이 쓰이면 지웠던 표가 되살아남 |
| **B(추천)** | `LibraryOptions` 에 `seeded_default_tables: Vec<String>`(기본 빈 목록, `#[serde(default)]`)을 추가. 시작 때(`lib.rs` 의 `--table` 처리 근처, 곡 스캔 전) 기본 표 목록을 돌며 id 가 표식에 없으면 (1) 별칭이 이미 목록에 있으면 넣지 않고 (2) 없으면 `TableSource { name: "", location }` 을 추가한 뒤 (3) id 를 표식에 넣고 저장 | 새 설치와 기존 사용자 모두 한 번만. 사용자가 지우면 표식 때문에 다시 안 들어감. 표를 더 늘릴 때 같은 방식 재사용 | 필드 추가. **표식도 `keep_local` 에 넣어야 한다**(안 넣으면 서버 설정 다운로드가 표식을 빈 목록으로 덮어 지운 표가 되살아남). 기본 설정 스키마 번호(`CURRENT_SCHEMA_VERSION`=5)는 serde default 라 올리지 않아도 읽히지만, 올릴지는 구현 웨이브가 `migrate` 규칙으로 판단 |
| C | 앱에 사본을 내장(`include_bytes!` 400 KB)하고 첫 실행에 저장본을 미리 만듦 | 오프라인 첫 실행에도 폴더가 보임 | 내장본과 웹 사본이 어긋남(갱신 경로 둘). 저장본 형식 변환 코드 필요. 보관본이라 갱신이 드문 점은 장점 |

- 등록 항목: `name` 은 빈 문자열로 둔다. 빈 이름이면 표의 이름(`発狂BMS難易度表`)이 표시된다(`pick_table_name`: 영문 대문자화는 일본어에 영향 없음). `location` 은 §7.1 의 정본 `https://bms.hyuns.uk/tables/insane1`. 이 문자열은 저장본 파일 이름의 해시 입력이라 한 번 배포하면 바꾸지 않는다(바꾸면 저장본이 고아가 되어 새로 받아 온다. 피해는 작다).
- 오프라인 첫 실행: B 안에서는 `NotFetched` -> 곡 목록 후 받기 실패 -> 폴더 없음 + 표 관리자에 실패 이유. 이후 R 로 복구 가능. 중요하다면 C 를 추가로 고려한다.

### 7.5 같은 표임을 알아보는 기준 (기존 사용자의 원본 주소 등록)

사용자는 `https://darksabun.club/table/archive/insane1/` 를 직접 등록해 쓰고 있을 수 있고, 다른 사이트의 같은 표(예: 다른 BMS 표 저장소의 발광 난이도표)도 있다. 문자열 비교만으로는 같은 표가 둘이 된다.

- 권장: **주소 정규화 키 + 별칭 목록**(순수 함수, 단위 테스트 용이).
  - 정규화: 스킴 무시(http/https 동일), 호스트 소문자, 끝 슬래시 제거, 끝 세그먼트가 `index.html` / `header.json` / `data.json` 이면 제거, 쿼리·해시 유지.
  - 별칭 집합(발광 표 id `insane1`): `darksabun.club/table/archive/insane1`, `bms.hyuns.uk/tables/insane1`. 정규화 키가 별칭에 들어가면 같은 표로 본다. 예: `https://darksabun.club/table/archive/insane1/header.json` 도 같은 키가 된다.
  - 쓰는 곳: (1) 기본 등록 때 이미 별칭이 목록에 있으면 넣지 않는다(§7.4 B), (2) 표 관리자에서 수동 추가할 때 같은 표 경고(선택).
- 내용 기준(보조, 구현 비용 큼): 받은 표의 md5 집합이 기존 표와 대부분 같은지. 로드 후 비교해야 해서 기본 등록 판단에는 쓸 수 없다. 이번 범위에서는 쓰지 않는다.
- 다른 사이트 주소(별칭에 없는 것)는 알 수 없어 둘로 보이는 경우가 남는다. 이는 사용자가 목록에서 지우면 된다. 다른 사이트의 주소 후보는 이 조사에서 확인하지 못했다.
- 이미 원본 주소를 쓰는 사용자는 별칭 덕분에 사본을 추가로 받지 않는다. 이 사용자를 사본 쪽으로 옮기지는 않는다(원본 보관본은 갱신이 없어 데이터가 같고, 원본이 내려갈 때만 문제).

### 7.6 클라이언트가 이 스냅샷을 읽을 때의 호환성 점검 (코드 읽기 근거)

- 페이지 메타: 루트 상대 경로 `content` 는 `join_url` 로 처리된다(§1.2, 테스트 있음).
- header: `data_url` 문자열 하나, `level_order` 26개, `symbol` `★`, `name` 모두 `TableHeader` 가 그대로 읽는다. `last_update`, `original_url` 은 무시.
- body: 1035개 전부 `md5` 가 있어 `names_a_chart` 통과(`skipped` 0이 될 것으로 예상). 모든 값이 문자열이라 `lenient_text` 변환이 필요 없다. `comment`, `name_diff`, `lr2_bmsid` 는 무시된다.
- 레벨: header 순서와 본문 레벨이 일치해 `by_level` 폴더가 `1`..`25`, `???` 순이다. 라이브러리에 곡이 없는 레벨도 폴더로 남는다(`match_levels_in` 설명).
- 실제 `cargo` 로 이 스냅샷을 읽어 본 것은 아니다(§8).

## 8. 확인하지 못한 것

- 스냅샷을 R-BMS(`rbms-table`)로 실제 파싱한 결과(`skipped` 0, 폴더 26개): 조사 워크플로의 범위와 빌드 시간을 고려해 코드 읽기로만 판단했다. 구현 웨이브에서 `DifficultyTable::parse_body` 로 한 번 확인한다.
- 사본 URL 이 실제로 배포되었을 때의 동작(메타가 `<head>` 에 나오는지, 308 이후 해석, Vercel 캐시 헤더): 아직 구현되지 않았다.
- Next 16 `metadata.other` 가 `<meta name="bmstable" ...>` 를 내는지, 정적 프리렌더에서 head 에 위치하는지: 문서를 읽지 않았다.
- Vercel 의 배포 단위 CDN 캐시 무효화 동작: 확인하지 않았다.
- `original_url`(`nekokan.dyndns.info/~lobsak/genocide/insane.html`)의 현재 상태, 작성자, 라이선스: 지시에 따라 받지 않았다. 원 운영자의 허락 여부는 알 수 없다.
- 링크 URL 의 생존 여부: 요청대로 확인하지 않았다(2640개 중 대부분이 오래된 `http://` 호스트라 상당수가 죽어 있을 가능성은 추정일 뿐).
- 다른 사이트에 있는 같은 발광 표의 주소와 내용이 이 보관본과 같은지: 확인하지 않았다.
- `web/.prettierignore` 에 `data` 가 없을 때 `format:check` 가 실제로 실패하는지: 파일 내용으로 추정했고 돌려 보지 않았다.
