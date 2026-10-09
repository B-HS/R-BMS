# B3. beatoraja 플레이 화면 전용 스킨 객체와 플레이 화면 수명주기

> 최종 갱신 2026-10-09 · 대응 단계: L1 조사(구현 전) · 기준 커밋 `9ce92bb` · 색인과 갱신 규칙은 [README.md](README.md)

## 0. 조사 범위와 표기

- 기준 소스: `/Users/hyunseokbyun/development/beatoraja` (origin `exch-bms2/beatoraja`, HEAD `8320241d`).
- 아래 경로는 모두 `src/bms/player/beatoraja/` 기준 상대 경로다. 예: `play/LaneRenderer.java:324` 는 `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/play/LaneRenderer.java` 324행.
- JSON 예시는 `skin/default/` (저장소 루트 기준) 의 `play7.json`, `play14.json`, `play_parts.lua`.
- 좌표계는 libGDX y-up 이다. `y` 는 사각형의 **아래변**, `y + h` 가 윗변이다. 판정선은 레인 사각형의 아래변이고 노트는 위에서 아래로 내려온다.
- 스킨 좌표는 `dw = dst.width / src.w`, `dh = dst.height / src.h` 로 스케일된다 (`skin/Skin.java:108-115`).
- "미확인" 은 소스에서 직접 확인하지 못한 항목이다.
- 배정 파일은 전부 끝까지 읽었다. 배정 밖이지만 질문에 답하려고 부분적으로 읽은 파일은 16절에 따로 적었다.

---

## 1. 플레이 스킨 객체 로딩 개요

`JsonPlaySkinObjectLoader.loadSkinObject` 는 destination 항목 하나마다 호출된다 (`skin/json/JSONSkinLoader.java:313-336`). 판정 순서는 다음과 같다.

| 순서 | 판정 | 근거 |
| --- | --- | --- |
| 1 | 공통 로더(`image`, `imageset`, `value`, `floatvalue`, `text`, `slider`, `graph`, `gaugegraph`, `judgegraph`, `bpmgraph`, `hiterrorvisualizer`, `timingvisualizer`, `timingdistributiongraph`, `gauge`). 여기서 객체가 나오면 즉시 반환 | `skin/json/JsonPlaySkinObjectLoader.java:31-34`, `skin/json/JsonSkinObjectLoader.java:39-559` |
| 2 | `note` (`dst.id == sk.note.id`) | `JsonPlaySkinObjectLoader.java:37-181` |
| 3 | `hiddenCover[]` | `:183-198` |
| 4 | `liftCover[]` | `:200-214` |
| 5 | `practice` | `:216-220` |
| 6 | `bga` | `:222-224` |
| 7 | `judge[]` | `:226-291` |
| 8 | `pmchara[]` | `:294-323` |

중요한 부수 효과:

- **플레이 전용 헤더 필드(`close`, `loadend`, `playstart`, `judgetimer`, `finishmargin`)는 note 객체가 destination 에 실제로 등장할 때에만 `PlaySkin` 에 설정된다** (`JsonPlaySkinObjectLoader.java:39-44`, TODO 주석 포함). note 를 배치하지 않은 스킨은 전부 0(판정 타이머만 기본 1)이 된다.
- `judgeregion` 은 destination 에 등장한 judge 객체의 `index + 1` 최댓값이다 (`:285-288`). `JudgeManager.init` 이 이 값으로 배열 크기를 정한다 (`play/JudgeManager.java:129-133`).
- destination 의 `dst` 가 비어 있어도(`{"id":"notes"}`, `{"id":2010}`) 객체는 추가되고, `offset`/`offsets` 만 `setOffsetID` 로 붙는다 (`skin/json/JSONSkinLoader.java:331-334`, `:470-475`).
- `setOffsetID` 는 이미 오프셋이 설정된 객체에는 다시 적용되지 않고, 1~199 범위의 id 만 받는다 (`skin/SkinObject.java:832-846`).
- 음수 정수 id 의 destination 은 `new SkinImage(-id)` (참조 이미지) 로 처리된다 (`JSONSkinLoader.java:315-322`).

---

## 2. PlaySkin 헤더 필드

| JSON 필드 | 기본값 | 의미와 사용처 |
| --- | --- | --- |
| `playstart` | 0 | STATE_READY 에서 STATE_PLAY 로 넘어가는 마진(ms). `TIMER_READY` 경과 > `playstart` 이면 전이 (`play/PlaySkin.java:16-18`, `play/BMSPlayer.java:606`) |
| `loadend` | 0 | 로드 최소 시간. `micronow > (loadstart + loadend) * 1000` 이어야 READY 로 전이 (`BMSPlayer.java:499`, `:549`) |
| `loadstart` | 0 | JSON 로더는 설정하지 않는다. LR2 로더만 설정 (`skin/lr2/LR2PlaySkinLoader.java:1009`). JSON/Lua 스킨에서는 항상 0 |
| `close` | 0 | STATE_FAILED 에서 화면을 떠나기까지의 마진(ms). `TIMER_FAILED` 경과 > `close` (`PlaySkin.java:29-32`, `BMSPlayer.java:711`) |
| `finishmargin` | 0 | STATE_FINISHED 에서 페이드아웃 시작까지의 마진(ms). `TIMER_MUSIC_END` 경과 > `finishmargin` 이면 `TIMER_FADEOUT` on (`PlaySkin.java:34-37`, `BMSPlayer.java:748-750`) |
| `judgetimer` | 1 | 레인별 봄(bomb) 타이머를 켜는 판정 상한. 0:PG, 1:GR, 2:GD, 3:BD. `judge <= judgetimer` 이면 켠다 (`PlaySkin.java:42-45`, `skin/json/JsonSkin.java:20`, `JudgeManager.java:673-675`) |
| `fadeout` (공통) | 0 | `TIMER_FADEOUT` 경과 > `fadeout` 이면 다음 화면 (`BMSPlayer.java:598`, `:751`) |
| `input` (공통) | 0 | `micronow > input * 1000` 이면 `TIMER_STARTINPUT` on (`BMSPlayer.java:470-472`) |
| `scene` (공통) | 0 | 플레이 화면에서는 쓰이지 않는다. decide/result 만 사용 (`decide/MusicDecide.java:47`, `result/MusicResult.java:250`) |
| `note.expansionrate` | `{100,100}` | PMS 리듬 확대율(%) w,h (`PlaySkin.java:47-50`, `JsonSkin.java:361`) |

기본 스킨 값: `playstart:1000, scene:3600000, input:500, close:1500, fadeout:1000` (`skin/default/play7.json:6-10`).
ModernChic 값: `loadend 3500, playstart 1000, scene 3600000, input 0, close 3000, fadeout 500` (`/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/require/header.lua:14-19`).

`PlaySkin` 의 그 밖의 상태: `line/time/bpm/stop` SkinImage 배열, `laneregion`, `lanegroupregion`, `judgeregion`, `practice`, `pomyu`, `laneCover`(LR2 전용) (`PlaySkin.java:20-56`).

플레이 스킨 전체 변환 오프셋: 플레이 계열 SkinType 에서는 `OFFSET_ALL`(10) 이 배치 전체 변환으로 적용된다. 평행이동 `(width * x / 100, height * y / 100)`, 배율 `((w + 100) / 100, (h + 100) / 100)` (`skin/Skin.java:382-388`, `:720-728`).

---

## 3. note 객체

### 3.1 JSON 필드와 기본값 (`skin/json/JsonSkin.java:339-367`)

| 필드 | 타입 | 기본값 | 설명 |
| --- | --- | --- | --- |
| `id` | string | - | destination 에서 참조하는 id |
| `note` | string[] | `[]` | 레인별 일반 노트 image id. **배열 길이가 레인 수** |
| `lnstart`, `lnend` | string[] | `[]` | LN 시작/끝 |
| `lnbody`, `lnactive` | string[] | `[]` | 구식 명명의 LN 본체 2종 |
| `lnbodyActive` | string[] | `[]` | 신식 명명. 있으면 매핑이 바뀐다 (3.2) |
| `hcnstart`, `hcnend` | string[] | `[]` | HCN 시작/끝 |
| `hcnbody`, `hcnactive`, `hcndamage`, `hcnreactive` | string[] | `[]` | 구식 명명의 HCN 본체 4종 |
| `hcnbodyActive`, `hcnbodyReactive`, `hcnbodyMiss` | string[] | `[]` | 신식 명명 |
| `mine` | string[] | `[]` | 지뢰 |
| `hidden`, `processed` | string[] | `[]` | **JSON 로더가 읽지 않는다.** 항상 내장 플레이스홀더 사용 (3.2) |
| `dst` | Animation[] | `[]` | 레인별 사각형 `{x,y,w,h}` |
| `dst2` | int | `Integer.MIN_VALUE` | PMS 놓침 POOR 노트가 떨어지는 하한 y. 가변 길이 용도가 아니다 (3.3.9) |
| `expansionrate` | int[2] | `{100,100}` | 4분 박자 확대율 |
| `size` | float[] | `[]` | 레인별 노트 높이(px, 스킨 좌표) |
| `group` | Destination[] | `[]` | 플레이어(레인 그룹)별 마디선 |
| `bpm`, `stop`, `time` | Destination[] | `[]` | BPM 변화선, 정지선, 시간선 |

### 3.2 로더 변환 (`JsonPlaySkinObjectLoader.java:46-180`)

1. `notes = getNoteTexture(note)`: image 목록에서 같은 id 를 찾아 `SkinSourceImage(regions, timer, cycle)` 를 만든다. id 가 없으면 그 칸은 `null` (`skin/json/JsonSkinObjectLoader.java:584-599`).
2. LN 이미지 10종 인덱스 매핑 (`:47-69`):

| 인덱스 | `lnbodyActive`/`hcnbodyActive` 없음(구식) | 있음(신식) | 그릴 때의 의미 |
| --- | --- | --- | --- |
| 0 | `lnend` | `lnend` | CN 끝 |
| 1 | `lnstart` | `lnstart` | LN/CN 시작 |
| 2 | `lnbody` | `lnbodyActive` | 본체, **누르는 중**(processing) |
| 3 | `lnactive` | `lnbody` | 본체, 누르지 않는 중 |
| 4 | `hcnend` | `hcnend` | HCN 끝 |
| 5 | `hcnstart` | `hcnstart` | HCN 시작 |
| 6 | `hcnbody` | `hcnbodyActive` | HCN 본체, 누르는 중 |
| 7 | `hcnactive` | `hcnbody` | HCN 본체, 기본(아직 처리 전) |
| 8 | `hcndamage` | `hcnbodyReactive` | HCN 통과 중이고 다시 눌러 게이지 증가 중 |
| 9 | `hcnreactive` | `hcnbodyMiss` | HCN 통과 중이고 떼어서 게이지 감소 중 |

   구식 명명은 이름과 의미가 어긋난다. 구식 `lnbody` 는 "누르는 중", 구식 `lnactive` 는 "누르지 않는 중" 이다. 구식 `hcndamage` 는 "재입력(증가)", 구식 `hcnreactive` 는 "미스(감소)" 다. 근거는 그리기 코드의 인덱스 선택 (`play/LaneRenderer.java:707-727`).
3. 전치: `lnss[lane][type]`, 크기는 `lnend` 길이 x 10 (`:70-75`). **10개 배열 모두 `lnend` 길이 이상이어야 한다.** 짧으면 `ArrayIndexOutOfBounds` 다. `lnend` 와 `mine` 은 `note` 길이 이상이어야 한다 (`play/SkinNote.java:26-30`).
4. `mines = getNoteTexture(mine)` (`:77`).
5. 레인 사각형 (`:79-93`): `dx = dstr.width / sk.w`, `dy = dstr.height / sk.h`. `region[i] = (dst.x*dx, dst.y*dy, dst.w*dx, dst.h*dy)`. `dst` 는 `note` 길이 이상이어야 한다 (`SkinNote.java:35-41`).
6. 노트 높이 `scale[i]`: `size[i]` 가 있으면 `size[i] * dy`, 없으면 **노트 원본 이미지 첫 프레임의 높이 * dy** (`:88-92`).
7. `dstnote2[i]`: 기본 `Integer.MIN_VALUE`. `dst2` 가 있으면 전 레인에 `round(dst2 * dy)` (`:81-82`, `:172-174`).
8. `group[i]`: `gregion[i] = group[i].dst[0]` 스케일 값. 같은 id 의 image 로 `SkinImage` 를 만들고 `setDestination` 으로 dst 전체(애니메이션, timer, op, draw, offset)를 붙인다 → `skin.setLine` (`:94-113`).
9. `bpm`, `stop`, `time`: 길이는 `gregion.length` 로 고정, 채우는 것은 `min(gregion.length, 배열 길이)` 까지 → `setBPMLine/StopLine/TimeLine` (`:115-170`).
10. `new SkinNote(notes, lnss, mines)` → `setLaneRegion`, `skin.setLaneRegion(region)`, `skin.setLaneGroupRegion(gregion)`, `skin.setNoteExpansionRate` (`:175-180`).

