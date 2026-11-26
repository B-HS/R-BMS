# IIDX SP 12 목록 만들기 (2026-11-26)

차단 표 `/tables/blocked` 의 `SP 12` 폴더(`web/data/chart-blocklist-sp12.json`)를 만든 보조 스크립트다. 한 번 만든 스냅숏이며 서버는 이 파일을 읽기만 한다. 서열표나 모음 폴더가 바뀌어 다시 만들 때만 쓴다. 결정과 수치는 [ADR](../../acknowledge/2026-11-25-adr-chart-blocklist.md) "SP 12 폴더" 절.

실행(모두 `web/` 에서, 임시 파일은 아무 폴더 `$T`):

1. `curl -sL https://iidx.hyns.dev/table -o $T/table.html` 뒤 `bun 1-extract-tier.ts $T/table.html $T/tier.json` — 페이지의 RSC 데이터에서 SP 12 서열 항목(`title`, `difficulty` A·H·L, `version`, 노말·하드 서열)을 뽑는다.
2. `bun 2-read-chart-headers.ts $T/tier.json /Volumes/SSD/iidxbms $T/charts.json` — 모음 폴더의 모든 차트 헤더(`#TITLE`·`#PLAYLEVEL`·`#DIFFICULTY`·`#PLAYER`, UTF-8 이 아니면 Shift_JIS)와 md5.
3. `bun 3-match-tier.ts $T/tier.json $T/charts.json $T/join.json` — 23(copula)까지의 서열 항목을 로컬 SP 차트와 맞춘다. 3 의 `ROOT` 는 모음 폴더 경로로 고정돼 있다.
4. `curl -s https://bms.hyuns.uk/tables/blocked/data.json -o $T/blocked.json` 뒤 `bun 4-build-sp12.ts $T/join.json $T/blocked.json ../web/data/chart-blocklist-sp12.json`.

매칭 규칙(3):

- 난이도: 파일 이름·제목의 `[SP ANOTHER]`·`[01 SP NORMAL]`, `[Light7]`·`[7key]`·`[Another7]`(N·H·A, `14` 는 DP), `[1A SIRIUS]`·`[DH SIRIUS]`·`[02 1H]`, `(2 A)`·`(4 DA)`, `a7`·`a14`·`leg (1)`, 그다음 `#DIFFICULTY`. 이름·제목에 LEGGENDARIA 가 있으면 L.
- SP/DP: 이름의 표기, `#PLAYER`(1 SP, 3 DP), 둘 다 없으면 2P 레인 채널(`#xxx21`~`29`)이 있으면 DP.
- 제목: 난이도 괄호를 지우고 NFKC·악센트 제거·문자와 숫자만 남긴 키. 같은 곡 폴더의 모든 차트 제목과 폴더 이름의 마지막 ` - ` 뒤 곡명을 그 폴더 차트들의 후보 키로 쓴다(`a7.bme` 만 로마자 제목인 경우). 악센트 글자를 통째로 뺀 키도 본다(`Mis?rables`).
- 결과 = 서열표와 맞은 SP 차트 ∪ `#PLAYLEVEL 12` 인 SP 차트. 제목은 맞은 것은 서열표 제목, 아니면 헤더 제목에 `[난이도]`.
