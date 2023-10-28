# Teleport

A fast implementation of a Farcaster Hub, in Rust.

⚠️⚠️ The project is still under active development and nowhere close to being production ready ⚠️⚠️

## Rough Features

There are a few things Teleport is currently capable of, and a lot more it is not currently capable of. In no specific order, Teleport can currently:

- [x] Start a libp2p gossip node and connect to other peers
- [x] Broadcast Gossip Messages to enable features like posting casts
- [-] Do pubsub peer discovery over gossipsub (partial support)

A lot is still left to do:

- [ ] Persistently store data in RocksDB
- [ ] gRPC APIs for users/admins
- [ ] Be able to sync historical data with nodes
- [ ] Exchange contact info
- [ ] Metrics
- [ ] Easier APIs for different "types" of FC Messages

Persistent Storage, handling received messages from peers, and historical data syncing are the highest priority items.

## Prerequisites

- Rust
- Protobufs Compiler
- RocksDB binaries