누락 이미지 플레이스홀더 (`play/SkinNote.java:112-164`): 32x8 단색 텍스처.

| 대상 | 색 | 형태 |
| --- | --- | --- |
| note | WHITE | 채움 |
| longnote[0..9] | YELLOW | 채움 |
| mine | RED | 채움 |
| hidden | ORANGE | 2px 테두리 |
| processed | CYAN | 2px 테두리 |

`hidden`/`processed` 는 JSON 로더가 3인자 생성자를 쓰므로 항상 위 플레이스홀더다 (`JsonPlaySkinObjectLoader.java:175`, `SkinNote.java:104-106`).

노트 이미지 애니메이션: `cycle == 0` 이면 0번 프레임. 아니면 timer 가 off 일 때 0, on 이면 `(time * length / cycle) % length`. `time` 은 스킨 시간(ms)에서 timer 값을 뺀 값이다 (`skin/SkinSourceImage.java` `getImageIndex`). 매 프레임 `SkinLane.prepare` 에서 note/LN 10종/mine/hidden/processed 의 현재 프레임을 고른다 (`SkinNote.java:166-176`).

`SkinNote` 자체 destination 은 생성자에서 time 0, 사각형 0 으로 하나 넣는다 (`SkinNote.java:32`). destination 의 `offset`/`offsets` 는 `drawLane` 에 전달되어 **모든 노트의 x, y, w, h 에 더해진다** (`SkinNote.java:60-62`, `LaneRenderer.java:288-298`, `:521-524`).

### 3.3 LaneRenderer.drawLane 알고리즘 (`play/LaneRenderer.java:288-683`)

#### 3.3.1 시간과 속도

```
offsetX/Y/W/H = SkinNote 에 붙은 SkinOffset 들의 합                         (:289-298)
time = (TIMER_PLAY on ? time - timer(TIMER_PLAY)
        : (timer 141 on ? time - timer(141) : 0)) + config.getJudgetiming()  (:300-301)
STATE_PRACTICE 이면 time = practice.starttime, pos = 0                        (:302-305)
microtime = time * 1000
hispeed = (PRACTICE 아님) ? playconfig.hispeed : 1.0                          (:309)
nbpm, nscroll = microtime 이하인 마지막 타임라인의 BPM, SCROLL (pos-5 부터 탐색) (:311-318)
enableConstant = playconfig.enableConstant && PRACTICE 아님                   (:319)
speed = enableConstant ? 1.0 : getCurrentSpeed(microtime)                    (:320)
region = nscroll > 0 ? (240000 / nbpm / hispeed / speed) / nscroll : 0       (:321)
hu = lanes[0].region.y + lanes[0].region.height                              (:324)
hl = lift 사용 ? lanes[0].region.y + lanes[0].region.height * lift
               : lanes[0].region.y                                           (:325)
rxhs = (hu - hl) * hispeed * speed                                           (:326)
lanecover = lanecover 사용 ? playconfig.lanecover : 0                         (:329)
currentduration = round(region * (1 - lanecover))                            (:330)
```

- `rxhs` 는 "1마디(section 1.0)당 픽셀 수" 다. hispeed 1.0 에서 1마디가 판정선부터 레인 상단까지의 높이와 같다.
- `hu`, `hl` 은 **0번 레인만** 기준으로 한다 (TODO 주석 `:323`). 레인마다 y/h 가 달라도 반영되지 않는다.
- `getCurrentSpeed`: `#SPEED` 오브젝트가 있는 타임라인 사이를 시간으로 선형 보간 (`:259-286`).
- 표시 대상 타임라인은 `init` 에서 추린다. BPM/STOP/SCROLL/SPEED 변화, 마디선, 노트 또는 히든 노트가 있는 타임라인만 남긴다 (`:106-123`).
- 하이스피드 고정: `resetHispeed(bpm)` = `(2400 / (bpm / 100) / duration) * (1 - lanecover)` (`:206-210`). `basebpm` 은 fixhispeed 설정에 따라 시작/최소/최대/메인 BPM (`:145-152`). 메인 BPM 은 총 노트 수가 가장 많은 BPM (`:130-144`).

#### 3.3.2 오프셋 갱신 (노트를 그릴 때마다 전역 오프셋에 기록)

| 오프셋 | 값 | 근거 |
| --- | --- | --- |
| `OFFSET_LIFT`(3).y | `hl - lanes[0].region.y` (리프트 높이 px, 꺼져 있으면 0) | `:332` |
| `OFFSET_LANECOVER`(4).y | `(hl - hu) * lanecover` (음수, 아래로 이동) | `:333` |
| `OFFSET_HIDDEN_COVER`(5) | hidden 사용: `a = 0`, `y = hidden * laneRegion[0].height` (lift 사용 시 `* (1 - lift)`). 미사용: `a = -255` | `:335-346` |

이 값은 **SkinNote 가 그려지는 시점에만** 갱신된다. note 객체보다 먼저 그려지는 객체는 직전 프레임 값을 쓴다. note 객체가 없는 스킨에서는 갱신되지 않는다.

#### 3.3.3 y 누적 규칙 (마디선 패스와 노트 패스 공통, `:390-400`, `:502-514`)

`y = hl` 에서 시작해 `i = pos` 부터 `y <= hu` 인 동안 타임라인을 순회한다. 타임라인 `tl` 이 미래(`tl.microtime >= microtime`)일 때만 y 를 올린다.

```
i > 0, prev = timelines[i-1]:
  prev.microtime + prev.microstop > microtime 이면
      y += (tl.section - prev.section) * prev.scroll * rxhs
  아니면
      y += (tl.section - prev.section) * prev.scroll
           * (tl.microtime - microtime) / (tl.microtime - prev.microtime - prev.microstop) * rxhs
i == 0:
  y += tl.section * (tl.microtime - microtime) / tl.microtime * rxhs
```

- 과거 타임라인은 y 를 올리지 않으므로 `y == hl` (판정선) 에 머문다.
- `y > hu` 가 되면 순회가 끝난다. 레인 상단을 넘은 타임라인은 그리지 않는다. 레인 커버는 순회에 영향을 주지 않고 덮는 그림일 뿐이다.
- `pos` 는 "더 그릴 것이 없는 과거 타임라인" 을 건너뛰는 캐시다. LN 의 끝이 아직 미래이거나, `showpastnote` 이고 미판정 일반 노트가 있으면 전진하지 않는다 (`:448-461`).

#### 3.3.4 마디선, BPM선, 정지선, 시간선 (`:354-462`)

미래 타임라인마다 다음을 그린다. 모두 `line.draw(sprite, time, main, 0, (int)(y - hl))` 로, 선 SkinImage 의 dst 에 y 오프셋 `(int)(y - hl)` 을 더한다.

| 선 | 조건 | 부가 텍스트(시스템 폰트 18px) | 근거 |
| --- | --- | --- | --- |
| time | PRACTICE 상태이고 직전 타임라인보다 초 단위가 커졌을 때 | `"%2d:%02d.%1d"`, 그룹 x+4, y+20, 색 `40c0c0` | `:401-412` |
| bpm | (`config.isBpmguide()` 또는 PRACTICE) 이고 `tl.BPM != nbpm` | `"BPM" + (int)bpm`, 그룹 중앙, 색 `00c000` | `:414-427` |
| stop | (같은 조건) 이고 `tl.stop > 0` | `"STOP " + (int)stop + "ms"`, 색 `c0c000` | `:428-438` |
| group(마디선) | `tl.getSectionLine()` | 없음 | `:441-446` |

- 선 SkinImage 에 넘기는 `time` 은 **차트 시간(ms, judgetiming 포함)** 이다. 스킨 시간이 아니다. 선 dst 에 timer/애니메이션을 걸면 차트 시간 기준으로 움직인다.
- 선의 최종 y 는 `dst.y + (오프셋 3 이 붙어 있으면 lift) + (y - hl)` 이다. 기본 스킨과 ModernChic 모두 선 dst 의 y 를 레인 아래변(판정선)에 두고 `offset: 3` 을 붙인다 (`skin/default/play7.json:303-326`, `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/notes.lua` `group/time/bpm/stop`).
- `group` 은 플레이어(레인 그룹) 수만큼 정의한다. 14키 기본 스킨은 2개 (`skin/default/play14.json:337-368`).

#### 3.3.5 일반 노트 (`:464-550`)

노트 패스 전에 `setColor(WHITE)`, `setBlend(0)`, `setType(TYPE_NORMAL)` 로 초기화한다. 스킨 destination 의 색/알파/블렌드는 노트에 적용되지 않는다.

```
dstx = lane.region.x + offsetX
dsty = y + offsetY
dstw = lane.region.width + offsetW
dsth = scale + offsetH
(확대율이 100 이 아니면 3.3.8 적용)
```

- 일반 모드(`dstnote2 == MIN_VALUE`): `tl.microtime >= microtime` 이거나 (`config.isShowpastnote()` 이고 `note.state == 0`) 이면 그린다 (`:546-550`). **일찍 쳐서 이미 판정된 노트도 판정선에 닿을 때까지 계속 보인다.**
- PMS 모드(`dstnote2 != MIN_VALUE`): `tl.microtime >= microtime` 이고 (`state == 0` 또는 `state >= 4`) 일 때만 (`:540-545`).
- 이미지: `config.isMarkprocessednote()` 이고 `state != 0` 이면 `processedImage`, 아니면 `noteImage`.
- 노트 사각형의 아래변이 타이밍 위치다. 높이는 `scale` 로 고정이고 레인 사각형의 높이와 무관하다.
- 그리는 순서: 타임라인 시간순, 같은 타임라인 안에서는 레인 인덱스순. 나중 노트가 위에 그려진다.

#### 3.3.6 롱노트 (`:551-584`, `:701-728`)

LN 시작 노트(`!ln.isEnd()`)이고 끝의 시간이 `microtime` 이상일 때만 처리한다.

```
dy = 시작 타임라인에서 끝 타임라인까지, 3.3.3 과 같은 식으로 미래 구간만 누적   (:560-576)
dy > 0 이면
  dscale = dsth > scale ? (dsth - scale) / 2 : 0
  height = (dsty < lane.region.y - dscale) ? dsty - (lane.region.y - dscale) : dy
  drawLongNote(longImage, x = dstx, y = dsty + dy, width = dstw, height, scale = dsth)
```

`drawLongNote` 의 `y` 는 **끝 노트의 아래변** 이다.

| 종류 | 판별 | 그리는 순서와 사각형 |
| --- | --- | --- |
| HCN | `(model.lntype == HELLCHARGENOTE 이고 ln.type == UNDEFINED)` 또는 `ln.type == HELLCHARGENOTE` | 본체 `(x, y - height + scale, width, height - scale)`, 끝 `longImage[4]` `(x, y, width, scale)`, 시작 `longImage[5]` `(x, y - height, width, scale)` |
| CN | lntype CHARGENOTE 또는 type CHARGENOTE | 본체 `[processing ? 2 : 3]`, 끝 `[0]`, 시작 `[1]` (사각형은 HCN 과 같음) |
| LN | lntype LONGNOTE 또는 type LONGNOTE | 본체 `[processing ? 2 : 3]`, 시작 `[1]` 만. **끝 이미지를 그리지 않는다** |

- HCN 본체 인덱스: `processing == ln.getPair()` 이면 6, 아니면 (`passing == ln` 이고 `ln.state != 0`) 일 때 `inclease ? 8 : 9`, 그 외 7 (`:707-711`).
- `processing` 은 "현재 누르고 있는 LN 의 끝 노트" 다 (`JudgeManager.getProcessingLongNote`, `play/JudgeManager.java:727-729`).
- 본체는 시작 노트의 윗변(`start_y + scale`)에서 끝 노트의 아래변까지다. 그 뒤에 끝, 시작 순으로 덮어 그린다.
- 잘림(클립)은 없다. 시작이 과거인 LN 은 시작 타임라인의 y 가 `hl` 에 머무르므로 **판정선에서 잘린 것처럼** 그려진다. 끝이 레인 상단을 넘으면 그대로 위로 삐져나오며, 스킨이 프레임/커버 이미지로 가린다.
- `height - scale < 0` (아주 짧은 LN) 이면 음수 높이로 그대로 그린다.
- `dsty < lane.region.y - dscale` 분기는 오프셋 y 가 음수이거나 확대로 아래로 내려간 경우에만 참이 되고, 이때 `height` 가 음수가 된다. 원본 코드 그대로의 동작이며 의도는 미확인이다 (`:578-581`).

