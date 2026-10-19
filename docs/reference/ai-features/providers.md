# AI 공급자 3곳 공식 API 조사 (BYOK·구조화 출력·어댑터 계층)

> 최종 갱신 2026-10-19 · 대응 커밋 1eadbb5
> 이 문서는 조사 보고서 전문이다(줄이지 않았다). 끝에 조사자가 함께 낸 "제약"과 "확인하지 못한 것" 목록을 원문 그대로 붙였다. 기획은 [../../plan/2026-10-19-web-ai-features.md](../../plan/2026-10-19-web-ai-features.md), 색인은 [README.md](README.md).
> 모델 id·가격·한도는 2026-10-19 문서 시점의 값이다. 공급자가 바꾸므로 구현 때 다시 확인한다.

확인 날짜: 모든 항목 2026-10-19. 근거는 공식 문서 URL(직접 읽음), Anthropic 부분은 `claude-api` 스킬(번들 경로 `/private/tmp/claude-501/bundled-skills/2.1.303/2e7cb8338aec0a4ace5d4f491786915e/claude-api/`)을 먼저 읽고 라이브 문서로 대조했다. 코드·Git·운영 시스템에는 손대지 않았다. "검색 요약만 확인"이라고 적은 항목은 원문 페이지를 열지 못했다(403 등).

저장소 쪽 사실(어댑터 설계 제약):
- 웹 의존성: `next 16.3.4`, `zod ^4.5.4`, `drizzle-orm ^0.45.2`, `mysql2`, `better-auth ^1.7.3`이고 AI SDK는 아직 없다(web/package.json:20-41).
- 환경변수는 Zod `envSchema`와 `getEnv()` 싱글톤으로 관리한다(web/src/server/lib/env.ts:6-25, 66-70). 새 KEK 변수는 여기에 키 이름만 추가하게 된다.
- `node:crypto`는 이미 쓰인다(`createHash`·`randomBytes`, web/src/server/lib/token.ts:1-8). AES-GCM에 새 패키지가 필요 없다.
- Vercel 리전은 `icn1`이다(web/vercel.json:4). 리플레이 기본 상한은 4 MiB다(web/src/server/lib/env.ts:4).

---

## 1. Anthropic Messages API

