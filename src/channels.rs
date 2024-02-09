use std::mem;

use log::{debug, warn};
use tokio::sync::{
    self, mpsc,
    oneshot::{self, error::TryRecvError},
};

pub fn new_channel_pair<T>() -> (mpsc::Sender<Message<T>>, mpsc::Receiver<Message<T>>) {
    mpsc::channel(255)
}

pub enum Message<TMsg> {
    Request {
        msg: TMsg,
        reply: oneshot::Sender<TMsg>,
    },
    Notification {
        msg: TMsg,
    },
}

pub struct AsyncRequestBridge<TMsg, TState> {
    tx: sync::mpsc::Sender<Message<TMsg>>,
    pub state: AsyncRequestBridgeState<TMsg, TState>,
}

pub enum AsyncRequestBridgeState<TMsg, TState> {
    Init,
    Awaiting {
        response: oneshot::Receiver<TMsg>,
        prev_state: Option<TState>,
        handler: Box<dyn FnOnce(TMsg, Option<TState>) -> TState>,
    },
    Updating,
    Complete(TState),
    Error(String),
}

impl<TMsg, TState> AsyncRequestBridge<TMsg, TState> {
    pub fn new(tx: sync::mpsc::Sender<Message<TMsg>>) -> AsyncRequestBridge<TMsg, TState> {
        AsyncRequestBridge {
            tx,
            state: AsyncRequestBridgeState::Init,
        }
    }
    pub fn send(&mut self, msg: TMsg, handler: Box<dyn FnOnce(TMsg, Option<TState>) -> TState>) {
        // Only one outbound request at a time
        let prev_state = match mem::replace(&mut self.state, AsyncRequestBridgeState::Updating) {
            AsyncRequestBridgeState::Init => None,
            AsyncRequestBridgeState::Awaiting {
                mut response,
                prev_state,
                handler: _,
            } => {
                // Cancel the previous request and start a new one.
                response.close();
                prev_state
            }
            AsyncRequestBridgeState::Updating => {
                warn!("Tried to send an outbound request for a bridge that was in the updating state, which should be impossible.");
                None
            }
            AsyncRequestBridgeState::Complete(s) => Some(s),
            AsyncRequestBridgeState::Error(_) => None,
        };
        let (resp_tx, resp_rx) = oneshot::channel();
        self.state = AsyncRequestBridgeState::Awaiting {
            response: resp_rx,
            prev_state,
            handler,
        };
        match self.tx.blocking_send(Message::Request {
            msg,
            reply: resp_tx,
        }) {
            Ok(_) => (),
            Err(e) => {
                warn!("Failed to send request: {:?}", e);
                self.state = AsyncRequestBridgeState::Error(format!("Failed to send req: {:?}", e));
            }
        }
    }

    pub fn pump_messages(&mut self) -> bool {
        let reciever = match &mut self.state {
            AsyncRequestBridgeState::Awaiting {
                response,
                prev_state,
                handler,
            } => Some(response),
            _ => None,
        };

        if reciever.is_none() {
            return false;
        }

        let mut incoming_msg: Result<TMsg, TryRecvError> = reciever.unwrap().try_recv();

        // Early exit
        match incoming_msg {
            Ok(_) => (),
            Err(oneshot::error::TryRecvError::Empty) => {
                return false;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                warn!("Response channel closed unexpectedly");
                return false;
            }
        };

        let incoming_msg = incoming_msg.unwrap();

        if let AsyncRequestBridgeState::Awaiting {
            response,
            prev_state,
            handler,
        } = mem::replace(&mut self.state, AsyncRequestBridgeState::Updating)
        {
            debug!("Bridge handling incoming message");
            let new_state = handler(incoming_msg, prev_state);
            self.state = AsyncRequestBridgeState::Complete(new_state);
            return true;
        } else {
            debug!("Unexpected: previous state was not awaiting?");
            return false;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct Spawner {
    rt: std::sync::Arc<tokio::runtime::Runtime>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Spawner {
    pub fn new() -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Unable to create tokio runtime");
        let _enter = rt.enter();

        Spawner {
            rt: std::sync::Arc::new(rt),
        }
    }

    pub fn spawn_root<F>(self, f: F) -> Self
    where
        F: std::future::Future + std::marker::Send + 'static,
        <F as std::future::Future>::Output: Send,
    {
        let rt_arc = self.rt.clone();
        std::thread::spawn(move || {
            rt_arc.block_on(async {
                debug!("start async service");
                f.await;
                debug!("finished async service.");
            })
        });
        self
    }

    pub fn spawn_async<F>(&self, f: F)
    where
        F: std::future::Future + std::marker::Send + 'static,
        <F as std::future::Future>::Output: Send,
    {
        debug!("enter spawn_async");
        self.rt.spawn(f);
        debug!("exit spawn_async");
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct Spawner {}

#[cfg(target_arch = "wasm32")]
impl Spawner {
    pub fn new() -> Self {
        Spawner {}
    }

    pub fn spawn_root<F>(self, f: F) -> Self
    where
        F: std::future::Future + 'static,
    {
        self.spawn_async(f);
        self
    }

    pub fn spawn_async<F>(&self, f: F)
    where
        F: std::future::Future + 'static,
    {
        wasm_bindgen_futures::spawn_local(async {
            f.await;
        });
    }
}
