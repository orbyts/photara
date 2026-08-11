CREATE TABLE project_asset_decision_events (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    project_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    decision text NOT NULL CHECK (decision IN ('photographer-final')),
    selected boolean NOT NULL,
    source text NOT NULL CHECK (source <> '' AND source = btrim(source)),
    note text CHECK (note IS NULL OR (note <> '' AND note = btrim(note))),
    changed_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (project_id, asset_id)
        REFERENCES project_assets(project_id, asset_id) ON DELETE CASCADE
);

CREATE INDEX project_asset_decision_events_history
    ON project_asset_decision_events (project_id, asset_id, decision, changed_at, id);

INSERT INTO project_asset_decision_events (
    project_id,
    asset_id,
    decision,
    selected,
    source,
    note,
    changed_at
)
SELECT
    project_id,
    asset_id,
    decision,
    selected,
    'migration-backfill',
    'Current state backfilled when append-only decision history was introduced',
    decided_at
FROM project_asset_decisions;
