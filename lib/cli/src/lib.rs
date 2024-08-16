use clap::{Args, Parser, Subcommand};

use self::{
    console::ConsoleCommand,
    identity::{create::CreateIdentityCommand, verify::VerifyIdentityCommand},
    profile::{
        gossip::GossipProfileCommand, rpc::RpcProfileCommand, storage::StorageProfileCommand,
    },
    reset::{events::EventsResetCommand, full::FullResetCommand},
    start::StartCommand,
    status::StatusCommand,
};

pub mod console;
pub mod identity;
pub mod profile;
pub mod reset;
pub mod start;
pub mod status;

#[derive(Parser, Debug)]
#[command(name = "teleport")]
#[command(
    about = "A fast implementation of a Farcaster Hub, in Rust",
    author = "Haardik (haardik@learnweb3.io)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Start(Box<StartCommand>),
    #[command(name = "identity", about = "Create and verify Peer IDs")]
    Identity(IdentityArgs),
    Status(StatusCommand),
    #[command(
        name = "profile",
        about = "Profile the Hub's RPC, Storge, and Gossip Server's performance"
    )]
    Profile(ProfileArgs),
    #[command(name = "reset", about = "Reset parts of the Hub's database")]
    Reset(ResetArgs),
    Console(ConsoleCommand),
}

#[derive(Args, Debug)]
pub struct IdentityArgs {
    #[command(subcommand)]
    pub command: IdentityCommands,
}

#[derive(Subcommand, Debug)]
pub enum IdentityCommands {
    Create(CreateIdentityCommand),
    Verify(VerifyIdentityCommand),
}

#[derive(Args, Debug)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommands,
}

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    Gossip(GossipProfileCommand),
    Rpc(RpcProfileCommand),
    Storage(StorageProfileCommand),
}

#[derive(Args, Debug)]
pub struct ResetArgs {
    #[command(subcommand)]
    pub command: ResetCommands,
}

#[derive(Subcommand, Debug)]
pub enum ResetCommands {
    Events(EventsResetCommand),
    Full(FullResetCommand),
}
