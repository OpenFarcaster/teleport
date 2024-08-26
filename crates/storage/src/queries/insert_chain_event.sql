INSERT INTO chain_events (
    id,
    block_timestamp,
    fid,
    chain_id,
    block_number,
    transaction_index,
    log_index,
    type,
    block_hash,
    transaction_hash,
    body,
    raw
) 
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);