use std::pin::{pin, Pin};
use futures::{AsyncRead, AsyncWrite};
use futures_util::SinkExt;
use futures_util::Future;
use tokio::sync::{mpsc, oneshot};
use yamux::{Connection, Stream};
use crate::errors::Error;
use std::task::{Context, Poll};

#[derive(Debug)]
pub(crate) enum ControlCommand {
    /// Open a new stream to the remote end.
    OpenStream(oneshot::Sender<Result<Stream, Error>>),
    /// Close the whole connection.
    CloseConnection(oneshot::Sender<()>),
}

pub struct ConnControl {
    sender: mpsc::Sender<ControlCommand>,
    //todo 关闭
}

pub struct ConnControlFuture<T: AsyncRead + AsyncWrite + Unpin + Send> {
    receiver: mpsc::Receiver<ControlCommand>,
    conn: Connection<T>,
}

impl<T: AsyncRead + AsyncWrite + Unpin + Send> Future for ConnControlFuture<T> {
    type Output = Result<(), Box<dyn std::error::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        match pin!(this.receiver.recv()).poll(cx) {
            Poll::Ready(Some(cmd)) => {
                match cmd {
                    ControlCommand::OpenStream(tx) => {
                        match this.conn.poll_new_outbound(cx) {
                            Poll::Ready(res) => {
                                let _ = tx.send(res.map_err(|e| Error::from(e)));
                            }
                            Poll::Pending => {
                                return Poll::Pending;
                            }
                        }
                    }
                    ControlCommand::CloseConnection(tx) => {
                        todo!();
                    }
                }
            }
            Poll::Pending => {}
            _ => {}
        }


        match this.conn.poll_next_inbound(cx)? {
            Poll::Ready(Some(stream)) => {
                drop(stream);
                panic!("Did not expect remote to open a stream");
            }
            Poll::Ready(None) => {
                panic!("Did not expect remote to close the connection");
            }
            Poll::Pending => {
                Poll::Pending
            }
        }

    }
}

impl ConnControl {
    pub fn new<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(mut conn: Connection<T>) -> (Self, ConnControlFuture<T>) {
        let (sender, mut receiver) = mpsc::channel(8);
        /*tokio::spawn(async {
            let future = ConnControlFuture {
                receiver,
                conn,
            };
            let _ = future.await;
        });*/
        return (Self {
            sender,
        }, ConnControlFuture {
            receiver,
            conn,
        });
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn open_stream(&self) -> Result<Stream, Error> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(ControlCommand::OpenStream(tx)).await?;
        rx.await?
    }
}