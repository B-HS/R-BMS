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

## 진행 중 — CN/HCN 판정 차별화 (2026-06-03 로드맵 Phase 1)

현재 `judge/matcher.rs`의 `from_model`이 `LongStart{ln}`에서 `LnKind`(LN/CN/HCN)를 폐기한다. 그 결과 `#LNMODE`로 파싱·구분된 CN(charge note)·HCN(hell charge note)이 **일반 LN과 동일하게 판정**된다(릴리스 미스가 LN처럼 구제됨). beatoraja는 CN 릴리스를 실제 시간 판정으로 보고(이른 릴리스 = BAD/POOR, 헤드 미스 시 헤드·페어 양쪽 +POOR), HCN은 홀딩 동안 연속 게이지 증감(`hcnmduration`)을 둔다. → 로드맵 **Phase 1: `LnKind` 판정 전파 + CN 종료 판정**, **Phase 7: HCN 연속 게이지**로 수정 예정. 차트 해시·기존 LN 판정 결과에는 영향 없음(파싱 단계 `LnKind` 구분은 이미 정확).

## 기각 (not-a-bug)
- `#SETRANDOM/#RONDAM` 수용(우리 확장, 무해)
- 데이터라인 감지 colon@byte5(현 구현 충분)
- mechanics.md §2 STOP 공식 본문이 `/192` 누락(코드는 정확)
- 미종결 LN(LN 채널 홀수 개) → dangling LongStart 잔존(허용)

## 설계 입장
**#RANDOM 계열 제어흐름은 beatoraja와 bug-for-bug 호환하지 않는다.** 차트 해시는 raw 바이트 기준이라 점수·IR·리플레이 키는 영향 없고, #RANDOM 분기는 본래 플레이 시 무작위다. 우리 구현은 #ELSE/#ELSEIF/중첩을 spec에 맞게 더 완전히 처리한다 — 이는 의도된 개선이다.
