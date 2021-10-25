use rosrust::error::Result as RosResult;
use rosrust::ServicePair;
use tokio::sync::mpsc;

use crate::oneshot_blocking as oneshot;

pub struct Service<S: ServicePair> {
    raii: rosrust::Service,
    // This should really be a Single Producer, Single Consumer. But there is no such
    // channel in the Tokio crate.
    rx: mpsc::Receiver<RequestHandle<S>>,
}

impl<S: ServicePair> Service<S> {
    pub fn new(topic: impl AsRef<str>) -> RosResult<Service<S>> {
        let (tx, rx) = mpsc::channel(1);

        let raii = rosrust::service::<S, _>(
            topic.as_ref(),
            move |req: S::Request| -> Result<S::Response, String> {
                let (response, handle) = RequestHandle::new_pair(req);
                tx.blocking_send(handle).unwrap();

                match response.recv() {
                    Ok(resp) => return resp,
                    Err(_) => panic!("Handle was dropped before responding"),
                }
            },
        )?;

        Ok(Service { raii, rx })
    }

    #[inline]
    pub async fn next_request(&mut self) -> RequestHandle<S> {
        self.rx.recv().await.unwrap()
    }
}

impl<S: ServicePair> std::ops::Deref for Service<S> {
    type Target = rosrust::Service;

    fn deref(&self) -> &rosrust::Service {
        &self.raii
    }
}

pub struct RequestHandle<S: ServicePair> {
    request: S::Request,
    tx: oneshot::Sender<Result<S::Response, String>>,
}

impl<S: ServicePair> RequestHandle<S> {
    fn new_pair(request: S::Request) -> (oneshot::Receiver<Result<S::Response, String>>, RequestHandle<S>) {
        let (tx, rx) = oneshot::channel();
        (rx, RequestHandle { request, tx })
    }
}

impl<S: ServicePair> RequestHandle<S> {
    pub fn request(&self) -> &S::Request {
        &self.request
    }

    // NOTE: The ideal implementation is for this function to return some kind of
    //       result indicating if the response was send successfully. But in the
    //       current wrapper design, I don't think that's possible.
    pub fn send_ok(self, response: S::Response) {
        if let Err(_) = self.tx.send(Ok(response)) {
            panic!("failed to send value");
        }
    }


    pub fn send_err(self, msg: impl Into<String>) {
        if let Err(_) = self.tx.send(Err(msg.into())) {
            panic!("failed to send value");
        }
    }
}

impl<S: ServicePair> std::fmt::Debug for RequestHandle<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RequestHandle {{..}}")
    }
}
