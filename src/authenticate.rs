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

    debug!("entered auth service");

    loop {
        // wait for messages
        match rx.recv().await {
            Some(rx) => match rx {
                Message::Request { msg, reply } => match msg {
                    AuthMessage::Initialize(instance) => {
                        debug!("Initializing masto client");
                        let registration = Registration::new(instance)
                            .client_name("hedgehog")
                            .build()
                            .await
                            .unwrap();
                        let url = registration.authorize_url().unwrap();
                        debug!("authorize url: {}", &url);
                        reply.send(AuthMessage::AuthorizeUrl(url)).unwrap();
                    }
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
    Initialize(String),
    MastodonData(String),
    AuthorizeUrl(String),
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
