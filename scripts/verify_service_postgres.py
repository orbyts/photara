#!/usr/bin/env python3
"""Provision, prove and stop an isolated PostgreSQL cluster; never reads deployed DB configuration."""
from pathlib import Path
import argparse, ast, hashlib, json, os, re, shutil, subprocess, tempfile

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--inventory', type=Path, help='Write the measured schema inventory here after success')
args = p.parse_args()
base = Path(tempfile.mkdtemp(prefix='photara-cxt3c-', dir='/private/tmp'))
base.chmod(0o700)
socket = base / 'socket'
socket.mkdir(mode=0o700)
log = base / 'proof.log'
started = False
pg_ctl = shutil.which('pg_ctl')
assert pg_ctl and shutil.which('initdb') and shutil.which('psql'), 'PostgreSQL tools must be installed'
env = {k:v for k,v in os.environ.items() if not k.startswith('PG') and k not in ('DATABASE_URL','PHOTARA_TEST_MIGRATOR_URL')}
env['CARGO_NET_OFFLINE'] = 'true'
env['PHOTARA_TEST_MIGRATOR_URL'] = f'postgresql://photara_test_migrator@localhost/photara_cxt3c?host={socket}&port=55439'

def run(cmd):
    result = subprocess.run(cmd, cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    with log.open('a') as f: f.write(result.stdout)
    if result.returncode:
        print(result.stdout)
        raise RuntimeError(f'{cmd[0]} failed ({result.returncode}); see {log}')
    return result.stdout

psql = ['psql','-X','-h',str(socket),'-p','55439','-U','photara_test_admin','-v','ON_ERROR_STOP=1']
try:
    run(['initdb','-D',str(base/'data'),'-U','photara_test_admin','-A','trust','--no-locale','-E','UTF8'])
    run([pg_ctl,'-D',str(base/'data'),'-l',str(base/'postgres.log'),'-o',f"-k {socket} -h '' -p 55439 -c timezone=UTC",'-w','start'])
    started = True
    roles = base/'roles.sql'
    roles.write_text('''CREATE ROLE photara_owner NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_api NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_control NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_auth_read NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_test_migrator LOGIN NOSUPERUSER NOBYPASSRLS IN ROLE photara_owner;
CREATE ROLE photara_test_api LOGIN NOSUPERUSER NOBYPASSRLS IN ROLE photara_api;
CREATE ROLE photara_test_control LOGIN NOSUPERUSER NOBYPASSRLS IN ROLE photara_control;
CREATE ROLE photara_test_auth LOGIN NOSUPERUSER NOBYPASSRLS IN ROLE photara_auth_read;
CREATE DATABASE photara_cxt3c OWNER photara_owner;
''')
    run(psql+['-d','postgres','-f',str(roles)])
    output=run(['cargo','test','--offline','-p','photara-service','--lib','postgres_','--','--ignored','--nocapture','--test-threads=1'])
    print(output)
    def query(sql): return json.loads(run(psql+['-d','photara_cxt3c','-At','-c',sql]))
    report={
        'server':query("SELECT to_json(current_setting('server_version'))"),
        'schema':query("SELECT row_to_json(m) FROM photara.schema_metadata m"),
        'roles':query("SELECT json_agg(r ORDER BY rolname) FROM (SELECT rolname,rolsuper,rolbypassrls,rolcanlogin,rolcreaterole FROM pg_roles WHERE rolname LIKE 'photara_%') r"),
        'tables':query("SELECT json_agg(r ORDER BY schema,name) FROM (SELECT n.nspname AS schema,c.relname AS name,c.relrowsecurity AS rls,c.relforcerowsecurity AS forced,pg_get_userbyid(c.relowner) AS owner FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN ('photara','photara_identity','photara_private') AND c.relkind='r') r"),
        'columns':query("SELECT json_agg(r ORDER BY table_schema,table_name,ordinal_position) FROM (SELECT table_schema,table_name,ordinal_position,column_name,data_type,is_nullable FROM information_schema.columns WHERE table_schema IN ('photara','photara_identity','photara_private')) r"),
        'constraints':query("SELECT json_agg(r ORDER BY schema,relation,name) FROM (SELECT n.nspname AS schema,c.relname AS relation,k.conname AS name,k.contype AS kind,pg_get_constraintdef(k.oid) AS definition FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN ('photara','photara_identity','photara_private')) r"),
        'policies':query("SELECT json_agg(r ORDER BY schemaname,tablename,policyname) FROM (SELECT * FROM pg_policies WHERE schemaname IN ('photara','photara_identity','photara_private')) r"),
        'functions':query("SELECT json_agg(r ORDER BY name,arguments) FROM (SELECT n.nspname||'.'||p.proname AS name,pg_get_function_identity_arguments(p.oid) AS arguments,p.prosecdef AS security_definer,p.proconfig AS settings,pg_get_userbyid(p.proowner) AS owner FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN ('photara','photara_identity','photara_private')) r"),
        'triggers':query("SELECT json_agg(r ORDER BY schema,relation,name) FROM (SELECT n.nspname AS schema,c.relname AS relation,t.tgname AS name,pg_get_triggerdef(t.oid) AS definition FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE NOT t.tgisinternal AND n.nspname IN ('photara','photara_identity','photara_private')) r"),
        'indexes':query("SELECT json_agg(r ORDER BY schemaname,tablename,indexname) FROM (SELECT schemaname,tablename,indexname,indexdef FROM pg_indexes WHERE schemaname IN ('photara','photara_identity','photara_private')) r"),
        'ledger':query("SELECT json_agg(r ORDER BY version) FROM (SELECT version,description,success,encode(checksum,'hex') AS sha384 FROM public._sqlx_migrations) r"),
    }
    # Reuse just the repository's lexical scanner, without running its proposal-report entry point.
    source=ast.parse((ROOT/'scripts/verify_generation_two_schema.py').read_text())
    selected=ast.Module(body=[n for n in source.body if isinstance(n,ast.FunctionDef) and n.name in ('lex','statements','clean')],type_ignores=[])
    scanner={'re':re}
    exec(compile(selected,'service-inventory-scanner','exec'),scanner)
    report['migrations']=[]
    for path in sorted((ROOT/'crates/photara-service/migrations/postgres').glob('*.sql')):
        data=path.read_bytes()
        report['migrations'].append({'path':str(path.relative_to(ROOT)),'sha256':hashlib.sha256(data).hexdigest(),'statements':len(scanner['statements'](data.decode()))})
    # Runtime timestamps are intentionally excluded from the stable schema artifact.
    report['schema'].pop('created_at',None)
    report['counts']={k:len(report[k]) for k in ('tables','columns','constraints','policies','functions','triggers','indexes','ledger')}
    report['counts']['statements']=sum(m['statements'] for m in report['migrations'])
    assert report['counts']['tables']==55
    assert all(t['owner']=='photara_owner' for t in report['tables'])
    assert all(not r['rolsuper'] and not r['rolbypassrls'] for r in report['roles'] if r['rolname']!='photara_test_admin')
    if args.inventory:
        target=args.inventory.resolve()
        target.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
        print(f'Measured inventory: {target}')
    print(json.dumps(report['counts'],sort_keys=True))
finally:
    if started:
        run([pg_ctl,'-D',str(base/'data'),'-m','fast','-w','stop'])
    print(f'Disposable cluster stopped. Evidence: {log}')
