INSERT INTO
    errors (
        msg,
        machine_id,
        session_id,
        ext_version,
        vscode_version
    )
VALUES (
    ${msg},
    ${machine_id},
    ${session_id},
    ${ext_version},
    ${vscode_version}
);
