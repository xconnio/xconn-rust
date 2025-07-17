use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use wampproto::messages::call::Call;
use wampproto::messages::publish::Publish;
pub use wampproto::messages::types::Value;
use wampproto::serializers::cbor::CBORSerializer;
use wampproto::serializers::json::JSONSerializer;
use wampproto::serializers::msgpack::MsgPackSerializer;
use wampproto::serializers::serializer::Serializer;
use wampproto::transports::rawsocket::SerializerID;

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

pub trait _SerializerSpec: Debug + Sync + Send {
    fn subprotocol(&self) -> String;
    fn serializer_id(&self) -> SerializerID;
    fn serializer(&self) -> Box<dyn Serializer>;
    fn is_binary(&self) -> bool;
}

pub trait SerializerSpec: _SerializerSpec {
    fn clone_box(&self) -> Box<dyn SerializerSpec>;
}

impl<T> SerializerSpec for T
where
    T: _SerializerSpec + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn SerializerSpec> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn SerializerSpec> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone, Default)]
pub struct JSONSerializerSpec;

impl _SerializerSpec for JSONSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.json".to_string()
    }

    fn serializer_id(&self) -> SerializerID {
        SerializerID::JSON
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

impl _SerializerSpec for CBORSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.cbor".to_string()
    }

    fn serializer_id(&self) -> SerializerID {
        SerializerID::CBOR
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

impl _SerializerSpec for MsgPackSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.msgpack".to_string()
    }

    fn serializer_id(&self) -> SerializerID {
        SerializerID::MSGPACK
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(MsgPackSerializer {})
    }

    fn is_binary(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct _OutgoingRequest {
    uri: String,
    options: HashMap<String, Value>,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
}

impl _OutgoingRequest {
    pub fn new<S: Into<String>>(uri: S) -> Self {
        Self {
            uri: uri.into(),
            args: Default::default(),
            kwargs: Default::default(),
            options: Default::default(),
        }
    }

    pub fn arg<T: Into<Value>>(mut self, arg: T) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: Vec<Value>) -> Self {
        self.args = args;
        self
    }

    pub fn kwarg<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.kwargs.insert(key.to_string(), value.into());
        self
    }

    pub fn kwargs(mut self, kwargs: HashMap<String, Value>) -> Self {
        self.kwargs = kwargs;
        self
    }

    pub fn option<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.options.insert(key.to_string(), value.into());
        self
    }

    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.options = options;
        self
    }
}

pub type CallRequest = _OutgoingRequest;
pub type PublishRequest = _OutgoingRequest;

impl CallRequest {
    pub(crate) fn to_call(&self, request_id: i64) -> Call {
        Call {
            request_id,
            options: self.options.clone(),
            procedure: self.uri.clone(),
            args: Some(self.args.clone()),
            kwargs: Some(self.kwargs.clone()),
        }
    }
}

impl PublishRequest {
    pub(crate) fn to_publish(&self, request_id: i64) -> Publish {
        Publish {
            request_id,
            options: self.options.clone(),
            topic: self.uri.clone(),
            args: Some(self.args.clone()),
            kwargs: Some(self.kwargs.clone()),
        }
    }
}

#[derive(Debug)]
pub struct _IncomingRequest {
    pub args: Vec<Value>,
    pub kwargs: HashMap<String, Value>,
    pub details: HashMap<String, Value>,
}

pub type Invocation = _IncomingRequest;
pub type Event = _IncomingRequest;

#[derive(Debug, Default)]
pub struct Yield {
    pub args: Vec<Value>,
    pub kwargs: HashMap<String, Value>,
    pub error: Option<WampError>,
}

impl Yield {
    pub fn new(args: Vec<Value>, kwargs: HashMap<String, Value>) -> Self {
        Self {
            args,
            kwargs,
            error: None,
        }
    }

    pub fn args(args: Vec<Value>) -> Self {
        Self {
            args,
            kwargs: Default::default(),
            error: None,
        }
    }

    pub fn arg<T: Into<Value>>(arg: T) -> Self {
        Self {
            args: vec![arg.into()],
            kwargs: Default::default(),
            error: None,
        }
    }

    pub fn kwarg<T: Into<Value>>(key: &str, value: T) -> Self {
        Self {
            args: Default::default(),
            kwargs: HashMap::from([(key.to_string(), value.into())]),
            error: None,
        }
    }

    pub fn kwargs(kwargs: HashMap<String, Value>) -> Self {
        Self {
            args: vec![],
            kwargs,
            error: None,
        }
    }

    pub fn error(uri: &str) -> Self {
        Self {
            args: Default::default(),
            kwargs: Default::default(),
            error: Some(WampError {
                uri: uri.to_string(),
                args: Default::default(),
                kwargs: Default::default(),
            }),
        }
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

pub type TransportType = usize;
pub const TRANSPORT_WEB_SOCKET: TransportType = 1;
pub const TRANSPORT_RAW_SOCKET: TransportType = 2;
