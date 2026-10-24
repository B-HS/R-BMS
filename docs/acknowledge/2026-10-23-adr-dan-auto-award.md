# ADR: 코스 기록으로 단위를 자동 판정해 user_dan 에 넣고 표시 단위를 고르게 함

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 19 는 사용자 확인 2026-10-23: "맞어", 결정 20 은 "추천대로". 인정 조건의 세부는 메인 결정이고 사용자 확인 2026-10-24(미확인 14 지금대로): 게이지 모드 무관 인정(J-11), ranked 판만·ASSIST·LIGHT ASSIST 램프 제외·0012 이전 판 소급 없음(J-13)) · 결정일 2026-10-23 · 결정자 사용자(결정 19·20), 세부 메인 · 커밋 9e39262(마이그레이션 0012 의 `user_dan`·`user_dan_display`·`dan_course`. 웨이브 1 전체가 섞인 커밋), d57c6c2(판정·표시 API·재판정 서비스), c79f53f(단위 코스 등록), 942ed71(표 갱신과 소급 연결), 608eae7(마이페이지 단위 탭·배지), df52b3c(`dan-recompute` 액션)
> 관계: 코스 제출의 스테이지 사실은 [코스 제출 게이지](2026-10-23-adr-course-submission-gauge.md), 단위 코스의 등록은 [GENOSIDE 데이터](2026-10-23-adr-genoside-2018-course-data.md)와 [표 매일 갱신](2026-10-23-adr-table-daily-refresh.md).

## 배경

- 사용자 요구 8: 단위를 획득하면 내 정보에 단위를 넣고, 마이페이지에서 클리어한 코스를 설정할 수 있게 한다.
- 3c0cd22 시점의 서버: `user.rank varchar(32)`·`rank_points` 열이 있으나 쓰는 코드가 없었고(쓰기 0건), 코스 클리어 → 단위 대응도 없었다. 사용자별로 클리어한 코스를 모으는 읽기는 `course_best` 로 가능했다.
- 결정 19: 서버가 코스 기록으로 단위를 자동 판정해 `user_dan` 에 넣고, 마이페이지에서 표시할 단위를 고른다. 결정 20: 단위 인정은 게이지가 남은 채 끝낸 판이고 미러·LN 제약을 지키며 어시스트·SCRATCH AUTO 는 제외한다(질문 원문은 저장소에 없고 `docs/PROCESS.md` 의 요약만 있다).

## 결정

### 1) 데이터 모델 (마이그레이션 0012)

- `dan_course`(PK `course_hash`; `system`, `label`, `rank`): 어느 코스가 어느 체계의 어느 단위를 주는가. 코스 등록 때 이름 규칙으로 채운다.
- `user_dan`(PK `user_id + system + label`; `rank`, `course_hash`, `course_score_id`, `course_gauge`, `gauge`, `acquired_at`): 획득 기록. 한 단위는 한 번만 남는다.
- `user_dan_display`(PK `user_id + system`; `label` NULL = 숨김): 표시 선택. 행이 없으면 자동.
- `user.rank` 는 쓰지 않는다. 한 칸 문자열이 네 체계와 표시 선택을 담지 못한다.

### 2) 체계 네 개와 단위 이름

| 체계(`system`) | 대상 코스 | label / rank |
| --- | --- | --- |
| `genoside2018_sp` | 묶음 `genoside-2018-sp`, 이름이 `GENOSIDE 2018 段位認定 ` 로 시작 | 접두사 뒤 글자(初段 … 十段, 発狂初段 … 発狂皆伝, Overjoy) / 묶음 안 순서 + 1 (1~22) |
| `genoside2018_dp` | 묶음 `genoside-2018-dp`, 접두사 `GENOSIDE 2018 DP段位認定 ` | 같은 방식 |
| `stella` | 표 코스, 이름이 `Stella Skill Simulator <판> st<n>` | `st<n>` / n (0~12) |
| `satellite` | 표 코스, 이름이 `Satellite Skill Analyzer <판> sl<n>` | `sl<n>` / n |

- `STELLA技能訓練 ...` 코스는 코스로 등록되고 순위도 있지만 단위를 주지 않는다(단계가 아니다). 표의 판이 바뀌어도(`\S+`) 같은 label 이면 PK 로 한 번만 남는다. 표에서 사라진 코스는 `active = false` 가 되지만 `dan_course` 는 남아 옛 판의 코스도 계속 단위를 준다.
- 구현 위치: `course-catalog/dan-of-course.ts`(설계서의 `dan-rules.ts` 가 아니다).

