-- Add migration script here

--- chain events
CREATE TABLE chain_events (
    id TEXT PRIMARY KEY,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    block_timestamp DATETIME NOT NULL,
    fid INTEGER NOT NULL,
    chain_id INTEGER NOT NULL,
    block_number INTEGER NOT NULL,
    transaction_index SMALLINT NOT NULL,
    log_index SMALLINT NOT NULL,
    type SMALLINT NOT NULL,
    block_hash BLOB NOT NULL,
    transaction_hash BLOB NOT NULL,
    body TEXT NOT NULL,
    raw BLOB NOT NULL
);

CREATE INDEX chain_events_fid_index ON chain_events(fid);
CREATE INDEX chain_events_block_hash_index ON chain_events(block_hash);
CREATE INDEX chain_events_block_timestamp_index ON chain_events(block_timestamp);
CREATE INDEX chain_events_transaction_hash_index ON chain_events(transaction_hash);

---- FID
CREATE TABLE fids (
    fid INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    registered_at TEXT NOT NULL,
    chain_event_id TEXT NOT NULL,  -- UUIDs are stored as TEXT in SQLite
    custody_address BLOB NOT NULL,
    recovery_address BLOB NOT NULL,
    PRIMARY KEY (fid),
    FOREIGN KEY (chain_event_id) REFERENCES chain_events(id) ON DELETE CASCADE
);

--- Signers
CREATE TABLE signers (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    added_at TEXT NOT NULL,
    removed_at TEXT,
    fid INTEGER NOT NULL,
    requester_fid INTEGER NOT NULL,
    add_chain_event_id TEXT NOT NULL,  -- UUID as TEXT
    remove_chain_event_id TEXT,
    key_type SMALLINT NOT NULL,
    metadata_type SMALLINT NOT NULL,
    key BLOB NOT NULL,
    metadata TEXT NOT NULL,  -- JSON as TEXT
    UNIQUE (fid, key),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (requester_fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (add_chain_event_id) REFERENCES chain_events(id) ON DELETE CASCADE,
    FOREIGN KEY (remove_chain_event_id) REFERENCES chain_events(id) ON DELETE CASCADE
);

CREATE INDEX signers_fid_index ON signers(fid);
CREATE INDEX signers_requester_fid_index ON signers(requester_fid);

--- username proofs
CREATE TABLE username_proofs (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    type SMALLINT NOT NULL,
    username TEXT NOT NULL,
    signature BLOB NOT NULL,
    owner BLOB NOT NULL,
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX username_proofs_username_timestamp_unique ON username_proofs (username, timestamp);

--- fnames
CREATE TABLE fnames (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    registered_at TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    type SMALLINT NOT NULL,
    username TEXT NOT NULL,
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX fnames_fid_unique ON fnames (fid);
CREATE UNIQUE INDEX fnames_username_unique ON fnames (username);

--- messages
CREATE TABLE messages (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    pruned_at TEXT,
    revoked_at TEXT,
    fid INTEGER NOT NULL,
    type SMALLINT NOT NULL,
    hash_scheme SMALLINT NOT NULL,
    signature_scheme SMALLINT NOT NULL,
    hash BLOB NOT NULL,
    signature BLOB NOT NULL,
    signer BLOB NOT NULL,
    body TEXT NOT NULL,  -- JSON as TEXT
    raw BLOB NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (fid, signer) REFERENCES signers(fid, key) ON DELETE CASCADE
);

CREATE INDEX messages_timestamp_index ON messages(timestamp);
CREATE INDEX messages_fid_index ON messages(fid);
CREATE INDEX messages_signer_index ON messages(signer);

--- casts
CREATE TABLE casts (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    parent_fid INTEGER,
    hash BLOB NOT NULL,
    root_parent_hash BLOB,
    parent_hash BLOB,
    root_parent_url TEXT,
    parent_url TEXT,
    text TEXT NOT NULL,
    embeds TEXT NOT NULL DEFAULT '[]',  -- JSON as TEXT
    mentions TEXT NOT NULL DEFAULT '[]',  -- JSON as TEXT
    mentions_positions TEXT NOT NULL DEFAULT '[]',  -- JSON as TEXT
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (hash) REFERENCES messages(hash) ON DELETE CASCADE
);

CREATE UNIQUE INDEX casts_hash_unique ON casts (hash);

CREATE INDEX casts_timestamp_index ON casts(timestamp);
CREATE INDEX casts_parent_hash_index ON casts(parent_hash);
CREATE INDEX casts_root_parent_hash_index ON casts(root_parent_hash);
CREATE INDEX casts_parent_url_index ON casts(parent_url);
CREATE INDEX casts_root_parent_url_index ON casts(root_parent_url);

--- reactions
CREATE TABLE reactions (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    target_cast_fid INTEGER,
    type INTEGER NOT NULL,
    hash BLOB NOT NULL,
    target_cast_hash BLOB,
    target_url TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (hash) REFERENCES messages(hash) ON DELETE CASCADE,
    FOREIGN KEY (target_cast_hash) REFERENCES casts(hash) ON DELETE CASCADE
);

CREATE UNIQUE INDEX reactions_hash_unique ON reactions (hash);

CREATE INDEX reactions_fid_timestamp_index ON reactions(fid, timestamp);
CREATE INDEX reactions_target_cast_hash_index ON reactions(target_cast_hash);
CREATE INDEX reactions_target_url_index ON reactions(target_url);

--- links
CREATE TABLE links (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    target_fid INTEGER NOT NULL,
    display_timestamp TEXT,
    type TEXT NOT NULL,
    hash BLOB NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (target_fid) REFERENCES fids(fid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX links_hash_unique ON links (hash);
CREATE UNIQUE INDEX links_fid_target_fid_type_unique ON links (fid, target_fid, type);

--- verifications
CREATE TABLE verifications (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    hash BLOB NOT NULL,
    signer_address BLOB NOT NULL,
    block_hash BLOB NOT NULL,
    signature BLOB NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (hash) REFERENCES messages(hash) ON DELETE CASCADE
);

CREATE UNIQUE INDEX verifications_signer_address_fid_unique ON verifications (signer_address, fid);
CREATE INDEX verifications_fid_timestamp_index ON verifications (fid, timestamp);

--- user data
CREATE TABLE user_data (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    timestamp TEXT NOT NULL,
    deleted_at TEXT,
    fid INTEGER NOT NULL,
    type INTEGER NOT NULL,
    hash BLOB NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (fid) REFERENCES fids(fid) ON DELETE CASCADE,
    FOREIGN KEY (hash) REFERENCES messages(hash) ON DELETE CASCADE
);

CREATE UNIQUE INDEX user_data_fid_type_unique ON user_data (fid, type);

---- storage allocations
CREATE TABLE storage_allocations (
    id TEXT,  -- UUID as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    rented_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    chain_event_id TEXT NOT NULL,
    fid INTEGER NOT NULL,
    units INTEGER NOT NULL,
    payer BLOB NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (chain_event_id) REFERENCES chain_events(id) ON DELETE CASCADE
);

CREATE INDEX storage_allocations_fid_expires_at_index ON storage_allocations(fid, expires_at);
