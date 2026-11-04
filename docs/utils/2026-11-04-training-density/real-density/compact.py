import re, sys
S = '/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/eb1fe13e-cb79-4798-a6c5-a80ce4887bfa/scratchpad/d1/real-density'
lines = open(f'{S}/agg.md', encoding='utf8').read().split('\n')

def block(h3, h4=None, nth=0):
    i3 = [i for i, l in enumerate(lines) if l.startswith('### ') and l[4:].startswith(h3)]
    if not i3:
        raise SystemExit(f'no h3 {h3}')
    start = i3[0]
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith('### ')), len(lines))
    seg = lines[start + 1:end]
    if h4 is None:
        return seg
    hits = [i for i, l in enumerate(seg) if l.startswith('#### ') and l[5:].startswith(h4)]
    if not hits:
        raise SystemExit(f'no h4 {h4} in {h3}')
    s = hits[nth]
    e = next((i for i in range(s + 1, len(seg)) if seg[i].startswith('#### ')), len(seg))
    return seg[s + 1:e]

def table(seg):
    rows = [l for l in seg if l.startswith('|')]
    return [[c.strip() for c in r.strip().strip('|').split('|')] for r in rows]

def strip_iqr(cell):
    return re.sub(r'\s*\[[^\]]*\]', '', cell)

def render(rows, keep=None, iqr=False, rename=None, drop_sep=True):
    head = rows[0]
    idx = list(range(len(head))) if keep is None else [0] + [next(i for i, h in enumerate(head) if h.startswith(k)) for k in keep]
    out = []
    for r_i, r in enumerate(rows):
        if r_i == 1:
            out.append('| ' + ' | '.join(['---'] + ['---:'] * (len(idx) - 1)) + ' |')
            continue
        cells = [r[i] for i in idx]
        if not iqr and r_i > 1:
            cells = [cells[0]] + [strip_iqr(c) for c in cells[1:]]
        if rename and r_i == 0:
            cells = [cells[0]] + [rename.get(c, c) for c in cells[1:]]
        out.append('| ' + ' | '.join(cells) + ' |')
    return '\n'.join(out)

KEEP8 = ['발광 ★1~5', '발광 ★6~10', '발광 ★11~15', '발광 ★16~20', '발광 ★21~25', 'Stella 전체', 'Satellite 전체', '표 밖']
REN = {'발광 ★1~5': '★1~5', '발광 ★6~10': '★6~10', '발광 ★11~15': '★11~15', '발광 ★16~20': '★16~20', '발광 ★21~25': '★21~25', 'Stella 전체(st0~12)': 'Stella', 'Satellite 전체(sl0~12)': 'Satellite', '표 밖(3개 표 어디에도 없음)': '표 밖', '표 3종 합침': '표 3종'}
sections = []
def add(title, text):
    sections.append(f'=== {title}\n{text}\n')

# 3.1
t = table(block('3.1 곡 단위', '발광 표 대역'))
add('3.1 insane', render(t, KEEP8[:5], rename=REN))
t = table(block('3.1 곡 단위', 'Stella·Satellite 대역'))
add('3.1 stsl', render(t, rename={}))
t = table(block('3.1 곡 단위', '전체 7,848곡'))
add('3.1 p90bins', render(t))
add('3.1b', render(table(block('3.1b')), rename=REN))
# 3.4
t = table(block('3.4 가장 어려운 1분', '발광 표 대역'))
add('3.4 insane', render(t, KEEP8[:5], rename=REN))
t = table(block('3.4 가장 어려운 1분', 'Stella·Satellite 대역'))
add('3.4 stsl', render(t))
t = table(block('3.4 가장 어려운 1분', '전체 곡을'))
add('3.4 p90bins', render(t))
# 3.3
add('3.3 peak', render(table(block('3.3 고점 지속', '최대 w초')), rename=REN))
# 3.2 flow
add('3.2 flow', render(table(block('3.2 흐름 모양 스칼라', '흐름 지표')), rename=REN))
for h in ['고점 구간(r>=0.8', '휴식(r<0.5', '깊은 휴식', '밀도 파동 간격(돌출 >=0.25', '밀도 파동 간격(돌출 >=0.5', '밀도 파동 돌출']:
    t = table(block('3.2 흐름 모양 스칼라', h))
    add('3.2 pooled ' + h, '\n'.join(['| ' + ' | '.join(r) + ' |' if i != 1 else '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |' for i, r in enumerate(t)]))
# curves
for h in ['K=10 중앙값 곡선', 'K=20 중앙값 곡선', 'K=10 최고 구간 대비', 'K=10 p25~p75', '최고 구간(K=10 중 가장']:
    add('3.2 curve ' + h, '\n'.join(block('3.2 곡 전체', h)))
for h in ['발광 표 레벨별', 'Stella 레벨별', 'Satellite 레벨별']:
    add('levels ' + h, '\n'.join(block('3.2 곡 전체', h)))
# bpm, tags
add('3.5 bpm', '\n'.join(block('3.5')))
add('3.6 tags', '\n'.join(block('3.6', '고점 노트') ) )
add('3.6 tags main', '\n'.join(l for l in block('3.6') if l.startswith('|') and False))
open(f'{S}/compact.txt', 'w', encoding='utf8').write('\n'.join(sections))
print(len('\n'.join(sections)))
