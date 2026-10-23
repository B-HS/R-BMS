# 곡 미러 준비 — 권리 조사 기록, R2 사실, 묶음 도구와 모달 자리 설계 (F7, 결정 17)

> 최종 갱신 2026-10-23 · 대응 커밋 1169efb
> 담당 체크리스트: [PROCESS.md](../PROCESS.md) "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" 의 F7. 조사 원본: [reference/2026-10-23-stella-course-dan-survey.md](../reference/2026-10-23-stella-course-dan-survey.md) 7절·A-3·B-3.
> 이 문서는 **업로드를 하지 않는다**. 실행·업로드·계정·토큰 사용은 사용자 승인 뒤의 단계(M1~M4)로 남기고, 지금 구현하는 것은 U1(로컬 묶음 도구)과 U2(F4 모달의 자리 계약)뿐이다.
> 이 문서를 쓰는 동안 곡 폴더(`/Volumes/SSD/bms`)는 열지 않았다. 용량은 실측이 아니라 공개 값으로 추정했다(7절).
> 설계 문서 필수 절 대응: 범위 1절, 현재 코드 사실 2절, 설계 5절(권리 3절·공식 사실 4절·비용 7절 포함), 원본 대비 차이 6절, 구현 단위 8절, 테스트 9절, 문서 갱신 10절, 메인 결정 필요 11절, 남은 위험·부채 14절. 사용자 할 일 12절, 마지막 보고 초안 13절.

---

## 1. 범위와 결정 번호 대응

### 1.1 사용자 원문

- 요구 7(2026-10-23): "현재 가지고있는 발광 전곡들도 내 서버에 올리면 좋을듯한데 .. 올려서 stella 의 다운로드 방식을 갖추면 좋을듯해. cloudflare 에 올리면 되니까 검토해보고"
- 결정 17(2026-10-23): "아마 전곡 다 업로드가능할건데 일단 업로드할 준비만해두고 조사한 기록을 나한테 맨마지막에 알려줘"

### 1.2 이 문서가 다루는 것과 다루지 않는 것

| 결정 | 이 문서에서 | 다른 묶음 |
|---|---|---|
| 17 곡 미러는 준비만, 조사 기록은 마지막 보고 | 전부. 권리 기록(3절), R2·Bun 공식 사실(4절), 준비 도구 U1(5.2~5.5), 업로드 이후 단계 M1~M4 의 설계만(5.6~5.9), 사용자 할 일(12절), 마지막 보고 초안(13절) | — |
| 15 직접 zip 링크만 앱이 받아 풀고(새 의존성 `zip` 승인) rar·7z·HTML 은 브라우저, 모달에 "차분만, 음원은 곡 URL" 안내 | 미러 묶음을 **zip 으로만** 만든다(클라이언트 해제기를 그대로 쓰게). 모달에 [R-BMS 미러에서 받기] 를 더할 자리 계약(U2) | F4 가 모달·내려받기·해제기를 소유 |
| 16 표 레벨 안 흐린 없는 곡 막대 + 전용 다운로드 폴더(곡 폴더 목록에 자동 포함) | 미러에서 받은 묶음도 같은 다운로드 폴더로 푼다(M3) | F4 |
| 13 Vercel Cron 이 원본을 받아 운영 DB 에 저장 | 관계 없음(미러는 표 갱신과 독립). 마이그레이션 번호만 F6 다음으로 잡는다(M2) | F6 |
| 14 Stella·단위는 출처 표기, 발광 insane1 은 흔적 제거 유지 | 미러 버튼 문구에 외부 사이트 이름을 넣지 않는다(5.9) | F3·F6 |

판정·점수 의미에는 닿지 않는다(곡 파일을 옮기는 일뿐이다). 램프 id·와이어 램프 이름·스킨 id·노트 색·화소에도 닿지 않는다.

---

## 2. 현재 코드 사실(2026-10-23, HEAD 1169efb 에서 다시 확인)

### 2.1 다시 확인한 것

| 사실 | 근거(파일:줄) |
|---|---|
| 차트 md5·sha256 은 차트 파일 **원 바이트**의 해시다(정규화 없음) | `crates/rbms-parser/src/lib.rs:135-137` (`Md5::digest(bytes)`, `Sha256::digest(bytes)`), bmson 은 `crates/rbms-parser/src/bmson.rs:349` |
| 차트 파일 판정은 확장자 bms·bme·bml·pms·bmson | `crates/rbms-library/src/lib.rs:61-63` `is_chart` |
| `BmsSource` 는 `wav`·`bmp` 정의 표를 공개 필드로 가진다(자원 경로 검사에 쓸 수 있다) | `crates/rbms-parser/src/lib.rs:113-123` |
| 문자 인코딩 판정 `decode_text` 는 `pub(crate)` 라 다른 크레이트가 못 쓴다 | `crates/rbms-parser/src/encoding.rs:65` |
| 라이브러리를 걷는 기존 도구 `rbms-cli chart-pattern` 이 있다. 깊이 우선, 심볼릭 링크를 따라가지 않음, 경로 순 정렬, 같은 md5 는 경로 순 첫 파일 | `apps/rbms-cli/src/chart_pattern/walk.rs:18-48`, `apps/rbms-cli/src/chart_pattern.rs:144-198` |
| 같은 도구의 "같은 바이트면 다시 쓰지 않음 + 임시 파일 뒤 이름 바꾸기" | `apps/rbms-cli/src/chart_pattern.rs:93-104` `store` |
| 표 본문 읽기 `DifficultyTable::parse_body` (알 수 없는 키 무시, Stella `score.json` 모양도 읽힘 — 조사 보고 4절) | `crates/rbms-table/src/lib.rs:122-125`, 표 md5 수집 예 `apps/rbms-cli/src/chart_pattern/coverage.rs:24-38` |
| `rbms-cli` 의존성: rbms-model·parser·chart·config·library·store·skin·table·serde·serde_json. zip·sha2·md-5·encoding_rs 는 없다 | `apps/rbms-cli/Cargo.toml:12-22` |
| 워크스페이스에 `md-5 = "0.10"`, `sha2 = "0.10"`, `encoding_rs = "0.8"` 가 이미 있다 | `Cargo.toml` `[workspace.dependencies]` |
| 하위 명령 분기와 USAGE 문자열, 그 문자열을 보는 테스트 | `apps/rbms-cli/src/main.rs:20-29`, `:45-75`, 테스트 `:368`, `:383-385` |
| 테스트 임시 폴더 관례: `std::env::temp_dir()` + 프로세스 id + 일련번호, 끝나면 지움 | `apps/rbms-cli/src/chart_pattern/tests.rs:61-69`, `:130` |
| 웹 관리 업로드 스크립트의 토큰 관례: macOS 키체인(`security find-generic-password -a … -s … -w`)에서 실행 중에만 읽고 출력·기록하지 않음, `--dry-run` | `web/scripts/upload-chart-patterns.ts:9-10`, `:80-87`, `:37-39` |
| 웹은 Vercel **Node** 런타임이다(`vercel.json` 에 `bunVersion` 없음) → 서버 코드에서 `Bun.S3Client` 를 쓸 수 없다 | `web/vercel.json` 전체 |
| 웹 bun 버전 1.4.0 | `web/package.json` `packageManager` |
| 저장 추상화는 리플레이 하나, `REPLAY_STORAGE='blob'` 은 자리만 있고 컴포지션이 던진다. `@vercel/blob` 의존 없음 | `web/src/server/service/shared/storage/replay-storage.ts:6-10`, `web/src/server/compose/replay.ts:44-47`(던지는 줄 46), `web/src/server/lib/env.ts:19-20` |
| 레이트리밋은 **인스턴스 메모리 Map** 이라 서버리스 인스턴스마다 따로 센다 | `web/src/server/lib/with-rate-limit.ts:4-14`(한도표), `:20`(Map), `:57-60`(`enforceUserRateLimit`) |
| 토큰 또는 세션 인증 HOF `withBearerOrSession` | `web/src/server/lib/with-api-token.ts:19-27` |
| 프로덕션 CSP `connect-src 'self'` (화면 fetch 는 자기 출처만. 링크 이동·다운로드는 이 지시의 대상이 아니다) | `web/next.config.ts:3-14`, `:21` |
| 웹 표 상세 위젯은 `url`(본편)·`url_diff`(차분) 버튼을 그린다. 미러 버튼 자리는 이 줄 묶음 | `web/src/widgets/table/table-chart-details.tsx:46-67` |
| 클라이언트에는 아직 없는 곡 모달·내려받기·해제기가 없다(F4 가 만든다) | 조사 보고 5절, `Command::new`·zip 코드 0건(이번에 `grep` 으로 다시 확인) |

### 2.2 원본(beatoraja)의 같은 기능 — 이번에 새로 찾은 사실

조사 보고는 다루지 않았지만 beatoraja 에는 "표 항목에 적힌 내용 주소로 곡 묶음을 받아 전용 폴더에 푸는" 기능이 이미 있다. 미러 설계의 기준으로 삼는다.

| 원본 동작 | 근거 |
|---|---|
| 표 항목의 `ipfs`(곡 묶음)·`appendipfs`(차분) 필드를 곡 데이터에 옮긴다 | `beatoraja/src/bms/player/beatoraja/TableDataAccessor.java:243`, `:248` |
| 선곡 명령 `DOWNLOAD_IPFS`: 지금 표가 허용 도메인 목록(stellabms.xyz, nekokan.dyndns.info 등 11개)일 때만, 고른 곡에 `ipfs` 가 있으면 내려받기 시작 | `beatoraja/src/bms/player/beatoraja/select/MusicSelectCommand.java:94-124` |
| 내려받기 폴더 `ipfs/` 를 만들고 곡 폴더 목록(roots)에 자동으로 넣는다 | `beatoraja/src/bms/player/beatoraja/MainController.java:147-153` |
| 묶음 형식은 tar.gz(commons-compress), 폴더 이름은 `ipfs/[아티스트]제목`(금지 문자 제거). 원곡(`orgmd5`)이 이미 있으면 차분만 받는다 | `beatoraja/src/bms/tool/mdprocessor/MusicDownloadProcessor.java:22-23`, `:115-156` |
| 기본값 `enableIpfs = true`, 게이트웨이 `https://gateway.ipfs.io/` | `beatoraja/src/bms/player/beatoraja/Config.java:181-182` |

