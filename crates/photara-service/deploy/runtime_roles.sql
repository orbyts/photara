-- Operator-only provisioning template. Does not contain or generate credentials.
-- Required psql identifier variables: api_login, control_login, auth_login.
-- No existing role is adopted or modified. Password authentication stays disabled
-- until the selected encrypted secret store's separate provisioning step runs.
\set ON_ERROR_STOP on
BEGIN;
CREATE ROLE :"api_login" LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 6 PASSWORD NULL;
CREATE ROLE :"control_login" LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 6 PASSWORD NULL;
CREATE ROLE :"auth_login" LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 6 PASSWORD NULL;
GRANT photara_api TO :"api_login";
GRANT photara_control TO :"control_login";
GRANT photara_auth_read TO :"auth_login";
COMMIT;