| 항목 | 확인 내용 | 근거 |
|---|---|---|
| 엔드포인트·인증 | `POST https://api.anthropic.com/v1/messages`. 헤더는 `x-api-key: <key>`, `anthropic-version: 2023-06-01`, `content-type: application/json`, 베타 기능만 `anthropic-beta`. | 스킬 curl/examples.md "Required Headers"; https://platform.claude.com/docs/en/api/errors.md (cURL 예) |
| 구조화 출력 | `output_config: { format: { type: "json_schema", schema: {...} } }`. 응답 text 블록에 스키마에 맞는 JSON이 들어온다. 예전 `output_format`은 deprecated. | https://platform.claude.com/docs/en/build-with-claude/structured-outputs.md; 스킬 SKILL.md "Structured outputs" |
| 지원 모델 | claude-fable-5-1, claude-opus-5-5, claude-opus-5, claude-sonnet-5-5, claude-sonnet-5, claude-haiku-5-5, claude-opus-4-8/4-7/4-6, claude-sonnet-4-6, claude-haiku-4-5-20251001 등. | 같은 structured-outputs 페이지 supportedModels |
| 스키마 지원 | 지원: object, array, string, integer, number, boolean, null / enum(원시값만) / const / anyOf·allOf(`allOf`+`$ref`는 불가) / `$ref`·`$def`·`definitions`(외부 ref 불가) / default / required / `additionalProperties:false`(객체마다 필수) / string format(date-time, time, date, duration, email, hostname, uri, ipv4, ipv6, uuid) / `minItems` 0·1. 정규식은 일부만(역참조·lookaround·`\b` 불가). | 같은 페이지 |
| 스키마 미지원 | 재귀 스키마, enum 안의 복합 타입, 외부 `$ref`, `minimum`/`maximum`/`multipleOf`, `minLength`/`maxLength`, `minItems` 0·1을 넘는 배열 제약, `additionalProperties`가 false가 아닌 값. 미지원 기능을 쓰면 400. | 같은 페이지 |
| 복잡도 한도 | strict 도구 20개, optional 파라미터 합계 24개, union 타입(anyOf·`["string","null"]`) 파라미터 합계 16개. 넘으면 400 "Schema is too complex for compilation". 컴파일 타임아웃 180초. 컴파일된 문법은 마지막 사용 뒤 24시간 캐시된다. `output_config.format`을 바꾸면 그 대화의 프롬프트 캐시가 무효화된다. | 같은 페이지 |
| 거부·잘림 | `stop_reason: "refusal"`(HTTP 200, 생성 토큰 과금, 스키마 불일치 가능). `stop_reason: "max_tokens"`이면 JSON이 미완성일 수 있다. 스키마에 "단계별 추론" 같은 필드를 넣으면 `reasoning_extraction` 거부가 날 수 있으니 짧은 설명 필드로 요청하라고 한다. | 같은 페이지 |
| 강제 tool use | Opus 5.5·Sonnet 5.5·Fable 5.1에서 `tool_choice: any/tool`은 400("not supported for this model"). JSON을 받으려면 `output_config.format`을 쓰라고 한다. | https://platform.claude.com/docs/en/api/errors.md "Forced tool use not supported" |
| thinking | Opus 5.5는 thinking이 항상 켜져 있다. `{type:"disabled"}`와 `budget_tokens`는 400이고, 깊이는 `output_config.effort`(low~max, 기본 medium)로만 조절한다. Sonnet 5.5는 끄려면 `{type:"between_tools"}`, Haiku 5.5는 disabled를 high 이하에서만 허용한다. | errors.md "Thinking cannot be disabled"; https://platform.claude.com/docs/en/about-claude/models/overview.md |
| 스트리밍 | `"stream": true`로 SSE가 온다. 이벤트는 `message_start`, `content_block_start`, `content_block_delta`(text_delta·input_json_delta·thinking_delta), `content_block_stop`, `message_delta`, `message_stop`, `ping`, `error`. `message_delta`의 usage는 누적값이다. 200 이후 `{"type":"error","error":{"type":"overloaded_error",...}}`가 올 수 있다. 알 수 없는 이벤트는 무시하도록 만든다. | https://platform.claude.com/docs/en/build-with-claude/streaming.md (Event types, Error events) |
| 모델 목록 | `GET /v1/models`(`after_id`·`before_id`·`limit` 1~1000·`lifecycle`). 필드: id, display_name, created_at, lifecycle, line, max_input_tokens, max_tokens, capabilities(structured_outputs.supported, thinking.types, effort 등), deprecated_at, retires_at. `capabilities.structured_outputs.supported`로 모델 선택지를 거를 수 있다. | https://platform.claude.com/docs/en/api/models/list.md |
| 키 확인용 가벼운 호출 | (a) `GET /v1/models?limit=1`: 문서상 인증 헤더가 필요하다. 과금 여부는 문서에서 확인하지 못했다. (b) `POST /v1/messages/count_tokens`: 문서에 "free to use"라고 적혀 있고, RPM 한도는 Start 5,000 / Build 10,000 / Scale 20,000이며 메시지 생성 한도와 분리된다. 401 `authentication_error`면 무효 키다. | models/list.md; https://platform.claude.com/docs/en/build-with-claude/token-counting.md "Pricing and rate limits" |
| 오류 형식 | `{"type":"error","error":{"type":"...","message":"..."},"request_id":"req_..."}`. 상태별: 400 invalid_request_error, 401 authentication_error, 402 billing_error, 403 permission_error, 404 not_found_error, 409 conflict_error, 413 request_too_large, 429 rate_limit_error, 500 api_error, 504 timeout_error, 529 overloaded_error. 응답 헤더 `request-id`. | https://platform.claude.com/docs/en/api/errors.md |
| rate limit 헤더 | `retry-after`, `anthropic-ratelimit-{requests,tokens,input-tokens,output-tokens}-{limit,remaining,reset}`(reset은 RFC 3339). 월 지출 상한 도달 429는 `retry-after`가 없고 `error.details.error_code: "enforced_spend_limit_reached"`다. 사용자가 직접 정한 지출 한도에 닿으면 400 invalid_request_error다. RPM·ITPM·OTPM은 토큰 버킷 방식이고, 캐시 읽기 토큰은 ITPM에 들어가지 않는다. 스킬 error-codes.md는 `x-ratelimit-*`로 적었지만 라이브 문서는 `anthropic-ratelimit-*`이므로 라이브 문서를 따른다. | https://platform.claude.com/docs/en/api/rate-limits.md "Response headers"; 스킬 shared/error-codes.md:152-154 |
| 프롬프트 캐싱 | `cache_control: {type:"ephemeral"}`(5분, 읽힐 때마다 갱신) 또는 `ttl:"1h"`. breakpoint 최대 4개. 요청 최상위 `cache_control`로 자동 배치도 된다. 최소 길이: Opus 5.5·Sonnet 5.5·Haiku 5.5 등 512 토큰, Opus 4.8·Sonnet 5 1024, Haiku 4.5 4096. 짧으면 오류 없이 캐시되지 않는다. 쓰기 1.25배(5분)·2배(1h), 읽기 0.1배(Opus 5.5·Sonnet 5.5는 0.05배). usage 필드: `cache_creation_input_tokens`, `cache_read_input_tokens`, `input_tokens`, `cache_creation.ephemeral_{5m,1h}_input_tokens`. | https://platform.claude.com/docs/en/build-with-claude/prompt-caching.md |
| 권장 모델 id | 문서: "If you're unsure which model to use, start with Claude Opus 5.5". id는 `claude-opus-5-5`($4/$20 per MTok, 1M ctx, 128K out, effort 기본 medium). 그 밖에 `claude-sonnet-5-5`($2/$10), `claude-haiku-5-5`(from $0.10/$0.50), `claude-fable-5-1`($10/$50). | https://platform.claude.com/docs/en/about-claude/models/overview.md |
| 거부 fallback(선택) | 스킬은 opus-5-5 코드에 `anthropic-beta: server-side-fallback-2026-07-01` + `fallbacks: "default"`를 기본으로 넣으라고 권한다. 라이브 문서와의 대조는 하지 않았다. | 스킬 SKILL.md "refusal stop reason" |
| SDK 버전 | `@anthropic-ai/sdk` 0.135.0 (npm 2026-10-15) | https://registry.npmjs.org/@anthropic-ai%2Fsdk |

## 2. OpenAI (Responses·Chat Completions·Codex)

