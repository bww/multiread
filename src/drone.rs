use std::io::Error;
use std::path::Path;

use tokio::io;
use tokio::net::{UnixStream};
use futures::executor::block_on;

use crate::io::{read_either};

pub struct Drone {
  stream: UnixStream,
}

impl Drone {
  
  pub fn new(stream: UnixStream) -> Drone {
    return Drone{stream};
  }

  pub fn connect(path: &Path) -> Result<Drone, Error> {
    return block_on(Drone::connect_async(path));
  }
  
  async fn connect_async(path: &Path) -> Result<Drone, Error> {
    match UnixStream::connect(path).await {
      Ok(stream) => {
        return Ok(Drone{stream});
      }
      Err(err) => {
        return Err(err.into());
      }
    }
  }
  
  pub fn handle(&mut self) {
    let mut stdin = io::stdin();
    match block_on(read_either(&mut self.stream, &mut stdin)) {
      Ok(data) => {
        println!(">>> GOTCHA: {}", data);
      }
      Err(err) => {
        println!("*** Could not read: {:?}", err);
      }
    }
  }
  
}
