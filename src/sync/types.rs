use std::collections::HashMap;
use wampproto::messages::types::Value;

pub type EventFn = fn(Event);
pub type RegisterFn = fn(Invocation) -> Yield;

#[derive(Debug)]
pub struct SubscribeRequest {
    topic: String,
    options: HashMap<String, Value>,
    callback: EventFn,
}

impl SubscribeRequest {
    pub fn new<S: Into<String>>(topic: S, callback: EventFn) -> Self {
        Self {
            topic: topic.into(),
            options: Default::default(),
            callback,
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
        self.callback
    }
}

#[derive(Debug)]
pub struct RegisterRequest {
    procedure: String,
    options: HashMap<String, Value>,

    callback: RegisterFn,
}

impl RegisterRequest {
    pub fn new<S: Into<String>>(procedure: S, callback: RegisterFn) -> Self {
        Self {
            procedure: procedure.into(),
            options: Default::default(),
            callback,
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
        self.callback
    }
}

// Re-export
pub use crate::common::types::*;
