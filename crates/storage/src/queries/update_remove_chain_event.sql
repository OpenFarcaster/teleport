UPDATE signers 
SET remove_chain_event_id = ? 
WHERE key = ? AND key_type = ?;