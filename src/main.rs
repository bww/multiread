use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::mpsc::Sender;

use dashmap::DashMap;

use multiread::server::Server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let listener = UnixListener::bind("__multiread").unwrap();
  let streams: DashMap<String, Sender<String>> = DashMap::new();
  let clients = Arc::new(streams);
  
  let server = Server::new(clients);
  server.run(listener);
  
  // {
  //   let clients = Arc::clone(&clients);
  //   thread::spawn(move || handle_server(rx, Arc::clone(&clients)));
  // }
  
  // let mut i = 0;
  // loop {
  //   match listener.accept().await {
  //     Ok((stream, _addr)) => {
  //       let dup = tx.clone();
  //       let (vtx, vrx) = mpsc::channel(2);
  //       let clients = Arc::clone(&clients);
  //       clients.insert(format!("{}", i), vtx);
  //       let client = Client::new(format!("{}", i), stream, clients);
  //       thread::spawn(move || client.handle(dup, vrx));
  //       i = i + 1;
  //     }
  //     Err(err) => {
  //       println!("Couldn't connect: {:?}", err);
  //       break;
  //     }
  //   }
  // }
  
  Ok(())
}
