use std::sync::Arc;

use crate::TraceBundle;

use self::server::JageServer;

use jage_api as proto;
use parking_lot::RwLock;
use proto::instrument::instrument_server::InstrumentServer;
use tonic::transport::Server;

mod server;

pub fn spawn_server(bundle: Arc<RwLock<TraceBundle>>) {
    tokio::spawn(async {
        let addr = "127.0.0.1:6000".parse().unwrap();
        let mut service = JageServer::new(bundle);
        service.bootstrap();
        Server::builder()
            .add_service(InstrumentServer::new(service))
            .serve(addr)
            .await
            .unwrap();
    });
}