#### 3.3.7 지뢰와 히든 노트

- 지뢰: `tl.microtime >= microtime` 일 때 `mineImage` 를 일반 노트와 같은 사각형에 그린다 (`:585-590`).
- 히든 노트: `config.isShowhiddennote()` 이고 미래 타임라인이면 `hiddenImage` 를 `(lane.region.x, y, lane.region.width, scale)` 에 그린다. **오프셋과 확대율을 적용하지 않는다** (`:592-598`).

#### 3.3.8 4분 박자 확대 (expansionrate, `:525-537`)

`rate[0] != 100` 또는 `rate[1] != 100` 일 때만 동작한다. `t = now - main.getNowQuarterNoteTime()` (ms).

- `t < 9`: `dstw *= 1 + (rate[0]/100 - 1) * t / 9`, `dsth` 도 같은 식.
- `9 <= t <= 159`: `dstw *= 1 + (rate[0]/100 - 1) * (150 - (t - 9)) / 150`.
- 확대 후 `dstx -= (dstw - lane.region.width) / 2`, `dsty -= (dsth - scale) / 2` 로 중심을 맞춘다.
- 4분 박자 시각은 `RhythmTimerProcessor` 가 계산한다 (`play/RhythmTimerProcessor.java:30-52`, `:72-79`). 확대율이 100 이면 계산 자체를 건너뛴다 (`play/BMSPlayer.java:430-431`).

#### 3.3.9 PMS 놓침 POOR 낙하 (`dst2`, `:604-682`)

`lanes[0].dstnote2 != MIN_VALUE` 일 때만 동작한다. 놓친 일반 노트(`state == 0` 또는 `state >= 4`, 시간이 지난 것)를 느린 BAD 판정 폭이 지난 시점부터 노스피(hispeed 무시) 속도 `(hu - hl) * BPM / 240` px/s 로 판정선 아래로 떨어뜨린다. 하한은 `orgy2 = clamp(dstnote2, -lane.height, hl)` 이다. ModernChic 은 `dst2` 를 쓰지 않는다 (ModernChic Play 폴더 grep 결과 없음).

#### 3.3.10 CONSTANT (`:356-388`, `:472-500`)

`targetTime = microtime + duration * 1000`, `alphaLimit = constantFadeinTime * 1000`, `diff = tl.microtime - targetTime`.

| `alphaLimit` | `tl.microtime >= targetTime` | 그 외 |
| --- | --- | --- |
| `>= 0` | `diff < alphaLimit` 이면 알파 `(alphaLimit - diff) / alphaLimit`, 아니면 건너뜀 | 흰색(불투명) |
| `< 0` | 건너뜀 | `diff > alphaLimit` 이면 알파 `1 - (alphaLimit - diff) / alphaLimit`, 아니면 흰색 |

건너뛸 때는 y 누적도 하지 않는다.

#### 3.3.11 판정 영역 오버레이 (`:738-781`)

`config.isShowjudgearea()` 일 때만. 레인마다 첫 미래 타임라인의 px/us 비율로 판정선 위(early 쪽)에 PG `0000ff20`, GR `00ff0020`, GD `ffff0020`, BD `ff800020`, MS `ff000020` 띠를 1x1 흰 텍스처로 그린다. 스크래치 레인은 `JudgeManager.isScratch(lane)` 로 판별해 SCRATCH 판정 폭을 쓴다.

### 3.4 note 관련 엣지 케이스 요약

- `group` 의 image id 를 찾지 못하면 `lines[i]` 가 null 이고, 마디선을 그릴 때 NullPointerException 이 난다 (`JsonPlaySkinObjectLoader.java:101-110`, `LaneRenderer.java:443-445`).
- `bpm`/`stop`/`time` 이 `group` 보다 짧으면 남은 칸이 null 이라 해당 선을 그릴 때 NullPointerException 이 난다 (`:116-117`).
- `bpm`/`stop`/`time` 을 아예 정의하지 않으면 빈 배열이라 안전하다 (`PlaySkin.java:20-23`).
- 레인 수는 `note` 배열 길이다. 차트 레인 수보다 작으면 `tl.getNote(lane)` 을 그 범위만 본다 (`LaneRenderer.java:516`).

---

## 4. 레인 커버, 리프트, 히든

### 4.1 hiddenCover / liftCover 객체 (`play/SkinHidden.java`, `JsonSkin.java:380-408`)

| 필드 | hiddenCover 기본 | liftCover 기본 | 설명 |
| --- | --- | --- | --- |
| `id`, `src`, `x`, `y`, `w`, `h`, `divx`, `divy`, `timer`, `cycle` | divx/divy 1 | 같음 | 자체 소스 사각형과 프레임 애니메이션 |
| `disapearLine` | -1 | -1 | 이 y 아래는 잘라낸다. 음수면 자르지 않음. `* skin.getScaleY()` 로 스케일 |
| `isDisapearLineLinkLift` | true | false | 소실선을 리프트만큼 올릴지 |

로더가 자동으로 붙이는 오프셋 (`JsonPlaySkinObjectLoader.java:190-194`, `:207-210`):

- hiddenCover: destination 의 `offsets` + `OFFSET_LIFT`(3) + `OFFSET_HIDDEN_COVER`(5)
- liftCover: destination 의 `offsets` + `OFFSET_LIFT`(3)

그리기 (`SkinHidden.java:55-91`):

1. `disapearLineAddedLift = disapearLine`. 연동이 켜져 있고 `disapearLine >= 0` 이면 `disapearLine + OFFSET_LIFT.y` (리프트 값이 바뀔 때만 다시 계산).
2. `disapearLine < 0` 이거나 `region.y + region.height > disapearLineAddedLift` 일 때만 그린다.
3. `region.y < disapearLineAddedLift` 이면 scissor 클립 `(region.x, disapearLineAddedLift, region.width, region.y + region.height - disapearLineAddedLift)` 안에서 전체 이미지를 그린다. 즉 소실선 아래 부분만 잘린다 (`:77-85`, 클립 구현 `skin/Skin.java:567-580`).
4. 프레임 인덱스는 `cycle == 0` 이면 0, 아니면 `(time * length / cycle) % length` (`:106-121`).
5. 텍스처가 없으면 hiddenCover 는 `null` 을 반환해 객체가 생기지 않는다. liftCover 는 반환하지 않고 다음 판정으로 넘어간다 (`JsonPlaySkinObjectLoader.java:186-196`, `:203-213`).

HIDDEN 오프셋의 `a = -255` 는 색 알파에 더해져 0 으로 클램프되고, 알파 0 인 객체는 그리지 않는다 (`skin/SkinObject.java:513-519`, `:624-626`).

기본 스킨 예: hiddenCover `disapearLine:140, isDisapearLineLinkLift:true`, dst `y:-440, h:580` (윗변 = 140 = 판정선) (`skin/default/play7.json:256-258`, `:523-525`).
ModernChic 예: hiddenCover 는 1x1 검은 픽셀 소스에 `disapearLine = 227`, dst `y = -626, w = 513, h = 853` (윗변 227). liftCover 는 `src = 18`, `disapearLine = 227`, 같은 dst (`/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/cover.lua:10-18`).

### 4.2 레인 커버(SUDDEN+) 자체

레인 커버는 전용 객체가 아니다. 두 가지 방식으로 만든다.

- slider `type: 4` (또는 5), `angle: 2`(아래), `range` = 레인 길이. 값은 `lanecover * (lift 사용 시 1 - lift)`, 커버가 꺼져 있으면 0 (`skin/property/FloatPropertyFactory.java:139-152`, `:233-234`). 기본 스킨 `id 1060` 은 dst `y:720` (화면 위 바깥)에서 내려온다 (`skin/default/play7.json:254`, `:526-528`).
- 일반 이미지에 `offset: 4` (`OFFSET_LANECOVER`) 를 붙인다. 기본 스킨은 커버에 따라 움직이는 숫자 표시에 사용한다 (`play7.json:712-717`).

### 4.3 레인 커버 값 표시용 id

| 종류 | id | 값 | 근거 |
| --- | --- | --- | --- |
| number | 14 | `lanecover * 1000` | `skin/property/IntegerPropertyFactory.java:68-73`, `skin/SkinProperty.java:280` |
| number | 314 | `lift * 1000` | `IntegerPropertyFactory.java:74-79` |
| number | 315 | `hidden * 1000` | `:80-85` |
| number | 316 | `(1 - lift) * lanecover * 1000` | `:86-92` |
| number | 312 | `currentduration` (3.3.1) | `:291-303` |
| number | 313 | `currentduration * 3 / 5` (녹색 숫자) | `:304-316` |
| number | 1312-1327 | `round(240000 / bpm / hispeed * (커버 on 이면 1 - lanecover) * (green 이면 0.6))`. `(id - 1312) / 4` 가 0:현재 BPM, 1:메인, 2:최소, 3:최대. `% 4 < 2` 가 커버 반영, 홀수가 green | `:541-560`, `SkinProperty.java:263-278` |
| number | 160 | 현재 BPM | `IntegerPropertyFactory.java:204` |
| option | 270 | START 또는 SELECT 를 누르는 중 | `skin/property/BooleanPropertyFactory.java:565-566` |
| option | 271 / 272 / 273 | 레인 커버 / 리프트 / 히든 사용 | `:567-572` |
| option | 400 | CONSTANT | `SkinProperty.java` `OPTION_CONSTANT` |
| slider(float) | 4, 5 | 4.2 참고 | `FloatPropertyFactory.java:233-234` |

### 4.4 레인 커버와 하이스피드 입력 (`play/ControlInputProcessor.java`)

| 입력 | 동작 | 근거 |
| --- | --- | --- |
| 위/아래 방향키 | 커버 값 -0.01 / +0.01 (누를 때 1회) | `:136-149` |
| 마우스 휠 | `-scroll * 0.005` | `:151-154` |
| START + 건반 | 건반 배열 `keybinds` 의 -1/+1 에 따라 하이스피드 감소/증가 | `:63-96` |
| START + 스크래치(±2) | 커버 값 변경. 아날로그는 `틱 * marginLow`, 디지털은 50ms 마다 `marginLow`, `switchDuration` 초과 후 `marginHigh` | `:246-269` |
| START 더블 탭(500ms 이내) | 레인 커버 on/off | `:162-171` |
| SELECT + 건반 / 스크래치 | duration ±1 | `:97-119`, `:271-292` |
| START + SELECT 짧게 | 리프트/히든 조정 대상 전환 | `:186-193` |
| START + SELECT 길게(`exitPressDuration` 초과), 또는 노트 종료 후 START/SELECT | `stopPlay()` | `:195-203` |
| ESC | `stopPlay()` | `:205-207` |
| 숫자 1/2/3/4 (오토/리플레이) | 재생 속도 25/50/200/300, 그 외 100 | `:209-221` |

커버 값의 대상 (`setCoverValue`, `:231-244`): 레인 커버가 켜져 있거나 리프트와 히든이 모두 꺼져 있으면 레인 커버, 리프트가 켜져 있고 (히든이 꺼져 있거나 `isChangeLift`) 이면 리프트(`- value`), 그 외는 히든(`- value`). `hispeedAutoAdjust` 이면 현재 BPM 으로 `resetHispeed`.

하이스피드 증감 폭: fixhispeed 사용 시 `basehispeed * hispeedmargin`, 미사용 시 `hispeedmargin`. 결과가 0 초과 20 미만일 때만 적용 (`LaneRenderer.java:237-247`). `NO_SPEED` 제약이 있으면 hispeed 1.0, 커버/리프트/히든 0, 조작 불가 (`LaneRenderer.java:95-103`, `BMSPlayer.java:419-424`).

`keybinds` 배열 (키 인덱스별, -1:감속, 1:가속, 2:스크래치 위, -2:스크래치 아래):

