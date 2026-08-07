CREATE TABLE projects (
    id uuid PRIMARY KEY,
    slug text NOT NULL UNIQUE,
    display_name text NOT NULL,
    scene_slug text NOT NULL,
    scene_snapshot jsonb NOT NULL,
    location_slug text NOT NULL,
    location_snapshot jsonb NOT NULL,
    origin text NOT NULL CHECK (origin IN ('native', 'proetus', 'adopted')),
    status text NOT NULL CHECK (status IN ('active', 'archived')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE project_people (
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    person_slug text NOT NULL,
    person_snapshot jsonb NOT NULL,
    PRIMARY KEY (project_id, person_slug)
);

CREATE TABLE workflow_events (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    project_id uuid REFERENCES projects(id) ON DELETE SET NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    idempotency_key text NOT NULL UNIQUE,
    occurred_at timestamptz NOT NULL DEFAULT now()
);
