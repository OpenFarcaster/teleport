use crate::core::protobufs::generated::{admin_service_server::AdminService, *};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct AdminServer {}

#[tonic::async_trait]
impl AdminService for AdminServer {
    async fn rebuild_sync_trie(
        &self,
        request: tonic::Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        todo!()
    }
    async fn delete_all_messages_from_db(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        todo!()
    }
    async fn submit_id_registry_event(
        &self,
        request: Request<IdRegistryEvent>,
    ) -> Result<Response<IdRegistryEvent>, Status> {
        todo!()
    }
    async fn submit_name_registry_event(
        &self,
        request: Request<NameRegistryEvent>,
    ) -> Result<Response<NameRegistryEvent>, Status> {
        todo!()
    }
    async fn submit_on_chain_event(
        &self,
        request: Request<OnChainEvent>,
    ) -> Result<Response<OnChainEvent>, Status> {
        todo!()
    }
}
