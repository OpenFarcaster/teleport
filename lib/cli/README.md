# Teleport CLI

A CLI for interacting with a Teleport Hub.

## Commands

### `teleport start`

Used to start a Teleport Hub.

### `teleport identity`

#### `teleport identity create`

Create a new Peer ID file

####  `teleport identity verify`

Verify a Peer ID file

### `teleport status`

Reports the database and syncing status of the hub.

### `teleport profile`

#### `teleport profile gossip`

Profile the gossip server's performance

#### `teleport profile rpc`

Profile the RPC server's performance

#### `teleport profile storage`

Profile the storage layout's performance

### `teleport reset`

#### `teleport reset events`

Clear L2 contract events from the database

#### `teleport reset full`

Completely clear the database

### `teleport console`

Start a REPL console to interact with the Hub