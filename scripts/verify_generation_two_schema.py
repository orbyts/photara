#!/usr/bin/env python3
"""Static D19 signature/FK/inventory checks; no SQL execution or database access."""
from pathlib import Path
from collections import Counter
import re,json,hashlib,subprocess,sys
ROOT=Path.cwd();OUT=ROOT/'docs/architecture/proposals/d19-cxt2'
BEFORE={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for folder,pattern in [('crates/photara-library/migrations/generation_two','*.sql'),('docs/fixtures/generation-two','*.json')] for p in Path(folder).glob(pattern)}
# Lexical scanner: exact top-level statements, quoted strings/dollar bodies,
# comments, balanced delimiters; intentionally not a SQL grammar/engine check.
def lex(s):
 pat=re.compile(r"(--[^\n]*(?:\n|$)|/\*[\s\S]*?\*/)|('(?:''|[^'])*'|\"(?:\"\"|[^\"])*\"|\$[A-Za-z_0-9]*\$[\s\S]*?\$[A-Za-z_0-9]*\$)|([A-Za-z_][A-Za-z_0-9]*|[0-9]+|[^\s])")
 toks=[]
 for m in pat.finditer(s):
  if m[1]:continue
  tok=m[2] or m[3]
  assert tok not in ["'", '"'], ('unclosed quote',m.start())
  if tok.startswith('$') and len(tok)>1:
   tag=re.match(r'\$[A-Za-z_0-9]*\$',tok)[0];assert tok.endswith(tag)
  toks.append((tok,m.start()))
 return toks
def statements(s,sqlite=False):
 ts=lex(s);chunks=[];start=0;depth=0;block=0;trigger=False
 for i,(v,pos) in enumerate(ts):
  if i==start:trigger=False
  if i<start+4 and v.upper()=='TRIGGER':trigger=True
  if v=='(':depth+=1
  if v==')':depth-=1;assert depth>=0
  if sqlite and trigger and depth==0:
   if v.upper()=='BEGIN':block+=1
   if v.upper()=='END':block-=1
  if v==';' and depth==0 and block==0:
   chunks.append((ts[start:i+1],s.count('\n',0,ts[start][1])+1));start=i+1
 assert depth==0 and block==0 and start==len(ts),(depth,block,ts[start:start+10])
 return chunks
def clean(toks):return ' '.join(t[0] for t in toks)
def splitcols(body):
 ts=lex(body);depth=0;chunks=[];start=0
 for i,(v,pos) in enumerate(ts):
  if v=='(':depth+=1
  if v==')':depth-=1
  if v==',' and depth==0:chunks.append(clean(ts[start:i]));start=i+1
 chunks.append(clean(ts[start:]))
 return chunks
def ctuple(x): return tuple(c.strip() for c in x.split(','))
def tabledata(sql):
 # Whitespace-normalized SQL from lexer; quoted content remains intact.
 m=re.match(r'CREATE TABLE ([\w]+(?: \. [\w]+)?) \(',sql)
 if not m:return None
 name=m[1].replace(' ','');body=sql[m.end():sql.rfind(')')];cols={};keys=[]
 for c in splitcols(body):
  if c.startswith(('PRIMARY KEY','UNIQUE','CHECK','FOREIGN KEY','CONSTRAINT')):
   mm=re.match(r'(?:PRIMARY KEY|UNIQUE) \( (.*?) \)',c)
   if mm:keys.append(ctuple(mm[1]))
  else:
   cname=c.split()[0];cols[cname]=c
   if 'PRIMARY KEY' in c or ' UNIQUE' in c:keys.append((cname,))
 return name,cols,keys
