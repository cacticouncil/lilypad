INSERT INTO
    events (
        category,
        info,
        machine_id,
        session_id,
        ext_version,
        vscode_version
    )
VALUES (
    ${category},
    ${info},
    ${machine_id},
    ${session_id},
    ${ext_version},
    ${vscode_version}
);
