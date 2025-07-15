use crate::async_::peer::Peer;
use crate::types::{
    CallRequest, CallResponse, Error, Event as XEvent, EventFn, Invocation as XInvocation, PublishRequest,
    PublishResponse, RegisterFn, RegisterRequest, RegisterResponse, SessionDetails, SubscribeRequest,
    SubscribeResponse, WampError,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use wampproto::idgen::SessionScopeIDGenerator;
use wampproto::messages::call::{Call, MESSAGE_TYPE_CALL};
use wampproto::messages::error::{Error as ErrorMsg, MESSAGE_TYPE_ERROR};
use wampproto::messages::event::{Event, MESSAGE_TYPE_EVENT};
use wampproto::messages::goodbye::{Goodbye, MESSAGE_TYPE_GOODBYE};
use wampproto::messages::invocation::{Invocation, MESSAGE_TYPE_INVOCATION};
use wampproto::messages::message::Message;
use wampproto::messages::publish::{MESSAGE_TYPE_PUBLISH, Publish};
use wampproto::messages::published::{MESSAGE_TYPE_PUBLISHED, Published};
use wampproto::messages::register::{MESSAGE_TYPE_REGISTER, Register};
use wampproto::messages::registered::{MESSAGE_TYPE_REGISTERED, Registered};
use wampproto::messages::result::{MESSAGE_TYPE_RESULT, Result_};
use wampproto::messages::subscribe::{MESSAGE_TYPE_SUBSCRIBE, Subscribe};
use wampproto::messages::subscribed::{MESSAGE_TYPE_SUBSCRIBED, Subscribed};
use wampproto::messages::types::Value;
use wampproto::messages::unregister::MESSAGE_TYPE_UNREGISTER;
use wampproto::messages::unregistered::{MESSAGE_TYPE_UNREGISTERED, Unregistered};
use wampproto::messages::unsubscribe::MESSAGE_TYPE_UNSUBSCRIBE;
use wampproto::messages::unsubscribed::{MESSAGE_TYPE_UNSUBSCRIBED, Unsubscribed};
use wampproto::messages::yield_::Yield;
use wampproto::serializers::serializer::Serializer;
use wampproto::session::Session as ProtoSession;

#[derive(Debug)]
pub struct Session {
    _details: SessionDetails,
    proto: Arc<ProtoSession>,
    idgen: SessionScopeIDGenerator,
    peer: Arc<Box<dyn Peer>>,

    state: Arc<State>,
    goodbye_receiver_channel: Mutex<mpsc::Receiver<()>>,
    exist_receiver_channel: Mutex<mpsc::Receiver<()>>,
}

#[derive(Debug)]
struct State {
    // RPC states
    call_requests: Mutex<HashMap<i64, mpsc::Sender<CallResponse>>>,
    register_requests: Mutex<HashMap<i64, mpsc::Sender<RegisterResponse>>>,
    unregister_requests: Mutex<HashMap<i64, mpsc::Sender<Option<WampError>>>>,
    registrations: Mutex<HashMap<i64, RegisterFn>>,

    // PubSub states
    publish_requests: Mutex<HashMap<i64, mpsc::Sender<PublishResponse>>>,
    subscribe_requests: Mutex<HashMap<i64, mpsc::Sender<SubscribeResponse>>>,
    unsubscribe_requests: Mutex<HashMap<i64, mpsc::Sender<Option<WampError>>>>,
    subscriptions: Mutex<HashMap<i64, EventFn>>,

    // goodbye stuff
    goodbye_sent: Mutex<bool>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            call_requests: Default::default(),
            register_requests: Default::default(),
            unregister_requests: Default::default(),
            registrations: Default::default(),
            publish_requests: Default::default(),
            subscribe_requests: Default::default(),
            unsubscribe_requests: Default::default(),
            subscriptions: Default::default(),

            goodbye_sent: Mutex::new(false),
        }
    }
}

impl Session {
    pub fn new(details: SessionDetails, peer: Box<dyn Peer>, serializer: Box<dyn Serializer>) -> Self {
        let stored_proto = Arc::new(ProtoSession::new(serializer));
        let task_proto = stored_proto.clone();

        let stored_state = Arc::new(State::default());
        let task_state = stored_state.clone();

        let stored_peer = Arc::new(peer);
        let task_peer = stored_peer.clone();

        let (goodbye_sender, goodbye_receiver): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel(1);
        let (exit_sender, exit_receiver): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel(1);

        tokio::spawn(async move {
            while let Ok(payload) = task_peer.read().await {
                match task_proto.receive(payload) {
                    Ok(msg) => {
                        Self::process_incoming_message(
                            msg,
                            task_state.clone(),
                            task_proto.clone(),
                            task_peer.clone(),
                            goodbye_sender.clone(),
                            exit_sender.clone(),
                        )
                        .await;
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        break;
                    }
                }
            }
        });

        Self {
            _details: details,
            peer: stored_peer,
            proto: stored_proto,
            idgen: SessionScopeIDGenerator::new(),

            state: stored_state,
            goodbye_receiver_channel: Mutex::new(goodbye_receiver),
            exist_receiver_channel: Mutex::new(exit_receiver),
        }
    }

