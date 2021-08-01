use std::thread;
use std::sync::Arc;

use tokio::net::{UnixListener};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use futures::executor::block_on;

use dashmap::DashMap;

use crate::client::Client;
use crate::message::Message;

pub struct Server {
  clients: Arc<DashMap<String, Sender<String>>>,
}

impl Server {
  
  pub fn new(clients: Arc<DashMap<String, Sender<String>>>) -> Server {
    return Server{clients};
  }
  
  pub fn run(&self, listener: UnixListener) {
    let (tx, rx) = mpsc::channel(2);
    let clients = Arc::clone(&self.clients);
    thread::spawn(move || forward(rx, Arc::clone(&clients)));
    self.listen(tx, listener);
  }
  
  fn listen(&self, tx: Sender<Message>, listener: UnixListener) {
    block_on(self._listen(tx, listener));
  }
  
  async fn _listen(&self, tx: Sender<Message>, listener: UnixListener) {
    let mut i = 0;
    loop {
      i = i + 1;
      match listener.accept().await {
        Ok((stream, _addr)) => {
          let dup = tx.clone();
          let (vtx, vrx) = mpsc::channel(2);
          let clients = Arc::clone(&self.clients);
          clients.insert(format!("{}", i), vtx);
          let client = Client::new(format!("{}", i), stream, clients);
          thread::spawn(move || client.handle(dup, vrx));
        }
        Err(err) => {
          println!("Couldn't connect: {:?}", err);
          break;
        }
      }
    }
  }
  
}

fn forward(rx: Receiver<Message>, clients: Arc<DashMap<String, Sender<String>>>) {
  block_on(_forward(rx, clients));
}

async fn _forward(mut rx: Receiver<Message>, clients: Arc<DashMap<String, Sender<String>>>) {
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
