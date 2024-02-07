use std::mem;

use log::{debug, warn};
use tokio::sync::{
    self,
    oneshot::{self, error::TryRecvError},
};

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

    pub fn pump_messages(&mut self) {
        let reciever = match &mut self.state {
            AsyncRequestBridgeState::Awaiting {
                response,
                prev_state,
                handler,
            } => Some(response),
            _ => None,
        };

        if reciever.is_none() {
            return;
        }

        let mut incoming_msg: Result<TMsg, TryRecvError> = reciever.unwrap().try_recv();

        // Early exit
        match incoming_msg {
            Ok(_) => (),
            Err(oneshot::error::TryRecvError::Empty) => {
                return;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                warn!("Response channel closed unexpectedly");
                return;
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
        } else {
            debug!("Unexpected: previous state was not awaiting?");
        }
    }
}