### 2.3 조사 보고 정정과 보충

- 7절 "요금 수치는 확인하지 못했다" → 이번에 공식 문서로 확인했다(4절).
- 7절 "bms.hyuns.uk 는 Cloudflare 를 거치지 않는다" 는 맞다. 다만 **도메인 `hyuns.uk` 의 이름 서버는 Cloudflare 다**(`dig NS hyuns.uk` → `jason.ns.cloudflare.com`, `cloe.ns.cloudflare.com`, 2026-10-23 관찰. `bms.hyuns.uk` 는 Vercel 로 가는 DNS 전용 레코드). 즉 그 Cloudflare 계정에 R2 버킷을 만들면 `mirror.hyuns.uk` 같은 사용자 지정 도메인을 붙일 수 있다(4.4). 같은 계정인지는 확인 못함(사용자 확인 Q-C1).
- 7절 `web/src/server/compose/replay.ts:45-46` 는 45가 환경 읽기, 46이 던지는 줄이다(뜻은 같다).
- A-3 의 GENOSIDE 감사 문구는 맞다. 문맥을 보충한다: Starter Package 는 "基本的にBMSは公開当初のまま収録させて貰う形でパッケージ化", Append 는 "差分作者の方々へ許可を頂きパッケージ化させて貰いました"(3.1).
- 조사 보고에 없는 사실: beatoraja 의 IPFS 내려받기(2.2).

---

## 3. 권리 조사 기록과 사용자 확인 질문

### 3.1 원문 인용(2026-10-23 관찰, 사본은 세션 scratch `web/` 아래)