### 3) 인정 조건 (순수 함수 `judgeDanEligibility`)

코스 판 하나를 위에서 아래로 보아 처음 걸리는 사유를 돌려준다(저장하지 않고 시험·로그용).

1. 단위 코스일 것 (`not_a_dan_course`), 로그인 사용자일 것 (`guest`), `ranked = true` 일 것 (`unranked`: 넓힌 판정·TOTAL 덮어쓰기·오토플레이·신뢰하지 않는 빌드가 막는다).
2. 램프가 EASY(id 4) 이상이고 끝 게이지 값이 0 초과일 것 (`not_cleared`). NO PLAY·FAILED·ASSIST·LIGHT ASSIST 램프는 제외된다.
3. SCRATCH AUTO 가 아니고 (`scratch_auto`) ASSIST 플래그가 없을 것 (`assisted`).
4. 판이 스테이지 사실(`stages`·`course_gauge`·`gauge`)을 가질 것 (`no_stage_facts`). 모든 게이지가 CLASS 계열(Class·ExClass·ExHardClass)일 것 (`not_a_course_gauge`).
5. 스테이지 수 = 코스 차트 수, 스테이지 i 의 차트 = 코스의 i 번째 차트(md5, md5 가 없으면 sha256) (`stage_count`, `chart_mismatch`).
6. RANDOM: 코스 제약 `grade` 면 모든 스테이지가 Off, `grade_mirror` 면 Off 또는 Mirror (`random_not_allowed`). LN: 제약 `ln`·`cn`·`hcn` 이면 모든 스테이지의 `lntype` 이 0·1·2 (`ln_mode_mismatch`).

- 확인하지 않는 것: 속도 제약(`no_speed`), NO GOOD·NO GREAT(본문에 사실이 없고 대상 코스도 없다), 게이지 표 제약(표는 플레이어의 COURSE GAUGE 설정이다). 트로피(은·금 메달)는 판정에 쓰지 않는다.
- 게이지 모드(LR2·BEATORAJA·IIDX)를 묻지 않고 모두 인정하며 `user_dan.course_gauge` 에 모드를 기록한다 (J-11, 사용자 확인 2026-10-24). 설계서는 이를 "결정 20 의 '모두 인정하되 기록'" 이라 적었으나 `docs/PROCESS.md` 의 결정 20 요약에 그 문구가 없어 근거로 쓰지 않는다. 근거는 메인 판단이다: 결정 20 의 조건에 모드 제한이 없고 결정 21 이 세 모드를 설정으로 고르게 했다.
- 같은 단위를 여러 번 얻어도 처음 얻은 판만 남는다(`insertUserDanIgnore`, PK 충돌은 무시). `acquired_at` 은 그 판의 `played_at` 이다.

### 4) 언제 판정하는가

- 코스 제출 때: `POST` 코스 제출이 판을 저장한 직후 같은 요청의 후처리로 `awardFromRun` 이 판정한다. 중복 제출(같은 판 재전송)은 저장된 행으로 다시 판정한다. 판정이 던지면 로그만 남기고 제출은 성공한다(모든 판을 저장한다는 기록 원칙). 새 단위를 얻으면 그 사용자의 프로필·플레이어 순위 캐시를 무효화한다(`revalidateOnDanAward`).
- 소급 (J-13, 사용자 확인 2026-10-24): 저장된 `stages` 가 있는 판만 다시 판정한다. `web-db.yml` 의 `dan-recompute-count`(세기)와 `dan-recompute`(적용, 멱등)가 `scripts/dan-recompute.ts` 로 모든 단위 코스를 오래된 판부터 읽고, 표 갱신이 새 단위 코스를 등록하면 그 코스만 자동으로 소급한다. `stages` 가 없는 판(0012 이전 판, 세 필드를 모르는 옛 클라이언트 판)은 제약 준수를 알 수 없어(결정 20 위반 가능) 판정하지 않으므로 이전에 클리어한 단위는 생기지 않는다.

### 5) 표시와 공개

- 표시 모드: `auto`(체계 안에서 가장 높은 rank), `hidden`(체계 단위를 숨김), `label`(가진 단위 중 하나. 없는 label 이면 검증 오류). 체계마다 따로 고른다.
- 공개 범위는 남에게 보이는 표시 단위(체계 이름 + label)뿐이다: 플레이어 프로필, 순위 행 배지, IR 프로필의 `extra.dans`. 획득 목록·판 id·게이지 모드·획득 시각은 본인만 본다.
- API: `GET /api/fe/me/dans`(내 체계별 획득·표시), `PUT /api/fe/me/dans/display`(표시 선택, 본문 1 KiB 제한). 마이페이지 설정 탭 "단위".

