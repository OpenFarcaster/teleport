# Teleport

A fast implementation of a Farcaster Hub, in Rust.

⚠️⚠️ The project is still under active development and nowhere close to being production ready ⚠️⚠️

## Rough Features

There are a few things Teleport is currently capable of, and a lot more it is not currently capable of. In no specific order, Teleport can currently:

- [x] Start a libp2p gossip node and connect to other peers
- [x] Broadcast Gossip Messages to enable features like posting casts
- [-] Do pubsub peer discovery over gossipsub (partial support)

A lot is still left to do:

- [ ] Persistently store data in SQLite
- [ ] gRPC APIs for users/admins
- [ ] Be able to sync historical data with nodes
- [ ] Exchange contact info
- [ ] Metrics
- [ ] Easier APIs for different "types" of FC Messages

Persistent Storage, handling received messages from peers, and historical data syncing are the highest priority items.

## Prerequisites

- Rust
- Protobufs Compiler (`brew install protobuf` or `apt install -y protobuf-compiler`)
- RocksDB binaries ([Instructions here](https://github.com/facebook/rocksdb/blob/master/INSTALL.md))

## Prost Patch

Up until recently, there was a Protobuf incompatibility issue with using `prost` in this codebase compared to `ts-proto` that is used in Hubble. As such, we have a patched version of `prost` that is used ([Found Here](https://github.com/OpenFarcaster/prost)).

In a recent update the Protobuf schema was updated to add a new field that allows us to get by that issue by serializing the message differently. That hasn't been implemented yet in Teleport but technically we don't need to maintain a patched version of `prost` anymore.

## Database

1. create the database

```bash
make db-create
```

1. run the migrations

```bash
make db-migrate
```
