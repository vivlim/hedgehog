use instant::Duration;

use log::{debug, warn};
use mastodon_async::{Mastodon, Registration};
use tokio::sync::mpsc;

use crate::channels::Message;

#[cfg(not(target_arch = "wasm"))]
use tokio::time::*;
#[cfg(target_arch = "wasm")]
use wasmtimer::tokio::*;

pub async fn start_auth_service(mut rx: mpsc::Receiver<Message<AuthMessage>>) {
    let mut state: AuthState = Default::default();

    loop {
        // wait for messages
        match rx.recv().await {
            Some(rx) => match rx {
                Message::Request { msg, reply } => match msg {
                    AuthMessage::Echo(n) => {
                        debug!("receive message. waiting 2 secs");
                        //sleep(Duration::from_secs(2)).await; // This panics on wasm, not
                        //essential so comment it out
                        match reply.send(AuthMessage::Echo(n + 1)) {
                            Ok(_) => debug!("replied"),
                            Err(e) => warn!("Failed to send echo reply for {}, {:?}", n, e),
                        }
                    }
                    AuthMessage::Initialize => {}
                    _ => {}
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
pub enum AuthMessage {
    Echo(u32),
    Initialize,
    MastodonData(String),
}

#[derive(Default)]
struct AuthState {
    mastodon: Option<Mastodon>,
}

/*
async fn register(url: String) -> Result<Mastodon> {
    let registration = Registration::new(url)
        .client_name("hedgehog")
        .build()
        .await?;

    Ok(mastodon)
}
*/