**Stella** (https://stellabms.xyz/rules, 사본 `page_rules.html`)
- "곡을 추가할 시, 올바른 곡 URL 및 차분 URL을 제시하여 주십시오."
- "상업용 곡의 BMS, 무키음 BMS, osu convert 등 저작권에 위배되는 곡의 제안을 금지합니다."
- 3-1 곡의 저작권: "모든 사항에 대해서는 기본적으로 미디어를 보유하고 있는 개인/단체/기업 등의 가이드라인(2차 창작 가이드 또는 이용 약관)을 참고하는 것으로 합니다." / "복사/리믹스/어레인지에 관하여, 비상업적 용도에 대해 허가하지 않는 경우 난이도표에서 취급하지 않습니다."
- 5 차분 업로더 규칙: "자기자신이 만든 차분만 업로드해주세요. (단, 차분제작자가 허가한 경우 대리 업로드가 가능한 것으로 합니다.)" / "저작권에 위배될수 있는 차분의 업로드를 금지합니다."
- https://stellabms.xyz/introduce (사본 `page_introduce.html`) 차분 업로더: "자신이 제작한 BMS 차분을 업로드 하는 곳입니다. 별도로 가입하지 않아도 업로드 할 수 있습니다."
- https://stellabms.xyz/link: "각 난이도표의 링크는 난이도표 내용을 따로 출력하지 않으며, 난이도표 내부의 곡 링크, 차분 링크는 곡 정보 페이지를 참고하거나, BeMusicSeeker에 등록 후 해당 프로그램에서 확인을 부탁드립니다."
- Stella 가 호스팅하는 것은 차분(`stellabms.xyz/upload/<번호>`, 미러 예 19~38KB)이고 음원 패키지가 아니다(조사 보고 A-3). 표 JSON·업로드 파일의 재배포 허락·금지 문구는 찾지 못했다.

**GENOCIDE(발광 표 운영)** (https://nekokan.dyndns.info/~lobsak/genocide/ about, 사본 `gen_about.html`, Shift_JIS)
- "表自体はLR2IR上の情報を取得し表にしているだけです。"
- "またこちら側で入手不可能なbms、差分の保管は原則しません。"
- "またリンク先で生じた問題は一切責任を負いません。"
- 入手不可になったBMSの扱い: "各自保管し合い入手できる様にするのは自由ですが、作者の方に連絡取れるなら取ってからしましょう。" / "そして表に掲載されている物を保管するというのはその後も長くその状態を保つ事が可能かどうかちゃんと考えましょう"
- 表の共通ルール: "解析音源使用したと確定しているものについても当然扱わない。" / "キー音なしBMSは権利的にクリアで有っても全面的に扱わない。"

**GENOSIDE 2018 패키지 페이지** (https://nekokan.dyndns.info/~lobsak/genocide/grade.html, 사본 `https___nekokan_dyndns_info__lobsak_genocide_grade_html.html`)
- "GENOSIDE -BMS Starter Package 2018- 容量:6.16GB", "…Append 容量:125MB", "BMS段位認定の為の全159曲のパッケージです", "*一部前回のGENOSIDE-StarterPackage-が必要になります"
- Packageについて: "Starter Package 基本的にBMSは公開当初のまま収録させて貰う形でパッケージ化しています。一部作者の方要望により音量調整や譜面追加・BGA追加が行われております。" / "Append Package 差分作者の方々へ許可を頂きパッケージ化させて貰いました。"
- THANKS: "BMSパッケージ化に許可頂いた作者様 / 差分作者様 / コースや実際の許可取り等の調整に多くのプレイヤーの方々にご協力頂き"
- "©GENOCIDE 2013 All rights reserved"

**BMS-IR 약관** (https://www.bms-ir.org/terms, 규약 버전 2026-06-13-1, 사본 `bmsir_terms.html`)
- "個人が運営している非公式の互換βサービスです。旧LR2IRの公式後継ではありません。"
- 禁止事項: "サーバーやネットワークへ過度の負荷をかける行為", "解析譜面や、権利侵害のおそれがある楽曲・譜面データの登録・投稿・配布"
- 配布ファイル: "配布ファイルに関する権利は各権利者に帰属します。"

### 3.2 읽은 것의 요약(법적 판단 아님)

1. 위 어느 출처도 **제3자(R-BMS)가 곡·차분 파일을 다시 배포해도 된다**고 말하지 않는다. 반대로 금지한다는 문구도 R-BMS 에 직접 적용되는 것은 없다(BMS-IR 약관은 BMS-IR 서비스 안의 행위를 다룬다).
2. GENOSIDE 패키지는 "작자의 허가를 받아" 묶었다고 밝힌다. 그 허가는 GENOCIDE 운영자가 받은 것이고, 그 묶음을 남이 다시 올려도 된다는 뜻까지는 문구에 없다.
3. GENOCIDE 는 입수 불가 곡의 보관·공유를 "자유" 라고 하면서 "작자에게 연락할 수 있으면 먼저 하라" 고 한다. 이것이 사용자가 말한 "아마 전곡 다 업로드가능할건데" 의 가장 가까운 근거로 보인다(사용자 확인 Q-R1).
4. Stella 의 대리 업로드 규칙("차분제작자가 허가한 경우")을 미러에 비추면, Stella 업로드 차분을 복제하는 것도 제작자 허가가 기준이다.
5. 곡마다 들어 있는 readme(예: 再配布禁止)는 곡 폴더를 읽지 않아 확인하지 못했다. U1 이 각 묶음의 텍스트 파일 이름과 재배포 관련 낱말이 든 파일을 매니페스트에 적어(5.3 `notice_files`), 사용자가 업로드 전에 확인할 수 있게 한다.

### 3.3 사용자에게 확인받을 질문(마지막 보고에 묶는다)

| 번호 | 질문 | 선택지(추천 먼저) |
|---|---|---|
| Q-R1 | "업로드 가능" 이라고 본 근거는? | **A. 작자·차분 작자에게 허락을 받았거나 받을 수 있다** / B. GENOCIDE about 의 "各自保管し合い…自由" 를 근거로 본다 / C. 원 배포처가 무료 공개 배포라서 / D. 기타(적어 주세요) |
| Q-R2 | 올릴 범위 | **A. 発狂 표(insane1) 1,035 md5 가 든 곡 폴더 전부**(2026-10-20 같은 SSD 에서 1,035/1,035 매칭됨, PROCESS AI-W3) / B. GENOSIDE 2018 수록 159곡만 / C. U1 이 `notice_files` 를 낸 묶음은 빼고 / D. Stella·Satellite 표도 포함 |
| Q-R3 | 누가 받을 수 있나 | **A. R-BMS 에 로그인한 사용자만(서명 URL)** / B. 누구나(공개 도메인) |
| Q-R4 | 삭제 요청 창구 | **A. 웹 안내 페이지에 "작자 요청 시 즉시 내림" 과 연락처(GitHub 이슈 또는 메일)** / B. 문구 없이 운영 |
| Q-R5 | 작자·GENOCIDE 운영자에게 미리 알리기 | **A. 알리지 않고 Q-R4 창구로 대응** / B. 먼저 연락(문안은 초안 승인 뒤에만 게시 — 외부 게시 규칙) |

---

## 4. Cloudflare R2 와 Bun S3 — 공식 문서 사실(2026-10-23 WebFetch)

### 4.1 가격·무료 한도·이그레스 — https://developers.cloudflare.com/r2/pricing/ (페이지 갱신일 2026-10-01)

| 항목 | Standard | Infrequent Access |
|---|---|---|
| 저장 | $0.015 / GB-month | $0.01 / GB-month |
| Class A(쓰기·목록) | $4.50 / 백만 요청 | $9.00 / 백만 요청 |
| Class B(읽기) | $0.36 / 백만 요청 | $0.90 / 백만 요청 |
| 무료(월) | 저장 10 GB-month, Class A 100만, Class B 1,000만 (Standard 에만) | 없음 |
| 이그레스 | 무료(S3 API·Workers·r2.dev 모두) | 무료, 단 꺼낼 때 $0.01/GB, 최소 보관 30일 |

- Class A: ListObjects, PutObject, CopyObject, CreateMultipartUpload, UploadPart, CompleteMultipartUpload, ListParts, ListMultipartUploads, ListBuckets, PutBucket 등.
- Class B: GetObject, HeadObject, HeadBucket, UsageSummary.
- 무료 작업: DeleteObject, DeleteBucket, AbortMultipartUpload.
- 결론: 곡 미러는 **Standard** 로 둔다(IA 는 내려받을 때마다 요금, 30일 최소).

### 4.2 한도 — https://developers.cloudflare.com/r2/platform/limits/ , https://developers.cloudflare.com/r2/objects/multipart-objects/

- 객체 하나 최대 5 TiB(각주: 4.995 TiB), 한 번의 PUT 최대 5 GiB, 파트 최대 10,000개.
- 멀티파트 파트: 최소 5 MiB(마지막 제외), 최대 5 GiB, 마지막을 뺀 모든 파트는 같은 크기. 끝나지 않은 멀티파트는 기본 7일 뒤 자동 중단.
- 멀티파트 객체의 ETag 는 파트 md5 들을 이은 해시 + `-파트수` 라 객체 md5 가 아니다 → 무결성은 매니페스트의 sha256 으로 본다(5.3).
- 객체 키 최대 1,024 바이트, 메타데이터 8,192 바이트, 같은 키 동시 쓰기 초당 1회(넘으면 429), 계정 REST API 5분에 1,200회.
- r2.dev 공개 주소: "variable rate limit", 넘으면 429, "not intended for production usage".

### 4.3 S3 호환 API·서명 URL·토큰

- 엔드포인트 `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`, 지역 값 `auto`(빈 값·`us-east-1` 도 auto 로 취급) — https://developers.cloudflare.com/r2/api/s3/api/
- PutObject 는 `Content-MD5` 를 받는다. `x-amz-checksum-sha256` 은 문서 표에 없다(확인 못함). 태그·ACL 헤더·SSE 는 지원하지 않는다.
- 서명 URL — https://developers.cloudflare.com/r2/api/s3/presigned-urls/ : SigV4, GET·HEAD·PUT·DELETE(POST 폼 업로드 미지원), 유효 1초~7일(604,800초), **S3 엔드포인트에서만 동작하고 사용자 지정 도메인에는 못 쓴다**, PUT 은 `Content-Type` 을 서명에 넣어 고정할 수 있다, "Treat presigned URLs as bearer tokens".
- 브라우저가 다른 출처의 서명 URL 을 fetch 하면 버킷 CORS 가 필요하다 — https://developers.cloudflare.com/r2/buckets/cors/ (링크 이동·다운로드만 하면 화면 fetch 가 아니므로 이 설계에서는 필요 없다. M4 는 링크 이동만 쓴다).
- API 토큰 — https://developers.cloudflare.com/r2/api/tokens/ : 권한 Admin Read & Write / Admin Read only / **Object Read & Write** / **Object Read only**, Object 권한은 버킷 단위로 좁힐 수 있다, 발급 화면에 Access Key ID 와 Secret Access Key 가 나오고 **Secret 은 한 번만 보인다**.

### 4.4 공개 버킷과 사용자 지정 도메인 — https://developers.cloudflare.com/r2/buckets/public-buckets/

- r2.dev: 개발용, 속도 제한, WAF·캐시·접근 제어·Bot Management 를 쓸 수 없다.
- 사용자 지정 도메인: 버킷과 **같은 계정의 zone** 이어야 한다. Cloudflare 캐시(기본은 일부 파일 형식만, 전부 캐시하려면 Cache Everything 규칙), WAF 사용자 규칙, Zero Trust Access·WAF 토큰 인증으로 접근 제한 가능. 서명 URL 대신 쓰려면 WAF HMAC 검증이 필요하고 Pro 이상(서명 URL 페이지).
- WAF 레이트리밋 규칙(Free 플랜) — https://developers.cloudflare.com/waf/rate-limiting-rules/ : 규칙 1개, 기준 IP 만, 집계 10초, 차단 10초.

### 4.5 Bun 내장 S3 클라이언트 — https://bun.com/docs/runtime/s3 , https://bun.com/blog/bun-v1.2

- Bun 1.2 에서 내장 S3 지원이 들어왔다("Bun 1.2 adds built-in support for S3"). 저장소의 bun 은 1.4.0 이라 쓸 수 있다.
- `new S3Client({ accessKeyId, secretAccessKey, bucket, endpoint })` — R2 예제가 문서에 있다(`endpoint: "https://<account-id>.r2.cloudflarestorage.com"`).
- `presign()` 은 동기·네트워크 없음(옵션 `expiresIn` 초, 기본 GET 24시간, `method`, `type`, `contentDisposition`, `acl`).
- 큰 파일 쓰기는 자동 멀티파트(`writer({ partSize, queueSize, retry })`).
- **주의**: Bun 은 `S3_ACCESS_KEY_ID`·`S3_SECRET_ACCESS_KEY`·`S3_ENDPOINT`·`S3_BUCKET`(없으면 `AWS_*`)를 **`.env` 파일과 환경에서 자동으로 읽는다**. 업로드 스크립트는 모든 값을 생성자에 명시해 넘기고, `.env` 에 기대지 않는다(에이전트는 `.env` 를 읽지 않는다).
- Vercel 의 Bun 런타임은 베타(`vercel.json` `bunVersion`)다 — https://vercel.com/docs/functions/runtimes/bun . 운영 웹 런타임을 바꾸지 않으므로(Node 유지), 서버의 서명 URL 은 `node:crypto` 로 만든다(5.7, 새 의존성 없음).

---

## 5. 설계

### 5.1 전체 흐름과 단계

흐름(순서대로):
1. U1(지금): 로컬 SSD 곡 폴더를 `rbms-cli mirror-pack` 이 읽어 결과 폴더에 `packs/<sha256>.zip` 과 `manifest.json` 을 쓴다.
2. M1(승인 뒤): `web/scripts/upload-song-mirror.ts` 가 zip 을 R2 버킷의 같은 키(`packs/<sha256>.zip`)로 올린다.
3. M2(승인 뒤): 매니페스트를 운영 DB `song_mirror_pack`·`song_mirror_chart` 에 등록하고, `POST /api/mirror/charts/:md5/link` 가 10분짜리 서명 GET URL 을 준다.
4. M3(승인 뒤): 클라이언트 모달의 [R-BMS 미러에서 받기] 가 그 URL 을 F4 내려받기·해제기에 넘겨 다운로드 폴더에 푼다.
5. M4(승인 뒤): 웹 표 상세의 [R-BMS 미러에서 받기] 가 `/api/mirror/charts/:md5/download` 로 가고, 서버가 서명 URL 로 302 를 준다.

| 단계 | 내용 | 지금 |
|---|---|---|
| U1 | 로컬 묶음 도구 `rbms-cli mirror-pack` (네트워크 없음) | 구현 |
| U2 | F4 모달의 미러 버튼 자리 계약(코드 소유는 F4) | F4 구현에 반영, F8 리뷰에서 확인 |
| M1 | 업로드 스크립트(키체인 자격 증명, 기본 dry-run) | 설계만. Q-R1~R5 답 + 버킷·토큰 준비 뒤 |
| M2 | 마이그레이션·관리 등록 라우트·서명 URL 라우트·환경변수 | 설계만 |
| M3 | 클라이언트 모달 버튼과 내려받기 연결 | 설계만 |
| M4 | 웹 표 상세 버튼 | 설계만 |

### 5.2 U1 `rbms-cli mirror-pack` — 명령줄

```text
rbms-cli mirror-pack <library folder> --out <folder> (--table <data.json>)... [--all] [--dry-run]
```

- `<library folder>`: 걷는 곳. 사용 예: `/Volumes/SSD/bms`(실행은 사용자).
- `--out <folder>`: 결과 폴더. 없으면 만든다. **라이브러리 폴더 안이면 거절**(다음 실행에서 자기 결과를 묶지 않게), 저장소 안 경로는 문서에서 금지한다(저작권. `docs/memory/test-library.md` 의 커밋 금지와 같은 이유). 추천 위치 `/Users/hyunseokbyun/rbms-song-mirror`(chart-pattern 의 `/Users/hyunseokbyun/rbms-chart-patterns` 와 같은 관례).
- `--table <data.json>`: 여러 번 줄 수 있다. 표 md5 중 하나라도 가진 곡 폴더를 묶는다. 発狂 표는 `web/data/tables/insane1/data.json`, Stella 는 받아 둔 `score.json`.
- `--all`: 표와 상관없이 모든 곡 폴더. `--table` 이 하나도 없고 `--all` 도 없으면 사용법 오류(실수로 11,336 차트 전부를 묶지 않게).
- `--dry-run`: zip 을 쓰지 않고 고를 폴더·파일 수·원본 바이트 합·거절 사유만 출력한다(사용자가 실제 용량을 재는 수단, 7절).
- 종료 코드: 성공 0, 사용법 오류 2(`EXIT_USAGE`, 기존과 같음), 실행 실패 1. 묶지 못한 폴더(거절)는 보고에 넣고 실패로 치지 않는다(chart-pattern 의 `Failure` 와 같은 태도).

### 5.3 U1 데이터 모델과 함수

모듈 배치(chart-pattern 과 같은 모양):

```text
apps/rbms-cli/src/mirror_pack.rs            진입: MirrorPackReport, export(), run(), 상수
apps/rbms-cli/src/mirror_pack/args.rs       MirrorPackArgs, parse()
apps/rbms-cli/src/mirror_pack/folders.rs    SongFolder 찾기, md5 색인
apps/rbms-cli/src/mirror_pack/pack.rs       결정적 zip 쓰기, sha256
apps/rbms-cli/src/mirror_pack/manifest.rs   매니페스트 serde 타입, 읽기·쓰기
apps/rbms-cli/src/mirror_pack/notice.rs     텍스트 파일의 재배포 낱말 찾기, NFD 이름 세기
apps/rbms-cli/src/mirror_pack/render.rs     출력 문장
apps/rbms-cli/src/mirror_pack/tests.rs
```

타입(이름은 그대로 쓴다. 필드 문서는 `///` 영어):

```rust
pub struct MirrorPackArgs {
    pub library: PathBuf,
    pub out: PathBuf,
    pub tables: Vec<PathBuf>,
    pub all: bool,
    pub dry_run: bool,
}

pub struct ChartFile {
    pub path: PathBuf,
    pub md5: String,
    pub sha256: String,
}

pub struct SongFolder {
    pub root: PathBuf,
    pub relative: String,
    pub top: String,
    pub charts: Vec<ChartFile>,
}

pub enum Refusal {
    TooLarge { uncompressed_bytes: u64 },
    TooManyEntries { entries: usize },
    NonUtf8Name { path: PathBuf },
    Unreadable { path: PathBuf, reason: String },
}

pub struct MirrorPackReport {
    pub folders_found: usize,
    pub folders_selected: usize,
    pub written: usize,
    pub reused: usize,
    pub refused: Vec<(String, Refusal)>,
    pub duplicates: usize,
    pub tables: Vec<TableCoverage>,
    pub total_bytes: u64,
    pub total_uncompressed_bytes: u64,
}
```

함수 시그니처:

| 모듈 | 시그니처 | 보조 타입 |
|---|---|---|
| `args.rs` | `pub fn parse(rest: &[String]) -> Result<MirrorPackArgs, String>` | — |
| `folders.rs` | `pub fn song_folders_under(library: &Path) -> Walked` | `Walked { folders: Vec<SongFolder>, failures: Vec<Failure> }` (`Failure` 는 chart-pattern 과 같은 모양을 이 모듈에 둔다) |
| `folders.rs` | `pub fn pack_entries(folder: &SongFolder, limits: PackLimits) -> Result<Vec<PackEntry>, Refusal>` | `PackEntry { source: PathBuf, name: String, bytes: u64 }`, `PackLimits { max_uncompressed_bytes: u64, max_entries: usize }`(기본은 5.3 의 상수, 시험이 낮춘 값을 넣는다) |
| `pack.rs` | `pub fn fingerprint(entries: &[PackEntry]) -> Result<String, String>` | 이름·크기·수정 시각(나노초)을 줄로 이은 sha256 |
| `pack.rs` | `pub fn write_pack(entries: &[PackEntry], packs_dir: &Path) -> Result<WrittenPack, String>` | `WrittenPack { sha256: String, bytes: u64, path: PathBuf, reused: bool }` |
| `notice.rs` | `pub fn scan_notices(folder: &SongFolder, entries: &[PackEntry]) -> Notices` | `Notices { text_files: Vec<String>, notice_files: Vec<String>, nfd_names: usize, escapes_root: usize }` |
| `manifest.rs` | `pub fn read_manifest(path: &Path) -> Result<Option<MirrorManifest>, String>` | `MirrorManifest`, `ManifestPack`, `ManifestChart`, `ManifestTable`, `ManifestRefusal`, `ManifestDuplicate`(아래 JSON 과 1:1, `Serialize`·`Deserialize`) |
| `mirror_pack.rs` | `pub fn export(args: &MirrorPackArgs) -> Result<MirrorPackReport, String>` | `TableCoverage { source: String, md5s: usize, matched: usize, missing: Vec<String> }` |
| `mirror_pack.rs` | `pub fn run(args: &MirrorPackArgs) -> ExitCode` | — |

곡 폴더 규칙(원본 beatoraja 의 "곡 = 차트가 든 폴더" 와 같게):
1. `rbms_library::is_chart` 가 참인 파일이 **바로 아래에 하나라도 있는** 폴더가 곡 폴더다. 걷기는 chart-pattern `walk.rs` 와 같다(깊이 우선, 심볼릭 링크를 따라가지 않음, 경로 순 정렬).
2. 묶음 내용 = 곡 폴더 아래 모든 일반 파일(하위 폴더 포함). 단 **하위 폴더가 그 자체로 곡 폴더면 그 하위 트리는 빼고** 따로 묶는다.
3. 빼는 이름(상수 `SKIPPED_FILE_NAMES = [".DS_Store", "Thumbs.db", "desktop.ini"]`, 접두 `SKIPPED_FILE_PREFIX = "._"`, 폴더 `SKIPPED_FOLDER_NAMES = ["__MACOSX"]`). 심볼릭 링크는 넣지 않고 센다.
4. 차트 md5·sha256 은 `md-5`·`sha2` 로 파일 원 바이트를 해시한다(플레이어와 같은 값: 2.1 첫 줄). 테스트로 `rbms_parser::parse(bytes).md5` 와 같은지 묶는다.
5. 표 md5 가 여러 폴더에 있으면 경로 순 첫 폴더를 쓰고 `duplicates` 에 센다(chart-pattern 과 같음). 고른 폴더의 **모든 차트**를 매니페스트에 올린다(같은 폴더의 다른 차분도 미러로 받을 수 있게).

결정적 zip(같은 입력이면 같은 바이트 → 같은 sha256 → 다시 올리지 않음):
- 항목 이름 `<top>/<폴더 안 상대 경로>`, 구분자 `/`, 바이트 순 정렬, 폴더 항목은 쓰지 않음(빈 폴더는 사라진다).
- 수정 시각은 zip 기본 `DateTime::default()`(1980-01-01 00:00:00) 고정, 권한 `PACK_FILE_MODE = 0o644`, 주석 없음, 추가 필드 없음.
- 압축: 이미 압축된 확장자(`STORED_EXTENSIONS = ["ogg", "mp3", "flac", "mp4", "m4a", "mpg", "mpeg", "wmv", "avi", "webm", "png", "jpg", "jpeg", "zip"]`)는 저장(Stored), 나머지(wav·bmp·bms 등)는 Deflate 수준 `DEFLATE_LEVEL = 6`.
- 상한(F4 해제기와 같아야 한다 — 11절 D-3): `MAX_PACK_UNCOMPRESSED_BYTES = 2 GiB`, `MAX_PACK_ENTRIES = 10_000`. 넘는 폴더는 `Refusal` 로 남기고 zip 을 만들지 않는다. 그래서 ZIP64 가 필요 없다.
- 쓰기 순서: `out/packs/.<프로세스 id>-<일련>.zip.tmp` 에 쓰며 sha256 을 함께 계산 → `out/packs/<sha256>.zip` 으로 이름 바꾸기(이미 같은 이름이 있으면 임시 파일 삭제, `reused`).
- 다시 실행: 이전 `manifest.json` 의 같은 `folder` 항목이 같은 `fingerprint` 이고 그 zip 이 같은 크기로 있으면 다시 묶지 않는다(30~40GB 재실행을 몇 분으로).

알림(권리 확인용, 업로드를 막지 않는다):
- `text_files`: 확장자 txt 인 파일 이름 목록.
- `notice_files`: 텍스트를 UTF-8 로 읽고 실패하면 `encoding_rs::SHIFT_JIS` 로 읽어, `NOTICE_KEYWORDS = ["再配布", "二次配布", "転載", "redistribut"]`(대소문자 무시) 중 하나가 든 파일. 크기 `MAX_NOTICE_SCAN_BYTES = 1 MiB` 를 넘는 텍스트는 앞부분만 읽는다.
- `nfd_names`: 이름에 결합 문자(U+3099, U+309A, U+0300~U+036F)가 든 항목 수(macOS 에서 만든 NFD 이름은 Windows 에서 차트의 `#WAV` 이름과 안 맞을 수 있다. 14절 R-4).
- `escapes_root`: BMS 계열 차트를 `rbms_parser::parse` 로 읽어 `wav`·`bmp` 값이 `..` 를 담거나 절대 경로인 수(묶음 밖 파일을 가리킴. bmson 은 세지 않는다).

매니페스트 `out/manifest.json`(JSON, snake_case, 시각 없음 — 결정적):

```json
{
  "format": "rbms-song-mirror-manifest",
  "version": 1,
  "generator": "rbms-cli 0.1.7",
  "tables": [{ "source": "data.json", "md5s": 1035, "matched": 1035, "missing": [] }],
  "packs": [
    {
      "key": "packs/<sha256>.zip",
      "sha256": "<64 hex>",
      "bytes": 41234567,
      "uncompressed_bytes": 52345678,
      "entries": 140,
      "folder": "<라이브러리 기준 상대 경로>",
      "top": "<곡 폴더 이름>",
      "fingerprint": "<64 hex>",
      "charts": [{ "md5": "<32 hex>", "sha256": "<64 hex>", "path": "<top>/<file>.bme" }],
      "notices": { "text_files": ["<top>/readme.txt"], "notice_files": [], "nfd_names": 0, "escapes_root": 0 }
    }
  ],
  "refused": [{ "folder": "<상대 경로>", "reason": "too_large", "detail": "2400000000" }],
  "duplicates": [{ "md5": "<32 hex>", "kept": "<상대 경로>", "also": ["<상대 경로>"] }]
}
```

- 절대 경로를 쓰지 않는다(`/Volumes/...` 가 서버 DB 로 가지 않게). `source` 는 파일 이름만.
- 상수: `MANIFEST_FORMAT = "rbms-song-mirror-manifest"`, `MANIFEST_VERSION = 1`, `MANIFEST_FILE_NAME = "manifest.json"`, `PACKS_FOLDER_NAME = "packs"`, `OBJECT_KEY_PREFIX = "packs/"`, `PACK_EXTENSION = "zip"`.
- 정렬: `packs` 는 `folder` 순, `charts` 는 `path` 순, md5 목록은 사전 순.

### 5.4 객체 키 규칙

- `packs/<sha256 소문자 64자>.zip` 하나뿐(내용 주소. beatoraja 의 IPFS CID 와 같은 생각). 길이 75 바이트 < 1,024.
- 곡 이름·폴더 이름을 키에 넣지 않는다(일본어·특수 문자·사생활, 이름이 바뀌어도 키 불변).
- 같은 내용은 한 객체. 내용이 바뀌면 새 키, 옛 키는 DB 가 더 이상 가리키지 않을 때 정리 명령으로만 지운다(DeleteObject 는 무료).
- 업로드 때 `Content-Type: application/zip`. 내려받을 때 파일 이름은 서명 URL 에 `response-content-disposition`(`attachment; filename="<top>.zip"`)을 넣는다 — R2 가 이 질의 값을 지원하는지는 확인 못함(M2 구현 때 확인, 안 되면 클라이언트가 이름을 정하고 웹은 `<sha256>.zip` 으로 받힌다).

### 5.5 U1 의존성

> 구현과 다름: 워크스페이스의 `zip` 은 9.0.5 가 아니라 9.0.0 이다([ADR](../acknowledge/2026-10-23-adr-missing-charts-and-download.md)).

- `zip`: 결정 15 로 승인된 크레이트를 **F4 가 워크스페이스에 넣은 줄 그대로** `zip = { workspace = true }` 로 쓴다(새 크레이트를 늘리지 않음. 다만 승인 문구가 "해제용" 이라 묶음 쓰기에도 쓰는 것은 11절 D-1 로 메인 확인).
  - crates.io 확인(2026-10-23): 최신 안정 **9.0.5**(2026-10-17, 미철회), MIT, 저장소 https://github.com/zip-rs/zip2 , MSRV 1.88(워크스페이스 1.95 충족), 9.0.0(2026-10-09) 이후 6일 사이 패치 5번.
  - 기본 기능(aes-crypto, bzip2, deflate64, deflate, lzma, ppmd, time, zstd, xz)은 크다. 추천 줄: `zip = { version = "9.0.5", default-features = false, features = ["deflate-flate2-zlib-rs"] }`. 이 경우 일반 의존은 crc32fast·indexmap·memchr·typed-path(+ macOS 등 nix)와 flate2/zlib-rs 이고, **Cargo.lock 에 없는 새 크레이트는 `zip`·`typed-path` 둘뿐**이다(crc32fast 1.5.2, indexmap 2.14.2, memchr 2.8.3, nix 0.31.3, flate2 1.1.10, zlib-rs 0.6.8 은 이미 잠금 파일에 있음. typed-path 의 하위 의존은 확인 못함 → 구현 때 `cargo tree -p rbms-cli -e normal -i typed-path` 로 확인).
  - F4 가 다른 버전·기능을 고르면 그쪽이 우선이다. 쓰기에는 Deflate 압축이 필요하므로 F4 의 해제 기능과 같은 deflate 기능이면 추가 기능이 없다.
- `md-5`, `sha2`, `encoding_rs`: 워크스페이스에 이미 있는 줄을 `{ workspace = true }` 로 rbms-cli 에 더한다(새 크레이트 아님).
- 업로드용 의존성은 Rust 쪽에 더하지 않는다(M1 은 Bun 내장 S3).

### 5.6 M1 업로드 스크립트(설계만, 사용자 승인 뒤)

- 파일 `web/scripts/upload-song-mirror.ts`, 시험 `web/tests/scripts/upload-song-mirror.test.ts`. 추천 근거: Bun 1.2+ 내장 `S3Client` 가 SigV4·멀티파트·재시도를 하므로 새 의존성이 없다(4.5). Rust 로 하려면 SigV4 HMAC 을 sha2 로 손수 짜야 해 코드가 늘어난다(11절 D-8 대안).
- 명령: `bun scripts/upload-song-mirror.ts <out 폴더> --account <ACCOUNT_ID> --bucket <버킷> [--execute]`. **기본은 dry-run**(무엇을 올릴지·이미 있는 것·총 바이트만 출력), `--execute` 가 있어야 올린다.
- 자격 증명: macOS 키체인에서만 실행 중에 읽는다. 계정 `rbms-r2`, 서비스 `rbms-r2-upload-access-key-id`·`rbms-r2-upload-secret-access-key`(기존 `readKeychainToken` 과 같은 `security find-generic-password -a … -s … -w`). 값은 출력·기록하지 않는다. `S3Client` 생성자에 `accessKeyId`·`secretAccessKey`·`bucket`·`endpoint`(`https://<ACCOUNT_ID>.r2.cloudflarestorage.com`)를 모두 명시한다(`.env`·`S3_*` 자동 읽기에 기대지 않음).
- 흐름: 매니페스트를 Zod 로 검증(`format`·`version`·64자 hex) → 객체마다 `client.file(key).exists()`(HEAD, Class B) 후 크기가 같으면 건너뜀 → 아니면 로컬 파일 sha256 을 다시 계산해 매니페스트와 같을 때만 `write(key, Bun.file(path), { type: 'application/zip' })`(큰 파일은 자동 멀티파트, `partSize` 8 MiB 상수 `UPLOAD_PART_BYTES`, 동시 4 `UPLOAD_CONCURRENCY`) → 다시 HEAD 로 크기 확인.
- 비용 안전: 같은 키 동시 쓰기 금지(키가 내용 해시라 한 실행에서 겹치지 않음), Class A 는 객체당 최대 `ceil(bytes / 8 MiB) + 2`.
- 업로드는 운영 외부 상태를 바꾸므로 에이전트가 실행하지 않는다. 사용자가 `! cd web && bun scripts/upload-song-mirror.ts … --execute` 로 직접 실행한다.

### 5.7 M2 서버(설계만)

DB(마이그레이션 번호는 그때의 마지막 다음. F6 이 0012 부터 쓰므로 F6 뒤):

```sql
song_mirror_pack(
  sha256 char(64) PRIMARY KEY,
  object_key varchar(128) NOT NULL UNIQUE,
  bytes bigint NOT NULL,
  uncompressed_bytes bigint NOT NULL,
  entries int NOT NULL,
  top_name varchar(255) NOT NULL,
  is_enabled boolean NOT NULL DEFAULT true,
  created_at timestamp(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
)
song_mirror_chart(
  md5 char(32) PRIMARY KEY,
  pack_sha256 char(64) NOT NULL REFERENCES song_mirror_pack(sha256),
  chart_sha256 char(64) NOT NULL,
  path_in_pack varchar(1024) NOT NULL
)
```

- 새 표만 더하므로 기존 표·와이어 호환에 영향 없음. `is_enabled=false` 가 삭제 요청 처리(Q-R4) 수단이다(객체 삭제는 그 뒤 정리 명령).
- 계층(web/AGENTS.md): `dto/song-mirror.ts`(Zod: 매니페스트 등록 본문, 링크 응답) → `route/song-mirror.ts` → `service/domain/song-mirror/song-mirror.service.ts`(`createSongMirrorService({ db, presigner, now })`) → `compose/song-mirror.ts`(Drizzle 로 `SongMirrorServiceDb` 구현) → `service/shared/storage/r2-presign.ts`(`createR2Presigner({ accountId, bucket, accessKeyId, secretAccessKey })` → `presignGet(key, { expiresInSeconds, downloadName })`, `node:crypto` 의 `createHmac('sha256')`·`createHash('sha256')` 로 SigV4 질의 서명. 새 의존성 없음).
- 라우트:
  - `POST /api/admin/song-mirror/manifest` — 관리자 토큰(기존 chart-pattern 업로드와 같은 관리자 인증), 매니페스트를 받아 두 표에 upsert, 응답 `successResponse({ packs, charts })`.
  - `GET /api/mirror/charts/[md5]` — 인증 없음, `PUBLIC_READ` 한도, 응답 `{ available: boolean, bytes?: number, top?: string }`(URL 없음).
  - `POST /api/mirror/charts/[md5]/link` — `withBearerOrSession`, 새 한도 `RATE_LIMIT.MIRROR_LINK = { limit: 20, windowMs: 60_000 }` 를 사용자 id 로(`enforceUserRateLimit`), 응답 `{ sha256, bytes, top, url, expires_at }`. POST 로 두는 이유: 링크가 캐시·미리 가져오기로 새지 않게.
  - `GET /api/mirror/charts/[md5]/download` — 세션만, 같은 한도, 302 로 서명 URL(웹 다운로드용, CSP 영향 없음).
- 환경변수(모두 선택, 없으면 미러 라우트가 `SERVICE_NOT_CONFIGURED` 503): `R2_ACCOUNT_ID`, `R2_MIRROR_BUCKET`, `R2_MIRROR_READ_ACCESS_KEY_ID`, `R2_MIRROR_READ_SECRET_ACCESS_KEY`, `MIRROR_LINK_TTL_SECONDS`(기본 상수 `DEFAULT_MIRROR_LINK_TTL_SECONDS = 600`). 서버 토큰은 **Object Read only + 이 버킷만**(4.3).
- 새 에러 코드 `MIRROR_NOT_FOUND`(404) — `error-code`·`error-message`·상태 표 세 곳.

### 5.8 U2·M3 클라이언트 모달 자리

> 구현과 다름: `DownloadChoice` 목록·`MAX_DOWNLOAD_CHOICES` 상수와, 내려받기 작업이 기대 바이트·SHA-256 을 입력으로 받는 계약(C3)은 만들지 않았다. 모달은 `ModalButton`(GetChart·OpenSongUrl·OpenChartUrl·Close·Cancel)과 `buttons()` 가 만드는 순서 있는 목록이고, 다운로드 작업은 URL 만 받는다. 미러 버튼은 M3 에서 이 목록 맨 앞에 변종 하나로 더한다([ADR](../acknowledge/2026-10-23-adr-song-mirror-prep.md)).

F4 가 모달을 만든다. U2 는 F4 구현이 지켜야 할 계약이고, M3 가 미러 버튼을 실제로 넣는다.

U2 계약(F4 구현에 반영, F8 리뷰가 확인):
- C1. 모달의 선택지는 한 함수가 만드는 **순서 있는 목록**(F4 가 정하는 타입, 예 `Vec<DownloadChoice>`)이고 그리기·키 이동은 목록 길이로 계산한다(버튼 수를 3으로 박지 않는다). 상수 `MAX_DOWNLOAD_CHOICES = 4`(미러·차분·곡 페이지·닫기)까지 겹치지 않게 배치.
- C2. 지금은 미러 선택지를 **넣지 않는다**(쓰이지 않는 변종·죽은 코드 금지). 넣는 자리는 목록 맨 앞(기본 포커스), 이유: 미러 묶음은 곡 폴더 전체(음원+차분)라 결정 15 의 "차분만, 음원은 곡 URL" 안내가 필요 없는 유일한 선택지다.
- C3. 내려받기 작업의 입력은 "URL + 기대 바이트(있으면) + 기대 sha256(있으면) + 묶음 안 최상위 폴더 이름(있으면)" 을 받을 수 있게 만든다. 직접 zip 링크는 앞의 두 값이 없고, 미러는 모두 있다. F4 가 sha256 확인을 지금 만들지 않으면 M3 에서 더한다.
- C4. 해제기 상한(최대 해제 바이트·항목 수)은 U1 의 `MAX_PACK_UNCOMPRESSED_BYTES`·`MAX_PACK_ENTRIES` 보다 작지 않다(11절 D-3).

M3(승인 뒤):
- 모달을 열 때 `GET /api/mirror/charts/<md5>` 로 `available` 을 묻는다(막대마다 묻지 않음). 서버가 꺼져 있거나 답이 없으면 버튼을 숨긴다(지금과 같은 모달).
- 로그인하지 않았으면 버튼은 보이되 흐리게, 누르면 "로그인하면 R-BMS 미러에서 받을 수 있습니다" 안내.
- 누르면 `POST …/link`(rbms-ir 에 `mirror_link(md5)` 추가, Bearer) → F4 내려받기 작업에 URL·`bytes`·`sha256`·`top` → 받은 뒤 sha256 이 다르면 버리고 오류 → 같으면 결정 16 의 다운로드 폴더에 푼다 → F4 의 재스캔.
- 버튼 문구: `R-BMS 미러에서 받기 (곡 전체, <크기>)`. 외부 사이트 이름 없음.
- 와이어: GET·빈 POST 라 `contract/client-requests/*.json`(요청 본문 계약)은 바뀌지 않는다. 응답 모양은 `crates/rbms-ir/src/contract_tests/fixtures.rs` 와 `web/tests/dto/ir-contract.test.ts` 에 고정 본문을 더한다.

### 5.9 M4 웹 표시와 인증·남용 방지

- 웹 표 상세(`web/src/widgets/table/table-chart-details.tsx:65-67` 의 버튼 줄)에 로그인 사용자에게만 `R-BMS 미러에서 받기` 버튼. 가용 여부는 `entities/song-mirror/song-mirror.query.ts` 의 `songMirrorAvailabilityQueryOptions(md5)`(`QUERY_KEY.SONG_MIRROR.AVAILABILITY(md5)`), 상세를 열 때만 조회. 버튼은 `<a href="/api/mirror/charts/<md5>/download">`(이동이라 CSP `connect-src` 와 R2 CORS 가 필요 없다).
- 인증: 서명 URL 은 로그인 사용자에게만, 유효 10분, 객체 하나·GET 하나(4.3 "bearer token" 취급). 서버 토큰은 읽기 전용·버킷 한정, 업로드 토큰은 사용자 키체인에만.
- 레이트리밋: 서버 `MIRROR_LINK` 사용자당 분 20회. 메모리 한도라 인스턴스마다 따로 센다(2.1) → 하루 상한은 DB 집계가 필요하다(부채 후보 T-3). 이그레스는 무료라 남용의 비용은 Class B 요청(백만 건 $0.36)뿐이다.
- 공개 도메인(Q-R3 의 B)을 고르면: 서명 URL 대신 `mirror.hyuns.uk` 사용자 지정 도메인 + Cloudflare 캐시 + WAF 레이트리밋(Free 1규칙, IP, 10초) + r2.dev 끄기. 서명 URL 은 사용자 지정 도메인에 쓸 수 없으므로 두 방식은 섞지 않는다.

### 5.10 호환성·마이그레이션

| 대상 | 지금(U1·U2) | 승인 뒤(M1~M4) |
|---|---|---|
| 설정 스키마(`CURRENT_SCHEMA_VERSION` 6) | 변경 없음 | 변경 없음(다운로드 폴더는 F4) |
| 점수 DB·곡 DB | 변경 없음 | 변경 없음 |
| 웹 DB | 변경 없음 | 새 표 2개(추가만), 운영 적용은 사용자 승인 |
| 와이어 | 변경 없음 | 새 라우트 3개(추가만). 기존 라우트 불변 |
| 스킨·골든 캡처 | 변경 없음 | 모달은 앱 UI(F4). 미러 정보가 없으면 지금 모달과 화소 동일 |

---

## 6. 원본 대비 차이

| 항목 | 원본 | R-BMS 설계 | 판정·점수 의미 |
|---|---|---|---|
| 묶음 주소 | beatoraja: 표 항목의 `ipfs`(CID), IPFS 게이트웨이 | R-BMS 매니페스트의 sha256 → R2 서명 URL | 무관 |
| 형식 | tar.gz(MusicDownloadProcessor.java:22-23) | zip(결정 15 의 해제기를 그대로 쓰기 위해) | 무관 |
| 곡·차분 | 곡 묶음 `ipfs` + 차분 `appendipfs` 따로, 원곡이 있으면 차분만 | 곡 폴더 통째(음원+모든 차분) 한 묶음. 차분만 받기는 결정 15 의 `url_diff` 경로가 맡는다 | 무관 |
| 누가 받나 | 누구나(허용 도메인 표에서) | 로그인 사용자(Q-R3 추천) | 무관 |
| 받는 곳 | `ipfs/[아티스트]제목`, roots 에 자동 추가 | 결정 16 의 다운로드 폴더 아래 `<top>`(원래 곡 폴더 이름) | 무관 |
| 트리거 | 선곡 명령 `DOWNLOAD_IPFS` | 없는 곡 막대 → 모달(결정 15·16) | 무관 |
| LR2·IIDX | 내려받기 기능 없음 | — | — |

판정·점수 의미가 원본과 달라지는 부분은 없다.

---

## 7. 비용 추정(실측 대신 공개 값)

- 근거 값: GENOSIDE 2018 Starter Package 6.16 GB / 159곡 → 곡당 평균 약 38.7 MB(공식 페이지 3.1). Append 125 MB.
- 범위 Q-R2 A(発狂 1,035 md5): 곡 폴더 수 ≤ 1,035(한 폴더에 여러 차분이면 더 적다). 상한으로 1,035 × 38.7 MB ≈ **40 GB**.
- 저장: (40 − 10 무료) × $0.015 ≈ **월 $0.45**. 2배(80 GB)여도 월 $1.05.
- 올리기(Class A): 8 MiB 파트 기준 곡당 평균 약 5파트 + 시작·완료 2 ≈ 7 → 1,035곡 ≈ 7,300회, 무료 100만 안. HEAD 확인(Class B) 1,035×2 회.
- 내려받기: 받을 때마다 GET 1(Class B), 무료 월 1,000만. 이그레스 무료.
- 결론: Q-R2 의 A·B·C 어느 것이든 월 $1 안팎. Stella·Satellite(D, 약 4,800곡)까지 넓혀도 같은 평균이면 약 186 GB → 월 약 $2.6.
- 실측 방법(사용자가 SSD 를 꽂고): `cargo run -q --release -p rbms-cli --locked -- mirror-pack /Volumes/SSD/bms --out /Users/hyunseokbyun/rbms-song-mirror --table web/data/tables/insane1/data.json --dry-run` 이 고른 폴더 수와 원본 바이트 합을 출력한다(압축 뒤 크기는 실제 실행에서).

---

## 8. 구현 단위

| 단위 | 소유 파일(수정 가능) | 읽기 전용 | 선행 | 다른 묶음과 겹치는 파일 | 병렬 |
|---|---|---|---|---|---|
| U1 묶음 도구 | `apps/rbms-cli/src/mirror_pack.rs`, `apps/rbms-cli/src/mirror_pack/{args,folders,pack,manifest,notice,render,tests}.rs`(새 파일), `apps/rbms-cli/src/main.rs`(USAGE·분기·`mod mirror_pack;`·USAGE 테스트 한 줄), `apps/rbms-cli/Cargo.toml`, `Cargo.lock` | `crates/rbms-library/src/lib.rs`, `crates/rbms-parser/src/lib.rs`, `crates/rbms-table/src/lib.rs`, `apps/rbms-cli/src/chart_pattern/**`, `web/data/tables/insane1/data.json` | F4 의 루트 `Cargo.toml` `zip` 워크스페이스 줄과 그 `Cargo.lock` 커밋 | 루트 `Cargo.toml`(F4 소유, U1 은 고치지 않음), `Cargo.lock`(F4 다음 순서로 갱신) | F4 의 의존성 커밋 뒤에는 F2·F3·F5·F6 과 병렬(겹치는 파일 없음) |
| U2 모달 자리 | 없음(F4 파일에 계약 C1~C4 반영) | 이 문서 5.8 | — | F4 의 모달·내려받기 파일 전부 | F4 안에서 |
| M1 업로드 | `web/scripts/upload-song-mirror.ts`, `web/tests/scripts/upload-song-mirror.test.ts` | `web/scripts/upload-chart-patterns.ts` | U1, Q-R1~R5 답, 버킷·토큰 | 없음 | 승인 뒤 |
| M2 서버 | `web/drizzle/<번호>_song_mirror.sql`, `web/src/server/db/schema.ts`, `web/src/server/dto/song-mirror.ts`, `web/src/server/route/song-mirror.ts`, `web/src/server/service/domain/song-mirror/*`, `web/src/server/compose/song-mirror.ts`, `web/src/server/compose/index.ts`, `web/src/server/service/shared/storage/r2-presign.ts`, `web/src/server/lib/{env,error-code,error-message,error,with-rate-limit}.ts`, `web/src/app/api/mirror/**`, `web/src/app/api/admin/song-mirror/**`, 테스트 | — | M1, F6 마이그레이션 | `schema.ts`·`env.ts`·`error-*.ts`·`compose/index.ts`(F6 과 겹침 → F6 뒤 직렬) | 승인 뒤 |
| M3 클라이언트 | F4 모달 파일, `crates/rbms-ir/src/{lib,http,dto}.rs`, rbms-ir 계약 고정 본문 | — | M2, F4 | F4 파일 | 승인 뒤 |
| M4 웹 | `web/src/widgets/table/table-chart-details.tsx`, `web/src/entities/song-mirror/song-mirror.query.ts`, `web/src/shared/constants/query-key.ts`(실제 경로는 구현 때 확인) | — | M2 | F6 의 표 페이지 파일 | 승인 뒤 |

---

## 9. 테스트 계획

### 9.1 U1 새 테스트(`apps/rbms-cli/src/mirror_pack/tests.rs`, 임시 폴더는 chart-pattern 의 `Scratch` 관례)

| 이름 | 단언 |
|---|---|
| `a_table_or_all_is_required` | `parse(["/lib", "--out", "/o"])` 가 Err, `--all` 이나 `--table x` 를 주면 Ok |
| `the_out_folder_inside_the_library_is_refused` | `out = lib/sub` 이면 `export` 가 Err 이고 아무것도 쓰지 않는다 |
| `a_song_folder_is_one_that_holds_a_chart` | 차트 없는 폴더는 묶음이 아니고, 차트를 가진 하위 폴더는 부모 묶음에서 빠지고 따로 묶인다 |
| `chart_hashes_match_the_parser` | 고정 차트의 md5·sha256 이 `rbms_parser::parse(bytes)` 의 값과 같다 |
| `only_folders_with_a_table_md5_are_packed` | 표에 있는 md5 의 폴더 하나만 `packs` 에 있고 `tables[0].matched == 1`, 없는 md5 는 `missing` |
| `every_chart_of_a_packed_folder_is_listed` | 폴더의 표 밖 차트도 `charts` 에 있다 |
| `two_runs_write_the_same_bytes` | 같은 입력 두 번의 zip 바이트·sha256·매니페스트 바이트가 같다 |
| `a_second_run_reuses_an_unchanged_pack` | 두 번째 실행 보고 `reused == 1`, `written == 0`, zip 수정 시각이 바뀌지 않는다 |
| `a_changed_file_makes_a_new_key` | 파일 한 바이트를 바꾸면 sha256·key 가 바뀐다 |
| `entries_are_sorted_and_carry_the_folder_name` | zip 항목 이름이 `<top>/…` 이고 바이트 순, 폴더 항목이 없다 |
| `system_files_are_left_out` | `.DS_Store`, `._x`, `__MACOSX/` 가 zip 에 없다 |
| `compressed_media_is_stored_and_the_rest_deflated` | `a.ogg` 는 Stored, `a.wav`·`x.bme` 는 Deflated |
| `a_folder_over_the_limit_is_refused` | `pack_entries` 에 낮춘 `PackLimits` 를 넣으면 `Refusal::TooLarge`, zip 없음 |
| `a_symbolic_link_is_not_followed` | 링크가 가리키는 파일이 zip 에 없다(unix 전용 `#[cfg(unix)]`) |
| `notices_find_redistribution_words_in_shift_jis` | Shift_JIS 로 "再配布禁止" 를 쓴 txt 가 `notice_files` 에 있다 |
| `decomposed_names_are_counted` | `"か\u{3099}.wav"` 이름이 `nfd_names == 1` |
| `a_parent_path_in_a_wav_is_counted` | `#WAV01 ../a.wav` 차트면 `escapes_root == 1` |
| `the_manifest_has_no_absolute_path` | 매니페스트 문자열에 라이브러리 절대 경로가 없다 |
| `a_dry_run_writes_nothing` | `--dry-run` 뒤 out 폴더에 파일이 없고 보고의 `total_uncompressed_bytes > 0` |
| `a_pack_opens_with_the_zip_reader` | 만든 zip 을 `zip::ZipArchive` 로 열어 항목 수·내용이 원본과 같다. F4 해제 함수가 라이브러리 크레이트에 있으면 그 함수로 푼 결과가 원본 폴더와 같다(D-3) |

`main.rs` 테스트에 한 줄: `USAGE.contains("rbms-cli mirror-pack")` 와 `run(["rbms-cli", "mirror-pack"])` 가 `EXIT_USAGE`.

### 9.2 뒤집히는 기존 테스트·골든·계약

- 기존 테스트는 뒤집히지 않는다(USAGE 는 줄이 늘 뿐, 기존 단언은 `contains`).
- 골든 캡처·스킨 덤프·`contract/client-requests` 영향 없음(U1 은 앱·와이어를 건드리지 않음). U2 는 F4 의 테스트에 들어간다.

### 9.3 실행 명령

```text
cargo fmt --all --check
cargo clippy -p rbms-cli --all-targets --all-features --locked -- -D warnings
cargo test -p rbms-cli --locked mirror_pack
cargo tree -p rbms-cli -e normal --locked | grep -E "zip|typed-path"
```

통합 게이트(F8·G)의 전체 `cargo test --workspace --locked` 에 포함된다. 실제 SSD 실행은 사용자 몫(7절 명령).

### 9.4 M 단계(참고, 승인 뒤)

- M2: `r2-presign` 을 AWS SigV4 문서의 질의 서명 예제(“examplebucket/test.txt”, 날짜 20130524)로 고정 시험(기대 서명값은 구현 때 AWS 원문에서 옮긴다), 링크 라우트 인증·한도·`is_enabled=false` 404, 매니페스트 Zod 거절.
- M1: dry-run 이 네트워크를 쓰지 않음(가짜 클라이언트 주입), sha256 불일치 파일을 올리지 않음.

---

## 10. 문서 갱신 목록

| 문서 | 내용 | 때 |
|---|---|---|
| `docs/crates.md` 575-577행 `apps/rbms-cli` | `mirror-pack` 명령과 산출물 | U1 커밋 |
| `docs/architecture.md` 29행(의존 표), 63행(rbms-cli 설명) | zip·md-5·sha2·encoding_rs 추가, 명령 한 줄 | U1 커밋 |
| `docs/development.md` | 실행 예(7절 명령)와 결과 폴더를 저장소 밖에 두라는 경고 | U1 커밋 |
| `docs/PROCESS.md` F7 | U1 완료 커밋, M1~M4 는 사용자 승인 대기 | U1 커밋 |
| `docs/HANDOFF.md` 6절 | Q-R1~R5, Q-C1~C3 를 미해결 질문으로 | 마지막 보고 때 |
| `docs/reference/2026-10-23-stella-course-dan-survey.md` 7절 | 요금 "확인 못함" → 이 문서 4절 링크, hyuns.uk 이름 서버 사실 | U1 커밋 |
| `docs/acknowledge/` 새 ADR `2026-10-23-adr-song-mirror-prep.md` | 결정 17 범위, 키 규칙, zip 형식, 로그인 한정 추천, 사용자 답 기록 | 사용자 답 뒤 |
| `docs/quality-assurance/2026-10-14-test-debt.md` | 14절의 T-1~T-4 | U1 커밋 |
| `docs/reference-divergences.md`(acknowledge 아래) | 6절 표(원본 IPFS·tar.gz 와의 차이) | M3 때 |

---

## 11. 메인 결정 필요

| 번호 | 결정 지점 | 추천 | 근거 | 대안 |
|---|---|---|---|---|
| D-1 | 결정 15 의 `zip` 크레이트를 rbms-cli 묶음 **쓰기**에도 쓰는가 | 쓴다(같은 워크스페이스 줄, 새 크레이트 없음) | 승인 문구는 "해제용" 이지만 같은 크레이트·같은 deflate 기능이라 의존 트리가 늘지 않는다. 미러 묶음이 클라이언트 해제기와 같은 구현으로 왕복 시험된다 | B. 압축 없는 zip 을 손으로 쓴다(crc32 필요 → crc32fast 직접 의존 추가, 용량 증가) / C. Bun 스크립트로 묶는다(Bun 에 zip 쓰기 내장 없음 → 손수 구현 또는 npm 의존) |
| D-2 | 묶음 도구 위치 | `apps/rbms-cli mirror-pack`(Rust) | md5 계산·걷기·표 읽기를 플레이어와 같은 크레이트로, 해제기 왕복 시험 가능, chart-pattern 선례 | B. `web/scripts` Bun 스크립트(zip 쓰기 문제는 D-1 C) |
| D-3 | F4 해제기의 위치와 상한 | F4 가 해제 함수와 상한 상수를 라이브러리 크레이트(추천 `crates/rbms-library` 의 `archive` 모듈)에 두고 U1 이 그 상수를 쓴다. 상한은 최대 해제 2 GiB·항목 1만 이상 | rbms-cli 는 앱 크레이트에 의존하지 않는다(architecture.md 63행 "It never depends on the player"). 같은 상수여야 미러 묶음이 반드시 풀린다 | B. F4 가 앱에 두면 U1 은 같은 값 상수를 따로 두고 왕복 시험은 부채(T-2) |
| D-4 | zip 버전 | F4 가 정한 줄. 정하지 않았다면 `9.0.5`, `default-features = false`, `["deflate-flate2-zlib-rs"]` | 최신 안정, MIT, 새 크레이트 zip·typed-path 둘 | 9.0.x 가 2026-10-09 이후 6일 동안 다섯 번 패치됐다. 2주 이상 된 줄을 원하면 8.x 의 마지막 판(이번에 확인 못함) |
| D-5 | 고르는 범위의 기본값 | `--table` 또는 `--all` 중 하나를 반드시 요구 | 11,336 차트 전부를 실수로 묶지 않게 | `--all` 을 기본 |
| D-6 | 텍스트 재배포 낱말 찾기(`notice_files`) | 넣는다(업로드를 막지 않고 표시만) | Q-R1 판단에 곡별 readme 근거가 필요한데 곡 폴더를 사람이 다 열 수 없다. encoding_rs 는 이미 워크스페이스에 있음 | 텍스트 파일 이름 목록만 |
| D-7 | NFD 이름 | 세기만 하고 고치지 않는다 | 고치려면 정규화 크레이트(새 의존성)가 필요. 실제로 문제가 되는지는 Windows 실기 전에는 모름 | 묶을 때 NFC 로 바꾼다(새 의존성 승인 필요) |
| D-8 | 업로드 도구 | M1 Bun `S3Client`(내장) 스크립트, 기본 dry-run | 새 의존성 없음, SigV4·멀티파트·재시도 내장, 키체인 관례 재사용 | Rust + sha2 로 SigV4 직접 구현 |
| D-9 | 서버 서명 방식 | Vercel Node 런타임 유지, `node:crypto` SigV4 | 운영 런타임 변경(Bun 베타) 위험 없이 새 의존성 0 | `vercel.json` `bunVersion` 으로 Bun 런타임 전환 후 `Bun.S3Client.presign` |
| D-10 | 객체 키 | `packs/<sha256>.zip` | 내용 주소: 중복 제거, 재업로드 판단, 이름 문제 없음 | `packs/<md5>.zip`(폴더에 차트가 여럿이면 중복) |

사용자에게 물을 것(마지막 보고에 묶음): Q-R1~Q-R5(3.3), 그리고

| 번호 | 질문 | 추천 |
|---|---|---|
| Q-C1 | `hyuns.uk` zone 이 있는 Cloudflare 계정에 R2 를 만드나 | 예(사용자 지정 도메인이 필요해질 때 바로 붙음) |
| Q-C2 | 버킷 이름 | `rbms-song-mirror`(위치 힌트 없음, Standard) |
| Q-C3 | 결과 폴더 | `/Users/hyunseokbyun/rbms-song-mirror`(저장소 밖) |

---

## 12. 사용자가 할 일(승인한 뒤, 시크릿 값이 대화에 남지 않는 방법)

1. 3.3 의 Q-R1~R5 와 11절 Q-C1~C3 에 답한다.
2. SSD 를 꽂고 dry-run 으로 실제 크기를 본다(값 없음, 출력만): `! cargo run -q --release -p rbms-cli --locked -- mirror-pack /Volumes/SSD/bms --out /Users/hyunseokbyun/rbms-song-mirror --table web/data/tables/insane1/data.json --dry-run`
3. Cloudflare 대시보드 → R2 → 버킷 만들기(이름 Q-C2, 위치 자동, Standard). 공개 접근(r2.dev)은 켜지 않는다.
4. R2 → API Tokens → 토큰 두 개: (가) 업로드용 **Object Read & Write**, 이 버킷만 / (나) 서버용 **Object Read only**, 이 버킷만. Secret 은 한 번만 보이므로 화면에서 바로 다음 단계로 옮긴다(채팅에 붙이지 않는다).
5. 업로드용 값을 키체인에 넣는다. `-w` 를 맨 끝에 두면 값을 프롬프트로 물어 셸 기록에 남지 않는다:
   - `security add-generic-password -a rbms-r2 -s rbms-r2-upload-access-key-id -w`
   - `security add-generic-password -a rbms-r2 -s rbms-r2-upload-secret-access-key -w`
6. 서버용 값은 Vercel 에 넣는다(대화형 프롬프트로 값 입력): `cd web && vercel env add R2_MIRROR_READ_ACCESS_KEY_ID production`, `vercel env add R2_MIRROR_READ_SECRET_ACCESS_KEY production`. 계정 id·버킷 이름(`R2_ACCOUNT_ID`, `R2_MIRROR_BUCKET`)은 비밀이 아니지만 같은 방법으로 넣는다.
7. GitHub 저장소 시크릿은 이 설계에서 필요 없다(업로드는 로컬 SSD 에서, CI 에서 하지 않음). 나중에 필요하면 `gh secret set <이름>` 이 값을 프롬프트로 받는다(명령줄 인자에 값을 쓰지 않는다).
8. 업로드 실행은 M1 구현 뒤 사용자가 `!` 로 직접: 먼저 dry-run, 그다음 `--execute`.

---

## 13. 메인이 마지막에 보고할 요약 초안

> 곡 미러는 결정 17 대로 업로드하지 않고 준비만 했습니다. 상태: 묶음 도구 완료, 업로드·서버·버튼은 승인 대기.
>
> 1. 만든 것: `rbms-cli mirror-pack` — 곡 폴더를 표 md5 로 골라 결정적인 zip(`packs/<sha256>.zip`)과 `manifest.json` 을 저장소 밖 폴더에 만듭니다. `--dry-run` 으로 실제 용량을 잴 수 있습니다. 네트워크는 쓰지 않습니다.
> 2. 비용(공개 값 추정): 発狂 1,035곡 ≈ 40 GB → R2 Standard 월 약 $0.45(무료 10 GB 제외), 이그레스 무료, 요청은 무료 한도 안.
> 3. 권리 조사: Stella 는 "대리 업로드는 차분 제작자 허가 시", GENOCIDE 는 "각자 보관·공유는 자유, 단 작자에게 연락할 수 있으면 먼저", GENOSIDE 패키지는 "작자 허가를 받아 패키지화", BMS-IR 은 권리 침해 우려 데이터 배포 금지(자기 서비스 안). 어느 곳도 제3자 재배포를 허락하는 문구는 없습니다. 곡별 readme 의 再配布 조건은 도구가 찾아 매니페스트에 표시합니다.
> 4. 확인 질문: Q-R1 업로드 가능하다고 본 근거, Q-R2 범위, Q-R3 로그인 사용자만, Q-R4 삭제 요청 창구, Q-R5 사전 연락, Q-C1~C3 계정·버킷·결과 폴더.
> 5. 답을 주시면: 버킷·토큰 만들기(12절 3~6, 값은 키체인·Vercel 프롬프트로만) → 업로드 스크립트 dry-run → 직접 실행 → 서버 등록·버튼 순서로 진행합니다.
>
> 다음 행동: 3.3 표의 Q-R1 에 A~D 중 하나로 답해 주세요.

---

## 14. 남은 위험과 테스트 부채 후보

| 번호 | 위험 | 왜 지금 안 하나 | 언제 |
|---|---|---|---|
| R-1 | 권리: 허락 없는 재배포가 될 수 있다(3.2) | 사용자 판단 영역. 업로드는 승인 뒤 | Q-R1 답 |
| R-2 | 곡 폴더 밖 파일을 참조하는 차트(`escapes_root`)는 미러로 받아도 소리·그림이 빠진다 | 세기만 한다 | 매니페스트에서 0 이 아니면 그 묶음을 빼거나 수동 확인 |
| R-3 | 한 곡이 여러 폴더(본편 + 차분 폴더 분리)에 걸친 배포는 한 묶음에 다 안 들어간다 | 원본 beatoraja 도 곡 = 차트 폴더 | 실제 매니페스트의 `missing`·보고로 판단 |
| R-4 | macOS NFD 이름이 Windows 에서 `#WAV` 이름과 안 맞을 수 있다 | 정규화는 새 의존성(D-7) | Windows 실기(test-debt 3.3·3.4) 때 |
| R-5 | 메모리 레이트리밋은 인스턴스마다 따로 센다 | M2 범위 | M2 구현 때 DB 하루 집계 |
| R-6 | zip 9.0.x 가 막 나온 줄이다(D-4) | F4 와 같은 줄을 써야 한다 | F4 의존성 커밋 때 |
| R-7 | `response-content-disposition` 의 R2 지원 확인 못함 | M2 범위 | M2 구현 때 |
| T-1 | 실제 `/Volumes/SSD/bms` 전체 실행(시간·용량·거절 수)은 사용자만 할 수 있다 | SSD·사용자 환경 | 사용자가 7절 명령을 돌릴 때 |
| T-2 | F4 해제기가 앱 크레이트에 있으면 U1 왕복 시험이 해제기를 직접 못 부른다(D-3 B) | F4 배치에 달림 | F4 설계 확정 때 |
| T-3 | 서명 URL 남용의 하루 상한(DB 집계) | M2 | M2 |
| T-4 | 2 GiB 를 넘는 곡 폴더(BGA 동영상)는 거절된다 | 상한은 해제기 안전과 같이 정함 | 거절 목록이 생기면 상한 재검토 |