## 이유

- 서버가 기록으로 판정해야 자기 신고 없이 같은 규칙이 모든 사용자에게 적용된다(결정 19). 스테이지별 사실(RANDOM·LN·게이지·차트)을 본문에 싣게 한 이유가 이 판정이다.
- 첫 획득만 남기고 `INSERT IGNORE` 형태로 두면 재판정·재전송·소급이 멱등이다.
- 판정 실패가 제출을 막지 않게 한 것은 "사람이 친 판은 전부 저장" 원칙 때문이고, 놓친 판정은 재전송이나 `dan-recompute` 로 따라잡을 수 있다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `user.rank` 에 대표 단위 하나를 복사 | 한 칸이 네 체계와 표시 선택을 못 담는다 |
| 사용자가 자기 단위를 신고 | 결정 19 가 기록에 의한 자동 판정을 정했다 |
| 램프만으로 옛 판까지 소급(제약 미확인 표시) | 결정 20 의 미러·LN 제약을 어겼을 수 있다. 소급은 `stages` 가 있는 판만 한다 |
| 게이지 모드별로 순위·단위를 나눔 | IR 순위 와이어와 `course_best` 키 변경이 필요하다. 모드는 기록만 한다 |
| `ranked` 를 무시하고 어시스트 플래그만 봄 | 넓힌 판정·신뢰하지 않는 빌드를 같은 기준으로 막으려 한다(J-13, 메인 판단) |
| 판정 실패를 제출 실패로 | 모든 판 저장 원칙에 어긋난다 |
| 획득 목록 전체를 공개 / 기본 숨김 | 사용자가 고른 표시 단위만 남에게 보인다 |

## 영향과 관련 파일

- 사용자 확인 2026-10-24(지금대로 확정): J-11 게이지 모드 무관 인정, J-13 `ranked` 판만·ASSIST·LIGHT ASSIST 제외·소급 없음. 질문 원문이 저장소에 없어 결정 19·20 의 정확한 문구와는 대조하지 못했다.
- 남은 위험: 차트 자체의 `#LNMODE` 가 LN 종류를 고정한 곡이면 `lntype` 이 달라 `ln_mode_mismatch` 로 단위가 안 나올 수 있다(실기에서 보이면 본문에 차트 고정 여부를 싣는 후속이 필요하다). 처음 획득만 남아 뒤에 더 높은 게이지로 다시 깨도 단위 기록은 바뀌지 않는다. 트로피와 DP 의 FLIP·DP 제약 원천은 구현하지 않았다.
- 확인하지 못한 사실: LR2IR 의 단위 표시 방식. 운영에서 `dan-recompute` 를 실행한 적이 없다(2026-10-23 확인 시점의 마지막 `web-db` 실행은 `tables-refresh-count`).
- 코드: `web/src/server/service/domain/dan/{dan-eligibility,dan.service}.ts`, `course-catalog/dan-of-course.ts`, `web/src/server/{route/dan,route/course,compose/dan,dto/dan}.ts`, `web/src/server/db/schema.ts`, `web/scripts/dan-recompute.ts`, `web/src/widgets/settings/{dan-panel,dan-system-card}.tsx`, `web/src/features/dan/*`.
- 시험: `web/tests/service/{dan-eligibility,dan,dan-of-course,player-dans}.test.ts`, `web/tests/db/dan-award.test.ts`, `web/tests/components/{dan-panel,dan-display}.test.tsx`, `web/tests/scripts/dan-recompute.test.ts`.

## 날짜와 사용자 원문

2026-10-23.

- 요구 8 의 해당 부분: "해당 단위를 획득하면 내정보에 단위를 넣도록하고, 다른 코스들도 생성하게 만들고. 마이페이지에서 내가 클리어한 코스를 설정가능하도록해."
- 결정 19: "맞어". 결정 20: "추천대로". 결정 21 과 포괄 지시: "나머지도 추천대로" (`docs/PROCESS.md` 에 기록된 원문).
- 결정 19 의 내용 요약(PROCESS): "서버가 코스 기록으로 단위를 자동 판정해 `user_dan` 에 넣고 마이페이지에서 표시할 단위를 고름". 결정 20 의 요약: "단위 인정 = 게이지가 남은 채 끝낸 판, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외".
- 2026-10-24 사용자 확인: "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로"(결정 20 = 미확인 13~18 을 지금대로 확정, 곡 미러는 19 로 보류).