| 항목 | 확인 내용 | 근거 |
|---|---|---|
| 인증 | `Authorization: Bearer OPENAI_API_KEY`. 여러 조직·프로젝트에 속하면 선택적으로 `OpenAI-Organization`, `OpenAI-Project`. 응답 헤더: `x-request-id`, `openai-processing-ms`, `openai-version`(2020-10-01). 클라이언트가 `X-Client-Request-Id`를 보낼 수 있다. | https://developers.openai.com/api/reference/overview |
| Responses API | `POST /v1/responses`. `max_output_tokens`(최소 16, reasoning 토큰 포함). `store`는 기본 true이고 true면 응답이 최소 30일 저장된다. BYOK 리포트에는 `store:false`가 맞다. | https://developers.openai.com/api/reference/resources/responses/methods/create |
| Structured Outputs (Responses) | `text: { format: { type: "json_schema", strict: true, schema: ... } }` | https://developers.openai.com/api/docs/guides/structured-outputs |
| Structured Outputs (Chat) | `response_format: { type: "json_schema", json_schema: { name(a-zA-Z0-9_- 64자), description, schema, strict } }`. `json_object`는 예전 JSON 모드. | https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create |
| strict 스키마 규칙 | 루트는 object여야 하고 anyOf가 될 수 없다. 모든 필드를 `required`에 넣는다(optional은 `["string","null"]`로 흉내). `additionalProperties:false` 필수. 출력 키 순서는 스키마 순서를 따른다. 지원: String(pattern, format date-time·time·date·duration·email·hostname·ipv4·ipv6·uuid), Number(multipleOf, maximum, exclusiveMaximum, minimum, exclusiveMinimum), Array(minItems, maxItems), Enum, anyOf, `$defs`·definitions, 재귀(`#`, `$defs`). 미지원: allOf, not, dependentRequired, dependentSchemas, if/then/else(파인튜닝 모델은 더 좁다, patternProperties 등). 한도: 객체 속성 합계 5000개, 중첩 10단, 이름·enum·const 문자열 길이 합 120,000자, enum 값 합계 1000개(한 enum에 250개를 넘으면 그 값들의 길이 합 15,000자). | 같은 guide (offset 100000 이후 "Supported schemas") |
| 거부 | content에 `refusal` 항목이 올 수 있고 스키마를 따르지 않는다. max 토큰에서 멈추면 `incomplete` 상태와 `max_output_tokens` 사유가 붙는다. | 같은 guide |
| 스트리밍 | Responses: `stream:true` → `response.created`, `response.output_text.delta`, `response.completed`(마지막 response 객체에 usage 포함), `error`, `response.failed` 등. Chat: `stream_options.include_usage:true`면 `[DONE]` 직전 chunk에 전체 usage가 온다(중단되면 못 받을 수 있다). | https://developers.openai.com/api/docs/guides/streaming-responses; chat create 레퍼런스 |
| usage 필드 | `usage.input_tokens`, `input_tokens_details.{cached_tokens, cache_write_tokens}`, `output_tokens`, `output_tokens_details.reasoning_tokens`, `total_tokens` | responses create 레퍼런스 |
| 모델 목록 | `GET /v1/models` → `data[]{id, created, object, owned_by, shutdown_date}`. 구조화 출력 지원 여부 같은 capability 필드는 없다. | https://developers.openai.com/api/reference/resources/models/methods/list |
| 키 확인 | `GET /v1/models`가 가장 가볍다(Bearer). 과금 여부는 문서에서 확인하지 못했다. | 같은 페이지 |
| 오류 | 필드: `error.message`·`type`·`param`·`code`. 401 Invalid Authentication / Incorrect API key / IP not authorized, 403 지역 미지원, 429 `credit_balance_exhausted`·`slow_down`·`organization_spend_limit_exceeded`·`project_spend_limit_exceeded`·`organization_usage_limit_exceeded`(type은 `insufficient_quota`일 수 있다. 재시도해도 소용없다), 500, 503 `server_is_overloaded`. | https://developers.openai.com/api/docs/guides/error-codes |
| rate limit 헤더 | `Retry-After`(있을 때만), `x-ratelimit-{limit,remaining,reset}-{requests,tokens}`, 프로젝트 한도가 있으면 `x-ratelimit-*-project-tokens`. 지표: RPM·RPD·TPM·TPD. 조직·프로젝트 단위로 걸린다. | https://developers.openai.com/api/docs/guides/rate-limits |
| 현재 모델 | API 모델 페이지: `gpt-6-astra`(가장 높은 성능), `gpt-6.1-sol`(Astra에 가까운 성능, 더 쌈), `gpt-6-luna`(효율). "Start with GPT-6 Astra for complex reasoning and coding." structured-outputs 가이드에도 "For new projects, start with gpt-6-astra". | https://developers.openai.com/api/docs/models |
| "codex"의 실체 | Codex는 OpenAI의 코딩 에이전트 제품이다(데스크톱 앱, CLI, IDE 확장, cloud). 로그인은 두 가지다. "Sign in with ChatGPT for subscription access"와 "Sign in with an API key for usage-based access". API 키를 쓰면 "standard API pricing instead of included ChatGPT plan credits"가 적용된다. Codex cloud는 ChatGPT 로그인이 필수다. | https://learn.chatgpt.com/docs/auth (developers.openai.com/codex/auth가 308로 넘겨 줌) |
| codex 계열 모델 | Codex 문서는 GPT-6.1 Sol을 권장하고 별도 Codex 전용 계열은 없다. `gpt-5.3-codex`·`gpt-5.3-codex-spark`는 deprecated(spark는 2026-09-14 은퇴)다. `gpt-5.3-codex` API 페이지 기준으로 Responses만 지원하고 Chat Completions는 미지원, Structured outputs는 지원한다. API 키 세션의 모델 가용성은 API 조직·프로젝트를 따른다. | https://learn.chatgpt.com/docs/models; https://developers.openai.com/api/docs/models/gpt-5.3-codex |
| 제3자 웹 서비스가 ChatGPT/Codex 구독으로 API를 부를 수 있나 | "Sign in with ChatGPT"에는 두 갈래가 있다. (1) 웹사이트용 신원 로그인: scope `openid profile email`, id_token만 받는다. "Identity scopes don't grant access to ChatGPT conversations or OpenAI API resources". 그것도 "currently available to selected commercial partners through a limited trial"이고 waitlist를 거친다. (2) 플랜 사용(ChatGPT plan usage): 문서 범위는 "open-source and locally hosted apps"다. "available to all open-source partners and selected private clients", 승인된 앱만, Plus·Pro 대상(헬프센터 검색 요약에는 Go도 있다고 해 출처끼리 엇갈림). paid 또는 remotely hosted 앱은 interest form으로 안내만 한다. Vercel에 올린 R-BMS 웹은 remotely hosted에 해당하므로 지금 공개된 경로로는 쓸 수 없다고 판단한다. | https://developers.openai.com/siwc/quickstart; https://developers.openai.com/siwc/website; https://developers.openai.com/siwc/token-sharing-open-source |
| 대안 | "OpenAI API key + 모델 선택(`GET /v1/models`로 고르게)"이 공식 문서와 맞는 대안이다. codex 이름 모델은 deprecated이고 권장도 일반 모델(gpt-6.1-sol·gpt-6-astra)이다. UI 문구는 "OpenAI (API key)"로 하고, 선택지에 codex 계열이 있으면 보여 주되 기본값으로 두지 않는 편이 맞다. | 위 근거 종합 |
| 키 공유 관련 공식 문구 | OpenAI 헬프 "Best Practices for API Key Safety"는 403이라 원문을 읽지 못했다. 검색 요약상으로는 키 공유를 지원하지 않는다, 클라이언트 측(브라우저·모바일)에 노출하지 말라, 백엔드를 거치라고 한다. 원문을 열린 production best practices에는 "Avoid exposing the API keys in your code or in public repositories", 환경변수나 secret manager를 쓰라는 문구가 있다. BYOK를 직접 다룬 공식 정책은 찾지 못했다(커뮤니티 글만 있음). | https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety (403); https://developers.openai.com/api/docs/guides/production-best-practices |
| SDK 버전 | `openai` 7.39.0 (2026-10-19) | npm registry |