| 모드 | 배열 |
| --- | --- |
| 5K, 10K | `-1,1,-1,1,-1,2,-2` 를 2회 |
| POPN 5K/9K | `-1,1,-1,1,-1,1,-1,2,-2` |
| 7K, 14K | `-1,1,-1,1,-1,1,-1,2,-2` 를 2회 |
| 24K, 24K DOUBLE | 26개 패턴을 2회 (`:67-71`) |

---

## 5. gauge 객체

### 5.1 JSON 필드 (`JsonSkin.java:369-378`)

| 필드 | 기본값 | 설명 |
| --- | --- | --- |
| `id` | - | |
| `nodes` | (필수) | image id 배열. 길이 4 / 8 / 12 / 36 만 유효 |
| `parts` | 50 | 게이지 알갱이 수 |
| `type` | 0 | 애니메이션 종류. 0:RANDOM, 1:INCLEASE, 2:DECLEASE, 3:FLICKERING |
| `range` | 3 | 애니메이션 범위(알갱이 수) |
| `cycle` | 33 | 애니메이션 간격(ms) |
| `starttime` | 0 | 리절트 화면에서 0 에서 차오르기 시작하는 시각(ms) |
| `endtime` | 500 | 차오르기가 끝나는 시각(ms) |

### 5.2 nodes 매핑 (`skin/json/JsonSkinObjectLoader.java:509-556`)

내부 이미지는 36칸이다. `칸 = 게이지 종류 인덱스 * 6 + 상태`.

게이지 종류 인덱스 (`play/SkinGauge.java:182`, `play/GrooveGauge.java:20-28`): `exgauge = (type >= CLASS(6) ? type - 3 : type) * 6`.

| 게이지 type | 이름 | 종류 인덱스 |
| --- | --- | --- |
| 0 | ASSISTEASY | 0 |
| 1 | EASY | 1 |
| 2 | NORMAL | 2 |
| 3 | HARD | 3 |
| 4 | EXHARD | 4 |
| 5 | HAZARD | 5 |
| 6 | CLASS | 3 (HARD 와 공유) |
| 7 | EXCLASS | 4 |
| 8 | EXHARDCLASS | 5 |

상태(0~5):

| 상태 | 의미 |
| --- | --- |
| 0 | 켜짐, 보더 이상 구간 |
| 1 | 켜짐, 보더 미만 구간 |
| 2 | 꺼짐, 보더 이상 구간 |
| 3 | 꺼짐, 보더 미만 구간 |
| 4 | 선단(현재 값의 마지막 알갱이), 보더 이상 구간 |
| 5 | 선단, 보더 미만 구간 |

nodes 길이별 규칙:

| 길이 | nodes[i] 가 채우는 칸 |
| --- | --- |
| 4 | `[0]` → 모든 종류의 상태 0, 4. `[1]` → 상태 1, 5. `[2]` → 상태 2. `[3]` → 상태 3 |
| 8 | `[0..3]` → NORMAL, HARD(종류 2, 3)의 상태 (0,4), (1,5), 2, 3. `[4..7]` → 나머지 종류(0, 1, 4, 5)의 같은 상태 |
| 12 | `[0..3]` → 종류 2, 3 의 상태 0, 1, 2, 3. `[4..7]` → 종류 0, 1, 4, 5 의 상태 0, 1, 2, 3. `[8], [9]` → 종류 2, 3 의 상태 4, 5. `[10], [11]` → 종류 0, 1, 4, 5 의 상태 4, 5 |
| 36 | 그대로 `[종류 * 6 + 상태]` |
| 그 외 | 매핑 없음. `pgaugetex[i]` 에 순서대로 넣고 나머지는 null 이라 전치 단계(`:544-549`)에서 예외가 난다 |

`SkinGauge(gaugetex, timer 0, cycle 0, parts, type, range, cycle)` 로 만든다 (`:551`). 이미지 세트의 timer/cycle 이 0 이므로 node 이미지의 `divx/divy` 프레임 애니메이션은 동작하지 않는다.

기본 스킨은 8개 (`skin/default/play7.json:328-331`), ModernChic 은 36개와 `parts = 50` (옵션에 따라 100) 이다 (`/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/gauge.lua:89-108`).

### 5.3 prepare (`play/SkinGauge.java:100-178`)

- 게이지 원본: 플레이는 `player.getGauge()`, 리절트는 `resource.getGrooveGauge()`. 없으면 그리지 않는다.
- 애니메이션 값 `animation`:
  - RANDOM: `atime < time` 이면 `(int)(random * (range + 1))`, `atime = time + cycle`.
  - INCLEASE: `(animation + range) % (range + 1)`.
  - DECLEASE: `(animation + 1) % (range + 1)`.
  - FLICKERING: `animation = time % cycle` (매 프레임).
- 모드가 바뀐 경우(`originalMode != model.mode`, 7to9 나 BATTLE 등) 1회에 한해 `parts` 를 조정한다. 각 게이지 종류에 대해 `i = parts..max` 중 `border % (max / i) == 0` 인 첫 i 를 찾고 그 최댓값을 `parts` 로 한다 (`:139-154`).
- `value = gauge.getValue()`, `type = gauge.getType()`, `max`, `border` 는 해당 종류의 속성.
- 리절트: 최종 게이지 값으로 바꾸고, `time < starttime` 이면 `min`, `starttime <= time < endtime` 이면 `min(value, max(max * (time - starttime) / (endtime - starttime), min))`.

### 5.4 draw (`:180-217`)

```
notes = value > 0 ? max(1, (int)(value * parts / max)) : 0
색 = dst 색, 블렌드 = dst 블렌드, 타입 NORMAL
알갱이 i = 1..parts:
  사각형 = (region.x + region.width * (i - 1) / parts, region.y, region.width / parts, region.height)
  nodeBorder = i * max / parts
  below = nodeBorder < border ? 1 : 0
  RANDOM/INCLEASE/DECLEASE:
     칸 = exgauge + (notes == i ? 4 : (notes - animation > i ? 0 : 2)) + below
  FLICKERING:
     칸 = exgauge + (notes >= i ? 0 : 2) + below
     i == notes 이면 그 위에 칸 exgauge + 4 + below 를
       알파 = 원래 알파 * (animation < cycle/2 ? animation / (cycle/2 - 1)
                                               : ((cycle - 1) - animation) / (cycle/2 - 1)) 로 덧그림
```

- RANDOM 등에서는 선단 바로 아래 `animation` 개의 알갱이가 꺼진 그림으로 바뀌어 깜빡이는 효과가 난다.
- 가로 방향 전용이다. 세로 게이지는 지원하지 않는다.
- 경계 표시(보더)는 별도 그림이 없고 상태 +1 여부로만 구분된다.

게이지 관련 타이머: `TIMER_GAUGE_MAX_1P`(44) 는 매 프레임 `gauge.isMax()` 로 전환 (`play/BMSPlayer.java:636`). `TIMER_GAUGE_INCLEASE_1P/2P`(42, 43), `TIMER_GAUGE_MAX_2P`(45) 는 소스 어디에서도 설정하지 않는다 (저장소 전체 grep 결과 정의만 존재).

---

## 6. judge 객체

### 6.1 JSON 필드 (`JsonSkin.java:429-435`)

| 필드 | 기본값 | 설명 |
| --- | --- | --- |
| `id` | - | |
| `index` | 0 | 판정 영역(플레이어) 번호. 0:1P, 1:2P, 2:3P |
| `images` | `[]` | Destination 배열. 순서 PG, GR, GD, BD, PR, MS, (7번째는 MAX 상태 PG) |
| `numbers` | `[]` | Destination 배열. `images` 와 같은 길이 이상이어야 한다 (`JsonPlaySkinObjectLoader.java:245` 가 `numbers[i]` 를 무조건 참조) |
| `shift` | false | 콤보 숫자 폭의 절반만큼 판정 이미지를 왼쪽으로 민다 |

### 6.2 로더 (`JsonPlaySkinObjectLoader.java:226-291`)

- `images[i]`: 같은 id 의 `image` 로 `SkinImage(regions, timer, cycle)` 를 만들고 Destination 전체를 붙인다 (timer, loop, offset, op, draw 포함).
- `numbers[i]`: 같은 id 의 `value` 로 `SkinNumber` 를 만든다.
  - `d = (이미지 수 % 10 == 0) ? 10 : 11`. 프레임 수는 `divx * divy / d`.
  - 생성 인자: `digit = value.digit`, `zeropadding = (d > 10 ? 2 : 0)`, `space = value.space`, `ref = value.ref`, **`align = 2`(가운데)**.
  - `setRelative(true)`: 커스텀 오프셋의 x, y 를 적용하지 않고 w, h 만 적용 (`skin/SkinObject.java:404-413`).
  - `value.offset` 이 있으면 자릿수별 오프셋.
  - **dst 의 모든 x 를 `x -= w * digit / 2` 로 바꾼 뒤** Destination 을 붙인다 (`:273-275`, 정수 연산).
- `SkinJudge(images, numbers, index, shift)`: 내부 배열은 7칸, 넘치는 것은 버린다 (`play/SkinJudge.java:20-24`, `:37-56`).
- 숫자의 `ref` 는 무시된다. 값은 항상 `JudgeManager.getNowCombo(index)` 다 (`SkinJudge.java:124`).

### 6.3 prepare / draw (`play/SkinJudge.java:96-139`)

```
judgenow = JudgeManager.getNowJudge(index) - 1        (0:PG .. 5:MS, 판정 전이면 -1 → 그리지 않음)
judgenow == 0 이고 현재 게이지가 MAX 이면
    nowJudge = judge[6] ?? judge[0], nowCount = count[6] ?? count[0]
아니면
    nowJudge = judge[judgenow]
    nowCount = judgenow < 3 ? count[judgenow] : null   (BD, PR, MS 는 콤보 숫자 없음)
nowJudge 가 null 이면 그리지 않음
nowJudge.prepare(time, state)                          (이미지 자신의 timer/loop/op/draw/offset 평가)
nowJudge.draw 가 false 면 그리지 않음
nowCount 가 있으면
    nowCount.prepare(time, state, combo, offsetX = nowJudge.region.x, offsetY = nowJudge.region.y)
    shift 면 nowJudge.region.x -= nowCount.getLength() / 2
draw: 숫자 먼저, 판정 이미지를 나중에
```

콤보 숫자 위치 계산 (`skin/SkinNumber.java:136-200`):

- 숫자 사각형 `region` = 숫자 자신의 dst(로더가 x 를 보정한 값) + 판정 이미지 사각형의 (x, y). 판정 이미지의 x 는 **shift 적용 전** 값이다.
- `shiftbase` = 앞쪽 빈 자릿수, `length = (region.width + space * scaleX) * (keta - shiftbase)`.
- 가운데 정렬: `shift = (region.width + space) * 0.5 * shiftbase`. j번째 자리는 `region.x + (region.width + space) * j - shift`.
- 결과적으로 **보이는 숫자열의 중심 x = 판정 이미지 왼쪽 변 + 원래 숫자 dst.x (스케일 적용)**. 숫자 dst.y 는 판정 이미지 아래변 기준 상대 y 다.
- `shift = true` 면 판정 이미지가 `length / 2` 만큼 왼쪽으로 가므로 "판정 글자 + 숫자" 묶음의 중심이 자릿수와 무관하게 고정된다. 기본 스킨: 판정 `w 180`, 숫자 `x 200, w 40, digit 6` (`skin/default/play7.json:336-394`).
- 숫자 간격 `space` 는 `scaleX`(dw) 로 스케일된다 (`SkinJudge.java:81-90`, `SkinNumber.java:128-131`).

표시 시간: `judgenow` 는 지워지지 않는다. 표시/소멸은 이미지와 숫자 Destination 의 `timer`(46/47/247)와 `loop: -1` 로 스킨이 제어한다. 기본 스킨은 0~500ms 표시다.

### 6.4 판정 타이머와 영역 (`play/JudgeManager.java:636-692`)

판정이 날 때마다(`updateMicro`):

1. `judge <= skin.judgetimer` 이면 그 레인의 봄 타이머 on (`:673-675`).
2. `judgeregion > 0` 이면 `judgeindex = lane / (레인 수 / judgeregion)` (정수 나눗셈).
   - `JUDGE_TIMER = {46, 47, 247}` 의 `[judgeindex]` on.
   - `judgeregion >= 3` 이면 다른 영역의 `COMBO_TIMER` 를 끈다.
   - `COMBO_TIMER = {446, 447, 448}` 의 `[judgeindex]` on.
   - `judgenow[judgeindex] = judge + 1`, `judgecombo[judgeindex] = 코스 누적 콤보`, `judgefast[judgeindex] = mfast / 1000` (ms, 양수 = 빠름).
