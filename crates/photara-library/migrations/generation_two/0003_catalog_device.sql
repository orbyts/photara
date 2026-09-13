CREATE TABLE storage_roots (
    storage_root_id BLOB PRIMARY KEY CHECK (length(storage_root_id) = 16),
    library_id BLOB NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    label_key TEXT NOT NULL CHECK (length(label_key) > 0),
    purpose TEXT NOT NULL CHECK (length(purpose) > 0),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned')),
    retired_at_ms INTEGER,
    UNIQUE (library_id, storage_root_id),
    UNIQUE (library_id, label_key),
    CHECK ((state = 'active' AND retired_at_ms IS NULL) OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL)),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE project_catalog (
    library_id BLOB NOT NULL,
    project_id BLOB NOT NULL CHECK (length(project_id) = 16),
    visibility TEXT NOT NULL CHECK (visibility IN ('visible', 'hidden')),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    active_locator_id BLOB,
    selected_observation_id BLOB,
    PRIMARY KEY (library_id, project_id),
    CHECK (selected_observation_id IS NULL OR active_locator_id IS NOT NULL),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, project_id, active_locator_id)
        REFERENCES project_locators(library_id, project_id, locator_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (library_id, project_id, selected_observation_id, active_locator_id)
        REFERENCES project_observations(library_id, project_id, observation_id, locator_id)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE project_locators (
    locator_id BLOB PRIMARY KEY CHECK (length(locator_id) = 16),
    library_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    storage_root_id BLOB,
    relative_path TEXT,
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'retired')),
    retired_at_ms INTEGER,
    UNIQUE (library_id, project_id, locator_id),
    CHECK ((storage_root_id IS NULL) = (relative_path IS NULL)),
    CHECK (relative_path IS NULL OR (length(relative_path) > 0
        AND substr(relative_path, 1, 1) <> '/'
        AND instr(relative_path, char(92)) = 0 AND instr(relative_path, ':') = 0
        AND instr(relative_path, char(0)) = 0 AND instr(relative_path, '//') = 0
        AND instr('/' || relative_path || '/', '/../') = 0
        AND instr('/' || relative_path || '/', '/./') = 0)),
    CHECK ((state = 'active' AND retired_at_ms IS NULL) OR (state = 'retired' AND retired_at_ms IS NOT NULL)),
    FOREIGN KEY (library_id, project_id) REFERENCES project_catalog(library_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, storage_root_id) REFERENCES storage_roots(library_id, storage_root_id) ON DELETE RESTRICT
) STRICT;
CREATE UNIQUE INDEX rooted_locator_path ON project_locators(library_id, storage_root_id, relative_path)
    WHERE storage_root_id IS NOT NULL AND state = 'active';
CREATE INDEX locators_by_project ON project_locators(library_id, project_id, state);

