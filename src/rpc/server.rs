use crate::common::protobufs::generated::{hub_service_server::HubService, *};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct HubServer {}

#[tonic::async_trait]
impl HubService for HubServer {
    async fn submit_message(&self, request: Request<Message>) -> Result<Response<Message>, Status> {
        todo!()
    }

    type SubscribeStream = ReceiverStream<Result<HubEvent, Status>>;
    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        todo!()
    }

    async fn get_event(
        &self,
        request: Request<EventRequest>,
    ) -> Result<Response<HubEvent>, Status> {
        todo!()
    }

    async fn get_cast(&self, request: Request<CastId>) -> Result<Response<Message>, Status> {
        todo!()
    }

    async fn get_casts_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }

    async fn get_casts_by_parent(
        &self,
        request: Request<CastsByParentRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_casts_by_mention(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Reactions
    async fn get_reaction(
        &self,
        request: Request<ReactionRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_reactions_by_fid(
        &self,
        request: Request<ReactionsByFidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_reactions_by_cast(
        &self,
        request: Request<ReactionsByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_reactions_by_target(
        &self,
        request: Request<ReactionsByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// User Data
    async fn get_user_data(
        &self,
        request: Request<UserDataRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_user_data_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_name_registry_event(
        &self,
        request: Request<NameRegistryEventRequest>,
    ) -> Result<Response<NameRegistryEvent>, Status> {
        todo!()
    }
    async fn get_on_chain_events(
        &self,
        request: Request<OnChainEventRequest>,
    ) -> Result<Response<OnChainEventResponse>, Status> {
        todo!()
    }
    async fn get_current_storage_limits_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<StorageLimitsResponse>, Status> {
        todo!()
    }
    /// Username Proof
    async fn get_username_proof(
        &self,
        request: Request<UsernameProofRequest>,
    ) -> Result<Response<UserNameProof>, Status> {
        todo!()
    }
    async fn get_user_name_proofs_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<UsernameProofsResponse>, Status> {
        todo!()
    }
    /// Verifications
    async fn get_verification(
        &self,
        request: Request<VerificationRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_verifications_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Signer
    async fn get_signer(
        &self,
        request: Request<SignerRequest>,
    ) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_on_chain_signer(
        &self,
        request: Request<SignerRequest>,
    ) -> Result<Response<OnChainEvent>, Status> {
        todo!()
    }
    async fn get_signers_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_id_registry_event(
        &self,
        request: Request<IdRegistryEventRequest>,
    ) -> Result<Response<IdRegistryEvent>, Status> {
        todo!()
    }
    async fn get_id_registry_event_by_address(
        &self,
        request: Request<IdRegistryEventByAddressRequest>,
    ) -> Result<Response<IdRegistryEvent>, Status> {
        todo!()
    }
    async fn get_fids(
        &self,
        request: Request<FidsRequest>,
    ) -> Result<Response<FidsResponse>, Status> {
        todo!()
    }
    /// Links
    async fn get_link(&self, request: Request<LinkRequest>) -> Result<Response<Message>, Status> {
        todo!()
    }
    async fn get_links_by_fid(
        &self,
        request: Request<LinksByFidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_links_by_target(
        &self,
        request: Request<LinksByTargetRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Bulk Methods
    async fn get_all_cast_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_reaction_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_verification_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_signer_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_user_data_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_all_link_messages_by_fid(
        &self,
        request: Request<FidRequest>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    /// Sync Methods
    async fn get_info(
        &self,
        request: Request<HubInfoRequest>,
    ) -> Result<Response<HubInfoResponse>, Status> {
        todo!()
    }
    async fn get_sync_status(
        &self,
        request: Request<SyncStatusRequest>,
    ) -> Result<Response<SyncStatusResponse>, Status> {
        todo!()
    }
    async fn get_all_sync_ids_by_prefix(
        &self,
        request: Request<TrieNodePrefix>,
    ) -> Result<Response<SyncIds>, Status> {
        todo!()
    }
    async fn get_all_messages_by_sync_ids(
        &self,
        request: Request<SyncIds>,
    ) -> Result<Response<MessagesResponse>, Status> {
        todo!()
    }
    async fn get_sync_metadata_by_prefix(
        &self,
        request: Request<TrieNodePrefix>,
    ) -> Result<Response<TrieNodeMetadataResponse>, Status> {
        todo!()
    }
    async fn get_sync_snapshot_by_prefix(
        &self,
        request: Request<TrieNodePrefix>,
    ) -> Result<Response<TrieNodeSnapshotResponse>, Status> {
        todo!()
    }
}