3. `judgeregion == 0` (스킨에 judge 객체가 없음) 이면 판정/콤보 타이머는 전혀 켜지지 않는다.

DP 대응: 14키(16레인)에서 judge 를 index 0, 1 두 개 두면 레인 0-7 이 타이머 46, 레인 8-15 가 타이머 47 이다 (`skin/default/play14.json:374-488`). judge 를 index 0 하나만 두면 양쪽 모두 46 이다. ModernChic SP 는 index 0 만 여러 개 쓰고(조건별 3세트) `draw` 함수로 전환한다 (`/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/judge.lua:54-227`).

판정 관련 값과 옵션:

| id | 의미 | 근거 |
| --- | --- | --- |
| number 525 / 526 / 527 | 최근 판정 오차(ms), 영역 0 / 1 / 2 | `skin/SkinProperty.java:617-619`, `skin/property/IntegerPropertyFactory.java:909-917` |
| option 241 / 261 / 361 | 최근 판정이 PG (영역 0 / 1 / 2) | `SkinProperty.java:829-848`, `skin/property/BooleanPropertyFactory.java:234-246` |
| option 1242 / 1262 / 1362 | PG 가 아니고 빠름(`judgefast > 0`) | 같은 곳 |
| option 1243 / 1263 / 1363 | PG 가 아니고 느림(`judgefast < 0`) | 같은 곳 |
| image index 500-509, 510-519, 1510-1599, 1610-1699 | 레인별 판정 값(키빔 imageset 의 `ref`) | `IntegerPropertyFactory.java:927-937`, `skin/SkinPropertyMapper.java:74-99` |

레인별 판정 값 (`JudgeManager.java:671-672`, `:266`, `:438`, `:462`): 0 = 판정 없음(빈 타격), 1 = PG, GR 2(빠름)/3(느림), GD 4/5, BD 6/7, MS 10/11, 8 = LN 누르는 중. PR(4)은 값을 바꾸지 않는다. 기본 스킨 키빔은 `imageset` `ref:501..507, 500` 으로 0/1 두 그림만 쓴다 (`skin/default/play7.json:182-189`).

커스텀 오프셋 상수: `OFFSET_NOTES_1P = 30`, `OFFSET_JUDGE_1P = 32`, `OFFSET_JUDGEDETAIL_1P = 33`. 2P/3P 상수도 값이 같은 32/33 이다 (`SkinProperty.java:952-958`). 사용자 정의 오프셋은 40번 이후 (`:965` 주석).

---

## 7. bga 객체

### 7.1 정의와 로딩

- JSON: `"bga": {"id": ...}` 뿐이다 (`JsonSkin.java:410-412`).
- `new SkinBGA(loader.bgaExpand)`. 설정값에 따라 스트레치가 정해진다 (`play/SkinBGA.java:24-31`, `Config.java:153-156`).

| `Config.bgaExpand` | StretchType | 동작 |
| --- | --- | --- |
| 0 FULL | STRETCH | 사각형에 맞춰 늘림 |
| 1 KEEP_ASPECT_RATIO (기본) | KEEP_ASPECT_RATIO_FIT_INNER | 비율 유지, 안쪽에 맞춤, 중앙 정렬 |
| 2 OFF | KEEP_ASPECT_RATIO_NO_EXPANDING | 확대하지 않고 넘칠 때만 축소, 중앙 정렬 |

destination 의 `stretch >= 0` 이면 그 값으로 덮어쓴다 (`skin/json/JSONSkinLoader.java:476-478`, `skin/StretchType.java`).

### 7.2 prepare / draw (`SkinBGA.java:34-58`, `play/bga/BGAProcessor.java`)

- prepare: 객체가 보일 때 `prepareBGA(t)` 호출. 상태가 PRELOAD, PRACTICE, READY 이면 `t = -1`, 그 외는 `TIMER_PLAY` 경과 ms.
- draw: 플레이 모드가 PRACTICE 이고 스킨에 practice 객체가 없으면 BGA 영역에 구식 프랙티스 UI 를 그린다. 그 외에는 `drawBGA`.
- `prepareBGA(time)` (`BGAProcessor.java:288-331`): BGA/레이어/이벤트 레이어가 있는 타임라인만 본다. `tl.time <= time` 인 것 중 새로 지난 것에 대해
  - `bga == -2` 면 BGA 끔, `>= 0` 이면 그 id 로 교체. -1 은 변화 없음.
  - `layer` 도 같은 규칙.
  - 이벤트 레이어 중 `EventType.MISS` 가 있으면 미스 레이어로 등록.
- `drawBGA` (`:334-371`): 색과 블렌드는 dst 값을 쓴다.
  1. `time < 0` 이면 1x1 검은 텍스처를 사각형 전체에 그린다.
  2. 미스 레이어 구간(`misslayertime != 0` 이고 `misslayertime <= time < misslayertime + duration`)이면 **미스 레이어 그림만** 그린다. 그림은 `seq[(int)((seq.length - 1) * (time - misslayertime) / duration)].id`, 필터 LINEAR, 비율 보정. 본 BGA 와 검은 배경은 그리지 않는다.
  3. 그 외: 본 BGA 텍스처가 있으면 그린다(동영상 TYPE_FFMPEG, 정지 그림 TYPE_LINEAR). 없으면 검은 텍스처로 사각형 전체를 채운다. 이어서 레이어를 그린다(동영상 TYPE_FFMPEG, 정지 그림 TYPE_LAYER).
- TYPE_LAYER 셰이더는 r, g, b 가 모두 0.0 인 픽셀을 알파 0 으로 만든다 (`/Users/hyunseokbyun/development/beatoraja/src/glsl/layer.frag`).
- 미스 레이어 시작: 판정 갱신 후 콤보가 0 이면 `setMisslayerTme(판정 시각 ms)` (`play/BMSPlayer.java:1019-1023`). 길이는 `PlayerConfig.misslayerDuration` 기본 500ms (`PlayerConfig.java:108`).

### 7.3 리소스 (`BGAProcessor.java:114-233`, `play/bga/BGImageProcessor.java`)

- 파일 탐색: 지정 이름이 있으면 같은 종류(동영상/그림) 확장자를 차례로 바꿔 가며 찾고, 없으면 확장자를 떼고 동영상 확장자 → 그림 확장자 순으로 찾는다.
- 그림 확장자: `jpg, jpeg, gif, bmp, png, tga` (`BGImageProcessor.java:25`).
- 긴 변이 256 이하인 그림은 256x256 캔버스에 **가로 중앙, 위쪽 정렬** 로 옮겨 그린다. 빈 곳은 0(투명 검정) (`BGImageProcessor.java:46-57`).
- 텍스처 캐시 256칸(`id % 256`), 곡 시작 전에 사용하는 그림을 미리 텍스처로 만든다 (`:103-129`).
- `bga.prepare` 는 READY 로 넘어갈 때 호출된다 (`BMSPlayer.java:507`, `:587`).

---

## 8. 그래프류 객체

### 8.1 judgegraph (`skin/SkinNoteDistributionGraph.java`)

필드 (`JsonSkin.java:263-271`): `id`, `type`(0), `backTexOff`(0), `delay`(500), `orderReverse`(0), `noGap`(0), `noGapX`(0).

- `type`: 0 노트 종류, 1 판정, 2 EARLY/LATE (`:69-73`). 분류 수는 7 / 6 / 10.
- 데이터: `data[초][분류]`. 길이는 `lastTime / 1000 + 1` (`:277`).
- `max`: 초당 합계의 최댓값 기준. 시작 20, `max < count` 이면 `min((count / 10) * 10 + 10, 100)` (`:306-309`, `:368-370`).
- 텍스처 크기: `w = data.length * 5`, `h = max * 5`. 노트 한 개가 `(i * 5, j * 5)` 에 `4 x 4` 칩. `noGapX` 면 가로 5, `noGap` 이면 세로 5 (`:510-541`).
- 쌓는 순서: 분류 0 부터 위로. `orderReverse` 면 마지막 분류부터.
- 화면에는 `draw(tex, region.x, region.y + region.height, region.width, -region.height)` 로 그려 pixmap 의 y=0 이 영역의 아래변이 된다 (`:235-242`).
- 등장 연출: `render = time >= delay ? 1 : time / delay`. shape 텍스처의 가로 `render` 비율만 왼쪽부터 드러난다 (`:149`, `:236-237`). `time` 은 스킨 시간이다.

배경(back) 텍스처 (`backTexOff == 0` 일 때, `:466-484`):

- 전체 `(0, 0, 0, 0.8)` 로 채움.
- `i = 10, 20, ... < max` 마다 `(0.007 * i, 0.007 * i, 0, 1)` 로 `(0, i * 5, 전체 폭, 50)` 사각형.
- 세로 보조선: 60초마다 `(0.25, 0.25, 0.25)`, 10초마다 `(0.125, 0.125, 0.125)`.
- `backTexOff == 1` 이면 투명.

분류와 색 (`:45-61`):

| type | 분류 | 색(기본) |
| --- | --- | --- |
| 0 | 0 스크래치 LN 끝(LN 모드에서만), 1 스크래치 LN 본체, 2 스크래치 일반, 3 건반 LN 끝, 4 건반 LN 본체, 5 건반 일반, 6 지뢰 | `44ff44, 228822, ff4444, 4444ff, 222288, cccccc, 880000` |
| 1 | 노트 state 0 미판정, 1 PG, 2 GR, 3 GD, 4 BD, 5 PR | `555555, 0088ff, 00ff88, ffff00, ff8800, ff0000` |
| 2 | 0 미판정, 1 PG, 2-5 빠른 GR/GD/BD/PR, 6-9 느린 GR/GD/BD/PR | `555555, 44ff44, 0088ff, 0066cc, 004488, 002244, ff8800, cc6600, 884400, 442200` |

- PMS(POPN_9K)이고 type 1, 2 면 별도 색표 `pmsGraphColor` (`:54-61`, `:159-160`).
- type 0 은 `SongInformation.getDistributionValues()` 가 있으면 그것을 쓴다 (`:171-175`).
- type 0 의 LN: 시작부터 끝까지의 모든 초에 본체 분류를 1씩 더한다 (`:323-333`).
- type 2 의 분류: `state <= 1` 이면 state, 아니면 `playTime >= 0 ? state : state + 4` (`:416-428`).
- 지뢰와 (LN 모드의) LN 끝 노트는 type 1, 2 에서 제외 (`:340-342`, `:352-354`).

플레이 중 갱신 (`:184-238`):

- type 이 0 이 아니면 750ms 마다, 처리 노트 수가 바뀌었을 때만 바뀐 초 구간을 다시 그린다.
- 커서 텍스처는 50ms 마다 다시 그린다. 폭 3px 세로선. 시작 커서(프랙티스) 연녹색 `80ff80`, 끝 커서 연적색 `ff8080`, 현재 위치 흰색. 현재 위치 x = `TIMER_PLAY 경과 ms(프랙티스면 * freq) * w / (data.length * 1000)`.
- 플레이 화면이 아니면 back 과 shape 만 그린다.
- 로더에서 judgegraph 는 `return` 이 아니라 `break` 로 빠져나온다 (`skin/json/JsonSkinObjectLoader.java:475-481`). 같은 id 가 뒤의 bpmgraph 등에도 있으면 그쪽이 반환된다.

### 8.2 bpmgraph (`skin/SkinBPMGraph.java`)

필드 (`JsonSkin.java:273-283`): `id`, `delay`(0), `lineWidth`(2), `mainBPMColor`(`00ff00`), `minBPMColor`(`0000ff`), `maxBPMColor`(`ff0000`), `otherBPMColor`(`ffff00`), `stopLineColor`(`ff00ff`), `transitionLineColor`(`7f7f7f`).

