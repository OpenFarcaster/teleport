use std::path::PathBuf;

use clap::Parser;

const PEER_ID_FILENAME: &str = "id.protobuf";
const DEFAULT_PEER_ID_DIR: &str = "./.hub";
const DEFAULT_PEER_ID_FILENAME: &str = "default_id.protobuf";
const DEFAULT_PEER_ID_LOCATION: &str = "./.hub/default_id.protobuf";
const DEFAULT_CHUNK_SIZE: u64 = 10000;
const DEFAULT_FNAME_SERVER_URL: &str = "https://fnames.farcaster.xyz";

#[derive(Parser)]
struct HubbleOptions {
    #[clap(
        short = 'n',
        long,
        name = "network",
        help = "ID of the Farcaster Network (default: 3 (devnet))"
    )]
    network: Option<String>,

    #[clap(short = 'i', long, name = "id", help = "Path to the PeerId file.")]
    id: Option<PathBuf>,

    #[clap(short = 'c', long, name = "config", help = "Path to the config file.")]
    config: Option<PathBuf>,

    #[clap(
        long,
        name = "db-name",
        help = "The name of the RocksDB instance. (default: rocks.hub._default)"
    )]
    db_name: Option<String>,

    #[clap(
        long,
        name = "admin-server-enabled",
        help = "Enable the admin server. (default: disabled)"
    )]
    admin_server_enabled: bool,

    #[clap(
        long,
        name = "admin-server-host",
        help = "The host the admin server should listen on. (default: '127.0.0.1')"
    )]
    admin_server_host: Option<String>,

    #[clap(
        long,
        name = "process-file-prefix",
        help = "Prefix for file to which hub process number is written. (default: '')"
    )]
    process_file_prefix: Option<String>,
}

#[derive(Parser)]
struct EthereumOptions {
    #[clap(
        short = 'm',
        long,
        name = "eth-mainnet-rpc-url",
        help = "RPC URL of a Mainnet ETH Node (or comma separated list of URLs)"
    )]
    eth_mainnet_rpc_url: Option<String>,

    #[clap(
        short = 'e',
        long,
        name = "eth-rpc-url",
        help = "RPC URL of a Goerli ETH Node (or comma separated list of URLs)"
    )]
    eth_rpc_url: Option<String>,

    #[clap(
        long,
        name = "rank-rpcs",
        help = "Rank the RPCs by latency/stability and use the fastest one (default: disabled)"
    )]
    rank_rpcs: bool,

    #[clap(
        long,
        name = "fname-server-url",
        help = format!("The URL for the FName registry server (default: {})", DEFAULT_FNAME_SERVER_URL)
    )]
    fname_server_url: Option<String>,

    #[clap(
        long,
        name = "fir-address",
        help = "The address of the Farcaster ID Registry contract"
    )]
    fir_address: Option<String>,

    #[clap(
        long,
        name = "first-block",
        help = "The block number to begin syncing events from Farcaster contracts"
    )]
    first_block: Option<u64>,
}

#[derive(Parser)]
struct L2Options {
    #[clap(
        short = 'l',
        long,
        name = "l2-rpc-url",
        help = "RPC URL of a mainnet Optimism Node (or comma separated list of URLs)"
    )]
    l2_rpc_url: Option<String>,

    #[clap(
        long,
        name = "l2-id-registry-address",
        help = "The address of the L2 Farcaster ID Registry contract"
    )]
    l2_id_registry_address: Option<String>,

    #[clap(
        long,
        name = "l2-key-registry-address",
        help = "The address of the L2 Farcaster Key Registry contract"
    )]
    l2_key_registry_address: Option<String>,

    #[clap(
        long,
        name = "l2-storage-registry-address",
        help = "The address of the L2 Farcaster Storage Registry contract"
    )]
    l2_storage_registry_address: Option<String>,

    #[clap(
        long,
        name = "l2-resync-events",
        help = "Resync events from the L2 Farcaster contracts before starting (default: disabled)"
    )]
    l2_resync_events: bool,

    #[clap(
        long,
        name = "l2-first-block",
        help = "The block number to begin syncing events from L2 Farcaster contracts"
    )]
    l2_first_block: Option<u64>,

    #[clap(
        long,
        name = "l2-chunk-size",
        help = "The number of events to fetch from L2 Farcaster contracts at a time"
    )]
    l2_chunk_size: Option<u64>,

    #[clap(
        long,
        name = "l2-chain-id",
        help = "The chain ID of the L2 Farcaster contracts are deployed to"
    )]
    l2_chain_id: Option<u64>,

    #[clap(
        long,
        name = "l2-rent-expiry-override",
        help = "The storage rent expiry in seconds to use instead of the default 1 year (ONLY FOR TESTS)"
    )]
    l2_rent_expiry_override: Option<u64>,
}

