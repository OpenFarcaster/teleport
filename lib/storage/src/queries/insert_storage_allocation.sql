INSERT INTO storage_allocations (
    id,
    rented_at,
    expires_at,
    chain_event_id,
    fid,
    units,
    payer
) 
VALUES (?, ?, ?, ?, ?, ?, ?);