## 3. Ollama Cloud

| 항목 | 확인 내용 | 근거 |
|---|---|---|
| 엔드포인트 | 네이티브 `https://ollama.com/api/*`(예: `/api/chat`), OpenAI 호환 `https://ollama.com/v1`, Anthropic 호환 `/v1/messages`(호스팅) | https://docs.ollama.com/cloud.md; https://docs.ollama.com/api/authentication.md; https://docs.ollama.com/api/openai-compatibility.md |
| 인증 | `Authorization: Bearer $OLLAMA_API_KEY`(키는 ollama.com/settings/keys에서 만든다). Anthropic 호환 엔드포인트도 Bearer만 받고 `x-api-key`만 보내면 안 된다. "API keys do not expire"이고 취소는 설정에서 한다. "Keep keys outside browser code and source control." | https://docs.ollama.com/api/authentication.md |
| 구조화 출력 | 로컬에서는 `/api/chat`의 `format`에 JSON schema나 `"json"`을 넣는다. 문서 맨 위 Note: **"Ollama's cloud currently does not support structured outputs."** OpenAI 호환은 `response_format`이 체크돼 있지만 json_schema를 명시하지 않았다. Anthropic 호환은 `output_config.effort`만 지원하고 `format`은 목록에 없으며 `tool_choice`는 미지원이다. 즉 Ollama Cloud에서는 스키마로 제약된 디코딩을 보장받을 수 없다. | https://docs.ollama.com/capabilities/structured-outputs.md 8행; openai-compatibility.md 211-247행; anthropic-compatibility.md 274-326행 |
| 스트리밍 | `/api/chat`은 `stream` 기본 true이고 NDJSON(`application/x-ndjson`)이며 마지막 객체에 `done:true`, `done_reason`이 붙는다. 스트림 중 오류는 상태코드가 바뀌지 않고 `{"error":"..."}` 줄로 온다. OpenAI 호환은 `stream_options.include_usage`를 지원한다. | https://docs.ollama.com/api/streaming.md; https://docs.ollama.com/api/errors.md |
| 토큰 사용량 | 네이티브 응답의 `prompt_eval_count`, `prompt_eval_cached_count`, `eval_count`(시간 필드는 ns) | https://docs.ollama.com/api/usage.md; api/chat.md ChatResponse |
| 모델 목록 | `GET https://ollama.com/api/tags`(문서 예시에 인증 헤더가 없다), OpenAI 호환 `/v1/models`. 직접 호출할 때는 tags가 주는 이름(예 `gemma4:31b`)을 쓴다. | cloud.md; openai-compatibility.md 354행 |
| 키 확인 | `GET https://ollama.com/api/balance`(Bearer, "10 requests per minute per user", 429 + `Retry-After`) 또는 `GET https://ollama.com/api/usage?range=24h`. tags는 인증이 없을 수 있어 키 확인에 맞지 않는다(실제로 확인하지는 않았다). | https://docs.ollama.com/api/balance.md; https://docs.ollama.com/api/cloud-usage.md |
| 오류 | 400, 404, 429, 500, 502(클라우드 모델에 닿지 못함). 본문은 `{"error":"message"}` 문자열 하나다. | https://docs.ollama.com/api/errors.md |
| 사용량 제한 | 요금제별 동시 요청 수: Free 1, Pro($20) 3, Max($100) 10, Team($500) 10. 한도를 넘는 요청은 대기열에 들어가고 대기열이 차면 거절된다. 사용량은 모델별 토큰 단가로 크레딧에서 차감되고 이월되지 않는다. RPM·TPM 숫자는 공개돼 있지 않다. | https://ollama.com/pricing |
| 데이터 | "do not store or log that content and never train on it"(FAQ), 가격 페이지 "Prompt or response data is never logged or trained on" | https://docs.ollama.com/faq.md 162행; ollama.com/pricing |
| JS 클라이언트 | 공식 `ollama` 0.6.4 (2026-09-29) | npm registry |

