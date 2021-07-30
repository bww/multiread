use std::thread;
use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::net::Shutdown;
use std::sync::Arc;
// use std::sync::mpsc;
// use std::sync::mpsc::{Sender, SyncSender, Receiver};

use tokio::io::Interest;
use tokio::net::{UnixStream, UnixListener};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};

use futures::executor::block_on;
use futures::{future::FutureExt, pin_mut, select};
use dashmap::DashMap;

struct Message {
  sender: String,
  data:   String,
}

async fn handle_forward(mut rx: Receiver<Message>, clients: Arc<DashMap<String, Sender<String>>>) {
  loop {
    let msg: Message;
    match rx.recv().await {
      Some(recv) => {
        msg = recv;
      }
      _ => {
        return;
      }
    }
    
    for e in clients.iter() {
      let key = e.key().clone();
      if key == msg.sender {
        println!("Skip! {}", key);
        continue;
      }
      let tx = e.value();
      match tx.send(msg.data.clone()).await {
        Ok(_) => {
          println!(">>> Sent: {}", key);
        },
        Err(err) => {
          println!("*** {:?}", err);
          clients.remove(&key).expect("Could not remove client");
        }
      }
    }
  }
}

async fn read_stream(stream: &UnixStream) -> Result<String, Error> {
  println!(">>> WILL Read");
  loop {
    let ready = stream.ready(Interest::READABLE | Interest::WRITABLE).await?;

    if ready.is_readable() {
      let mut data = vec![0; 1024];
      // Try to read data, this may still fail with `WouldBlock`
      // if the readiness event is a false positive.
      match stream.try_read(&mut data) {
        Ok(n) => {
          println!("read {} bytes", n);        
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
          continue;
        }
        Err(e) => {
          return Err(e.into());
        }
      }
    }
    
  }
  
  // let mut data = String::new();
  // match stream.read_to_string(&mut data) {
  //   Ok(_) => {
  //     println!(">>> Read: {}", data);
  //     return Ok(data);
  //   }
  //   Err(err) => {
  //     return Err(err);
  //   }
  // }
}

async fn recv_message(mut rx: Receiver<String>) -> Result<String, Error> {
  println!(">>> WILL Recv");
  match rx.recv().await {
    Some(data) => {
      println!(">>> Recv: {}", data);
      return Ok(data);
    }
    _ => {
      return Err(Error::new(ErrorKind::UnexpectedEof, "Nothing to receive"));
    }
  }
}

async fn read_or_recv(stream: &UnixStream, rx: Receiver<String>) -> Result<String, Error> {
  let read = read_stream(stream).fuse();
  let recv = recv_message(rx).fuse();
  pin_mut!(read, recv);
  
  let name = format!("{:?}", stream);
  println!("Selecting! {}", name);
  select! {
    recv_res = recv => {
      println!(">>> Recv for: {}", name);
      return recv_res;
    },
    read_res = read => {
      println!(">>> Read for: {}", name);
      return read_res;
    },
  }
}

async fn send_tx(tx: Sender<Message>, msg: Message) {
  match tx.send(msg).await {
    Ok(_) => {
      // nothing
    },
    Err(err) => {
      println!("*** {}", err);
    }
  }
}

fn handle_client(id: String, stream: UnixStream, clients: Arc<DashMap<String, Sender<String>>>, tx: Sender<Message>, rx: Receiver<String>) {
  println!("Client started! {}", id);
  
  match block_on(read_or_recv(&stream, rx)) {
    Ok(data) => {
      let id = id.clone();
      send_tx(tx, Message{sender: id, data: data});
    }
    Err(err) => {
      println!("*** Could not read: {:?}", err);
    }
  }
  
  let id = id.clone();
  clients.remove(&id).expect("Could not deregister client");
  println!("Client ended! {}", id);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let listener = UnixListener::bind("__multiread").unwrap();
  let streams: DashMap<String, Sender<String>> = DashMap::new();
  let clients = Arc::new(streams);
  let (tx, rx) = mpsc::channel(2);

  {
    let clients = Arc::clone(&clients);
    handle_forward(rx, clients);
  }
  
  let mut i = 0;
  loop {
    match listener.accept().await {
      Ok((stream, addr)) => {
        let dup = tx.clone();
        let (vtx, vrx) = mpsc::channel(2);
        let clients = Arc::clone(&clients);
        let id = format!("{}", i);
        clients.insert(id, vtx);
        let id = format!("{}", i);
        thread::spawn(move || handle_client(id, stream, clients, dup, vrx));
        i = i + 1;
      }
      Err(err) => {
        println!("Couldn't connect: {:?}", err);
        break;
      }
    }
  }
  
  Ok(())
}
