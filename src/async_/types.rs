use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

type RegisterCallbackType = dyn Fn(Invocation) -> Pin<Box<dyn Future<Output = Yield> + Send>> + Send + Sync;
type EventCallbackType = dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

#[derive(Clone)]
pub struct RegisterFn(pub Arc<RegisterCallbackType>);

impl fmt::Debug for RegisterFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<RegisterFn>")
    }
}

impl RegisterFn {
    pub async fn invoke(&self, inv: Invocation) -> Yield {
        self.0(inv).await
    }
}

#[derive(Clone)]
pub struct EventFn(pub Arc<EventCallbackType>);

impl fmt::Debug for EventFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<EventFn>")
    }
}

impl EventFn {
    pub async fn invoke(&self, event: Event) {
        self.0(event).await
    }
}

pub struct RegisterRequest {
    procedure: String,
    options: HashMap<String, Value>,

    callback: RegisterFn,
}

impl RegisterRequest {
    pub fn new<S, F, Fut>(procedure: S, callback: F) -> Self
    where
        S: Into<String>,
        F: Fn(Invocation) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Yield> + Send + 'static,
    {
        Self {
            procedure: procedure.into(),
            options: HashMap::new(),
            callback: RegisterFn(Arc::new(move |inv| Box::pin(callback(inv)))),
        }
    }

    pub fn with_option<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.options.insert(key.to_string(), value.into());
        self
    }

    pub fn with_options(mut self, options: HashMap<String, Value>) -> Self {
        self.options = options;
        self
    }

    pub fn options(&self) -> &HashMap<String, Value> {
        &self.options
    }

    pub fn procedure(&self) -> String {
        self.procedure.clone()
    }

    pub fn callback(&self) -> RegisterFn {
        self.callback.clone()
    }
}

#[derive(Debug)]
pub struct SubscribeRequest {
    topic: String,
    options: HashMap<String, Value>,
    callback: EventFn,
}

impl SubscribeRequest {
    pub fn new<S, F, Fut>(topic: S, callback: F) -> Self
    where
        S: Into<String>,
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            topic: topic.into(),
            options: Default::default(),
            callback: EventFn(Arc::new(move |inv| Box::pin(callback(inv)))),
        }
    }

    pub fn with_option<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.options.insert(key.to_string(), value.into());
        self
    }

    pub fn with_options(mut self, options: HashMap<String, Value>) -> Self {
        self.options = options;
        self
    }

    pub fn options(&self) -> &HashMap<String, Value> {
        &self.options
    }

    pub fn topic(&self) -> String {
        self.topic.clone()
    }

    pub fn callback(&self) -> EventFn {
        self.callback.clone()
    }
}

// Re-export
pub use crate::common::types::*;
