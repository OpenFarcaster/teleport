INSERT INTO signers (
    id,
    added_at,
    removed_at,
    fid,
    requester_fid,
    add_chain_event_id,
    remove_chain_event_id,
    key_type,
    metadata_type,
    key,
    metadata
) 
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);