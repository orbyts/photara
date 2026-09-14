#!/usr/bin/env python3
"""Bounded Neon main operator provisioning. Secrets never leave memory/Keychain.

Run after installing the native helper at the fixed host-local path. A failed
or uncertain provisioning stops without deleting/overwriting any Keychain item
or database role; inspect the exact boundary before operator recovery.
"""
import base64
import json
import os
from pathlib import Path
import resource
import secrets
import subprocess
import sys
from urllib.parse import quote, unquote, urlsplit

PROJECT = "steep-waterfall-28781561"
BRANCH = "br-shiny-glitter-afu6adtk"
HELPER = (Path.home() / ".local/share/photara-development-service/"
          "Photara Development Operator.app/Contents/MacOS/photara-development-service")
ROLES = {"PHOTARA_DB_API_URL": ("photara_dev_api", "photara_api"),
         "PHOTARA_DB_CONTROL_URL": ("photara_dev_control", "photara_control"),
         "PHOTARA_DB_AUTH_READ_URL": ("photara_dev_auth_read", "photara_auth_read")}
NEON = "/opt/homebrew/bin/neonctl"
PSQL = "/opt/homebrew/bin/psql"


def refuse(code):
    print(f"development-provision:{code}", file=sys.stderr)
    raise SystemExit(1)


def run(argv, *, env=None, data=None):
    try:
        return subprocess.run(argv, input=data, env=env, text=True,
                              capture_output=True, timeout=45, check=False)
    except (OSError, subprocess.TimeoutExpired):
        refuse("child-failed-or-timed-out-inspect-state-before-retry")


def connection_environment(url):
    p = urlsplit(url)
    if (p.scheme != "postgresql" or not p.hostname or
            not p.hostname.endswith(".neon.tech") or p.path != "/neondb" or
            not p.username or not p.password):
        refuse("connection-coordinates-refused")
    # libpq does not expand a connection URI supplied through PGDATABASE.
    env = {"PATH": "/usr/bin:/bin", "PGHOST": p.hostname,
           "PGPORT": str(p.port or 5432), "PGDATABASE": "neondb",
           "PGUSER": unquote(p.username), "PGPASSWORD": unquote(p.password),
           "PGSSLMODE": "verify-full", "PGSSLROOTCERT": "system",
           "PGCONNECT_TIMEOUT": "10"}
    return env


