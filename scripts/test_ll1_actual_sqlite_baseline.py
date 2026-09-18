#!/usr/bin/env python3
"""Partial LL1 actual SQLite baseline: memory-only; no user DB/path arguments.

Loads unchanged generation-two SQL and tests actual guards. No retirement permit,
trigger bypass, FK rewrite, production migration installer or package access.
"""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

sys.dont_write_bytecode = True
from audit_ll1_schema_inventory import inventory, lex, statements

ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = ROOT / 'crates/photara-library/migrations/generation_two'


def bid(n):
    return n.to_bytes(16, 'big')


def literal(value):
    return "X'" + value.hex() + "'"


def install(db):
    hashes = {}
    declarations = {}
    for path in sorted(MIGRATIONS.glob('*.sql')):
        raw = path.read_bytes()
        hashes[path.name] = hashlib.sha256(raw).hexdigest()
        for sql, _ in statements(raw.decode(), True):
            if sql.startswith(('CREATE TABLE ', 'CREATE TRIGGER ')):
                declarations[sql.split()[2]] = sql.rstrip(' ;')
        db.executescript(raw.decode())
    return hashes, declarations


def catalog(db, declarations):
    source = inventory('sqlite')
    tables = {row[0] for row in db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'")}
    assert tables == set(source['tables'])
    triggers = {(row[0],row[1]) for row in db.execute("SELECT tbl_name,name FROM sqlite_schema WHERE type='trigger'")}
    assert triggers == {(row['table'],row['name']) for row in source['triggers']}
    for name, sql in db.execute("SELECT name,sql FROM sqlite_schema WHERE type IN ('table','trigger') AND name NOT LIKE 'sqlite_%'"):
        assert ' '.join(v for v,_ in lex(sql)).rstrip(' ;') == declarations[name], name
    observed = []
    for table in tables:
        groups = defaultdict(list)
        for row in db.execute(f'PRAGMA foreign_key_list("{table}")'):
            groups[row[0]].append(row)
        for rows in groups.values():
            rows.sort(key=lambda row: row[1])
            parent = rows[0][2]
            parent_columns = tuple(row[4] for row in rows)
            if all(column is None for column in parent_columns):
                parent_columns = tuple(source['tables'][parent]['primary_key'])
            observed.append((table,tuple(row[3] for row in rows),parent,parent_columns,rows[0][6]))
    expected = [(e['child'],tuple(e['columns']),e['parent'],tuple(e['parent_columns']),e['delete']) for e in source['fks']]
    assert sorted(observed) == sorted(expected)
    assert db.execute('PRAGMA foreign_keys').fetchone() == (1,)
    assert db.execute('PRAGMA foreign_key_check').fetchall() == []
    return source,dict(tables=len(tables),foreign_keys=len(observed),triggers=len(triggers),fingerprint=source['fingerprint'])


def seed(db):
    database_id = bid(42)
    library = hashlib.sha256(b'photara.default-library.v2'+database_id).digest()[:16]
    principal = hashlib.sha256(b'photara.local-principal.v2'+database_id).digest()[:16]
    target = literal(library)
    other = literal(bid(2))
    device = literal(bid(10))
    project = literal(bid(20))
    digest = literal(hashlib.sha256(b'{}').digest())
    db.executescript(f"""
BEGIN;
INSERT INTO schema_metadata VALUES(1,'photara.local.g2',{literal(database_id)},1,5,5,'photara.canonical-json.v1',0);
INSERT INTO normalization_policies VALUES(1,'16.0.0',{digest},'disposable fixture');
INSERT INTO local_device VALUES(1,{device},'Fixture device',0);
INSERT INTO libraries VALUES({target},'My Library','active',1,0,0,NULL,1,'{{}}'),({other},'Unrelated','active',1,0,0,NULL,1,'{{}}');
INSERT INTO library_contract_state VALUES({target},1,'local-only',{literal(principal)},1,1,1,0,0),({other},1,'local-only',{literal(principal)},1,1,1,0,0);
INSERT INTO people(person_id,library_id,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,display_name,sort_key) VALUES({literal(bid(11))},{target},1,1,0,0,'tombstoned',0,'Tombstone','tombstone');
INSERT INTO project_ownership VALUES({project},{target},'closed',NULL,NULL,NULL,'fixture',1,1,0,0);
INSERT INTO project_catalog VALUES({target},{project},'hidden',1,0,0,NULL,NULL);
INSERT INTO project_locators VALUES({literal(bid(21))},{target},{project},NULL,NULL,1,0,0,'retired',0);
INSERT INTO storage_roots VALUES({literal(bid(30))},{target},'Retired','retired','fixture',1,0,0,'tombstoned',0);
INSERT INTO storage_slots VALUES({literal(bid(31))},{target},'first','First',{literal(bid(30))},'tombstoned',0,1,1,0,0),({literal(bid(32))},{target},'second','Second',{literal(bid(30))},'tombstoned',0,1,1,0,0);
INSERT INTO storage_slot_names VALUES({target},'first',{literal(bid(31))},0),({target},'second',{literal(bid(32))},0);
INSERT INTO library_variables VALUES({literal(bid(41))},{target},'fixture','first','First','','fixture',1,'fixture',1,NULL,'[]','ordinary','portable','{{}}','tombstoned',0,1,1,0,0),({literal(bid(43))},{target},'fixture','second','Second','','fixture',1,'fixture',1,NULL,'[]','ordinary','portable','{{}}','tombstoned',0,1,1,0,0);
INSERT INTO library_variable_names VALUES({target},'first',{literal(bid(41))},0),({target},'second',{literal(bid(43))},0);
INSERT INTO device_root_bindings VALUES({device},{target},{literal(bid(30))},'bookmark',NULL,{literal(bid(33))},1,0);
INSERT INTO device_context_snapshots VALUES({literal(bid(50))},{device},{project},{literal(bid(51))},X'7b7d',{digest},{digest},0);
INSERT INTO legacy_external_resource_resolutions VALUES({device},{project},{literal(bid(52))},{digest},{literal(bid(53))},{literal(bid(54))},{literal(bid(55))},{digest},0);
COMMIT;
""")
    assert db.execute('PRAGMA foreign_key_check').fetchall() == []
    return target,other,project


def snapshot(db, tables):
    # repr preserves BLOBs/NULLs exactly; ordering is deterministic for fixture rows.
    return {table:sorted(repr(row) for row in db.execute(f'SELECT * FROM "{table}"')) for table in tables}


def run(db, source, target, other, project):
    initial = snapshot(db,source['tables'])
    outcomes=[]
    def case(name, sql, expected=None):
        db.execute('BEGIN')
        message=None
        changed=False
        try:
            for statement in sql:
                db.execute(statement)
            changed=snapshot(db,source['tables']) != initial
            assert db.execute('PRAGMA foreign_key_check').fetchall() == []
        except sqlite3.DatabaseError as error:
            message=str(error)
        finally:
            db.execute('ROLLBACK')
        if expected is not None:
            assert message is not None and expected in message, (name,expected,message)
        else:
            assert message is None, (name,message)
            assert changed, (name,'expected a real rolled-back change')
        assert snapshot(db,source['tables']) == initial,name
        assert db.execute('PRAGMA foreign_key_check').fetchall() == []
        outcomes.append(name)
        print(json.dumps({'case':name,'result':'pass','observed':message or 'rollback'}),flush=True)
    for table in ('libraries','library_contract_state','people','project_catalog','project_locators','project_ownership','storage_roots','storage_slots','storage_slot_names','library_variables','library_variable_names'):
        expected='hard_delete_not_supported' if table in ('libraries','people','project_catalog','project_locators','storage_roots') else 'd19_retained_record'
        case('ordinary_delete_'+table,[f'DELETE FROM {table} WHERE library_id={target}'],expected)
    # This disposable local binding has no baseline no-delete trigger. Do not
    # misrepresent actual leaf cleanup as an aggregate-retirement authorization.
    case('ordinary_binding_delete_rollback',[f'DELETE FROM device_root_bindings WHERE library_id={target}'])
    for table in ('device_context_snapshots','legacy_external_resource_resolutions'):
        case('ordinary_delete_'+table,[f'DELETE FROM {table} WHERE project_id={project}'],'d19_retained_record')
    case('onboarding_session_retained',['DELETE FROM onboarding_session'],'onboarding_retained')
    case('onboarding_generation_stale',['UPDATE onboarding_session SET generation=generation'],'onboarding_generation')
    case('library_revision_stale',[f"UPDATE libraries SET display_name='stale',local_revision=local_revision WHERE library_id={target}"],'identity_or_revision_conflict')
    case('controller_generation_jump',[f'UPDATE library_contract_state SET authorization_generation=authorization_generation+2,local_revision=local_revision+1 WHERE library_id={target}'],'d19_generation_step')
    case('closed_project_cannot_reopen',[f"UPDATE project_ownership SET registration_state='pending',local_revision=local_revision+1 WHERE library_id={target}"],'d19_registration_lifecycle')
    case('ordinary_update_rollback',[f"UPDATE libraries SET display_name='temporary',local_revision=local_revision+1 WHERE library_id={target}"])
    for parent,names in (('storage_slots','storage_slot_names'),('library_variables','library_variable_names')):
        for first,second in ((parent,names),(names,parent)):
            case('dml_cte_'+first,[f'WITH a AS (DELETE FROM {first} WHERE library_id={target} RETURNING library_id) DELETE FROM {second} WHERE library_id={target}'],'syntax error')
    # Verify typed logical joins can identify the seeded non-FK rows. Not codec proof.
    for table in ('device_context_snapshots','legacy_external_resource_resolutions'):
        assert db.execute(f'SELECT count(*) FROM {table} x JOIN project_ownership p USING(project_id) WHERE p.library_id={target}').fetchone() == (1,)
        assert db.execute(f'SELECT count(*) FROM {table} x JOIN project_ownership p USING(project_id) WHERE p.library_id={other}').fetchone() == (0,)
    return outcomes


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    if not __debug__:
        raise RuntimeError('Assertions required')
    db=sqlite3.connect(':memory:',isolation_level=None)
    try:
        db.execute('PRAGMA foreign_keys=ON')
        hashes,declarations=install(db)
        source,measured=catalog(db,declarations)
        target,other,project=seed(db)
        print(json.dumps({'version':sqlite3.sqlite_version,'catalog':measured}),flush=True)
        cases=run(db,source,target,other,project)
        assert hashes=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in MIGRATIONS.glob('*.sql')}
        print(json.dumps({'unchanged_migration_sha256':hashes}),flush=True)
        assert db.execute("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name IN ('lifecycle_receipts','removed_library_identities','account_inventory_positions','retirement_work')").fetchone()==(0,)
        print(json.dumps({'stage':'partial actual SQLite baseline only','passed':len(cases),'logical_join_checks':4,'catalog':measured,'retirement_relations':0}),flush=True)
    finally:
        db.close()
    print(json.dumps({'database':'in-memory connection closed; no database file created'}),flush=True)


if __name__=='__main__':
    main()