## 4. 셋을 하나로 감쌀 때

### 4.1 공통 분모와 차이

- **구조화 출력.** Anthropic과 OpenAI는 스키마로 제약된 디코딩을 한다. Ollama Cloud는 문서상 지원하지 않는다. 따라서 어댑터 계약은 두 등급으로 둔다. "native strict"는 Anthropic·OpenAI이고, "best-effort JSON"은 Ollama Cloud다(스키마를 프롬프트에 넣고 응답을 Zod로 검증). 검증에 실패하면 UI에 "해석 생성 실패"를 그대로 보여 준다. 자동 재시도로 덮으면 HANDOFF.md 5절의 "재시도로 덮기 금지"에 걸릴 수 있으니 재시도 정책은 사용자에게 결정을 받아야 한다.
- **공통 JSON Schema 부분집합.** 두 strict 구현에서 모두 안전한 범위다.
  - 사용: object(모든 속성 required + `additionalProperties:false`), array, string, number, integer, boolean, null, enum(원시값), anyOf(루트 금지), `$defs`/`$ref`(내부만).
  - 금지: allOf(OpenAI 미지원), 재귀(Anthropic 미지원), `minimum`/`maximum`/`multipleOf`, `minLength`/`maxLength`(Anthropic 미지원), `minItems`가 0·1을 넘거나 `maxItems`(Anthropic 미지원), `pattern`(Anthropic 부분 지원), `const`(OpenAI 지원 목록에 없어 확인하지 못함).
  - optional 필드는 nullable union으로 표현하되, Anthropic의 "union 타입 파라미터 합계 16개", "optional 24개" 한도 안에 들어야 한다. 그래서 10개 기능의 응답 스키마는 얕고 작게 설계해야 한다.
  - 숫자 범위 같은 제약은 서버 Zod 검증으로 옮긴다(Anthropic SDK도 미지원 제약은 빼고 클라이언트에서 검증한다고 스킬 tool-use-concepts.md "JSON Schema Limitations"에 적혀 있다).
- **스트리밍.** 전송 형식이 다르다. Anthropic은 SSE 이벤트, OpenAI Responses는 SSE `response.*`, Ollama 네이티브는 NDJSON이고, 셋 다 200 이후 스트림 안에서 오류가 올 수 있다. 구조화 JSON은 다 모은 뒤에 파싱해야 한다. 현재 기능(리포트·코치 등)은 결과가 JSON이므로 스트리밍은 "진행 표시"용으로만 쓰거나, 비스트리밍(max_tokens를 작게)으로 단순화하는 편이 맞다.
- **토큰 사용량 필드.**
  - Anthropic: `usage.input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`
  - OpenAI: `usage.input_tokens`, `output_tokens`, `input_tokens_details.cached_tokens`, `output_tokens_details.reasoning_tokens`
  - Ollama: `prompt_eval_count`, `eval_count`, `prompt_eval_cached_count`
  - 공통 정규형은 `{ inputTokens, outputTokens, cachedInputTokens }`로 둘 수 있다.
- **거부·잘림 신호.**
  - Anthropic: `stop_reason` refusal / max_tokens
  - OpenAI: content의 `refusal` 항목, `status: incomplete`
  - Ollama: `done_reason`
- **키 확인 호출.**
  - Anthropic: `POST /v1/messages/count_tokens`(무료라고 문서화됨) 또는 `GET /v1/models`
  - OpenAI: `GET /v1/models`
  - Ollama: `GET /api/balance`(10 rpm)
- **모델 목록.** Anthropic만 capability(structured_outputs 등)를 준다. OpenAI·Ollama는 id만 주므로 구조화 출력 지원 여부를 목록에서 거를 수 없다.
- **오류·한도 헤더.** Anthropic은 `anthropic-ratelimit-*` + `retry-after` + `request-id`, OpenAI는 `x-ratelimit-*` + `Retry-After` + `x-request-id`, Ollama는 429 + `Retry-After`(balance 문서)만 확인됐다.

### 4.2 Vercel AI SDK