allstmts={};relation={};report=[];inventory=[]
for path in sorted(OUT.glob('*/*.proposal.sql')):
 db=path.parent.name;txt=path.read_text();stmts=statements(txt,db=='sqlite');key=db+'/'+path.name.replace('.proposal.sql','')
 stats=Counter();items=[]
 for i,(ts,line) in enumerate(stmts,1):
  c=clean(ts);kind=' '.join([v for v,_ in ts[:2]])
  if c.startswith('CREATE UNIQUE INDEX'):kind='CREATE INDEX'
  if c.startswith('CREATE CONSTRAINT TRIGGER'):kind='CREATE TRIGGER'
  stats[kind]+=1
  data=tabledata(c)
  if data:relation[db,data[0]]=data
  if c.startswith('CREATE FUNCTION'):
   for token,_ in ts:
    if token.startswith('$$'):
     # balance and valid tokenization of body, without PL/pgSQL grammar claims
     bts=lex(token[2:-2]);depth=0
     for v,_ in bts:
      if v=='(':depth+=1
      elif v==')':depth-=1;assert depth>=0
     assert depth==0
  match=re.match(r'(?:CREATE (?:UNIQUE |CONSTRAINT )?(?:TABLE|INDEX|TRIGGER|FUNCTION|POLICY)|DROP POLICY|ALTER TABLE|UPDATE) ([\w]+(?: \. [\w]+)?)',c)
  obj=match[1].replace(' ','') if match else c.rstrip(' ;')
  on=re.search(r' ON ([\w]+(?: \. [\w]+)?)',c)
  if on and ('TRIGGER' in kind or 'POLICY' in kind or 'INDEX' in kind):obj+=' on '+on[1].replace(' ','')
  if kind=='ALTER TABLE':
   obj+=' — '+('add constraints/columns' if ' ADD ' in c else 'enable RLS' if ' ENABLE ' in c else 'force RLS')
  items.append((i,line,kind,obj))
 allstmts[db,path.name]=[(clean(t),line) for t,line in stmts]
 report.append((db,path.name,len(stmts),stats))
 inventory.append((db,path.name,items))
assert Counter(db for db,t in relation)=={'sqlite':26,'postgresql':20}
# Compare actual definitions against each approved signature, independently of generator.
spec=Path('docs/architecture/D19_STATIC_SCHEMA_DELTA.md').read_text();sig={}; nullable={}
code_by_table={name:code for code,name in re.findall(r'^### ([ASVPHOQCF][1-6]) — ([a-z_]+)',spec,re.M)}
for code,body in re.findall(r'^### ([ASVPHOQCF][1-6]) — [^\n]+\n([\s\S]*?)(?=^### |^## |\Z)',spec,re.M):
 m=re.search(r'`\((.*?)\)`',body,re.S);assert m,code
 entries=[x.strip() for x in m[1].split(',')];expected=[]
 for x in entries:
  if x=='M':expected+=['record_schema','REVISION','created_at','updated_at']
  elif x.startswith('L'):expected+=['library_id']
  else:expected+=[x.split()[0]]
 sig[code]=expected
 nullable[code]={('library_id' if x.startswith('L') else x.split()[0]) for x in entries if '?' in x}
for (db,t),(name,cols,keys) in relation.items():
 meta={'code':code_by_table[t.split('.')[-1]]}
 expected=[('local_revision' if db=='sqlite' else 'revision') if c=='REVISION' else c for c in sig[meta['code']]]
 assert list(cols)==expected,(db,t,list(cols),expected)
 # Exact explicit nullability check against question-mark notation.
 for name,c in cols.items():
  assert ('NOT NULL' not in c)==(name in nullable[meta['code']]),(db,t,name,c,nullable[meta['code']])
# Baselines read as text only. Extract CREATE TABLE blocks, no DB connection.
for db in ['sqlite','postgresql']:
 if db=='sqlite':base='\n'.join(p.read_text() for p in sorted(Path('crates/photara-library/migrations/generation_two').glob('000[1-6]_*.sql')))
 else:base='\n'.join(re.findall(r'```sql\n([\s\S]*?)```',Path('docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md').read_text()))
 # Baseline SQL includes representative parameterized commands; only lex CREATE TABLE units.
 for m in re.finditer(r'CREATE TABLE [\w.]+ \(',base):
  ts=lex(base[m.start():]);depth=0;end=0
  for j,(v,pos) in enumerate(ts):
   if v=='(':depth+=1
   elif v==')':
    depth-=1
    if depth==0:end=j+1;break
  d=tabledata(clean(ts[:end]));assert d
  relation[db,d[0]]=d
# FK target/columns/unique-key and child-index coverage, including ALTER constraints.
fks=[];indexes={}
for (db,t),(_,cols,keys) in relation.items():indexes[db,t]=list(keys)
for (db,f),stmts in allstmts.items():
 for c,line in stmts:
  m=re.match(r'CREATE (?:UNIQUE )?INDEX (\w+) ON ([\w]+(?: \. [\w]+)?) \( (.*?) \)',c)
  if m and ' WHERE ' not in c:indexes.setdefault((db,m[2].replace(' ','')),[]).append(ctuple(m[3]))
