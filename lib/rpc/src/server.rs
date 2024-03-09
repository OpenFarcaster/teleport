use teleport_common::protobufs::generated::{hub_service_server::HubService, *};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct HubServer {}

#[tonic::async_trait]
impl HubService for HubServer {
    async fn submit_message(
        &self,
        _request: Request<Message>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }

    type SubscribeStream = ReceiverStream<Result<HubEvent, Status>>;
    async fn subscribe(
        &self,
        _request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        todo!()
    }

    async fn validate_message(
        &self,
        _request: Request<Message>,
    ) -> Result<Response<ValidationResponse>, Status> {
        todo!()
    }

    async fn get_event(
        &self,
        _request: Request<EventRequest>,
    ) -> Result<Response<HubEvent>, Status> {
        todo!()
    }

    async fn get_cast(&self, _request: Request<CastId>) -> Result<Response<Message>, Status> {
        todo!()
    }

    async fn get_casts_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }

    async fn get_casts_by_parent(
        &self,
        _request: Request<CastsByParentRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_casts_by_mention(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Reactions
    async fn get_reaction(
        &self,
        _request: Request<ReactionRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_reactions_by_fid(
        &self,
        _request: Request<ReactionsByFidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_reactions_by_cast(
        &self,
        _request: Request<ReactionsByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_reactions_by_target(
        &self,
        _request: Request<ReactionsByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// User Data
    async fn get_user_data(
        &self,
        _request: Request<UserDataRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_user_data_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }

    async fn get_on_chain_events(
        &self,
        _request: Request<OnChainEventRequest>,
    ) -> Result<Response<OnChainEventResponse>, Status> {
        todo!()
    }
    async fn get_id_registry_on_chain_event(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<OnChainEvent>, Status> {
        todo!()
    }
    async fn get_id_registry_on_chain_event_by_address(
        &self,
        _request: Request<IdRegistryEventByAddressRequest>,
    ) -> Result<Response<OnChainEvent>, Status> {
        todo!()
    }
    async fn get_current_storage_limits_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<StorageLimitsResponse>, Status> {
        todo!()
    }
    /// Username Proof
    async fn get_username_proof(
        &self,
        _request: Request<UsernameProofRequest>,
    ) -> Result<Response<UserNameProof>, Status> {
        todo!()
    }
    async fn get_user_name_proofs_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<UsernameProofsResponse>, Status> {
        todo!()
    }
    /// Verifications
    async fn get_verification(
        &self,
        _request: Request<VerificationRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_verifications_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Signer
    async fn get_on_chain_signer(
        &self,
        _request: Request<SignerRequest>,
    ) -> Result<Response<OnChainEvent>, Status> {
        todo!()
    }
    async fn get_on_chain_signers_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<OnChainEventResponse>, Status> {
        todo!()
    }

    async fn get_fids(
        &self,
        _request: Request<FidsRequest>,
    ) -> Result<Response<FidsResponse>, Status> {
        todo!()
    }
    /// Links
    async fn get_link(&self, _request: Request<LinkRequest>) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_links_by_fid(
        &self,
        _request: Request<LinksByFidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_links_by_target(
        &self,
        _request: Request<LinksByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Bulk Methods
    async fn get_all_cast_messages_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_reaction_messages_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_verification_messages_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }

    async fn get_all_user_data_messages_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_link_messages_by_fid(
        &self,
        _request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Sync Methods
    async fn get_info(
        &self,
        _request: Request<HubInfoRequest>,
    ) -> Result<Response<HubInfoResponse>, Status> {
        todo!()
    }
    async fn get_sync_status(
        &self,
        _request: Request<SyncStatusRequest>,
    ) -> Result<Response<SyncStatusResponse>, Status> {
        todo!()
    }
    async fn get_all_sync_ids_by_prefix(
        &self,
        _request: Request<TrieNodePrefix>,
    ) -> Result<Response<SyncIds>, Status> {
        todo!()
    }
    async fn get_all_messages_by_sync_ids(
        &self,
        _request: Request<SyncIds>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_sync_metadata_by_prefix(
        &self,
        _request: Request<TrieNodePrefix>,
    ) -> Result<Response<TrieNodeMetadataResponse>, Status> {
        todo!()
    }
    async fn get_sync_snapshot_by_prefix(
        &self,
        _request: Request<TrieNodePrefix>,
    ) -> Result<Response<TrieNodeSnapshotResponse>, Status> {
        todo!()
    }
}