- 최신 버전(npm, 설치하지 않았다): `ai` 7.0.143, `@ai-sdk/anthropic` 4.0.82, `@ai-sdk/openai` 4.0.96, `@ai-sdk/openai-compatible` 3.0.71, `@ai-sdk/provider` 4.0.29(전부 2026-10-16, peer `zod ^3.25.76 || ^4.1.8`).
- Anthropic(`@ai-sdk/anthropic`)과 OpenAI(`@ai-sdk/openai`)는 공식 provider다. Ollama는 커뮤니티 provider다: `ollama-ai-provider`(1.2.0, 2025-01, zod 3 peer라 오래됨), `ollama-ai-provider-v2`(4.0.1, 2026-07, nordwestt), `ai-sdk-ollama`(4.5.1, 2026-10-17, jagreehal, 공식 ollama JS 클라이언트 기반). 커뮤니티 페이지는 Ollama Cloud API 키 인증을 다루지 않는다. 근거: https://ai-sdk.dev/docs/foundations/providers-and-models; https://ai-sdk.dev/providers/community-providers/ollama
- 구조화 출력 API는 `generateText`/`streamText` + `output: Output.object({ schema })`(Zod·Valibot·JSON Schema)다. 실패하면 `NoObjectGeneratedError`(usage 보존). `generateObject`의 deprecated 여부는 이 페이지에서 확인하지 못했다. 근거: https://ai-sdk.dev/docs/ai-sdk-core/generating-structured-data
- `createOpenAICompatible`에는 `baseURL`·`apiKey`(Bearer)·`headers`·`includeUsage`·`supportsStructuredOutputs` 옵션이 있어 Ollama Cloud(`https://ollama.com/v1`)에 붙일 수 있다. 다만 Ollama Cloud가 구조화 출력을 지원하지 않으므로 `supportsStructuredOutputs`는 false로 두게 된다. 근거: https://ai-sdk.dev/providers/openai-compatible-providers
- 장점: 세 공급자 호출·스트리밍·usage 정규화를 한 API로 받고, Zod 스키마를 그대로 넘길 수 있다.
- 단점:
  - 의존성 4~5개가 늘고 major 버전이 자주 바뀐다(ai 7.x).
  - Ollama가 커뮤니티 품질이다.
  - 공급자별 신기능(Anthropic `fallbacks`, `cache_control`, effort; OpenAI `store:false`·`safety_identifier`)은 `providerOptions`로 넘겨야 하고, 그 지원 여부는 이번에 확인하지 않았다.
  - rate limit 헤더처럼 raw 응답 접근이 덜 직접적이다.
- 직접 fetch 어댑터
  - 장점: 의존성이 0이고 이미 있는 Zod·`node:crypto`만으로 된다. 요청 본문·헤더·오류를 정확히 통제하고, 키가 로그에 남지 않게 다루기 쉽다.
  - 단점: SSE·NDJSON 파서와 공급자별 오류 매핑을 직접 유지해야 한다.
  - 사용자 컨벤션(ai-process.md §6.6 의존성 다이어트)과 claude-api 스킬의 "Anthropic은 공식 SDK 사용" 원칙이 서로 당긴다. 공급자별 공식 SDK(`@anthropic-ai/sdk`, `openai`, `ollama`)를 각 어댑터 안에서만 쓰는 중간안도 가능하다. 어느 쪽으로 갈지는 사용자 결정 사항이다.

## 5. BYOK 키 보관

- **알고리즘.**
  - OWASP: 대칭 암호는 AES 키 128비트 이상(가능하면 256)이고 인증 모드(GCM·CCM)를 먼저 쓴다. IV는 같은 키를 쓰는 모든 인스턴스에서 고유해야 한다.
  - NIST SP 800-38D: IV는 96비트로 제한해 구현하라고 권장한다. "The total number of invocations of the authenticated encryption function shall not exceed 2^32 ... with the given key"(무작위 IV를 쓰면 키당 호출 2^32회 이하. 사용자 키 저장 빈도에서는 여유가 크지만 키 버전을 두는 근거가 된다). IV 고유성이 깨지면 "may compromise the security assurance almost entirely".
  - Node: `cipher.getAuthTag()`는 `final()` 뒤에 부른다. `decipher.setAuthTag()`는 GCM에서 `final()` 전에 한 번만 부른다. tag가 틀리거나 위조됐으면 `final()`이 throw한다. 128비트가 아닌 tag 길이는 `authTagLength` 없이 쓰면 v26부터 거부된다. `setAAD()`는 `update()` 전에 부른다.
  - 근거: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html; https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf (5.2.1.1, 8.3); https://nodejs.org/api/crypto.html
- **봉투 암호화.** OWASP: "DEK is used to encrypt the data", "KEK is used to encrypt the DEK", "the KEK must be stored separately from the DEK", 키는 데이터와 다른 곳에 두고 코드·VCS에 넣지 말라, 회전 기준에 닿으면 새 키로 새 데이터를 암호화하라. 이 프로젝트에 맞춘 안은 다음과 같다(제안).
  - DB 행: `provider`, `ciphertext`, `iv(12B)`, `authTag(16B)`, `wrappedDek`, `dekIv`, `dekTag`, `kekVersion`, `keyHint`(마지막 4자 정도), `createdAt`, `lastValidatedAt`.
  - AAD: `userId|provider|kekVersion`을 묶어 다른 사용자 행에 붙여 넣는 공격을 막는다.
  - KEK: 환경변수에 버전별로 둔다(예: 키 이름 `AI_KEY_KEKS`, 값은 `{v: base64}` 묶음). env.ts Zod 스키마에 키 이름만 추가하고 값은 사용자가 Vercel Secret 타입으로 넣는다(HANDOFF.md 5절의 `.env` 접근 금지·`vercel link` 금지에 맞다).
  - 회전: 새 버전으로 쓰고, 읽을 때 옛 버전으로 풀어 새 버전으로 다시 감싸는 lazy rewrap이나 일괄 작업으로 한다.
