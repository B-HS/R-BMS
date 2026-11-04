import json, os, sys, datetime, collections
repo = '/Users/hyunseokbyun/development/R-BMS'
home = os.path.expanduser('~')
out = {}
ins = json.load(open(f'{repo}/web/data/tables/insane1/data.json'))
out['insane'] = {e['md5'].lower(): e['level'] for e in ins}
for key, fn in (('st', 'b52c506d2e68f6f4.json'), ('sl', 'af3e0990a0df5bec.json')):
    d = json.load(open(f'{home}/.config/rbms/tables/{fn}'))
    out[key] = {e['md5'].lower(): e['level'] for e in d['entries']}
    print(key, d['name'], d['symbol'], d['source'], datetime.datetime.fromtimestamp(d['fetched_at'], datetime.timezone.utc).isoformat(), len(d['entries']), 'unique md5', len(out[key]), 'level_order', d['level_order'])
patterns = {f[:-5] for f in os.listdir(f'{S}/patterns') if f.endswith('.json') and not f.startswith('_')} if (S:=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))) else set()
for k, m in out.items():
    hit = [x for x in m if x in patterns]
    print(k, 'table md5', len(m), 'in 7K patterns', len(hit), 'by level', sorted(collections.Counter(m[x] for x in hit).items(), key=lambda kv: (len(kv[0]), kv[0])))
allt = set().union(*[set(m) for m in out.values()])
print('union in patterns', len([x for x in allt if x in patterns]), 'patterns not in any table', len(patterns - allt))
json.dump(out, open(f'{S}/tables.json', 'w'))
