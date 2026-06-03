# beatoraja와의 의도적/보류 차이 (M0/M1 적대적 리뷰 결과)

> 워크플로우 `wfgr2tk4o`(21 에이전트) 결과. 18개 발견 중 14개 확정, 그러나 검증자 판정상 **거의 전부 현 corpus(727곡)에 미발현**. 아래는 처리 결정.

## 수정 완료 (이번 커밋)
| # | 항목 | 파일 | 수정 |
|---|---|---|---|
| 1 | 데이터필드 토크나이저가 비영숫자 필터 후 재페어링 → 위치 어긋남 | parser/lib.rs | **위치 기준 페어링**, 잘못된 쌍=빈칸(beatoraja `parseInt36==-1→empty`와 일치) |
| 14 | `#BASE 62`에서 헤더 id는 base36 하드코딩, 객체값은 base62 → 불일치 | parser/lib.rs | `#BASE` 사전 스캔 + 헤더 id도 `src.base` 사용 |
| 11 | LNOBJ가 mine/LN으로 덮인 슬롯을 변환 → 손상 | chart/lib.rs | 추적 슬롯이 `Normal`일 때만 변환 |
| 8 | BPM≤0 / 음수 STOP → inf/NaN 전파 | chart/lib.rs | BPM≤0은 직전값 폴백, 음수 STOP은 0 클램프 |

검증: 회귀 테스트 4종 추가, 전체 30 테스트 통과, corpus 727/727 파싱·MD5 불변, 노트수 불변(α=2582, A+=2418, FELYS=812).

## 보류 (현 corpus 미발현 — 추후 필요 시)
| # | 항목 | 이유 |
|---|---|---|
| 2 | 중첩 `#IF`를 모든 프레임으로 게이트(beatoraja는 최내곽만) | corpus 최대 중첩 깊이 1, #RANDOM 사용 4곡뿐. **#RANDOM은 런타임 랜덤이라 해시·점수 무관** |
| 3 | `#RANDOM` 없는 고아 `#IF`를 비활성화(beatoraja는 유지) | 에디터가 항상 짝을 생성, 발현 희박 |
| 4 | `#ELSE/#ELSEIF` 구현(beatoraja는 미구현) | 우리 쪽이 spec-complete. corpus에 #ELSE 0건 |
| 5,10 | ch03 inline BPM의 G–Z 글자 처리 차이 | 실차트는 00–FF(hex)만 사용 → 우리 base16이 정확 |
| 6 | `#BASE 62`는 beatoraja에 없음 | 우리 확장. #14로 자체 일관성 확보. corpus 0건 |
| 7 | ch06(POOR layer)가 base BGA 필드 덮음 | BGA 렌더러 미구현(필드 write-only). BGA 단계에서 분리 |
| 9,12 | `#LNTYPE 2`(MGQ 연속형 LN) 미구현 | 사실상 사장된 레거시. corpus 0건. LNTYPE1/#LNOBJ는 구현·테스트됨 |
| 13 | 고아 `#LNOBJ` 꼬리를 Normal로 방출 | 권위 있는 동작 불명, Normal이 안전 폴백 |

## CN/HCN 판정 차별화 — ✅ 종단 판정 적용 (2026-06-03)

> 충실 포팅 스펙·구현 = `docs/reference/cn-hcn-judgment.md` (beatoraja `JudgeManager` 라인 대조).

`judge/matcher.rs`가 `LnKind`를 운반해 **CN/HCN을 2-판정(head@press + end@release)으로** 처리한다(beatoraja `updateMicro` ×2). 이른 릴리스는 end 윈도우로 판정(head로 capping 안 함), 미히트는 head+end 2 Miss. `count_playable_notes`/`total_notes`/gauge 분모를 CN/HCN=2로 일관. **`Cn`/`Hcn`에만 게이트**해 LN/Normal은 byte 불변. 합성 픽스처 5종으로 고정.
**잔여:** HCN 연속 게이지(`hcnmduration` ±0.5, Phase 7), CN deferral/재홀드, BSS. 노트-카운트 분모는 beatoraja BMS 모델이 외부 라이브러리라 미검증 — `JudgeManager` 2×updateMicro 증거 기반 + 내부 일관(→ 스펙 "미검증 경계").

## 기각 (not-a-bug)
- `#SETRANDOM/#RONDAM` 수용(우리 확장, 무해)
- 데이터라인 감지 colon@byte5(현 구현 충분)
- mechanics.md §2 STOP 공식 본문이 `/192` 누락(코드는 정확)
- 미종결 LN(LN 채널 홀수 개) → dangling LongStart 잔존(허용)

## 설계 입장
**#RANDOM 계열 제어흐름은 beatoraja와 bug-for-bug 호환하지 않는다.** 차트 해시는 raw 바이트 기준이라 점수·IR·리플레이 키는 영향 없고, #RANDOM 분기는 본래 플레이 시 무작위다. 우리 구현은 #ELSE/#ELSEIF/중첩을 spec에 맞게 더 완전히 처리한다 — 이는 의도된 개선이다.
