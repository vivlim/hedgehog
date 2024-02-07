use std::time::Duration;

use log::{debug, warn};
use tokio::sync::mpsc;

use crate::channels::Message;

pub fn new_async_service_channels() -> (
    mpsc::Sender<Message<AsyncServiceMessage>>,
    mpsc::Receiver<Message<AsyncServiceMessage>>,
) {
    mpsc::channel(255)
}
pub async fn start_async_service(
    mut tx: mpsc::Sender<Message<AsyncServiceMessage>>,
    mut rx: mpsc::Receiver<Message<AsyncServiceMessage>>,
) {
    loop {
        // wait for messages
        match rx.recv().await {
            Some(rx) => match rx {
                Message::Request { msg, reply } => match msg {
                    AsyncServiceMessage::Echo(n) => {
                        debug!("receive message. waiting 2 secs");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        match reply.send(AsyncServiceMessage::Echo(n + 1)) {
                            Ok(_) => debug!("replied"),
                            Err(e) => warn!("Failed to send echo reply for {}, {:?}", n, e),
                        }
                    }
                },
                Message::Notification { msg } => warn!("Unhandled mssage type"),
            },
            None => {
                debug!("Async service out of messages");
                break;
            }
        };
    }
}

#[derive(Debug)]
pub enum AsyncServiceMessage {
    Echo(u32),
}
