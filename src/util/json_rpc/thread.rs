use super::socket;
use crate::{
    util::json_rpc::socket::{next_request, send_response},
    MainResult,
};
use tokio::{
    net::ToSocketAddrs,
    sync::mpsc::{error::TryRecvError, Receiver, Sender},
    task::JoinHandle,
};

pub struct RpcListeningThread {
    pub recv: Receiver<socket::Request>,
    pub sender: Sender<socket::Response>,
    _thread: JoinHandle<()>,
}

const CHANNEL_BUF_SIZE: usize = 5;
impl RpcListeningThread {
    pub async fn try_recv(&mut self) -> MainResult<Option<socket::Request>> {
        match self.recv.try_recv() {
            Err(TryRecvError::Disconnected) => return Err("Thread died".into()),
            Err(TryRecvError::Empty) => Ok(None),
            Ok(req) => Ok(Some(req)),
        }
    }
    pub async fn new(addr: impl ToSocketAddrs) -> MainResult<Self> {
        let (req_send, req_recv) = tokio::sync::mpsc::channel::<socket::Request>(CHANNEL_BUF_SIZE);
        let (res_send, mut res_recv) =
            tokio::sync::mpsc::channel::<socket::Response>(CHANNEL_BUF_SIZE);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::warn!("rpc api listening on: {:#?}", listener.local_addr());
        let _thread = tokio::task::spawn(async move {
            loop {
                let (stream, addr) = listener
                    .accept()
                    .await
                    .expect("thread failed to accept stream");

                tracing::warn!("JSON RPC Connected to {addr:#?}");
                loop {
                    tracing::warn!("checking for request");

                    // if let Some(req) = next_request(&stream).await.unwrap() {
                    //     tracing::warn!("thread received: {req:#?}");
                    //     req_send
                    //         .send(req)
                    //         .await
                    //         .expect("thread failed to send request");
                    // }
                    //
                    // tracing::warn!("checking for response");
                    // if let Ok(res) = res_recv.try_recv().map_err(|err| {
                    //     if let TryRecvError::Disconnected = err {
                    //         panic!("receiver disconnected!");
                    //     }
                    // }) {
                    //     tracing::warn!("got res: {res:#?}");
                    //     send_response(&stream, res)
                    //         .await
                    //         .expect("thread failed to send response");
                    // }
                    tokio::select! {
                        Some(req) = async { next_request(&stream).await.unwrap() } => {
                            tracing::warn!("thread received: {req:#?}");
                            req_send
                                .send(req)
                                .await
                                .expect("thread failed to send request");
                        }
                        Some(res) = res_recv.recv() => {
                            tracing::warn!("got res: {res:#?}");
                            send_response(&stream, res).await.expect("thread failed to send response");
                        }
                    }
                }
            }
        });
        Ok(Self {
            recv: req_recv,
            sender: res_send,
            _thread,
        })
    }
}
