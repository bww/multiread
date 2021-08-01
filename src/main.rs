use std::thread;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};

use futures::executor::block_on;

use dashmap::DashMap;

use multiread::client::Client;
use multiread::message::Message;

fn handle_server(rx: Receiver<Message>, clients: Arc<DashMap<String, Sender<String>>>) {
  block_on(handle_forward(rx, clients));
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
        println!("Skip! #{}", key);
        continue;
      }
      let tx = e.value();
      match tx.send(msg.data.clone()).await {
        Ok(_) => {},
        Err(err) => {
          println!("*** {:?}", err);
          clients.remove(&key).expect("Could not remove client");
        }
      }
    }
  }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let listener = UnixListener::bind("__multiread").unwrap();
  let streams: DashMap<String, Sender<String>> = DashMap::new();
  let clients = Arc::new(streams);
  let (tx, rx) = mpsc::channel(2);
  
  {
    let clients = Arc::clone(&clients);
    thread::spawn(move || handle_server(rx, Arc::clone(&clients)));
  }
  
  let mut i = 0;
  loop {
    match listener.accept().await {
      Ok((stream, _addr)) => {
        let dup = tx.clone();
        let (vtx, vrx) = mpsc::channel(2);
        let clients = Arc::clone(&clients);
        clients.insert(format!("{}", i), vtx);
        let client = Client::new(format!("{}", i), stream, clients);
        thread::spawn(move || client.handle(dup, vrx));
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
