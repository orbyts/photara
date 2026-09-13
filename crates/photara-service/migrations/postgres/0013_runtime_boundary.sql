-- CXT3c executable role boundary: auth-read accesses only explicitly granted facts.
GRANT USAGE ON SCHEMA photara TO photara_auth_read;
GRANT SELECT ON photara.schema_metadata TO photara_control,photara_auth_read;
GRANT SELECT ON public._sqlx_migrations TO photara_api,photara_control,photara_auth_read;
-- Authenticated device checks are protected controller operations.
-- No ordinary API role receives identity, feed, receipt or object-key access.