- `delay <= 0`, `lineWidth <= 0` 이면 기본값 유지 (`:57-58`).
- 색 문자열은 16진 문자만 남기고 앞 6자만 쓴다. 비면 기본색 (`:230-233`, `:65-82`).
- 데이터 `data[k] = {속도, 시각(ms)}`. `SongInformation.getSpeedchangeValues()` 가 있으면 그것, 없으면 모델에서 계산: 정지 중이면 0, 아니면 `BPM * SCROLL` (`:110-165`).
- 텍스처: `data.length < 2` 이거나 `mainbpm <= 0` 이면 1x1 빈 텍스처. 아니면 **최초 그릴 때의 영역 크기** `|region.width| x |region.height|` (`:169-174`).
- `lastTime = min(마지막 데이터 시각, song.getLength()) + 1000` (`:176-181`).
- y 좌표: `(log10(clamp(속도 / mainbpm, 1/8, 8)) - log10(1/8)) / (log10(8) - log10(1/8)) * (height - lineWidth)` (`:188`, `:51-54`).
- 구간 i 마다 (`:185-207`):
  - 세로선(변화): x = `width * data[i].time / lastTime`, `|y2 - y1| - lineWidth > 0` 이면 `transitionLineColor` 로 `(x, min(y1, y2) + lineWidth, lineWidth, |y2 - y1| - lineWidth)`.
  - 가로선: `(x1, y, x2 - x1 + lineWidth, lineWidth)`. 색은 `data[i-1].속도` 가 mainbpm 이면 main, minbpm 이면 min, maxbpm 이면 max, 0 이하면 stop, 그 외 other (이 순서로 판정).
- 마지막 구간은 영역 오른쪽 끝까지 (`:208-219`).
- 화면에는 세로로 뒤집어 그려 높은 BPM 이 위다. `delay` 동안 왼쪽부터 드러난다 (`:105-107`).
- 곡이 바뀔 때만 다시 만든다 (`:95-103`).

### 8.3 timingvisualizer (`skin/SkinTimingVisualizer.java`)

필드 (`JsonSkin.java:307-321`): `width`(301), `judgeWidthMillis`(150), `lineWidth`(1, 1~4 로 클램프), `lineColor`(`00FF00FF`), `centerColor`(`FFFFFFFF`), `PGColor`(`000088FF`), `GRColor`(`008800FF`), `GDColor`(`888800FF`), `BDColor`(`880000FF`), `PRColor`(`000000FF`), `transparent`(0), `drawDecay`(1).

- 색 검증: 16진 외 문자가 있거나 6자 미만이면 `FF0000FF` (`:185-191`). `transparent == 1` 이면 PR 색은 투명.
- `center = judgeWidthMillis`, `judgeWidthRate = width / (judgeWidthMillis * 2 + 1)`. `width` 는 스킨 스케일을 받지 않는 원시 값이다 (`:61-63`).
- 판정 폭: `BMSPlayerRule(originalMode).judge.getNoteJudge(judgerank, rate)` (ms). 순서는 PG, GR, GD, BD, MS 각각 `{LATE 하한(음수), EARLY 상한(양수)}`. 커스텀 판정 폭과 NO_GREAT/NO_GOOD 제약을 반영 (`:153-172`, `play/JudgeProperty.java:107-109`, `:147-149`).
- 배경 텍스처: `(center * 2 + 1) x 1`. 가운데 1px 에 `centerColor`, 안쪽 판정부터 `x1 = center + clamp(area[i*2])`, `x2 = center + clamp(area[i*2+1]) + 1` 로 바깥쪽을 덧칠. 마지막에 `center % 10` 부터 10px 간격으로 `(0, 0, 0, 0.25)` 눈금. 모델이 바뀔 때만 다시 만든다 (`:86-123`).
- 그리기 (`:138-151`): 배경을 영역에 늘려 그린 뒤 최근 판정 100개(`JudgeManager.recentJudges`, ms, 양수 = 빠름)를 오래된 것부터 그린다.
  - 선 색 알파 = `lineColor.a / 100 * (i + 1)` (새것일수록 진함).
  - `-center <= 값 <= center` 일 때만.
  - x = `region.x + (region.width - lineWidth) / 2 + 값 * judgeWidthRate`. **빠름이 오른쪽.**
  - `drawDecay`: y = `region.y + region.height * (100 - i) / 100 / 2`, h = `region.height * i / 100`. 아니면 영역 전체 높이.
- `recentJudges` 는 길이 100 순환 버퍼, 초기값 `Long.MIN_VALUE`, 판정이 PR 미만(`judge < 4`)일 때만 기록 (`play/JudgeManager.java:114-119`, `:654-658`).

### 8.4 hiterrorvisualizer (`skin/SkinHitErrorVisualizer.java`)

필드 (`JsonSkin.java:285-305`): `width`(301), `judgeWidthMillis`(150), `lineWidth`(1, 1~4), `colorMode`(1), `hiterrorMode`(1), `emaMode`(1), `lineColor`(`99CCFF80`), `centerColor`(`FFFFFFFF`), `PGColor`(`99CCFF80`), `GRColor`(`F2CB3080`), `GDColor`(`14CC8f80`), `BDColor`(`FF1AB380`), `PRColor`(`CC292980`), `emaColor`(`FF0000FF`), `alpha`(0.1), `windowLength`(30, 1~100), `transparent`(0), `drawDecay`(1).

- pixmap 크기 `width x (windowLength * 2)`. 최근 판정 인덱스가 바뀔 때만 다시 그린다 (`:106-116`).
- `hiterrorMode == 1`: 최근 `windowLength` 개를 그린다. `i = windowLength..1`, 대상은 `recent[(index - windowLength + i) mod 100]` (`:124-159`).
  - `colorMode == 1`: 값이 PG/GR/GD/BD 폭 안이면 해당 색, 아니면 PR 색. `colorMode == 0`: `lineColor` 에 알파 `lineColor.a * i / (windowLength / 2)`.
  - x = `(width - lineWidth) / 2 + (int)(clamp(값, -center, center) * -judgeWidthRate)`. **빠름이 왼쪽**(timingvisualizer 와 반대).
  - `drawDecay`: `(x, windowLength - i, lineWidth, i * 2)`. 아니면 `(x, 0, lineWidth, 200)`.
- 중앙선: `centerColor` 로 `((width - lineWidth) / 2, 0, lineWidth, windowLength * 2)`.
- EMA(`emaMode != 0`): 최신 값이 BD 폭 안이면 `ema += (long)(alpha * (값 - ema))`. x 는 같은 식. `emaMode` 1 또는 3 이면 세로선, 2 또는 3 이면 삼각형 `(x + lineWidth / 2, windowLength * 2 / 3)`, `(x + w, 0)`, `(x - w, 0)`, `w = (int)(width * 0.01)` 을 짝수로 올림 (`:165-187`).
- 텍스처는 뒤집지 않고 영역에 그린다 (`:115`).
- 플레이 화면이 아니면 그리지 않는다 (`:85-88`).

### 8.5 timingdistributiongraph (`skin/SkinTimingDistributionGraph.java`)

리절트(`MusicResult`) 전용이다. 플레이 화면에서는 그려지지 않는다 (`:58-62`). 필드: `width`(301), `lineWidth`(1), `graphColor`, `averageColor`, `devColor`, 판정 5색, `drawAverage`(1), `drawDev`(1) (`JsonSkin.java:323-337`). 텍스처는 `(width / lineWidth) x max`, 한 번만 만든다 (`:70-136`).

### 8.6 ModernChic 사용 현황 (교차 확인)

`judgegraph`(type 과 backTexOff 지정), `bpmgraph`(기본값), `timingvisualizer`(width 300, drawDecay 0, 반투명 색), `hiterrorvisualizer`(width 300, emaMode 1~3) 을 모두 쓴다 (`/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/graph.lua:6-70`, `:132-148`, `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/prepare.lua:88`, `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/detailinfo/bgaareainfo.lua:237-238`). `pmchara`, `dst2`, `expansionrate`, `lnbodyActive`, `hcnbodyActive` 는 Play 폴더에서 찾지 못했다.

---

## 9. practice 객체와 pmchara

### 9.1 practice (`play/SkinPractice.java`, `play/PracticeConfiguration.java`)

- JSON: `"practice": {"id", "visibleItems": 10}` (`JsonSkin.java:424-427`). `visibleItems` 는 0~16 으로 클램프.
- `visibleItems > 0`: 객체는 아무것도 그리지 않고 `setVisibleItemCount` 만 한다. 스킨이 텍스트/옵션/버튼 슬롯으로 직접 그린다. 관련 id: `OPTION_PRACTICE_ITEM1..16`(3000-3015), `..._SELECTED`(3020-3035), `BUTTON_PRACTICE_ITEM1..16` (`skin/SkinProperty.java` `OPTION_PRACTICE_*`, `play/BMSPlayer.java:986-991`).
- `visibleItems == 0`, 또는 스킨에 practice 가 없어 BGA 영역에서 대체할 때: 구식 UI (`SkinPractice.java:96-148`).
  - 시스템 폰트 18px. 기준 `x = region.x + region.width / 8`, `y = region.y + region.height * 7 / 8`, 줄 간격 22.
  - 항목 i: 라벨 `(x, y - 22 * i)`, 값 `(x + 150, ...)`. 선택 항목 색 YELLOW(터보면 ORANGE), 그 외 CYAN(가로 입력 모드면 GRAY).
  - 도움말 2줄 `(x, y - 22 * 12 - 12)`, `(x, y - 22 * 13 - 12)` ORANGE.
  - 판정 수 6줄 `(x + 250, y - 22 * i)` WHITE, 형식 `"%s %d %d %d"`.
  - 아래 1/4 영역에 노트 분포 그래프(선택한 graphtype, delay 500).
- 항목 순서와 라벨 (`PracticeConfiguration.java:545-637`): START TIME, END TIME, GAUGE TYPE, GAUGE CATEGORY, GAUGE VALUE, JUDGERANK, TOTAL, FREQUENCY, GRAPHTYPE, OPTION-1P, OPTION-2P(2인용만), OPTION-DP(2인용만).
- 기본값 (`:669-719`): starttime 0, endtime 10000(생성 시 `lastTime + 1000`), gaugetype 2, startgauge 20, random 0, random2 0, doubleop 0, judgerank 100(생성 시 모델 값), freq 100, total 0(생성 시 모델 값), graphtype 0.
- 설정 저장 위치: `practice/<sha256>.json` (`:61`, `:94-106`).
- 프랙티스 상태에서 노트 화면은 `time = starttime`, hispeed 1.0, 시간선/BPM선/정지선 표시 (3.3).

### 9.2 pmchara (`skin/PomyuCharaLoader.java`, `play/PomyuCharaProcessor.java`)

ModernChic 은 사용하지 않는다. 요약만 적는다.

- JSON: `id`, `src`, `color`(1), `type`, `side`(1) (`JsonSkin.java:531-537`).
- type: 0 플레이, 1 배경, 2 이름, 3 상반신, 4 전신, 5 아이콘, 6 NEUTRAL, 7 FEVER, 8 GREAT, 9 GOOD, 10 BAD, 11 FEVERWIN, 12 WIN, 13 LOSE, 14 OJAMA, 15 DANCE (`PomyuCharaLoader.java:36-51`).
- `.chp` 파일(MS932, 탭 구분)을 읽어 `#CharBMP`, `#CharTex`, `#Pattern`, `#Texture`, `#Layer`, `#Frame`, `#Anime`, `#Size`, `#Loop`, 좌표 정의를 해석하고 모션별 `SkinImage` 를 스킨에 직접 추가한다. 오른쪽 아래 1px 색을 투명색으로 처리한다 (`:93-180`, `:548-579`).
- 타이머 900-907, 909 는 `PomyuCharaProcessor.updateTimer` 가 판정에 따라 전환한다 (`PomyuCharaProcessor.java:40-73`). `TIMER_PM_CHARA_DANCE`(909) 는 플레이 중 항상 on.

---

## 10. 플레이 화면 수명주기 (`play/BMSPlayer.java`)

### 10.1 생성

- 생성자: 리플레이 로드, RANDOM 분기, 패턴 옵션 적용, 어시스트 판정 (`:85-368`). `playtime = (오토플레이 ? lastTime : lastNoteTime) + 5000` (`:198`, `TIME_MARGIN = 5000` `:65`).
- `getSkinType()`: `SkinType.values()` 순서로 모드가 같은 첫 타입. BATTLE 타입은 선택되지 않는다 (`:370-377`).
- `create()` (`:379-455`): LaneProperty, KeySoundProcessor, 게이지, JudgeManager, ControlInputProcessor, KeyInputProccessor 생성 → `loadSkin` → LaneRenderer → `judge.init` → RhythmTimerProcessor → 스코어/타깃 설정. 프랙티스면 `state = STATE_PRACTICE`.

### 10.2 상태 전이

