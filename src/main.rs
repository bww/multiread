use std::fs;
use std::env;
use std::sync::Arc;

use tokio::io;
use tokio::net::UnixListener;
use tokio::sync::mpsc::Sender;

use dashmap::DashMap;

use multiread::server::Server;
use multiread::drone::Drone;

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let mut args = env::args();
  if args.len() < 2 {
    println!("*** Usage: multiread <key>");
    return Ok(());
  }
  
  let key = args.nth(1).unwrap();
  let mut dir = env::temp_dir();
  dir.push("multiread");

  let dup = dir.clone();
  fs::create_dir_all(dup).expect("Could not create socket path");
  
  dir.push(key);
  let dup = dir.clone();
  let listener = match UnixListener::bind(dup) {
    Ok(listener) => listener,
    Err(ref err) if err.kind() == io::ErrorKind::AddrInUse => {
      println!(">>> Client: {}", dir.display());
      let dup = dir.clone();
      Drone::connect(dup.as_path())?.handle();
      return Ok(());
    }
    Err(err) => {
      println!("*** Could not bind: {:?}", err);
      return Ok(());
    }
  };
  
  let streams: DashMap<String, Sender<String>> = DashMap::new();
  let clients = Arc::new(streams);
  
  println!(">>> Server: {}", dir.display());
  let server = Server::new(clients);
  server.run(listener);
  
  Ok(())
}
