INSERT INTO fids (
    fid,
    registered_at,
    chain_event_id,
    custody_address,
    recovery_address
) 
VALUES (?, ?, ?, ?, ?)
ON CONFLICT (fid) DO NOTHING;