    async fn process_incoming_message(
        msg: Box<dyn Message>,
        state: Arc<State>,
        proto: Arc<ProtoSession>,
        peer: Arc<Box<dyn Peer>>,
        goodbye_sender: mpsc::Sender<()>,
        exist_sender: mpsc::Sender<()>,
    ) {
        match msg.message_type() {
            MESSAGE_TYPE_REGISTERED => {
                let registered = msg.as_any().downcast_ref::<Registered>().unwrap();
                let mut register_requests = state.register_requests.lock().await;
                if let Some(callback) = register_requests.remove(&registered.request_id) {
                    _ = callback
                        .send(RegisterResponse {
                            registration_id: registered.registration_id,
                            error: None,
                        })
                        .await;
                }
            }
            MESSAGE_TYPE_UNREGISTERED => {
                let unregistered = msg.as_any().downcast_ref::<Unregistered>().unwrap();
                let mut unregister_requests = state.unregister_requests.lock().await;
                if let Some(callback) = unregister_requests.remove(&unregistered.request_id) {
                    _ = callback.send(None).await;
                }
            }
            MESSAGE_TYPE_RESULT => {
                let result = msg.as_any().downcast_ref::<Result_>().unwrap();
                let mut call_requests = state.call_requests.lock().await;
                if let Some(callback) = call_requests.remove(&result.request_id) {
                    _ = callback
                        .send(CallResponse {
                            args: result.args.clone(),
                            kwargs: result.kwargs.clone(),
                            error: None,
                        })
                        .await;
                }
            }
            MESSAGE_TYPE_INVOCATION => {
                let invocation = msg.as_any().downcast_ref::<Invocation>().unwrap();
                let registrations = state.registrations.lock().await;
                let callback = registrations.get(&invocation.registration_id).cloned();
                if callback.is_none() {
                    return;
                }

                let inv = XInvocation {
                    args: invocation.args.clone(),
                    kwargs: invocation.kwargs.clone(),
                    details: Some(invocation.details.clone()),
                };

                let request_id = invocation.request_id;
                let callback = callback.unwrap();
                tokio::spawn(async move {
                    let response = callback(inv);
                    let yield_ = Yield {
                        request_id,
                        options: Default::default(),
                        args: response.args,
                        kwargs: response.kwargs,
                    };

                    match proto.send_message(Box::new(yield_)) {
                        Ok(to_send) => match peer.write(to_send).await {
                            Ok(()) => {}
                            Err(e) => {
                                eprintln!("Error sending message: {e}");
                            }
                        },
                        Err(e) => {
                            eprintln!("Error sending message: {e}");
                        }
                    }
                });
            }
            MESSAGE_TYPE_SUBSCRIBED => {
                let subscribed = msg.as_any().downcast_ref::<Subscribed>().unwrap();
                let mut subscribe_requests = state.subscribe_requests.lock().await;
                if let Some(callback) = subscribe_requests.remove(&subscribed.request_id) {
                    _ = callback
                        .send(SubscribeResponse {
                            subscription_id: subscribed.subscription_id,
                            error: None,
                        })
                        .await;
                }
            }
            MESSAGE_TYPE_UNSUBSCRIBED => {
                let unsubscribed = msg.as_any().downcast_ref::<Unsubscribed>().unwrap();
                let mut unsubscribe_requests = state.unsubscribe_requests.lock().await;
                if let Some(callback) = unsubscribe_requests.remove(&unsubscribed.request_id) {
                    _ = callback.send(None).await;
                }
            }
            MESSAGE_TYPE_PUBLISHED => {
                let published = msg.as_any().downcast_ref::<Published>().unwrap();
                let mut publish_requests = state.publish_requests.lock().await;
                if let Some(callback) = publish_requests.remove(&published.request_id) {
                    _ = callback.send(PublishResponse { error: None }).await;
                }
            }
            MESSAGE_TYPE_EVENT => {
                let event = msg.as_any().downcast_ref::<Event>().unwrap();
                let subscriptions = state.subscriptions.lock().await;
                if let Some(callback) = subscriptions.get(&event.subscription_id) {
                    let xevent = XEvent {
                        args: event.args.clone(),
                        kwargs: event.kwargs.clone(),
                        details: Some(event.details.clone()),
                    };

                    let callback = *callback;
                    tokio::spawn(async move {
                        callback(xevent);
                    });
                }
            }
            MESSAGE_TYPE_ERROR => {
                let error = msg.as_any().downcast_ref::<ErrorMsg>().unwrap();
                match error.message_type {
                    MESSAGE_TYPE_CALL => {
                        let mut call_requests = state.call_requests.lock().await;
                        if let Some(response) = call_requests.remove(&error.request_id) {
                            let _ = response
                                .send(CallResponse {
                                    args: None,
                                    kwargs: None,
                                    error: Some(WampError {
                                        uri: error.uri.clone(),
                                        args: error.args.clone(),
                                        kwargs: error.kwargs.clone(),
                                    }),
                                })
                                .await;
                        }
                    }

                    MESSAGE_TYPE_REGISTER => {
                        let mut register_requests = state.register_requests.lock().await;
                        if let Some(response) = register_requests.remove(&error.request_id) {
                            let _ = response
                                .send(RegisterResponse {
                                    registration_id: 0,
                                    error: Some(WampError {
                                        uri: error.uri.clone(),
                                        args: error.args.clone(),
                                        kwargs: error.kwargs.clone(),
                                    }),
                                })
                                .await;
                        }
                    }

                    MESSAGE_TYPE_UNREGISTER => {
                        let mut unregister_requests = state.unregister_requests.lock().await;
                        if let Some(response) = unregister_requests.remove(&error.request_id) {
                            let _ = response
                                .send(Some(WampError {
                                    uri: error.uri.clone(),
                                    args: error.args.clone(),
                                    kwargs: error.kwargs.clone(),
                                }))
                                .await;
                        }
                    }

                    MESSAGE_TYPE_SUBSCRIBE => {
                        let mut subscribe_requests = state.subscribe_requests.lock().await;
                        if let Some(response) = subscribe_requests.remove(&error.request_id) {
                            let _ = response
                                .send(SubscribeResponse {
                                    subscription_id: 0,
                                    error: Some(WampError {
                                        uri: error.uri.clone(),
                                        args: error.args.clone(),
                                        kwargs: error.kwargs.clone(),
                                    }),
                                })
                                .await;
                        }
                    }

                    MESSAGE_TYPE_UNSUBSCRIBE => {
                        let mut unsubscribe_requests = state.unsubscribe_requests.lock().await;
                        if let Some(response) = unsubscribe_requests.remove(&error.request_id) {
                            let _ = response
                                .send(Some(WampError {
                                    uri: error.uri.clone(),
                                    args: error.args.clone(),
                                    kwargs: error.kwargs.clone(),
                                }))
                                .await;
                        }
                    }

                    MESSAGE_TYPE_PUBLISH => {
                        let mut publish_requests = state.publish_requests.lock().await;
                        if let Some(response) = publish_requests.remove(&error.request_id) {
                            let _ = response
                                .send(PublishResponse {
                                    error: Some(WampError {
                                        uri: error.uri.clone(),
                                        args: error.args.clone(),
                                        kwargs: error.kwargs.clone(),
                                    }),
                                })
                                .await;
                        }
                    }

                    _ => {}
                }
            }
            MESSAGE_TYPE_GOODBYE => {
                let goodbye_was_sent = { state.goodbye_sent.lock().await };
                if *goodbye_was_sent {
                    goodbye_sender.send(()).await.unwrap();
                }

                exist_sender.send(()).await.unwrap();
            }
            _ => {}
        }
    }

