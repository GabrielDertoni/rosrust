mod subscriber;
mod publisher;
mod service;
mod client;
mod action;

mod oneshot_blocking;

pub use subscriber::*;
pub use publisher::*;
pub use service::*;
pub use client::*;
pub use action::*;

use std::time::Duration;
use rosrust::error::{Result as RosResult, Error as RosError, ErrorKind as RosErrorKind};

pub async fn wait_until_available(topic: String) -> RosResult<()> {
    tokio::task::spawn_blocking(move || loop {
        match rosrust::wait_for_service(&topic, Some(Duration::from_millis(100))) {
            Err(RosError(RosErrorKind::TimeoutError, _)) => {
                if rosrust::is_ok() { continue }
                return Err("Shutdown".into());
            }
            Ok(_) => return Ok(()),
            e => return e,
        }
    })
    .await
    .unwrap()
}