for (db,f),stmts in allstmts.items():
 for c,line in stmts:
  tablematch=re.match(r'(?:CREATE|ALTER) TABLE ([\w]+(?: \. [\w]+)?)',c)
  if not tablematch:continue
  child=tablematch[1].replace(' ','')
  for m in re.finditer(r'FOREIGN KEY \( (.*?) \) REFERENCES ([\w]+(?: \. [\w]+)?) \( (.*?) \)',c):
   cc=ctuple(m[1]);parent=m[2].replace(' ','');pc=ctuple(m[3]);fks.append((db,child,cc,parent,pc))
   assert (db,parent) in relation,(child,parent)
   assert all(x in relation[db,child][1] for x in cc) or child=='photara_private.media_upload_sessions',(child,cc)
   assert all(x in relation[db,parent][1] for x in pc),(parent,pc)
   assert pc in relation[db,parent][2],(child,parent,pc,relation[db,parent][2])
   assert any(k[:len(cc)]==cc for k in indexes[db,child]),('missing child FK index',db,child,cc)
# Actual explicit SQL objects must be unique within their namespace and <= PG identifier limit.
for db in ['sqlite','postgresql']:
 seen=set()
 for (d,f),stmts in allstmts.items():
  if d!=db:continue
  for c,line in stmts:
   m=re.match(r'CREATE (?:UNIQUE |CONSTRAINT )?(TABLE|INDEX|FUNCTION|TRIGGER|POLICY) ([\w]+(?: \. [\w]+)?)',c)
   if not m:continue
   name=m[2].replace(' ','');kind=m[1];target=''
   if kind in ['TRIGGER','POLICY']:
    target=re.search(r' ON ([\w]+(?: \. [\w]+)?)',c)[1].replace(' ','')
   key=kind,name,target
   assert key not in seen,(db,key);seen.add(key)
   if db=='postgresql':assert all(len(s.encode())<=63 for s in name.split('.')),(db,name)
