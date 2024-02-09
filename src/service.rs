use std::pin::Pin;

use instant::Duration;

use log::{debug, warn};
use mastodon_async::{Mastodon, Registration};
use tokio::sync::{self, mpsc};

use crate::{
    authenticate::{start_auth_service, AuthMessage},
    channels::{new_channel_pair, AsyncRequestBridge, Message, Spawner},
};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::*;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::*;

pub fn new_async_service_channels() -> (
    mpsc::Sender<Message<AsyncServiceMessage>>,
    mpsc::Receiver<Message<AsyncServiceMessage>>,
) {
    mpsc::channel(255)
}

pub fn start_async_service() -> mpsc::Sender<Message<AsyncServiceMessage>> {
    let spawner = Spawner::new();
    let spawner_clone = spawner.clone();

    let (ui_async_tx, svc_async_rx) = new_channel_pair::<AsyncServiceMessage>();

    spawner.spawn_root(Box::pin(async move {
        debug!("start async service");
        start_async_service_impl(svc_async_rx, spawner_clone).await;
        warn!("done async service.");
    }));

    ui_async_tx
}

pub async fn start_async_service_impl(
    mut rx: mpsc::Receiver<Message<AsyncServiceMessage>>,
    spawner: Spawner,
) {
    let mut state: AsyncServiceState = Default::default();

    loop {
        // wait for messages
        match rx.recv().await {
            Some(rx) => match rx {
                Message::Request { msg, reply } => match msg {
                    AsyncServiceMessage::Echo(n) => {
                        debug!("receive message. waiting 2 secs");
                        sleep(Duration::from_secs(2)).await;
                        match reply.send(AsyncServiceMessage::Echo(n + 1)) {
                            Ok(_) => debug!("replied"),
                            Err(e) => warn!("Failed to send echo reply for {}", n),
                        }
                    }
                    AsyncServiceMessage::StartAuth => {
                        debug!("received start auth message");
                        let (auth_tx, auth_rx) = new_channel_pair::<AuthMessage>();
                        spawner.spawn_async(async {
                            start_auth_service(auth_rx).await;
                        });
                        //let auth_bridge = AsyncRequestBridge::<AuthMessage, u32>::new(auth_tx);
                        match reply.send(AsyncServiceMessage::AuthChannel(auth_tx)) {
                            Ok(_) => debug!("replied"),
                            Err(e) => warn!("Failed to send auth service tx"),
                        }
                    }
                    _ => todo!("unhandled service message"),
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

pub enum AsyncServiceMessage {
    Echo(u32),
    StartAuth,
    AuthChannel(sync::mpsc::Sender<Message<AuthMessage>>),
}

#[derive(Default)]
struct AsyncServiceState {}
