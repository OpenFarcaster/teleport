use std::path::PathBuf;

use clap::Parser;

const PEER_ID_FILENAME: &str = "id.protobuf";
const DEFAULT_PEER_ID_DIR: &str = "./.hub";
const DEFAULT_PEER_ID_FILENAME: &str = "default_id.protobuf";
const DEFAULT_PEER_ID_LOCATION: &str = "./.hub/default_id.protobuf";
const DEFAULT_CHUNK_SIZE: u64 = 10000;
const DEFAULT_FNAME_SERVER_URL: &str = "https://fnames.farcaster.xyz";

#[derive(Parser, Debug)]
struct TeleportOptions {
    #[arg(
        short,
        long,
        help = "ID of the Farcaster Network (default: 3 (devnet))"
    )]
    network: Option<String>,

    #[arg(short, long, help = "Path to the PeerId file.")]
    id: Option<PathBuf>,

    #[arg(short, long, help = "Path to the config file.")]
    config: Option<PathBuf>,

    #[arg(
        long,
        help = "The name of the RocksDB instance. (default: rocks.hub._default)"
    )]
    db_name: Option<String>,

    #[arg(long, help = "Enable the admin server. (default: disabled)")]
    admin_server_enabled: bool,

    #[arg(
        long,
        help = "The host the admin server should listen on. (default: '127.0.0.1')"
    )]
    admin_server_host: Option<String>,

    #[arg(
        long,
        help = "Prefix for file to which hub process number is written. (default: '')"
    )]
    process_file_prefix: Option<String>,
}

#[derive(Parser, Debug)]
struct EthereumOptions {
    #[arg(
        short = 'm',
        long,
        help = "RPC URL of a Mainnet ETH Node (or comma separated list of URLs)"
    )]
    eth_mainnet_rpc_url: Option<String>,

    #[arg(
        short = 'e',
        long,
        help = "RPC URL of a Goerli ETH Node (or comma separated list of URLs)"
    )]
    eth_rpc_url: Option<String>,

    #[arg(
        long,
        help = "Rank the RPCs by latency/stability and use the fastest one (default: disabled)"
    )]
    rank_rpcs: bool,

    #[arg(
        long,
        help = format!("The URL for the FName registry server (default: {})", DEFAULT_FNAME_SERVER_URL)
    )]
    fname_server_url: Option<String>,

    #[arg(long, help = "The address of the Farcaster ID Registry contract")]
    fir_address: Option<String>,

    #[arg(
        long,
        help = "The block number to begin syncing events from Farcaster contracts"
    )]
    first_block: Option<u64>,
}

#[derive(Parser, Debug)]
struct L2Options {
    #[arg(
        short = 'l',
        long,
        help = "RPC URL of a mainnet Optimism Node (or comma separated list of URLs)"
    )]
    l2_rpc_url: Option<String>,

    #[arg(long, help = "The address of the L2 Farcaster ID Registry contract")]
    l2_id_registry_address: Option<String>,

    #[arg(long, help = "The address of the L2 Farcaster Key Registry contract")]
    l2_key_registry_address: Option<String>,

    #[arg(
        long,
        help = "The address of the L2 Farcaster Storage Registry contract"
    )]
    l2_storage_registry_address: Option<String>,

    #[arg(
        long,
        help = "Resync events from the L2 Farcaster contracts before starting (default: disabled)"
    )]
    l2_resync_events: bool,

    #[arg(
        long,
        help = "The block number to begin syncing events from L2 Farcaster contracts"
    )]
    l2_first_block: Option<u64>,

    #[arg(
        long,
        help = "The number of events to fetch from L2 Farcaster contracts at a time"
    )]
    l2_chunk_size: Option<u64>,

    #[arg(
        long,
        help = "The chain ID of the L2 Farcaster contracts are deployed to"
    )]
    l2_chain_id: Option<u64>,

    #[arg(
        long,
        help = "The storage rent expiry in seconds to use instead of the default 1 year (ONLY FOR TESTS)"
    )]
    l2_rent_expiry_override: Option<u64>,
}

#[derive(Parser, Debug)]
struct NetworkingOptions {
    #[arg(
        short = 'a',
        long,
        help = "Only peer with specific peer ids. (default: all peers allowed)"
    )]
    allowed_peers: Option<Vec<String>>,

    #[arg(
        long,
        help = "Do not peer with specific peer ids. (default: no peers denied)"
    )]
    denied_peers: Option<Vec<String>>,

    #[arg(
        short = 'b',
        long,
        help = "Peers to bootstrap gossip and sync from. (default: none)"
    )]
    bootstrap: Option<Vec<String>>,

    #[arg(
        short = 'g',
        long,
        help = "Port to use for gossip (default: DEFAULT_GOSSIP_PORT)"
    )]
    gossip_port: Option<u16>,

    #[arg(
        short = 'r',
        long,
        help = "Port to use for gRPC  (default: DEFAULT_RPC_PORT)"
    )]
    rpc_port: Option<u16>,

    #[arg(long, help = "IP address to listen on (default: '127.0.0.1')")]
    ip: Option<String>,

    #[arg(
        long,
        help = "Public IP address announced to peers (default: fetched with external service)"
    )]
    announce_ip: Option<String>,

    #[arg(
        long,
        help = "Server name announced to peers, useful if SSL/TLS enabled. (default: 'none')"
    )]
    announce_server_name: Option<String>,

    #[arg(
        long,
        help = "A list of peers for libp2p to directly peer with (default: [])"
    )]
    direct_peers: Option<Vec<String>>,

    #[arg(
        long,
        help = "RPC rate limit for peers specified in rpm. Set to -1 for none. (default: 20k/min)"
    )]
    rpc_rate_limit: Option<i32>,
}

#[derive(Parser, Debug)]
struct MetricsOptions {
    #[arg(
        long,
        help = "The host to send statsd metrics to, eg '127.0.0.1:8125'. (default: disabled)"
    )]
    statsd_metrics_server: Option<String>,

    #[arg(
        long,
        help = "Generate tracing and metrics for the gossip network. (default: disabled)"
    )]
    gossip_metrics_enabled: bool,
}

#[derive(Parser, Debug)]
struct DebuggingOptions {
    #[arg(
        long,
        help = "Immediately log to STDOUT, and disable console status and progressbars. (default: disabled)"
    )]
    disable_console_status: bool,

    #[arg(long, help = "Profile a full hub sync and exit. (default: disabled)")]
    profile_sync: bool,

    #[arg(
        long,
        help = "Rebuild the sync trie before starting (default: disabled)"
    )]
    rebuild_sync_trie: bool,

    #[arg(
        long,
        help = "Resync events from the Farcaster contracts before starting (default: disabled)"
    )]
    resync_eth_events: bool,

    #[arg(
        long,
        help = "Resync events from the Fname server before starting (default: disabled)"
    )]
    resync_name_events: bool,

    #[arg(
        long,
        help = "The number of blocks to batch when syncing historical events from Farcaster contracts. (default: DEFAULT_CHUNK_SIZE)"
    )]
    chunk_size: Option<u64>,

    #[arg(
        long,
        help = "Rocks DB commit lock timeout in milliseconds (default: 500)"
    )]
    commit_lock_timeout: Option<u64>,

    #[arg(long, help = "Rocks DB commit lock max pending jobs (default: 1000)")]
    commit_lock_max_pending: Option<u64>,

    #[arg(
        long,
        help = "Require username-password auth for RPC submit. (default: disabled)"
    )]
    rpc_auth: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "start", about = "Start a Hub")]
pub struct StartCommand {
    #[clap(flatten)]
    teleport_options: TeleportOptions,

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
