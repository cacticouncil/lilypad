CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    category VARCHAR NOT NULL,
    info JSON NOT NULL,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    machine_id VARCHAR NOT NULL,
    session_id VARCHAR NOT NULL,
    ext_version VARCHAR NOT NULL,
    vscode_version VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS errors (
    id SERIAL PRIMARY KEY,
    msg VARCHAR NOT NULL,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    machine_id VARCHAR NOT NULL,
    session_id VARCHAR NOT NULL,
    ext_version VARCHAR NOT NULL,
    vscode_version VARCHAR NOT NULL
);
