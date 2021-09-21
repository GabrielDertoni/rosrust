use tokio::sync::mpsc;

use rosrust::ServicePair;
use rosrust::error::Result;

pub struct Service<S: ServicePair> {
    raii: rosrust::Service,
    rx: mpsc::Receiver<RequestHandle<S>>
}

impl<S: ServicePair> Service<S> {
    fn new(raii: rosrust::Service, rx: mpsc::Receiver<RequestHandle<S>>) -> Service<S> {
        Service { raii, rx }
    }
}

impl<S: ServicePair> Service<S> {
    #[inline]
    pub fn create(topic: impl AsRef<str>) -> Result<Service<S>> {
        service(topic)
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
    tx: mpsc::Sender<std::result::Result<S::Response, String>>,
}

impl<S: ServicePair> RequestHandle<S> {
    fn new_pair(request: S::Request) -> (RequestHandle<S>, mpsc::Receiver<std::result::Result<S::Response, String>>) {
        let (tx, rx) = mpsc::channel(1);
        (RequestHandle { tx, request }, rx)
    }
}

impl<S: ServicePair> RequestHandle<S> {
    pub fn request(&self) -> &S::Request {
        &self.request
    }

    // NOTE: The ideal implementation is for this function to return some kind of
    //       result indicating if the response was send successfully. But in the
    //       current wrapper design, I don't think that's possible.
    pub async fn send_ok(self, response: S::Response) {
        if let Err(_) = self.tx.send(Ok(response)).await {
            panic!("failed to send value");
        }
    }

    pub async fn send_err(self, msg: impl Into<String>) {
        if let Err(_) = self.tx.send(Err(msg.into())).await {
            panic!("failed to send err");
        }
    }
}

impl<S: ServicePair> std::fmt::Debug for RequestHandle<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RequestHandle {{..}}")
    }
}

pub fn service<S: ServicePair>(topic: impl AsRef<str>) -> Result<Service<S>> {
    let (tx, rx) = mpsc::channel(1);

    let raii = rosrust::service::<S, _>(
        topic.as_ref(),
        move |req: S::Request| -> std::result::Result<S::Response, String> {
            let (handle, mut response) = RequestHandle::new_pair(req);
            tx.blocking_send(handle).unwrap();
            response.blocking_recv().unwrap()
        }
    )?;

    Ok(Service::new(raii, rx))
}
