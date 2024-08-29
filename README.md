# Teleport

A fast implementation of a Farcaster Hub, in Rust.

⚠️⚠️ The project is still under active development and nowhere close to being production ready ⚠️⚠️

## Introduction

If you are new to Farcaster Hubs and/or Teleport - here's a quick video that does a high-level overview of the responsibilities of a Hub and where Teleport is at.

Note that the codebase will outpace the video, and things mentioned `TODO` in the video might have been done now.

[![Video](https://img.youtube.com/vi/YXu2DGMhIao/0.jpg)](https://www.youtube.com/watch?v=YXu2DGMhIao)

## Rough Features

There are a few things Teleport is currently capable of, and a lot more it is not currently capable of. In no specific order, Teleport can currently:

- [x] Start a libp2p gossip node and connect to other peers
- [x] Broadcast Gossip Messages to enable features like posting casts
- [x] Sync on-chain events from contracts deployed to Optimism
- [-] Do PubSub peer discovery over GossipSub (partial support)

A lot is still left to do:

- [ ] Diff sync with other hubs
- [ ] gRPC APIs
- [ ] REST API
- [ ] CLI
- [ ] Metrics
- [ ] Easier APIs for different "types" of FC Messages

## Prerequisites

Run the following command to install all the prerequisites for you automatically and if it fails for any reason then simply manually install them.

``` bash
make install
```

- Rust
- Protobufs Compiler (`brew install protobuf` or `apt install -y protobuf-compiler`)
- SQLx CLI (`cargo install sqlx-cli`)

## Prost Patch

Up until recently, there was a Protobuf incompatibility issue with using `prost` in this codebase compared to `ts-proto` that is used in Hubble. As such, we have a patched version of `prost` that is used ([Found Here](https://github.com/OpenFarcaster/prost)).

In a recent update the Protobuf schema was updated to add a new field that allows us to get by that issue by serializing the message differently. That hasn't been implemented yet in Teleport but technically we don't need to maintain a patched version of `prost` anymore.


## Setup the Hub

Copy the example .env file using the following command and then place both your farcaster private key and optimism l2 key.

``` bash
cp env.example .env
```

### Setup the Database

1. create the database

```bash
make db-create
```

1. run the migrations

```bash
make db-migrate
```


### Start the hub

```bash
cargo run
```



## Contributing

Make sure to run the following command before pushing commits as this will run all the checks needed to pass the CI workflow.

``` bash
make verify
```