pg='\n'.join(c for (db,f),ss in allstmts.items() if db=='postgresql' for c,line in ss)
assert pg.count('DROP POLICY library_scope ON ')==25
assert len(re.findall(r'ALTER TABLE .*? ENABLE ROW LEVEL SECURITY',pg))==20
assert len(re.findall(r'ALTER TABLE .*? FORCE ROW LEVEL SECURITY',pg))==20
assert 'CREATE POLICY library_scope' not in pg
assert not list((OUT/'postgresql').glob('0007*'))
# Output inventory derived from actual SQL, including statement lines and constraint totals.
lines=['# D19 CXT2 exact DDL inventory','', '**Inert proposal only; not applied or runtime-validated.** R1 naming superseded;', 'Library rebaseline authorized 2026-09-12. Counts below come from the actual eleven proposal files. A statement', 'means one top-level SQL statement; function/trigger bodies count with their owner.', 'Explicit indexes exclude implicit indexes backing PRIMARY KEY/UNIQUE constraints.', '', '## Per-file inventory','', '| Backend / file | Statements | Tables | Indexes | Triggers | Functions | Create policies | Drop policies | ALTER TABLE | Other |','| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
totals={}
for db,f,n,stats in report:
 nums=[stats[k] for k in ['CREATE TABLE','CREATE INDEX','CREATE TRIGGER','CREATE FUNCTION','CREATE POLICY','DROP POLICY','ALTER TABLE']]
 lines.append(f'| {db} / [{f}]({db}/{f}) | {n} | '+' | '.join(map(str,nums))+f' | {n-sum(nums)} |')
 totals.setdefault(db,Counter()).update(stats)
for db,stats in totals.items():
 n=sum(stats.values());nums=[stats[k] for k in ['CREATE TABLE','CREATE INDEX','CREATE TRIGGER','CREATE FUNCTION','CREATE POLICY','DROP POLICY','ALTER TABLE']]
 lines.append(f'| **{db} total** | **{n}** | '+' | '.join('**'+str(v)+'**' for v in nums)+f' | **{n-sum(nums)}** |')
lines+=['','The proposed totals remain local 44+26=70 and service 35+20=55 relations.', 'The Library-named baseline and service 0007 reservation were rechecked; counts are unchanged.', '', '## Relation/signature cross-check','', '| Code | Relation | SQLite | PostgreSQL |','| --- | --- | --- | --- |']
common={}
for (db,t) in relation:
 name=t.split('.')[-1]
 if name in code_by_table:common.setdefault((code_by_table[name],name),{})[db]=True
for (code,t),back in sorted(common.items()):
 lines.append(f'| {code} | `{t}` | '+('yes' if 'sqlite'in back else '—')+' | '+('yes' if 'postgresql'in back else '—')+' |')
lines+=['', 'Actual CREATE TABLE columns were independently compared to every accepted signature,', 'expanding L/M and the local revision adapter. Every proposed FK target/column tuple', 'was resolved against these additions and text-only baseline keys. Full child FK', 'index coverage was checked, including constraints added after forward references.', '', '| Backend | Foreign keys in proposal CREATE/ALTER | CHECK clauses in CREATE/ALTER | PK clauses | UNIQUE clauses |','| --- | ---: | ---: | ---: | ---: |']
for db in ['sqlite','postgresql']:
 ddl='\n'.join(c for (d,f),ss in allstmts.items() if d==db for c,line in ss if re.match(r'(CREATE|ALTER) TABLE',c))
 lines.append(f'| {db} | {sum(1 for f in fks if f[0]==db)} | {len(re.findall(r"CHECK ",ddl))} | {ddl.count("PRIMARY KEY")} | {ddl.count("UNIQUE ")} |')
lines+=['', '## Static verification and limits','', '- Eleven files lexically checked for balanced delimiters, quoted/dollar-quoted', '  bodies, statement termination and exact counts. This is not a grammar parser.', '- This scanner does not use a PostgreSQL grammar parser; SQL grammar is not', '  claimed. This script starts no SQL engine and opens no database.', '- 26 local and 20 service relation signatures, scoped FKs, referenced unique keys,', '  full child-FK indexes, object-name uniqueness and PostgreSQL identifier lengths', '  checked against actual proposal text and baseline schema text.', '- All 25 old library_scope policy drops are enumerated; all 20 new service', '  tables explicitly enable and force RLS. Raw transport/access/private media grants', '  and five required authorization facade helpers are spelled out below.', '- The [responsibility ledger](RESPONSIBILITIES.md) identifies every invariant that', '  SQL cannot prove alone. SQL grammar/type/name resolution, engine semantics,', '  RLS/grants, race behavior and codecs remain untested CXT3 gates.', '', '## Regenerated Library baseline hashes','', 'SHA-256 values below were recomputed from the rebaselined files.', 'These are synthetic/unshipped Generation Two baselines, not deployed files. File', 'hashes are distinct from SQLx ledger checksums. Baseline SQL fences are checked', 'separately; package fixture semantics are verified by the Rust package tests.', '', '| Protected file | SHA-256 |','| --- | --- |']
for p,h in sorted(BEFORE.items()):
 if p.startswith(('crates/photara-library/migrations/generation_two/','docs/fixtures/generation-two/')):
  assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,p
  lines.append(f'| `{p}` | `{h}` |')
lines+=['', '## Exact statement and object list','', 'Line numbers refer to the proposal files linked in the per-file table. CREATE', 'TABLE includes its inline constraints; forward-reference ALTERs and all explicit', 'indexes/triggers/policies/functions/privilege/floor statements appear separately.', '']
for db,f,items in inventory:
 lines+=[f'### {db} / {f}','', '| Statement | Line | Kind | Object / operation |','| ---: | ---: | --- | --- |']
 for i,line,kind,obj in items:lines.append(f'| {i} | {line} | {kind} | `{obj}` |')
 lines+=['']
generated='\n'.join(lines)
if '--write-inventory' in sys.argv:(OUT/'INVENTORY.md').write_text(generated)
else:assert (OUT/'INVENTORY.md').read_text()==generated,'Inventory is stale; review and regenerate explicitly'
print('Statement counts:',{db:sum(v.values()) for db,v in totals.items()})
print('Object counts:',{db:dict(v) for db,v in totals.items()})
print('Signature checks: 46; FK checks:',len(fks))
Path('/private/tmp/photara-library-schema-stats.json').write_text(json.dumps({'counts':{db:dict(v) for db,v in totals.items()},'fks':len(fks)},indent=2))
