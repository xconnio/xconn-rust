use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use wampproto::messages::types::Value;
use wampproto::serializers::cbor::CBORSerializer;
use wampproto::serializers::json::JSONSerializer;
use wampproto::serializers::msgpack::MsgPackSerializer;
use wampproto::serializers::serializer::Serializer;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Error { message: msg.into() }
    }
}

// Implement Display so errors print nicely
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

// Implement the std::error::Error trait
impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct SessionDetails {
    id: i64,
    realm: String,
    authid: String,
    auth_role: String,
}

impl SessionDetails {
    pub fn new(id: i64, realm: String, authid: String, auth_role: String) -> Self {
        Self {
            id,
            realm,
            authid,
            auth_role,
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn realm(&self) -> String {
        self.realm.clone()
    }

    pub fn authid(&self) -> String {
        self.authid.clone()
    }

    pub fn auth_role(&self) -> String {
        self.auth_role.clone()
    }
}

pub trait WSSerializerSpec: Debug + Sync + Send {
    fn subprotocol(&self) -> String;
    fn serializer(&self) -> Box<dyn Serializer>;
    fn is_binary(&self) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct JSONSerializerSpec;

impl WSSerializerSpec for JSONSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.json".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(JSONSerializer {})
    }

    fn is_binary(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct CBORSerializerSpec;

impl WSSerializerSpec for CBORSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.cbor".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(CBORSerializer {})
    }

    fn is_binary(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default)]
pub struct MsgPackSerializerSpec;

impl WSSerializerSpec for MsgPackSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.msgpack".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(MsgPackSerializer {})
    }

    fn is_binary(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct CallRequest {
    procedure: String,
    options: HashMap<String, Value>,

    args: Vec<Value>,
    kw_args: HashMap<String, Value>,
}

impl CallRequest {
    pub fn new<S: Into<String>>(procedure: S) -> Self {
        Self {
            procedure: procedure.into(),
            args: Default::default(),
            kw_args: Default::default(),
            options: Default::default(),
        }
    }

    pub fn with_arg<T: Into<Value>>(mut self, arg: T) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args(mut self, args: Vec<Value>) -> Self {
        self.args = args;
        self
    }

    pub fn with_kwarg<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.kw_args.insert(key.to_string(), value.into());
        self
    }

    pub fn with_kwargs(mut self, kwargs: HashMap<String, Value>) -> Self {
        self.kw_args = kwargs;
        self
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

    pub fn args(&self) -> &Vec<Value> {
        &self.args
    }

    pub fn kwargs(&self) -> &HashMap<String, Value> {
        &self.kw_args
    }

    pub fn procedure(&self) -> String {
        self.procedure.clone()
    }
}

#[derive(Debug)]
pub struct PublishRequest {
    topic: String,
    options: HashMap<String, Value>,

    args: Vec<Value>,
    kw_args: HashMap<String, Value>,
}

impl PublishRequest {
    pub fn new<S: Into<String>>(topic: S) -> Self {
        Self {
            topic: topic.into(),
            args: Default::default(),
            kw_args: Default::default(),
            options: Default::default(),
        }
    }

    pub fn with_arg<T: Into<Value>>(mut self, arg: T) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args(mut self, args: Vec<Value>) -> Self {
        self.args = args;
        self
    }

    pub fn with_kwarg<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.kw_args.insert(key.to_string(), value.into());
        self
    }

    pub fn with_kwargs(mut self, kwargs: HashMap<String, Value>) -> Self {
        self.kw_args = kwargs;
        self
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

    pub fn args(&self) -> &Vec<Value> {
        &self.args
    }

    pub fn kwargs(&self) -> &HashMap<String, Value> {
        &self.kw_args
    }

    pub fn topic(&self) -> String {
        self.topic.clone()
    }
}

pub type RegisterFn = fn(Invocation) -> Yield;

#[derive(Debug)]
pub struct Invocation {
    pub args: Option<Vec<Value>>,
    pub kwargs: Option<HashMap<String, Value>>,
    pub details: Option<HashMap<String, Value>>,
}

#[derive(Debug, Default)]
pub struct Yield {
    pub args: Option<Vec<Value>>,
    pub kwargs: Option<HashMap<String, Value>>,
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

pub type EventFn = fn(Event);

#[derive(Debug)]
pub struct Event {
    pub args: Option<Vec<Value>>,
    pub kwargs: Option<HashMap<String, Value>>,
    pub details: Option<HashMap<String, Value>>,
}

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

#[derive(Debug, Default)]
pub struct SubscribeResponse {
    pub subscription_id: i64,
    pub error: Option<WampError>,
}

impl SubscribeResponse {
    pub fn unsubscribe(&self) {
        println!("Unsubscribing: {}", self.subscription_id);
    }
}

#[derive(Debug, Default)]
pub struct CallResponse {
    pub args: Option<Vec<Value>>,
    pub kwargs: Option<HashMap<String, Value>>,
    pub error: Option<WampError>,
}

#[derive(Debug, Default)]
pub struct WampError {
    pub uri: String,
    pub args: Option<Vec<Value>>,
    pub kwargs: Option<HashMap<String, Value>>,
}

#[derive(Debug, Default)]
pub struct PublishResponse {
    pub error: Option<WampError>,
}

#[derive(Debug, Default)]
pub struct RegisterResponse {
    pub registration_id: i64,
    pub error: Option<WampError>,
}
