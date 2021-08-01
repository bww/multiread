use std::sync::Arc;

use tokio::net::{UnixStream};
use tokio::sync::mpsc::{Sender, Receiver};
use futures::executor::block_on;
use futures::{join};

use dashmap::DashMap;

use crate::io::{read_recv, write_stream, send_channel};
use crate::message::Message;

pub struct Client {
  id: String,
  stream: UnixStream,
  clients: Arc<DashMap<String, Sender<String>>>,
}

impl Client {
  
  pub fn new(id: String, stream: UnixStream, clients: Arc<DashMap<String, Sender<String>>>) -> Client {
    return Client{id, stream, clients};
  }
  
  pub fn handle(&self, tx: Sender<Message>, rx: Receiver<String>) {
    match block_on(read_recv(&self.stream, rx)) {
      Ok(data) => {
        let id = self.id.clone();
        let f1 = send_channel(tx, Message{sender: id, data: data.clone()});
        let f2 = write_stream(&self.stream, data.clone());
        block_on(async {
          return join!(f1, f2);
        });
      }
      Err(err) => {
        println!("*** Could not read: {:?}", err);
      }
    }
    
    let id = self.id.clone();
    self.clients.remove(&id).expect("Could not deregister client");
    println!("Client ended: {}", id);
  }

}
