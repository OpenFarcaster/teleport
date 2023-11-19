-- Add migration script here
CREATE TABLE chainEvents (
    id TEXT PRIMARY KEY,  
    created_at TEXT NOT NULL,  
    updated_at TEXT NOT NULL,  
    fid BIGINT NOT NULL,
    type TEXT NOT NULL,
    data TEXT NOT NULL,  
    block_number BIGINT NOT NULL,
    transaction_index INTEGER NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (block_number, transaction_index, log_index)
);