    pub async fn call(&mut self, request: CallRequest) -> Result<CallResponse, Error> {
        let request_id = self.idgen.next_id();
        let msg = Call {
            request_id,
            options: request.options().clone(),
            procedure: request.procedure(),
            args: Some(request.args().clone()),
            kwargs: Some(request.kwargs().clone()),
        };

        let (sender, mut receiver): (mpsc::Sender<CallResponse>, mpsc::Receiver<CallResponse>) = mpsc::channel(1);

        match self.proto.send_message(Box::new(msg)) {
            Ok(to_send) => {
                {
                    let mut lock = self.state.call_requests.lock().await;
                    lock.insert(request_id, sender)
                };

                match self.peer.write(to_send).await {
                    Ok(()) => match receiver.recv().await {
                        Some(response) => Ok(response),
                        None => Err(Error::new("call failed")),
                    },
                    Err(e) => {
                        {
                            let mut lock = self.state.call_requests.lock().await;
                            lock.remove(&request_id)
                        };
                        Err(Error::new(format!("failed to send message: {e}")))
                    }
                }
            }

            Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
        }
    }

    pub async fn publish(&self, request: PublishRequest) -> Result<Option<PublishResponse>, Error> {
        let request_id = self.idgen.next_id();
        let msg = Publish {
            request_id,
            options: request.options().clone(),
            topic: request.topic(),
            args: Some(request.args().clone()),
            kwargs: Some(request.kwargs().clone()),
        };

        let acknowledge = {
            if let Some(Value::Bool(acknowledge)) = msg.options.get("acknowledge") {
                *acknowledge
            } else {
                false
            }
        };

        if acknowledge {
            let (sender, mut receiver): (mpsc::Sender<PublishResponse>, mpsc::Receiver<PublishResponse>) =
                mpsc::channel(1);

            match self.proto.send_message(Box::new(msg)) {
                Ok(to_send) => {
                    {
                        let mut lock = self.state.publish_requests.lock().await;
                        lock.insert(request_id, sender)
                    };

                    match self.peer.write(to_send).await {
                        Ok(()) => match receiver.recv().await {
                            Some(response) => Ok(Some(response)),
                            None => Err(Error::new("publish failed")),
                        },
                        Err(e) => {
                            {
                                let mut lock = self.state.publish_requests.lock().await;
                                lock.remove(&request_id)
                            };
                            Err(Error::new(format!("failed to send message: {e}")))
                        }
                    }
                }

                Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
            }
        } else {
            match self.proto.send_message(Box::new(msg)) {
                Ok(to_send) => match self.peer.write(to_send).await {
                    Ok(()) => Ok(None),
                    Err(e) => Err(Error::new(format!("failed to send message: {e}"))),
                },

                Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
            }
        }
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse, Error> {
        let request_id = self.idgen.next_id();
        let msg = Register {
            request_id,
            options: request.options().clone(),
            procedure: request.procedure(),
        };

        let (sender, mut receiver): (mpsc::Sender<RegisterResponse>, mpsc::Receiver<RegisterResponse>) =
            mpsc::channel(1);

        match self.proto.send_message(Box::new(msg)) {
            Ok(to_send) => {
                {
                    let mut lock = self.state.register_requests.lock().await;
                    lock.insert(request_id, sender)
                };

                match self.peer.write(to_send).await {
                    Ok(()) => match receiver.recv().await {
                        Some(response) => Ok(response),
                        None => Err(Error::new("register failed")),
                    },
                    Err(e) => {
                        {
                            let mut lock = self.state.register_requests.lock().await;
                            lock.remove(&request_id)
                        };
                        Err(Error::new(format!("failed to send message: {e}")))
                    }
                }
            }

            Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
        }
    }

    pub async fn subscribe(&self, request: SubscribeRequest) -> Result<SubscribeResponse, Error> {
        let request_id = self.idgen.next_id();
        let msg = Subscribe {
            request_id,
            options: request.options().clone(),
            topic: request.topic(),
        };

        let (sender, mut receiver): (mpsc::Sender<SubscribeResponse>, mpsc::Receiver<SubscribeResponse>) =
            mpsc::channel(1);
        match self.proto.send_message(Box::new(msg)) {
            Ok(to_send) => {
                {
                    let mut lock = self.state.subscribe_requests.lock().await;
                    lock.insert(request_id, sender)
                };

                match self.peer.write(to_send).await {
                    Ok(()) => match receiver.recv().await {
                        Some(response) => Ok(response),
                        None => Err(Error::new("subscribe failed")),
                    },
                    Err(e) => {
                        {
                            let mut lock = self.state.subscribe_requests.lock().await;
                            lock.remove(&request_id)
                        };
                        Err(Error::new(format!("failed to send message: {e}")))
                    }
                }
            }

            Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
        }
    }

    pub async fn leave(&mut self) -> Result<(), Error> {
        let msg = Goodbye {
            details: Default::default(),
            reason: "wamp.close.close_realm".to_string(),
        };

        match self.proto.send_message(Box::new(msg)) {
            Ok(to_send) => {
                {
                    let mut sent = self.state.goodbye_sent.lock().await;
                    *sent = true;
                }
                match self.peer.write(to_send).await {
                    Ok(()) => match self.goodbye_receiver_channel.lock().await.recv().await {
                        Some(_reason) => Ok(()),
                        None => Err(Error::new("leave failed")),
                    },
                    Err(e) => Err(Error::new(format!("failed to send message: {e}"))),
                }
            }
            Err(e) => Err(Error::new(format!("proto failed to parse message: {e}"))),
        }
    }

    pub async fn wait_disconnect(&self) {
        self.exist_receiver_channel.lock().await.recv().await;
    }
}