#[derive(Parser)]
struct NetworkingOptions {
    #[clap(
        short = 'a',
        long,
        name = "allowed-peers",
        help = "Only peer with specific peer ids. (default: all peers allowed)"
    )]
    allowed_peers: Option<Vec<String>>,

    #[clap(
        long,
        name = "denied-peers",
        help = "Do not peer with specific peer ids. (default: no peers denied)"
    )]
    denied_peers: Option<Vec<String>>,

    #[clap(
        short = 'b',
        long,
        name = "bootstrap",
        help = "Peers to bootstrap gossip and sync from. (default: none)"
    )]
    bootstrap: Option<Vec<String>>,

    #[clap(
        short = 'g',
        long,
        name = "gossip-port",
        help = "Port to use for gossip (default: DEFAULT_GOSSIP_PORT)"
    )]
    gossip_port: Option<u16>,

    #[clap(
        short = 'r',
        long,
        name = "rpc-port",
        help = "Port to use for gRPC  (default: DEFAULT_RPC_PORT)"
    )]
    rpc_port: Option<u16>,

    #[clap(
        long,
        name = "ip",
        help = "IP address to listen on (default: '127.0.0.1')"
    )]
    ip: Option<String>,

    #[clap(
        long,
        name = "announce-ip",
        help = "Public IP address announced to peers (default: fetched with external service)"
    )]
    announce_ip: Option<String>,

    #[clap(
        long,
        name = "announce-server-name",
        help = "Server name announced to peers, useful if SSL/TLS enabled. (default: 'none')"
    )]
    announce_server_name: Option<String>,

    #[clap(
        long,
        name = "direct-peers",
        help = "A list of peers for libp2p to directly peer with (default: [])"
    )]
    direct_peers: Option<Vec<String>>,

    #[clap(
        long,
        name = "rpc-rate-limit",
        help = "RPC rate limit for peers specified in rpm. Set to -1 for none. (default: 20k/min)"
    )]
    rpc_rate_limit: Option<i32>,
}

#[derive(Parser)]
struct MetricsOptions {
    #[clap(
        long,
        name = "statsd-metrics-server",
        help = "The host to send statsd metrics to, eg '127.0.0.1:8125'. (default: disabled)"
    )]
    statsd_metrics_server: Option<String>,

    #[clap(
        long,
        name = "gossip-metrics-enabled",
        help = "Generate tracing and metrics for the gossip network. (default: disabled)"
    )]
    gossip_metrics_enabled: bool,
}

#[derive(Parser)]
struct DebuggingOptions {
    #[clap(
        long,
        name = "disable-console-status",
        help = "Immediately log to STDOUT, and disable console status and progressbars. (default: disabled)"
    )]
    disable_console_status: bool,

    #[clap(
        long,
        name = "profile-sync",
        help = "Profile a full hub sync and exit. (default: disabled)"
    )]
    profile_sync: bool,

    #[clap(
        long,
        name = "rebuild-sync-trie",
        help = "Rebuild the sync trie before starting (default: disabled)"
    )]
    rebuild_sync_trie: bool,

    #[clap(
        long,
        name = "resync-eth-events",
        help = "Resync events from the Farcaster contracts before starting (default: disabled)"
    )]
    resync_eth_events: bool,

    #[clap(
        long,
        name = "resync-name-events",
        help = "Resync events from the Fname server before starting (default: disabled)"
    )]
    resync_name_events: bool,

    #[clap(
        long,
        name = "chunk-size",
        help = "The number of blocks to batch when syncing historical events from Farcaster contracts. (default: DEFAULT_CHUNK_SIZE)"
    )]
    chunk_size: Option<u64>,

    #[clap(
        long,
        name = "commit-lock-timeout",
        help = "Rocks DB commit lock timeout in milliseconds (default: 500)"
    )]
    commit_lock_timeout: Option<u64>,

    #[clap(
        long,
        name = "commit-lock-max-pending",
        help = "Rocks DB commit lock max pending jobs (default: 1000)"
    )]
    commit_lock_max_pending: Option<u64>,

    #[clap(
        long,
        name = "rpc-auth",
        help = "Require username-password auth for RPC submit. (default: disabled)"
    )]
    rpc_auth: Option<String>,
}

#[derive(Parser)]
#[clap(name = "start", about = "Start a Hub")]
struct HubCli {
    #[clap(flatten)]
    hubble_options: HubbleOptions,

    #[clap(flatten)]
    ethereum_options: EthereumOptions,

    #[clap(flatten)]
    l2_options: L2Options,

    #[clap(flatten)]
    networking_options: NetworkingOptions,

    #[clap(flatten)]
    metrics_options: MetricsOptions,

    #[clap(flatten)]
    debugging_options: DebuggingOptions,
}
