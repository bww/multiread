use std::thread;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::Arc;

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
  let mut value: String = String::new();
  loop {
    let ready = stream.ready(Interest::READABLE).await?;
    
    if ready.is_readable() {
      let mut data = vec![0; 1024];
      match stream.try_read(&mut data) {
        Ok(n) => {
          let v = String::from_utf8(data[..n].to_vec()).unwrap();
          value.push_str(&v);
          if let Some(x) = value.find("\n") {
            return Ok(value[..x].to_string());
          }else if n < 1 {
            return Ok(value);
          }
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
}

async fn recv_message(mut rx: Receiver<String>) -> Result<String, Error> {
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
      println!(">>> Did send");
    },
    Err(err) => {
      println!("*** Could not send: {}", err);
    }
  }
}

fn handle_client(id: String, stream: UnixStream, clients: Arc<DashMap<String, Sender<String>>>, tx: Sender<Message>, rx: Receiver<String>) {
  println!("Client started! {}", id);
  
  match block_on(read_or_recv(&stream, rx)) {
    Ok(data) => {
      let id = id.clone();
      block_on(send_tx(tx, Message{sender: id, data: data}));
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
