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
                        let client = build_http_client().unwrap();
                        let registration = Registration::new_with_client(instance, client)
                            .client_name("hedgehog")
                            .redirect_uris("urn:ietf:wg:oauth:2.0:oob")
                            .build()
                            .await
                            .unwrap();
                        debug!("registration created");
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

#[cfg(not(target_arch = "wasm32"))]
fn build_http_client() -> reqwest::Result<reqwest::Client> {
    // Outside of a browser, we must provide a user agent, or some servers will reject us (such as
    // GTS)
    reqwest::Client::builder()
        .user_agent("hedgehog.rs/0.0.0 https://github.com/vivlim/hedgehog")
        .build()
}

#[cfg(target_arch = "wasm32")]
fn build_http_client() -> reqwest::Result<reqwest::Client> {
    // Inside a browser, providing a user agent will cause a CORS error.
    reqwest::Client::builder().build()
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
