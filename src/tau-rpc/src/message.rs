use super::errors::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{from_str, to_vec, Value};

fn serialize_json_rpc_result<S>(
    val: &Result<Value, Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Ok(v) => serializer.serialize_newtype_variant("", 0, "result", v),
        Err(v) => serializer.serialize_newtype_variant("", 1, "error", v),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum JsonRpcResult<T, E> {
    Result(T),
    Error(E),
}

pub fn deserialize_json_rpc_result<'de, D>(
    deserializer: D,
) -> Result<Result<Value, Value>, D::Error>
where
    D: Deserializer<'de>,
{
    match JsonRpcResult::<Value, Value>::deserialize(deserializer)? {
        JsonRpcResult::Result(value) => Ok(Ok(value)),
        JsonRpcResult::Error(value) => Ok(Err(value)),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

#[derive(Serialize, PartialEq, Clone, Debug, Deserialize)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Response {
    pub id: u64,
    #[serde(flatten)]
    #[serde(serialize_with = "serialize_json_rpc_result")]
    #[serde(deserialize_with = "deserialize_json_rpc_result")]
    pub result: Result<Value, Value>,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct Request {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl Message {
    pub fn decode(rd: &str) -> Result<Message, DecodeError> {
        Ok(from_str(rd)?)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        match *self {
            Message::Request(ref request) => to_vec(request).expect("Request serialization failed"),
            Message::Response(ref response) => {
                to_vec(response).expect("Response serialization failed")
            }
            Message::Notification(ref notification) => {
                to_vec(notification).expect("Notification serialization failed")
            }
        }
    }
}