def sql(url, statement):
    result = run([PSQL, "-X", "-w", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1"],
                 env=connection_environment(url), data=statement)
    if result.returncode:
        # PostgreSQL errors can include whole statements/passwords. Never print.
        refuse("sql-refused-or-commit-uncertain-inspect-state-before-retry")
    return result.stdout.strip()


EMPTY = """SELECT json_build_object(
 'accounts',(SELECT count(*) FROM photara_identity.accounts),
 'libraries',(SELECT count(*) FROM photara.libraries),
 'defaults',(SELECT count(*) FROM photara_identity.account_defaults),
 'receipts',(SELECT count(*) FROM photara_private.onboarding_receipts));"""


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if sys.argv[1:] not in (["--preflight"], ["--provision"], ["--postflight"]):
        refuse("expected-preflight-provision-or-postflight")
    mode = sys.argv[1]
    result = run([NEON, "connection-string", BRANCH, "--project-id", PROJECT,
                  "--database-name", "neondb", "--role-name", "neondb_owner",
                  "--ssl", "verify-full", "--no-analytics"])
    if result.returncode:
        refuse("operator-connection-unavailable")
    owner_url = result.stdout.strip()
    owner = urlsplit(owner_url)
    if owner.username != "neondb_owner":
        refuse("unexpected-operator")
    counts = json.loads(sql(owner_url, EMPTY))
    print("development-provision:empty-data=" + json.dumps(counts, sort_keys=True))
    if any(counts.values()):
        refuse("nonempty-data-outside-this-gate")
    metadata = sql(owner_url, "SELECT schema_family || ':' || schema_epoch || ':' || minimum_api FROM photara.schema_metadata; SELECT count(*) FROM public._sqlx_migrations WHERE success;")
    if metadata != "photara.service.g2:1:3\n14":
        refuse("unexpected-schema-or-ledger")
    print("development-provision:schema=photara.service.g2:1:3 ledger=14")
    names = ",".join("'" + role + "'" for role, _ in ROLES.values())
    existing = sql(owner_url, "SELECT rolname FROM pg_roles WHERE rolname IN (" + names + ") ORDER BY rolname;")
    if mode == "--preflight":
        print("development-provision:runtime-role-count=" + str(len(existing.splitlines())))
        return
    if mode == "--postflight":
        for role, capability in ROLES.values():
            audit_role(owner_url, role, capability)
        return
    if existing:
        refuse("runtime-role-collision-no-adoption")
    status = run([str(HELPER), "check"])
    if status.returncode == 0:
        refuse("keychain-item-already-exists-no-overwrite")
    if status.stderr.strip() != "development-service:keychain-item-missing":
        refuse("keychain-access-blocked-before-role-creation")
    passwords = {name: secrets.token_urlsafe(32) for name in ROLES}
    values = {name: "postgresql://" + role + ":" + quote(passwords[name], safe="") + "@" + owner.hostname + "/neondb?sslmode=verify-full"
              for name, (role, _) in ROLES.items()}
    values["PHOTARA_CURSOR_KEY_B64"] = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode().rstrip("=")
    # First establish secure durability. Failure creates no database identities.
    stored = run([str(HELPER), "store"], data=json.dumps(values))
    if stored.returncode:
        print(stored.stderr.strip() if stored.stderr.startswith("development-service:") else "development-service:keychain-store-refused", file=sys.stderr)
        refuse("keychain-store-failed-no-roles-created")
    print("development-provision:keychain-stored")
    statements = ["BEGIN; SET LOCAL password_encryption='scram-sha-256';"]
    for name, (role, capability) in ROLES.items():
        statements.append(f"CREATE ROLE {role} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 6 PASSWORD '{passwords[name]}';")
        statements.append(f"GRANT {capability} TO {role};")
    statements.append("COMMIT;")
    sql(owner_url, "\n".join(statements))
    print("development-provision:three-runtime-logins-created")
    for name, (role, capability) in ROLES.items():
        audit_role(values[name], role, capability)
    after = json.loads(sql(owner_url, EMPTY))
    if after != counts:
        refuse("unexpected-data-count-change")
    print("development-provision:after-empty-data=" + json.dumps(after, sort_keys=True))
    print("development-provision:complete-cursor-key-stable-in-keychain")


def audit_role(url, role, capability):
    # Verify effective membership and privileges, including the Neon elevated
    # role. Neither object ownership nor a grant with admin option is admitted.
    statement = f"""SELECT rolcanlogin AND rolinherit AND NOT rolsuper
      AND NOT rolcreatedb AND NOT rolcreaterole AND NOT rolreplication
      AND NOT rolbypassrls AND rolconnlimit=6
      AND pg_has_role(oid,'{capability}','MEMBER')
      AND NOT pg_has_role(oid,'photara_owner','MEMBER')
      AND NOT pg_has_role(oid,'neon_superuser','MEMBER')
      AND NOT EXISTS(SELECT 1 FROM pg_roles other WHERE other.rolname IN
        ('photara_api','photara_control','photara_auth_read')
        AND other.rolname<>'{capability}' AND pg_has_role(r.oid,other.oid,'MEMBER'))
      AND (SELECT count(*) FROM pg_auth_members WHERE member=r.oid)=1
      AND NOT EXISTS(SELECT 1 FROM pg_auth_members WHERE member=r.oid AND admin_option)
      AND NOT EXISTS(SELECT 1 FROM pg_class WHERE relowner=r.oid)
      AND NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspowner=r.oid)
      AND NOT has_database_privilege(oid,current_database(),'CREATE')
      FROM pg_roles r WHERE rolname='{role}';"""
    if sql(url, statement) != "t":
        refuse("runtime-role-audit-refused:" + role)
    print("development-provision:role-safe=" + role + ":" + capability)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError):
        refuse("invalid-operator-result")