| 상태 | 값 | 진입 조건 | 진입 시 동작 |
| --- | --- | --- | --- |
| STATE_PRELOAD | 0 | 초기 상태 | - |
| STATE_PRACTICE | 1 | 프랙티스 모드 생성 시, 프랙티스 플레이 종료 후 | - |
| STATE_PRACTICE_FINISHED | 2 | PRACTICE/PRELOAD/READY 에서 `stopPlay()` | `TIMER_FADEOUT` on (`:939-949`) |
| STATE_READY | 3 | PRELOAD: 미디어 로드 완료, `micronow > (loadstart + loadend) * 1000`, START/SELECT 를 뗀 지 1초 초과 (`:499-500`). PRACTICE: 1번 키를 누름 + 같은 조건 (`:549-550`) | `bga.prepare`, `TIMER_READY` on, `PLAY_READY` 효과음 (`:507-516`) |
| STATE_PLAY | 4 | `TIMER_READY` 경과 > `playstart` (`:606`) | `TIMER_PLAY`, `TIMER_RHYTHM` = `micronow - starttimeoffset * 1000`, 판정 스레드 시작, BG 재생 시작 (`:607-616`) |
| STATE_FAILED | 5 | 게이지 0 이고 자동 전환 없음 (`:651-662`), 또는 곡 도중 `stopPlay()` (`:961-969`) | `TIMER_FAILED` on, 음성 정지, `PLAY_STOP` 효과음 |
| STATE_FINISHED | 6 | `playtime < TIMER_PLAY 경과` (`:679-687`), 또는 모든 노트 처리 후/오토플레이 중 `stopPlay()` (`:953-958`) | 전자는 `TIMER_MUSIC_END` on, 후자는 `TIMER_FADEOUT` on |

상태별 매 프레임 처리:

- 공통: `micronow > input * 1000` 이면 `TIMER_STARTINPUT` on. START/SELECT 를 누르고 있으면 `startpressedtime = micronow` (`:470-475`).
- PRELOAD: 차트 미리보기(`config.isChartPreview()`). START/SELECT 를 누르는 프레임에 타이머 141 을 `micronow - 첫 노트 시각 + 1000000` 으로 설정하고, 떼면 끄고 레인 렌더러를 초기화한다 (`:481-497`). 미리보기 중 노트 시간은 `time - timer(141)` (`play/LaneRenderer.java:300-301`).
- PLAY: 재생 속도 보정(`TIMER_PLAY += deltatime * (100 - playspeed) / 100`), 리듬 타이머, 500ms 마다 게이지 로그, `TIMER_GAUGE_MAX_1P`, 게이지 자동 전환 또는 FAILED 전이, 포뮤 타이머, 종료 판정 (`:620-691`).
- FAILED: 판정/BG 정지. START 와 SELECT 중 하나만 누르면(코스가 아니고 일반 플레이) 퀵 리트라이. `TIMER_FAILED` 경과 > `close` 이면 리절트 또는 선곡 (`:693-743`).
- FINISHED: 판정/BG 정지. `TIMER_MUSIC_END` 경과 > `finishmargin` 이면 `TIMER_FADEOUT` on. `TIMER_FADEOUT` 경과 > `fadeout` 이면 리절트/다음 곡/선곡 (`:745-785`).
- PRACTICE_FINISHED: `TIMER_FADEOUT` 경과 > `fadeout` 이면 선곡 (`:597-603`).
- `getNowTime(id)` 는 타이머가 off 면 0 을 돌려준다 (`TimerManager.java:37-42`).

`stopPlay()` 는 `TIMER_FAILED` 또는 `TIMER_FADEOUT` 이 이미 on 이면 아무것도 하지 않는다 (`:950-952`).

FAILED 전이는 페이드아웃 타이머를 켜지 않는다. 폐점 연출은 스킨이 타이머 3 으로 그린다 (`skin/default/play7.json:733-744`).

---

## 11. 타이머 전체 표

타이머 값은 "켜진 시각(us)" 이다. `Long.MIN_VALUE` 가 off. `setTimerOn` 은 현재 시각으로 덮어쓰고, `switchTimer(id, true)` 는 off 일 때만 켠다 (`TimerManager.java:67-95`). 화면이 바뀌면 전부 off 로 초기화된다 (`:101-107`).

### 11.1 화면/진행 타이머

| id | 상수 | 켜는 시점 | 끄는 시점 | 근거 |
| --- | --- | --- | --- | --- |
| 1 | TIMER_STARTINPUT | 화면 시작 후 `input` ms 경과 | - | `play/BMSPlayer.java:470-472` |
| 2 | TIMER_FADEOUT | FINISHED 에서 `finishmargin` 경과, 또는 `stopPlay()` | 프랙티스 재시작 | `:536`, `:748-750`, `:941`, `:946`, `:957`, `:960` |
| 3 | TIMER_FAILED | FAILED 전이 | 프랙티스 재시작 | `:535`, `:656`, `:963` |
| 40 | TIMER_READY | READY 전이 | 끄지 않음 | `:514`, `:591` |
| 41 | TIMER_PLAY | PLAY 전이 (`micronow - starttimeoffset * 1000`). 재생 속도에 따라 매 프레임 보정 | 프랙티스 재시작 | `:533`, `:609`, `:624` |
| 44 | TIMER_GAUGE_MAX_1P | 게이지 MAX | MAX 가 아님 | `:636` |
| 48 | TIMER_FULLCOMBO_1P | 처리 노트 수 == 총 노트 수 이고 == 콤보 (판정마다 평가) | 조건 불만족 | `:1028-1029` |
| 140 | TIMER_RHYTHM | PLAY 전이, 그 뒤 마디선마다 재설정. 매 프레임 값 보정(11.2) | 프랙티스 재시작 | `:534`, `:610`, `play/RhythmTimerProcessor.java:60-71` |
| 141 | (상수 없음) | 차트 미리보기 시작 | 미리보기 종료, READY 전이 | `:482-503` |
| 143 | TIMER_ENDOFNOTE_1P | `TIMER_PLAY` 경과 > `playtime - 5000` (마지막 노트 이후) | 프랙티스 재시작 | `:537`, `:688-690` |
| 348 / 349 / 350 | TIMER_SCORE_A / AA / AAA | `qualifyRank(18 / 21 / 24)` (판정마다 평가) | 조건 불만족 | `:1033-1035` |
| 351 | TIMER_SCORE_BEST | EX 스코어 >= 자기 베스트 | 조건 불만족 | `:1036` |
| 352 | TIMER_SCORE_TARGET | EX 스코어 >= 타깃 | 조건 불만족 | `:1037` |
| 908 | TIMER_MUSIC_END | FINISHED 전이(`playtime` 초과) | - | `:681` |
| 900-907, 909 | TIMER_PM_CHARA_* | PRELOAD/PRACTICE 에서 900, 905 on. 플레이 중 판정에 따라 전환, 909 항상 on | FINISHED 전이, 프랙티스 재시작 | `:518-521`, `:539-544`, `:682-685`, `play/PomyuCharaProcessor.java:40-73` |

정의만 있고 설정되지 않는 타이머: 42, 43(GAUGE_INCLEASE), 45(GAUGE_MAX_2P), 49(FULLCOMBO_2P), 144(ENDOFNOTE_2P).

### 11.2 리듬 타이머 (`play/RhythmTimerProcessor.java:60-71`)

```
매 프레임: rhythmtimer += deltatime * (100 - nowbpm * playspeed / 60) / 100
           TIMER_RHYTHM = rhythmtimer
마디선 시각 도달: sections++, TIMER_RHYTHM = now, rhythmtimer = micronow
```

경과 시간 `now - TIMER_RHYTHM` 은 실시간의 `bpm * playspeed / 6000` 배로 흐른다. BPM 과 무관하게 **1박 = 경과 1000ms** 가 되고 마디선마다 0 으로 돌아간다. 프랙티스 주파수 보정은 `sectiontimes * (100 / freq)` 인데 `100 / freq` 가 정수 나눗셈이다 (`:67`).

### 11.3 판정/콤보 타이머 (`play/JudgeManager.java:636-692`)

| id | 상수 | 켜는 시점 |
| --- | --- | --- |
| 46 / 47 / 247 | TIMER_JUDGE_1P / 2P / 3P | 해당 영역에서 판정이 날 때마다 `setTimerOn` (재시작) |
| 446 / 447 / 448 | TIMER_COMBO_1P / 2P / 3P | 같은 시점. 영역이 3개 이상이면 다른 영역 것을 끈다 |

### 11.4 레인별 타이머 (`skin/SkinPropertyMapper.java:7-71`)

`player = lane / (레인 수 / 플레이어 수)`, `offset = laneToSkinOffset[lane]` (12절). `offset < 10` 이면 `base + offset + player * 10`, `10 <= offset < 100` 이면 `base10 + (offset - 10) + player * 100`.

| 종류 | base (1P 스크래치) | 1P 범위 | 2P 범위 | base10 (1P / 2P) | 켜고 끄는 시점 |
| --- | --- | --- | --- | --- | --- |
| 봄(bomb) | 50 | 50-59 | 60-69 | 1010 / 1110 | 판정 `<= judgetimer` 일 때 `setTimerOn`. 끄지 않음 (`JudgeManager.java:673-675`) |
| 홀드(hold) | 70 | 70-79 | 80-89 | 1210 / 1310 | 매 판정 루프에서 `switchTimer(processing != null 또는 (passing != null 이고 inclease))` (`:632`) |
| 키 on | 100 | 100-109 | 110-119 | 1410 / 1510 | 아래 설명 |
| 키 off | 120 | 120-129 | 130-139 | 1610 / 1710 | 아래 설명 |
| HCN active | 250 | 250-259 | 260-269 | 1810 / 1910 | HCN 통과 중이고 누르는 중 on, 아니면 off (`:306-338`) |
| HCN damage | 270 | 270-279 | 280-289 | 2010 / 2110 | HCN 통과 중이고 떼고 있을 때 on, 아니면 off |

`offset` 0 이 스크래치, 1~9 가 건반이다. 예: 7키 1번 건반 봄 = 51, 스크래치 봄 = 50, 14키 2P 1번 건반 봄 = 61, 2P 스크래치 키 on = 110.

키 on/off (`play/KeyInputProccessor.java:51-122`):

- 매 프레임 레인마다 "눌림" 을 계산한다: 그 레인에 배정된 키 중 하나가 눌렸거나 오토플레이 가상 입력이 있으면 눌림. `keyBeamStop` 이면 항상 안 눌림.
- 눌림 + (판정 스레드 시작 전 또는 오토플레이): 키 on 이 off 이거나 스크래치 방향이 바뀌었으면 키 on `setTimerOn`, 키 off `setTimerOff`.
- 눌림 + 판정 스레드 동작 중(일반 플레이): 이 루프에서는 켜지 않는다. 판정 스레드가 키 입력을 처리한 직후 `inputKeyOn(lane)` 으로 켠다. 키 on 이 off 이거나 스크래치 레인이면 재시작 (`:112-122`, `JudgeManager.java:489`).
- 안 눌림 + 키 on 이 on: 키 off `setTimerOn`, 키 on `setTimerOff`.
- `stopJudge()` (FAILED/FINISHED) 이후 `keyBeamStop = true` 로 모든 키빔이 꺼진다 (`:124-131`).

오토플레이 가상 입력은 노트 처리 시각부터 80ms 유지된다 (`JudgeManager.java:107`, `:295-302`).

### 11.5 스크래치 회전 오프셋 (`KeyInputProccessor.java:92-108`)

`OFFSET_SCRATCHANGLE_1P`(1), `2P`(2) 의 `r` (도) = `scratch[s] / 6`, `scratch[s]` 는 0~2159.

| 상태 | 1P (s 짝수) 프레임당 증분 | 2P (s 홀수) |
| --- | --- | --- |
| 입력 없음 | `-deltatime` | `+deltatime` |
| 스크래치 두 번째 키(`scratchToKey[s][1]`) | `+deltatime` | `+3 * deltatime` |
| 스크래치 첫 번째 키(`scratchToKey[s][0]`) | `-3 * deltatime` | `-deltatime` |

`deltatime` 은 ms 다. 2160 단위가 한 바퀴이므로 무입력 시 2.16초에 한 바퀴.

---

## 12. 키 모드별 레인 인덱스 (`play/LaneProperty.java:37-112`)

스킨의 `note`/`dst` 배열 인덱스 = 차트 레인 인덱스다. `laneToSkinOffset` 은 타이머/판정 값 id 에 쓰는 번호(0 = 스크래치).

