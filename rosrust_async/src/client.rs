use tokio::task;

use rosrust::api::error::tcpros::Result as TCPResult;
use rosrust::error::Result as RosResult;
use rosrust::ServicePair;

#[derive(Clone)]
pub struct Client<Srv: ServicePair> {
    cli: rosrust::Client<Srv>,
}

impl<Srv: ServicePair> Client<Srv> {
    pub fn new(topic: impl AsRef<str>) -> RosResult<Self> {
        let cli = rosrust::client(topic.as_ref())?;

        Ok(Client {
            cli
        })
    }

    pub async fn req(&self, req: Srv::Request) -> TCPResult<Result<Srv::Response, String>> {
        let cli = self.cli.clone();
        task::spawn_blocking(move || cli.req(&req))
            .await
            .unwrap()
    }
}