- **Vercel 쪽 사실.**
  - Secret 타입 환경변수는 "write-only after saving"이고, 32자 이상 값이 빌드 로그에 나오면 `[REDACTED]`로 가린다(https://vercel.com/docs/environment-variables/sensitive-environment-variables).
  - Vercel KMS(Beta)는 서명 전용이다. "KMS does not support symmetric (HS*) keys"라 봉투 암호화의 KEK로 쓸 수 없다(https://vercel.com/docs/kms).
  - Functions 최대 실행 시간(fluid): Hobby 300s, Pro 기본 300s·최대 800s. 요청·응답 본문 4.5 MB(https://vercel.com/docs/functions/limitations). 차트 패턴 업로드와 AI 호출 시간 설계 둘 다 이 한도를 받는다.
- **로그·오류·클라이언트 노출 금지.**
  - Anthropic: "don't share your API key", "always add your API key as an encrypted secret", 90일 회전 예시, GitHub secret scanning으로 노출된 키를 자동 비활성화한다(https://support.claude.com/en/articles/9767949-api-key-best-practices-keeping-your-keys-safe-and-secure).
  - Ollama: "Keep keys outside browser code and source control".
  - OpenAI: 클라이언트 측에 노출하지 말라(검색 요약).
  - 구현 원칙(제안):
    - 평문 키는 서버 함수 안에서만 복호화하고 응답·로그·오류 메시지·Sentry에 넣지 않는다.
    - 공급자 오류 본문은 status·type·request id만 남기고 버린다.
    - UI에는 `keyHint`만 보여 준다.
    - 키 저장 API는 POST만 두고 GET으로 평문을 돌려주지 않는다.
- **약관상 주의.**
  - Anthropic Claude Code 법무 페이지(범위: Claude Code·Agent SDK 등 Claude 기능과 상호작용하는 제품)는 다음과 같이 말한다.
    - 제3자 개발자가 Claude.ai 로그인을 자기 앱에 제공하거나 Free/Pro/Max 플랜 자격으로 요청을 보내는 것을 허용하지 않는다.
    - "developers may not collect, store, or intermediate Claude.ai credentials or session tokens".
    - Claude Code를 넣은 제품에 대해서는 "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key ..."라고 적고 있다.
    - R-BMS의 BYOK는 사용자 자신의 Console API 키이고 과금도 키 주인에게 간다. 그래서 금지된 형태(구독 로그인 중계, 재판매)에는 해당하지 않는다고 판단한다. 다만 이 문단은 Claude Code 문서에 있는 문구라 Messages API BYOK에 직접 적용되는지는 법적으로 확정하지 못했다.
    - Claude.ai 구독 로그인 연동은 금지 대상이므로 만들면 안 된다.
    - 근거: https://code.claude.com/docs/en/legal-and-compliance
  - Anthropic 지원 문서는 "Uploading your key to third-party tools gives the developer ... access"라는 취지라(검색 요약 문구, 원문에서 찾은 문장은 "If you don't trust their reputation, don't trust them with your API key"), 사용자 고지 문구를 화면에 넣는 것이 맞다.
  - OpenAI ChatGPT 구독 연동은 4절대로 원격 호스팅 앱에는 공개 경로가 없다.

## 추천 구조 요약(조사 결론, 결정은 사용자)

1. 어댑터 계약 `generateStructured({ provider, model, system, input, schema(Zod) }) → { data, usage: {inputTokens, outputTokens, cachedInputTokens}, stopKind, requestId }`. 공급자마다 하나씩 만든다.
   - Anthropic: `output_config.format`(json_schema), thinking은 생략, effort를 명시(Opus 5.5 기본 medium), 큰 고정 system 프롬프트에 `cache_control`.
   - OpenAI: Responses `text.format` strict + `store:false`.
   - Ollama Cloud: 네이티브 `/api/chat` + `stream:false`. 스키마는 프롬프트에 넣고 Zod로 검증하며, 화면에 "엄격 보장 없음" 등급을 표시한다.
2. 스키마는 4.1의 공통 부분집합으로만 쓰고 union·optional 수를 Anthropic 한도 안에 둔다.
3. 키 확인: Anthropic count_tokens, OpenAI models, Ollama balance.
4. 저장: AES-256-GCM 봉투 + KEK 버전 + AAD. KEK는 Vercel Secret 환경변수로 둔다.

---

## 조사자가 낸 제약 목록 (원문)

1. Ollama Cloud는 공식 문서상 구조화 출력을 지원하지 않는다(https://docs.ollama.com/capabilities/structured-outputs.md 맨 위 Note). 세 공급자에 같은 엄격 보장을 줄 수 없으므로, 어댑터는 native strict(Anthropic·OpenAI)와 best-effort+Zod 검증(Ollama) 두 등급을 둬야 한다
2. 공통 strict 스키마 부분집합: 모든 객체 required 전부 + additionalProperties:false, allOf·재귀·숫자/문자열 길이 제약·minItems>1·maxItems·외부 $ref 금지, 루트는 object(anyOf 금지). 제약은 서버 Zod로 검증한다
3. Anthropic 스키마 복잡도 한도: strict 도구 20, optional 파라미터 24, union 타입 파라미터 16(합계). 넘으면 400. nullable로 optional을 흉내 내면 union 한도를 소모한다
4. Claude Opus 5.5·Sonnet 5.5·Fable 5.1은 tool_choice any/tool이 400이다. JSON은 output_config.format으로 받는다. Opus 5.5는 thinking을 끌 수 없다(disabled·budget_tokens는 400). effort 기본값이 medium이므로 명시한다
5. OpenAI Responses의 store 기본값은 true(최소 30일 저장)이다. 사용자 데이터 해석 요청은 store:false로 보내는 것이 맞다
6. OpenAI·Anthropic 모두 구독(ChatGPT·Claude.ai) 로그인을 원격 호스팅 웹 서비스가 중계하는 공개 경로가 없다. Anthropic은 Claude.ai 자격 수집·저장·중계를 금지한다고 명시한다. 그래서 "codex"는 OpenAI API key + 모델 선택으로 구현한다
7. codex 이름 모델(gpt-5.3-codex 등)은 deprecated이고 Chat Completions를 지원하지 않는다(Responses만). OpenAI 어댑터는 Responses API 기준으로 만든다
8. Vercel KMS는 서명 전용이고 대칭 키를 지원하지 않는다. 봉투 암호화의 KEK는 Vercel Secret 환경변수(write-only)에 버전별로 두고, web/src/server/lib/env.ts Zod 스키마에 키 이름만 추가한다(값은 사용자가 넣는다, .env 읽기·쓰기 금지)
9. AES-256-GCM: 96비트 무작위 IV를 매번 새로 만들고, 키당 호출 2^32회 이하(NIST SP 800-38D 8.3), 128비트 tag, AAD에 userId·provider·kekVersion을 묶는다. node:crypto로 충분하므로 새 패키지가 필요 없다(web/src/server/lib/token.ts:1 이미 사용 중)
10. 평문 API 키는 서버 함수 안에서만 복호화한다. 응답·로그·오류 메시지·클라이언트 번들로 보내지 않고, 공급자 오류 본문은 status·type·request id만 남긴다
11. Vercel Functions: Hobby 최대 300s, Pro 기본 300s·최대 800s, 요청·응답 본문 4.5 MB. AI 호출 시간과 차트 패턴 업로드 크기 설계가 이 한도를 받는다
12. AI SDK의 Ollama provider는 커뮤니티 패키지뿐이다(ollama-ai-provider-v2 4.0.1, ai-sdk-ollama 4.5.1). 공식 provider는 Anthropic·OpenAI만 해당된다
13. 스트리밍은 세 공급자의 전송 형식이 다르고(SSE 두 종류, NDJSON), 셋 다 200 이후 스트림 안에서 오류가 올 수 있다. 구조화 JSON은 다 모은 뒤 파싱해야 한다

## 확인하지 못한 것 (원문)

1. Ollama Cloud에서 format:"json"(스키마 없는 JSON 모드)이 동작하는지, OpenAI 호환 /v1 response_format json_schema가 클라우드에서 무시되는지 오류가 나는지 확인하지 못했다(실제 키로 호출하지 않았다)
2. Ollama Cloud의 RPM·TPM 같은 수치 한도와 429 응답 헤더 형식은 공개 문서에서 찾지 못했다. 요금제별 동시 요청 수(1/3/10)만 확인했다
3. ollama.com/api/tags가 인증 없이 응답하는지, 키를 넣으면 계정별 목록이 달라지는지 확인하지 못했다
4. Anthropic GET /v1/models, OpenAI GET /v1/models 호출의 과금 여부는 문서에서 확인하지 못했다(Anthropic count_tokens는 문서상 무료)
5. OpenAI 헬프센터 'Best Practices for API Key Safety'와 'Can I share my API key' 원문은 403이라 읽지 못했다. 검색 요약만 근거로 썼다
6. OpenAI strict 스키마에서 const 지원 여부는 지원 타입 목록에 없어 확인하지 못했다
7. OpenAI·Ollama의 BYOK(사용자 키를 제3자 서비스에 저장)를 직접 허용하거나 금지하는 공식 약관 문구는 찾지 못했다. Anthropic 문구도 Claude Code 법무 페이지에 있어 Messages API BYOK에 그대로 적용되는지는 법적으로 확정하지 못했다
8. AI SDK 7의 providerOptions로 Anthropic fallbacks·cache_control·effort, OpenAI store:false·safety_identifier를 모두 전달할 수 있는지 확인하지 않았다. generateObject의 deprecated 여부도 확인하지 못했다
9. Anthropic server-side fallback(fallbacks:"default", beta server-side-fallback-2026-07-01)은 스킬 문서로만 확인했고 라이브 문서와 대조하지 않았다
10. Sign in with ChatGPT 플랜 사용의 대상 요금제는 개발자 문서(Plus·Pro)와 헬프센터 검색 요약(Go·Plus·Pro)이 엇갈린다. paid·remotely hosted 앱용 interest form의 조건은 공개돼 있지 않다
11. 어댑터를 직접 fetch로 만들지, 공급자별 공식 SDK로 만들지, Vercel AI SDK로 만들지와 새 웹 의존성 허용 여부는 사용자 결정이 필요하다(HANDOFF.md 5절의 '새 의존성 금지'는 Rust 항목에 적혀 있어 웹에 적용되는지 확인이 필요하다)
12. Ollama 결과의 스키마 검증 실패를 자동 재시도할지, 실패로 그대로 보여 줄지는 '재시도로 덮기 금지' 규칙과 관련된 사용자 결정 사항이다
