#!/usr/bin/env python3
"""Read-only bounded SQL-source inventory. No SQL execution or file writes.

Lexically extracts the checked-in 79/59-table baseline. It is not a SQL parser,
effective-role evaluator, function-body dependency resolver or closure proof.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
DIRS = {
    "sqlite": "crates/photara-library/migrations/generation_two",
    "postgres": "crates/photara-service/migrations/postgres",
}
TOKEN = re.compile(r"(--[^\n]*(?:\n|$)|/\*[\s\S]*?\*/)|('(?:''|[^'])*'|\"(?:\"\"|[^\"])*\"|(?P<tag>\$[A-Za-z_0-9]*\$)[\s\S]*?(?P=tag))|([A-Za-z_][A-Za-z_0-9]*|[0-9]+|[^\s])")
NAME = r"[a-z_][a-z_0-9]*(?: \. [a-z_][a-z_0-9]*)?"


def lex(text):
    return [(m[2] or m[4], m.start()) for m in TOKEN.finditer(text) if not m[1]]


def statements(text, sqlite):
    tokens = lex(text)
    start = depth = block = 0
    for index, (token, _) in enumerate(tokens):
        prefix = [v.upper() for v, _ in tokens[start:start + 3]]
        trigger = prefix[:2] == ["CREATE", "TRIGGER"]
        if token == "(":
            depth += 1
        elif token == ")":
            depth -= 1
        if sqlite and trigger and depth == 0:
            if token.upper() in ("BEGIN", "CASE"):
                block += 1
            elif token.upper() == "END":
                block -= 1
        if token == ";" and depth == block == 0:
            chunk = tokens[start:index + 1]
            yield " ".join(v for v, _ in chunk), text.count("\n", 0, chunk[0][1]) + 1
            start = index + 1
    assert depth == block == 0 and start == len(tokens), "Unparsed SQL tail"


def columns(body):
    tokens = lex(body)
    start = depth = 0
    for index, (token, _) in enumerate(tokens):
        depth += (token == "(") - (token == ")")
        if token == "," and depth == 0:
            yield " ".join(v for v, _ in tokens[start:index])
            start = index + 1
    yield " ".join(v for v, _ in tokens[start:])


def names(text):
    return [s.strip() for s in text.split(",")]


def dispositions(engine):
    text = (ROOT / "docs/architecture/LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md").read_text()
    text = text.split("## Enumerated table dispositions", 1)[1].split("## Inventory,", 1)[0]
    text = text.split("### SQLite" if engine == "sqlite" else "### PostgreSQL", 1)[1]
    if engine == "sqlite":
        text = text.split("### PostgreSQL", 1)[0]
    result = {}
    for row in text.splitlines():
        if row.startswith("| `"):
            parts = row.split("|")
            for table in re.findall(r"`([^`]+)`", parts[1]):
                result[table] = parts[2].strip()
    return result


def inventory(engine):
    tables, fks, triggers, functions, policies, security = {}, [], [], {}, {}, []
    fingerprint = hashlib.sha256()
    declarations = Counter()
    for path in sorted((ROOT / DIRS[engine]).glob("*.sql")):
        raw = path.read_bytes()
        fingerprint.update(path.name.encode() + b"\0" + raw + b"\0")
        for sql, line in statements(raw.decode(), engine == "sqlite"):
            source = f"{path.relative_to(ROOT)}:{line}"
            table_match = re.match(rf"CREATE TABLE ({NAME}) \( (.*) \)(?: STRICT)? ;$", sql, re.S)
            alter = re.match(rf"ALTER TABLE ({NAME}) (.*)", sql, re.S)
            definitions = []
            child = None
            if table_match:
                child = table_match[1].replace(" ", "")
                definitions = list(columns(table_match[2]))
                primary = []
                for definition in definitions:
                    match = re.match(r"PRIMARY KEY \( (.*?) \)", definition)
                    if match:
                        primary = names(match[1])
                    elif " PRIMARY KEY" in definition and not definition.startswith("CONSTRAINT"):
                        primary = [definition.split()[0]]
                assert child not in tables, child
                tables[child] = {"source": source, "primary_key": primary, "rls": [], "columns": [d.split()[0] for d in definitions if not d.startswith(("PRIMARY", "FOREIGN", "UNIQUE", "CHECK", "CONSTRAINT"))]}
            elif alter and "REFERENCES" in sql:
                child = alter[1].replace(" ", "")
                definitions = list(columns(alter[2]))
            if alter:
                for part in columns(alter[2]):
                    added_column = re.match(r"ADD COLUMN ([a-z_][a-z_0-9]*) ", part)
                    if added_column:
                        tables[alter[1].replace(" ", "")]["columns"].append(added_column[1])
            if child:
                for definition in definitions:
                    match = re.search(rf"REFERENCES ({NAME})(?: \( ([^)]*?) \))?(.*)", definition, re.S)
                    if not match:
                        continue
                    column_match = re.search(r"FOREIGN KEY \( ([^)]*?) \)", definition)
                    action = re.search(r"ON DELETE (NO ACTION|RESTRICT|CASCADE|SET NULL|SET DEFAULT)", match[3])
                    fks.append({"id": f"E{len(fks)+1}", "child": child,
                                "columns": names(column_match[1]) if column_match else [definition.split()[0]],
                                "parent": match[1].replace(" ", ""), "parent_columns": names(match[2]) if match[2] else None,
                                "delete": action[1] if action else "NO ACTION",
                                "deferred": "DEFERRABLE INITIALLY DEFERRED" in match[3], "source": source})
            trigger = re.match(rf"CREATE (?:CONSTRAINT )?TRIGGER ({NAME}) (BEFORE|AFTER|INSTEAD OF) (.*?) ON ({NAME}) (.*)", sql, re.S)
            if trigger:
                function = re.search(rf"EXECUTE (?:FUNCTION|PROCEDURE) ({NAME})", trigger[5])
                triggers.append({"id": f"T{len(triggers)+1}", "name": trigger[1].replace(" ", ""), "table": trigger[4].replace(" ", ""), "events": trigger[3], "timing": trigger[2], "deferred": "DEFERRABLE INITIALLY DEFERRED" in trigger[5], "function": function[1].replace(" ", "") if function else None, "source": source})
            function = re.match(rf"CREATE (?:OR REPLACE )?FUNCTION ({NAME}) \(", sql)
            if function:
                key = function[1].replace(" ", "")
                old = functions.get(key, {})
                functions[key] = {"source": source, "security_definer": "SECURITY DEFINER" in sql, "prior_sources": old.get("prior_sources", []) + ([old["source"]] if old else [])}
            policy = re.match(rf"CREATE POLICY ({NAME}) ON ({NAME}) (.*)", sql, re.S)
            if policy:
                policies[policy[2].replace(" ", "") + ":" + policy[1]] = {"source": source, "sql": sql}
            drop = re.match(rf"DROP POLICY ({NAME}) ON ({NAME})", sql)
            if drop:
                del policies[drop[2].replace(" ", "") + ":" + drop[1]]
            if alter and "ROW LEVEL SECURITY" in sql:
                tables[alter[1].replace(" ", "")]["rls"].append(alter[2].rstrip(" ;"))
            if sql.startswith(("GRANT ", "REVOKE ", "ALTER DEFAULT PRIVILEGES ", "CREATE ROLE ", "ALTER ROLE ")):
                security.append({"source": source, "sql": sql})
            # Independent declaration/ref counts from lexical tokens, not strings.
            tokens = [v.upper() for v, _ in lex(sql) if not v.startswith(("'", '"', "$"))]
            declarations["references"] += tokens.count("REFERENCES")
            declarations["tables"] += tokens[:2] == ["CREATE", "TABLE"]
            declarations["triggers"] += tokens[:2] == ["CREATE", "TRIGGER"] or tokens[:3] == ["CREATE", "CONSTRAINT", "TRIGGER"]
    disposition = dispositions(engine)
    assert set(disposition) == set(tables), (set(disposition)-set(tables), set(tables)-set(disposition))
    for edge in fks:
        assert edge["child"] in tables and edge["parent"] in tables, edge
        if edge["parent_columns"] is None:
            edge["parent_columns"] = tables[edge["parent"]]["primary_key"]
        assert len(edge["columns"]) == len(edge["parent_columns"]), edge
        assert set(edge["columns"]) <= set(tables[edge["child"]]["columns"]), edge
        assert set(edge["parent_columns"]) <= set(tables[edge["parent"]]["columns"]), edge
    assert len(fks) == declarations["references"]
    assert len(tables) == declarations["tables"]
    assert len(triggers) == declarations["triggers"]
    assert (len(tables), len(fks), len(triggers)) == ((79, 137, 190) if engine == "sqlite" else (59, 114, 122)), (engine, len(tables), len(fks), len(triggers))
    for trigger in triggers:
        assert trigger["table"] in tables
        if trigger["function"]:
            assert trigger["function"] in functions, trigger
    root = "libraries" if engine == "sqlite" else "photara.libraries"
    graph = {table: set() for table in tables}
    for edge in fks:
        graph[edge["child"]].add(edge["parent"])
    def reachable(table):
        seen, todo = set(), list(graph[table])
        while todo:
            node = todo.pop()
            if node not in seen:
                seen.add(node)
                todo.extend(graph[node] - seen)
        return seen
    reach = {table: reachable(table) for table in tables}
    remaining = set(tables)
    cycles = []
    while remaining:
        table = min(remaining)
        component = {other for other in remaining if other == table or (other in reach[table] and table in reach[other])}
        remaining -= component
        if len(component) > 1 or table in graph[table]:
            cycles.append(sorted(component))
    for table, data in tables.items():
        data.update(disposition=disposition[table], library_fk_path=(table == root or root in reach[table]),
                    incoming=[e["id"] for e in fks if e["parent"] == table],
                    triggers=[t["id"] for t in triggers if t["table"] == table])
    return dict(engine=engine, fingerprint=fingerprint.hexdigest(), tables=tables, fks=fks, triggers=triggers, functions=functions, policies=policies, security=security, cycles=cycles)


def summary(data):
    return {"engine": data["engine"], "fingerprint": data["fingerprint"],
            "tables": len(data["tables"]), "foreign_keys": len(data["fks"]), "triggers": len(data["triggers"]),
            "delete_triggers": sum("DELETE" in t["events"] for t in data["triggers"]),
            "deferred_triggers": [t for t in data["triggers"] if t["deferred"]],
            "functions": len(data["functions"]), "policies": len(data["policies"]),
            "role_statements": len(data["security"]),
            "delete_or_detach_without_library_fk": [name for name, t in data["tables"].items() if not t["library_fk_path"] and not t["disposition"].startswith("R")],
            "cycles": data["cycles"]}


def link(source):
    path, line = source.rsplit(":", 1)
    return f"[{Path(path).name}:{line}](../../../{path})"


def markdown(all_data):
    out = ["# LL1 generated full-schema static inventory", "", "Generated by `python3 scripts/audit_ll1_schema_inventory.py --markdown`. Read-only SQL-source extraction; no database, role simulation or closure proof. E/T identifiers are local to each engine. Links identify the containing declaration's source line. Current policy/function replacements are resolved; grant/revoke statements retain source order rather than claiming effective privileges.", ""]
    for data in all_data:
        out += [f"## {data['engine']}", "", f"Source fingerprint: `{data['fingerprint']}`. {len(data['tables'])} tables; {len(data['fks'])} FK edges; {len(data['triggers'])} triggers.", "", "### Complete table coverage", "", "Disposition text comes from LL1's enumerated matrix. A Library FK path is structural reachability, **not** deletion authority. Incoming edge and trigger definitions follow below.", "", "| Table | Disposition | Library FK path | Incoming | Triggers | RLS source flags |", "| --- | --- | --- | --- | --- | --- |"]
        for name, table in data["tables"].items():
            out.append(f"| `{name}` | {table['disposition']} | {'yes' if table['library_fk_path'] else 'no'} | {', '.join(table['incoming']) or '—'} | {', '.join(table['triggers']) or '—'} | {', '.join(table['rls']) or '—'} |")
        out += ["", "### Every FK edge", "", "Unspecified action is shown as SQL default NO ACTION. `deferred` means INITIALLY DEFERRED, not that a RESTRICT action is deferable.", "", "| Edge | Referencing table/columns → referenced table/columns | Delete | Initially deferred | Source |", "| --- | --- | --- | --- | --- |"]
        for edge in data["fks"]:
            out.append(f"| {edge['id']} | `{edge['child']}({','.join(edge['columns'])})` → `{edge['parent']}({','.join(edge['parent_columns'])})` | {edge['delete']} | {edge['deferred']} | {link(edge['source'])} |")
        out += ["", "### Every trigger", "", "| Trigger | Table | Event/timing | Deferred | Function | Source |", "| --- | --- | --- | --- | --- | --- |"]
        for trigger in data["triggers"]:
            out.append(f"| {trigger['id']} `{trigger['name']}` | `{trigger['table']}` | {trigger['timing']} {trigger['events']} | {trigger['deferred']} | `{trigger['function'] or 'inline SQLite body'}` | {link(trigger['source'])} |")
        out += ["", "### Cyclic FK components", ""]
        out.extend("- " + ", ".join(f"`{t}`" for t in cycle) for cycle in data["cycles"])
        if data["functions"]:
            out += ["", "### Current function definitions and trigger consumers", "", "Function calls inside bodies are not resolved. SECURITY DEFINER here reports source syntax only, not effective runtime owner authority.", "", "| Function | Security definer | Trigger consumers | Source; superseded definitions |", "| --- | --- | --- | --- |"]
            for name, function in data["functions"].items():
                consumers = [t["id"] for t in data["triggers"] if t["function"] == name]
                out.append(f"| `{name}` | {function['security_definer']} | {', '.join(consumers) or 'none'} | {link(function['source'])}; {', '.join(link(s) for s in function['prior_sources']) or 'none'} |")
            out += ["", "### Current policies", ""]
            for name, policy in data["policies"].items():
                command = re.search(r" FOR (ALL|SELECT|INSERT|UPDATE|DELETE) ", policy["sql"])
                roles = re.search(r" TO (.*?)(?= USING | WITH CHECK | ;)", policy["sql"])
                out += [f"- `{name}` — {link(policy['source'])}: FOR {command[1] if command else 'ALL (default)'}, TO `{roles[1] if roles else 'PUBLIC (default)'}`. Predicate: see source."]
            out += ["", "### Ordered role/grant/default-privilege statements", ""]
            for grant in data["security"]:
                out += [f"- {link(grant['source'])}: `{grant['sql']}`"]
    return "\n".join(out) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--markdown", action="store_true")
    parser.add_argument("--check", type=Path, help="Compare existing generated report; never write it")
    args = parser.parse_args()
    if not __debug__:
        raise RuntimeError("Assertions must be enabled")
    data = [inventory(engine) for engine in DIRS]
    if args.check:
        assert args.check.read_text() == markdown(data), "Generated report drift"
        print("Generated inventory exact match; FK/trigger/disposition coverage checks passed")
    elif args.markdown:
        print(markdown(data), end="")
    else:
        print(json.dumps([summary(item) for item in data], indent=2))


if __name__ == "__main__":
    main()
