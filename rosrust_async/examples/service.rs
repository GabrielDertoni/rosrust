use tokio::signal;

use rosrust_async::Service;

mod msg {
    rosrust::rosmsg_include!(roscpp_tutorials/TwoInts);
}

async fn handle_requests() {
    while let handle = service.next_request().await {
        rosrust::ros_info!("Received a request!");
        tokio::spawn(async move {
            let req = handle.request();
            let sum = req.a + req.b;
            handle.send_ok(msg::roscpp_tutorials::TwoIntsRes { sum }).await;
            rosrust::ros_info!("Sent a response!");
        });
    }
}

#[tokio::main]
async fn main() {
    rosrust::init("add_two_ints");
    
    let mut service: Service<msg::roscpp_tutorials::TwoInts> = Service::create("add_two_ints").unwrap();

    tokio::select! {
        _ = handle_requests() => (),
        _ = signal.ctrl_c()   => (),
    }
}