CREATE TABLE device_root_bindings (
    device_id BLOB NOT NULL,
    library_id BLOB NOT NULL,
    storage_root_id BLOB NOT NULL,
    binding_kind TEXT NOT NULL CHECK (binding_kind IN ('path', 'bookmark', 'provider')),
    host_path TEXT,
    secure_handle_ref BLOB CHECK (length(secure_handle_ref) = 16),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    updated_at_ms INTEGER NOT NULL,
    CHECK ((binding_kind = 'path' AND host_path IS NOT NULL AND secure_handle_ref IS NULL)
        OR (binding_kind IN ('bookmark', 'provider') AND host_path IS NULL AND secure_handle_ref IS NOT NULL)),
    PRIMARY KEY (device_id, storage_root_id),
    FOREIGN KEY (device_id) REFERENCES local_device(device_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, storage_root_id) REFERENCES storage_roots(library_id, storage_root_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE device_project_bindings (
    device_id BLOB NOT NULL,
    library_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    locator_id BLOB NOT NULL,
    direct_host_path TEXT,
    secure_bookmark_ref BLOB CHECK (length(secure_bookmark_ref) = 16),
    availability TEXT NOT NULL CHECK (availability IN ('unknown', 'available', 'unavailable', 'denied', 'ambiguous')),
    verified_project_id BLOB CHECK (length(verified_project_id) = 16),
    last_commit_id BLOB CHECK (length(last_commit_id) = 16),
    last_commit_sha256 BLOB CHECK (length(last_commit_sha256) = 32),
    checked_at_ms INTEGER,
    diagnostic_code TEXT,
    PRIMARY KEY (device_id, locator_id),
    CHECK (direct_host_path IS NULL OR secure_bookmark_ref IS NULL),
    CHECK ((last_commit_id IS NULL) = (last_commit_sha256 IS NULL)),
    CHECK (availability <> 'available' OR (verified_project_id IS NOT NULL AND verified_project_id = project_id AND checked_at_ms IS NOT NULL)),
    FOREIGN KEY (device_id) REFERENCES local_device(device_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, project_id, locator_id)
        REFERENCES project_locators(library_id, project_id, locator_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE project_observations (
    observation_id BLOB PRIMARY KEY CHECK (length(observation_id) = 16),
    library_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    locator_id BLOB NOT NULL,
    commit_id BLOB NOT NULL CHECK (length(commit_id) = 16),
    commit_sha256 BLOB NOT NULL CHECK (length(commit_sha256) = 32),
    package_revision TEXT NOT NULL CHECK (length(package_revision) BETWEEN 1 AND 20
        AND package_revision NOT GLOB '*[^0-9]*' AND substr(package_revision, 1, 1) <> '0'
        AND (length(package_revision) < 20 OR package_revision <= '18446744073709551615')),
    title TEXT NOT NULL,
    project_lifecycle TEXT NOT NULL CHECK (project_lifecycle IN ('active', 'archived')),
    asset_count INTEGER NOT NULL CHECK (asset_count >= 0),
    graph_count INTEGER NOT NULL CHECK (graph_count >= 0),
    observed_at_ms INTEGER NOT NULL,
    index_schema INTEGER NOT NULL CHECK (index_schema = 1),
    UNIQUE (library_id, project_id, observation_id, locator_id),
    UNIQUE (locator_id, commit_id, commit_sha256, index_schema),
    FOREIGN KEY (library_id, project_id, locator_id)
        REFERENCES project_locators(library_id, project_id, locator_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX observation_commit ON project_observations(library_id, project_id, commit_id, commit_sha256);

CREATE TABLE project_party_projection (
    observation_id BLOB NOT NULL,
    assignment_id BLOB NOT NULL CHECK (length(assignment_id) = 16),
    source_library_id BLOB NOT NULL CHECK (length(source_library_id) = 16),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('person', 'organization')),
    source_record_id BLOB NOT NULL CHECK (length(source_record_id) = 16),
    source_revision TEXT NOT NULL,
    display_name_snapshot TEXT NOT NULL,
    roles_json TEXT NOT NULL CHECK (json_valid(roles_json) AND json_type(roles_json) = 'array'),
    PRIMARY KEY (observation_id, assignment_id),
    FOREIGN KEY (observation_id) REFERENCES project_observations(observation_id) ON DELETE CASCADE
) STRICT;
CREATE INDEX projects_by_party ON project_party_projection(source_library_id, source_kind, source_record_id, observation_id);

CREATE TABLE project_location_projection (
    observation_id BLOB NOT NULL,
    assignment_id BLOB NOT NULL CHECK (length(assignment_id) = 16),
    source_library_id BLOB NOT NULL CHECK (length(source_library_id) = 16),
    location_id BLOB NOT NULL CHECK (length(location_id) = 16),
    location_kind_id BLOB NOT NULL CHECK (length(location_kind_id) = 16),
    location_name_snapshot TEXT NOT NULL,
    kind_name_snapshot TEXT NOT NULL,
    schedule_json TEXT CHECK (schedule_json IS NULL OR (json_valid(schedule_json) AND json_type(schedule_json) = 'object')),
    PRIMARY KEY (observation_id, assignment_id),
    FOREIGN KEY (observation_id) REFERENCES project_observations(observation_id) ON DELETE CASCADE
) STRICT;
CREATE INDEX projects_by_location ON project_location_projection(source_library_id, location_id, observation_id);
CREATE INDEX projects_by_kind ON project_location_projection(source_library_id, location_kind_id, observation_id);

CREATE TABLE project_graph_projection (
    observation_id BLOB NOT NULL,
    graph_id BLOB NOT NULL CHECK (length(graph_id) = 16),
    graph_name TEXT NOT NULL,
    graph_digest BLOB NOT NULL CHECK (length(graph_digest) = 32),
    graph_revision TEXT NOT NULL,
    latest_run_id BLOB CHECK (length(latest_run_id) = 16),
    latest_run_status TEXT CHECK (latest_run_status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled', 'interrupted')),
    latest_run_source_graph_digest BLOB CHECK (length(latest_run_source_graph_digest) = 32),
    latest_run_ended_at_ms INTEGER,
    PRIMARY KEY (observation_id, graph_id),
    CHECK ((latest_run_id IS NULL AND latest_run_status IS NULL AND latest_run_source_graph_digest IS NULL AND latest_run_ended_at_ms IS NULL)
        OR (latest_run_id IS NOT NULL AND latest_run_status IS NOT NULL AND latest_run_source_graph_digest IS NOT NULL)),
    FOREIGN KEY (observation_id) REFERENCES project_observations(observation_id) ON DELETE CASCADE
) STRICT;