| 모드 | SkinType id | 레인 수 | 레인 → 스킨 오프셋 | 스크래치 레인 | 레인 → 입력 키 |
| --- | --- | --- | --- | --- | --- |
| BEAT_5K | 1 | 6 | `1,2,3,4,5,0` | 5 | `{0},{1},{2},{3},{4},{5,6}` |
| BEAT_7K | 0 | 8 | `1,2,3,4,5,6,7,0` | 7 | `{0}..{6},{7,8}` |
| BEAT_10K | 3 | 12 | `1,2,3,4,5,0,1,2,3,4,5,0` | 5, 11 | `{0}..{4},{5,6},{7}..{11},{12,13}` |
| BEAT_14K | 2 | 16 | `1..7,0,1..7,0` | 7, 15 | `{0}..{6},{7,8},{9}..{15},{16,17}` |
| POPN_9K | 4 | 9 | `1..9` | 없음 | `{0}..{8}` |
| KEYBOARD_24K | 16 | 26 | `1..26` | 없음(`laneToScratch` 전부 -1) | `{i}` |
| KEYBOARD_24K_DOUBLE | 17 | 52 | `i % 26 + 1` | 없음 | `{i}` |

- SkinType id 근거: `skin/SkinType.java:12-30`.
- 플레이어 번호: `laneToPlayer[i] = i / (mode.key / mode.player)` (`:108-111`). 2인용 모드는 앞 절반이 0, 뒤 절반이 1.
- 레인 수는 LaneProperty 배열 길이로 확인했다. `bms.model.Mode` 소스는 저장소에 없어 `mode.key`, `mode.player`, `scratchKey`, `isScratchKey` 의 정의는 미확인이다.
- 24K 의 26레인 중 마지막 2개의 의미(휠 등)는 미확인이다. LaneProperty 상으로는 전부 일반 레인이며 오프셋 10 이상은 11.4 의 base10 범위를 쓴다.
- 스크래치는 한 레인에 키 2개(양방향)가 배정된다. `scratchToKey` 는 BEAT 계열만 가진다.

DP/2P 대응 정리:

| 요소 | 대응 |
| --- | --- |
| note | 하나의 note 객체에 전 레인을 나열한다. 14키는 `[1P 건반 1-7, 1P 스크래치, 2P 건반 1-7, 2P 스크래치]` (`skin/default/play14.json:305`, `:319-336`) |
| group/bpm/stop/time | 플레이어 수만큼(2개) (`play14.json:337-368`) |
| judge | `index 0`, `index 1` 두 객체. 타이머 46, 47. 레인 절반으로 나뉜다 (6.4) |
| gauge | 하나뿐이다. `BMSPlayer.players` 배열 길이가 1 이고 2P 게이지는 없다 (`play/BMSPlayer.java:34`, `:426`) |
| hiddenCover / 레인 커버 | 같은 id 를 destination 에 두 번 배치 (`play14.json:648-659`) |
| 리프트/커버 값 | 0번 레인 기준 하나의 값을 양쪽이 공유 (3.3.1) |
| BATTLE | 7K→14K, 5K→10K, 24K→24K DOUBLE 로 모드를 바꾸므로 DP 스킨이 쓰인다 (`BMSPlayer.java:238-258`) |

---

## 13. ModernChic 교차 확인 메모 (Play 폴더 일부만 확인)

전수 조사는 아니다. 이 보고서의 규칙이 실제로 필요한지 확인하려고 grep 과 부분 열람만 했다.

| 항목 | ModernChic 사용 내용 | 근거 |
| --- | --- | --- |
| note | 구식 명명(`lnbody`, `lnactive`, `hcnbody`, `hcnactive`, `hcndamage`, `hcnreactive`), `size = {}`, `hidden = {}`, `processed = {}` | `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/notes.lua:7-61`, `parts.note` 블록 |
| note destination | `{id = "notes", offset = MAIN.OFFSET.NOTES_1P}` → 오프셋 30 의 x, y, w, h 가 모든 노트에 더해진다 | 같은 파일 `parts.destination` |
| 노트 애니메이션 | `notesAnimation` 옵션 시 LN/HCN 본체에 `divy = 2`, `cycle` 지정 | 같은 파일 157-175행 부근 |
| 마디선 | `group/time/bpm/stop` 각 1개(SP), `offset = LIFT`, 마디선에 알파 지정 | 같은 파일 `group` 블록 |
| gauge | nodes 36개, parts 50 또는 100, destination 을 `draw` 함수로 2벌 배치(알파 점멸) | `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/gauge.lua:89-125` |
| judge | index 0 을 3세트, 이미지/숫자마다 `draw` 함수, `offsets = {LIFT, JUDGE_1P}`, 숫자 `divy = 2~3` 과 `cycle` (프레임 애니메이션), `shift = true` | `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/judge.lua:28-227` |
| 커버 | hiddenCover 1x1 소스, liftCover, slider type LANECOVER, `OP.CONSTANT`, `TIMER.ENDOFNOTE_1P` 로 내려오는 종료 커버 | `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/sp/cover.lua:10-60` |
| 헤더 | `loadend 3500, playstart 1000, close 3000, fadeout 500, input 0` | `/Users/hyunseokbyun/Downloads/ModernChic/Play/lua/require/header.lua:14-19` |

---

## 14. R-BMS 구현 체크리스트 (이 보고서에서 직접 도출되는 요구)

1. 플레이 헤더 필드는 note 객체가 destination 에 있을 때만 적용한다 (1절, 2절).
2. `OFFSET_LIFT`, `OFFSET_LANECOVER`, `OFFSET_HIDDEN_COVER` 를 3.3.2 의 식으로 매 프레임 계산해 오프셋 테이블에 넣는다. beatoraja 는 note 를 그릴 때 갱신하므로 note 보다 앞에 그려지는 객체는 한 프레임 늦은 값을 쓴다.
3. 노트 y 는 3.3.3 의 section 누적식을 쓴다. STOP, SCROLL, SPEED 를 모두 반영한다.
4. 노트 높이는 `size` 또는 원본 이미지 높이 * dh, 폭은 레인 사각형 폭이다. note destination 의 오프셋 합을 x, y, w, h 에 더한다.
5. LN 본체 이미지 선택은 3.2 의 인덱스 표를 따른다. LN 타입은 끝 이미지를 그리지 않는다.
6. 마디선/BPM선/정지선은 선 이미지의 dst 에 `(int)(y - hl)` 을 더해 그린다. 조건은 3.3.4.
7. hiddenCover/liftCover 는 자동 오프셋과 소실선 클립을 구현한다 (4.1).
8. gauge 는 nodes 4/8/12/36 매핑과 5.4 의 칸 선택식을 구현한다.
9. judge 는 6.3 의 상대 좌표와 shift, MAX 상태 7번째 이미지, BD 이하 콤보 숫자 없음을 구현한다.
10. 타이머는 11절 표대로 켜고 끈다. 특히 봄은 `judgetimer`, 판정/콤보는 `judgeregion` 에 의존한다.
11. 리듬 타이머(140)는 "1박 = 1000" 으로 정규화되고 마디선마다 재시작한다.
12. BGA 는 7.2 의 순서(검은 배경 → 본 BGA → 레이어, 미스 레이어 단독 표시)와 스트레치, 256 캔버스 보정, 레이어의 검정 투명 처리를 구현한다.
13. 그래프 4종은 8절의 텍스처 생성 규칙을 따른다.

---

## 15. 위험과 주의점

- 구식 명명의 의미가 이름과 반대다 (3.2). 이름만 보고 매핑하면 누르는 중/아닌 중이 뒤바뀐다.
- `timingvisualizer`/`hiterrorvisualizer` 의 `width` 는 스킨 스케일을 받지 않는다. 선 위치는 `width` 기준, 배경은 영역 기준이라 스킨 해상도와 출력 해상도가 다르면 어긋난다 (8.3, 8.4).
- 두 비주얼라이저의 좌우 방향이 반대다 (8.3 빠름 = 오른쪽, 8.4 빠름 = 왼쪽).
- 마디선 이미지에 넘기는 시간은 차트 시간이다 (3.3.4).
- `hu`/`hl` 이 0번 레인만 기준이다. 레인별 y/h 가 다른 스킨은 beatoraja 에서도 0번 레인 기준으로 그려진다.
- 일찍 친 노트가 판정선까지 계속 보인다 (3.3.5). 똑같이 재현하려면 판정된 노트를 즉시 지우면 안 된다.
- `bpmgraph` 텍스처 크기는 최초 그릴 때의 영역 크기다. 영역이 애니메이션되는 스킨에서는 첫 프레임 크기가 쓰인다 (8.2).
- judgegraph 는 실시간 갱신이 750ms 주기다 (8.1).

---

## 16. 읽은 파일과 미확인 항목

### 16.1 배정 파일 (전부 끝까지 읽음)

`skin/json/JsonPlaySkinObjectLoader.java`, `play/PlaySkin.java`, `play/SkinNote.java`, `play/LaneRenderer.java`, `play/SkinGauge.java`, `play/SkinJudge.java`, `play/SkinBGA.java`, `play/SkinHidden.java`, `play/SkinPractice.java`, `play/bga/BGAProcessor.java`, `play/bga/BGImageProcessor.java`, `play/PomyuCharaProcessor.java`, `skin/PomyuCharaLoader.java`, `skin/SkinNoteDistributionGraph.java`, `skin/SkinBPMGraph.java`, `skin/SkinHitErrorVisualizer.java`, `skin/SkinTimingVisualizer.java`, `skin/SkinTimingDistributionGraph.java`, `play/BMSPlayer.java`, `play/KeyInputProccessor.java`, `play/RhythmTimerProcessor.java`, `play/ControlInputProcessor.java`, `play/PracticeConfiguration.java`, `skin/default/play_parts.lua`.

`skin/default/play7.json` 은 1-140행, 240-765행을 읽고 나머지는 grep 으로 확인했다(752행 이후는 2P 사이드용 반복 블록). `skin/default/play14.json` 은 1-12행, 288-377행, 636-660행, 760-800행을 읽고 나머지는 grep 으로 확인했다.

### 16.2 배정 밖에서 부분적으로 읽은 파일

| 파일 | 읽은 범위 |
| --- | --- |
| `skin/json/JsonSkin.java` | 클래스/필드 목록 전체(grep), 500-557행 |
| `skin/json/JsonSkinObjectLoader.java` | 30-624행, 702-759행 |
| `skin/json/JSONSkinLoader.java` | 296-340행, 422-491행 |
| `play/JudgeManager.java` | 120-349행, 350-497행, 590-888행 |
| `play/LaneProperty.java`, `skin/SkinPropertyMapper.java`, `TimerManager.java`, `skin/SkinNumber.java` | 전체 |
| `skin/SkinObject.java` | 340-639행, 820-875행 |
| `skin/SkinImage.java` | 95-180행 |
| `skin/StretchType.java` | 전체 |
| `skin/Skin.java` | 100-175행, 368-390행, 505-525행, 567-580행, 700-740행 |
| `skin/SkinProperty.java` | 타이머/오프셋 상수(grep), 940-975행 |
| `skin/property/IntegerPropertyFactory.java`, `FloatPropertyFactory.java`, `BooleanPropertyFactory.java` | 레인 커버/판정 관련 부분만 |
| `play/GrooveGauge.java`, `play/JudgeProperty.java`, `skin/SkinSourceImage.java`, `skin/SkinType.java`, `Config.java`, `PlayerConfig.java` | 관련 부분만 |
| `src/glsl/layer.frag`, `src/glsl/ffmpeg.frag` | 전체 |

### 16.3 미확인

- `bms.model.Mode`, `TimeLine`, `LongNote`, `Layer` 등 `bms.model` 패키지 소스는 저장소에 없다. `mode.key`, `mode.player`, `scratchKey`, `isScratchKey`, `getSection`, `getScroll`, `getMicroStop`, LN 타입 상수 값은 호출부에서의 쓰임만 확인했다.
- 24K 26레인의 마지막 두 레인의 의미.
- `LaneRenderer.java:578-581` 의 `dsty < lane.region.y - dscale` 분기의 의도.
- `SkinSlider`(type 4 레인 커버)의 내부 그리기와 마우스 조작, `SkinGaugeGraphObject`, `SkinDistributionGraph` 는 이 배정의 범위 밖이라 읽지 않았다.
- `FFmpegProcessor` 의 프레임 취득/동기 방식.
- `JudgeManager.java` 1-119행, 497-590행(키 뗌 처리)은 읽지 않았다.
- `GaugeProperty.java` 의 종류별 `max`, `border`, `min` 값.
- ModernChic 은 Play 폴더의 일부 파일만 부분 열람했다. DP(`Play/lua/dp/*`) 는 grep 결과만 확인했다.
