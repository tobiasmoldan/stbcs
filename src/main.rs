use anyhow::Result;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::signal;
use tokio::sync::{broadcast, watch};

async fn spawn_listener(mut listener: TcpListener, mut shutdown_rx: watch::Receiver<bool>) {
    tokio::spawn(async move {
        let (brd_tx, _brd_rx) = broadcast::channel::<(u16, Arc<Vec<u8>>)>(10);
        let mut next_id = 0u16;

        loop {
            tokio::select! {
                Some(val) = shutdown_rx.recv() => {
                    if val {
                        break;
                    }
                }
                res = listener.accept() => {
                    if res.is_ok() {
                        let (stream, _addr) = res.unwrap();
                        spawn_stream(stream, next_id, shutdown_rx.clone(), brd_tx.clone(), brd_tx.subscribe()).await;
                        next_id = next_id.overflowing_add(1).0;
                    } else {
                        break;
                    }
                }
            }
        }
    });
}

async fn spawn_stream(
    mut stream: TcpStream,
    id: u16,
    mut done: watch::Receiver<bool>,
    brd_tx: broadcast::Sender<(u16, Arc<Vec<u8>>)>,
    mut brd_rx: broadcast::Receiver<(u16, Arc<Vec<u8>>)>,
) {
    tokio::spawn(async move {
        let _ = stream
            .write_all(format!("Welcome User#{}\n", id).as_bytes())
            .await;

        let mut mem = [0u8; 4096];

        loop {
            tokio::select! {
                Some(val) = done.recv() => {
                    if val {
                        break;
                    }
                }
                Ok(size) = stream.read(&mut mem) => {
                    if size == 0 {
                        let vec = Vec::from(format!("User#{} disconnected!\n", id));
                        let _ = brd_tx.send((id, Arc::new(vec)));
                        break;
                    }
                    let mut vec = Vec::from(format!("User#{}: ", id));
                    vec.extend_from_slice(&mem[..size]);
                    match *vec {
                        [.., b'\n'] => {}
                        _ => {
                            vec.push(b'\n');
                        }
                    }
                    brd_tx.send((id, Arc::new(vec))).unwrap();
                }
                Ok((sender_id,val)) = brd_rx.recv() => {
                    if sender_id != id {
                        let _ = stream.write_all(val.as_ref()).await;
                    }
                }
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let (mut shutown_tx, shutdown_rx) = watch::channel::<bool>(false);

    let listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), 8000)).await?;

    spawn_listener(listener, shutdown_rx).await;

    signal::ctrl_c().await?;

    shutown_tx.broadcast(true)?;

    shutown_tx.closed().await;

    Ok(())
}
