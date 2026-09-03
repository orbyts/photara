CREATE TABLE project_selection_overrides (
    project_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    selection_kind text NOT NULL CHECK (selection_kind IN (
        'client-favorite',
        'client-shortlist',
        'hero'
    )),
    action text NOT NULL CHECK (action IN ('add', 'remove')),
    reason text NOT NULL CHECK (reason <> '' AND reason = btrim(reason)),
    source text NOT NULL CHECK (source <> '' AND source = btrim(source)),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, asset_id, selection_kind),
    FOREIGN KEY (project_id, asset_id)
        REFERENCES project_assets(project_id, asset_id) ON DELETE CASCADE
);

CREATE TABLE project_selection_override_events (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    project_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    selection_kind text NOT NULL CHECK (selection_kind IN (
        'client-favorite',
        'client-shortlist',
        'hero'
    )),
    action text NOT NULL CHECK (action IN ('add', 'remove')),
    reason text NOT NULL CHECK (reason <> '' AND reason = btrim(reason)),
    source text NOT NULL CHECK (source <> '' AND source = btrim(source)),
    changed_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (project_id, asset_id)
        REFERENCES project_assets(project_id, asset_id) ON DELETE CASCADE
);

CREATE INDEX project_selection_override_events_history
    ON project_selection_override_events (
        project_id,
        asset_id,
        selection_kind,
        changed_at,
        id
    );
