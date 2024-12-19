// https://www.jsonrpc.org/specification
use serde::{Deserialize, Serialize};
use tokio::{io::Interest, net::TcpStream};

use crate::MainResult;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Request {
    /// A String specifying the version of the JSON-RPC protocol. MUST be exactly "2.0".
    pub jsonrpc: String,
    /// A String containing the name of the method to be invoked. Method names that begin with the word rpc followed by a period character (U+002E or ASCII 46) are reserved for rpc-internal methods and extensions and MUST NOT be used for anything else.
    pub method: String,
    /// A Structured value that holds the parameter values to be used during the invocation of the method. This member MAY be omitted.
    pub params: serde_json::Value,
    /// An identifier established by the Client that MUST contain a String, Number, or NULL value if included. If it is not included it is assumed to be a notification. The value SHOULD normally not be Null [1] and Numbers SHOULD NOT contain fractional parts [2]
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Response {
    /// A String specifying the version of the JSON-RPC protocol. MUST be exactly "2.0".
    pub jsonrpc: String,

    /// This member is REQUIRED on success.
    /// This member MUST NOT exist if there was an error invoking the method.
    /// The value of this member is determined by the method invoked on the Server.
    pub result: Option<serde_json::Value>,

    /// This member is REQUIRED on error.
    /// This member MUST NOT exist if there was no error triggered during invocation.
    pub error: Option<Error>,

    /// This member is REQUIRED.
    /// It MUST be the same as the value of the id member in the Request Object.
    /// If there was an error in detecting the id in the Request object (e.g. Parse error/Invalid Request), it MUST be Null.
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Error {
    /// A Number that indicates the error type that occurred.
    /// This MUST be an integer.
    pub code: String,
    /// A String providing a short description of the error.
    /// The message SHOULD be limited to a concise single sentence.
    pub message: String,
    /// A Primitive or Structured value that contains additional information about the error.
    /// This may be omitted.
    /// The value of this member is defined by the Server (e.g. detailed error information, nested errors etc.).
    pub data: serde_json::Value,
}

type Listener = tokio::net::TcpListener;
const REQ_SIZE: usize = std::mem::size_of::<Request>();
const RES_SIZE: usize = std::mem::size_of::<Response>();

pub async fn next_request(stream: &TcpStream) -> MainResult<Option<Request>> {
    let mut buffer = [0u8; REQ_SIZE];
    let read_ready = stream.ready(Interest::READABLE).await?;
    if read_ready.is_readable() {
        let read_amt = stream.try_read(&mut buffer)?;
        if read_amt == REQ_SIZE {
            match serde_json::from_slice::<Request>(&buffer) {
                Ok(req) => {
                    return Ok(Some(req));
                }
                Err(err) => {
                    tracing::warn!("error deserializing request: {err}");
                }
            }
        }
    }
    Ok(None)
}

pub async fn send_response(stream: &TcpStream, response: Response) -> MainResult<()> {
    let bytes: Vec<u8> = serde_json::to_vec(&response)?;
    let read_ready = stream.ready(Interest::WRITABLE).await?;
    if read_ready.is_writable() {
        stream.try_write(&bytes)?;
        Ok(())
    } else {
        Err("Socket not writable!".into())
    }
}
