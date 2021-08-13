use std::io;
use std::io::Error;
use std::marker::Unpin;

use tokio::io::{Interest, AsyncRead, AsyncReadExt};
use tokio::net::{UnixStream};
use tokio::sync::mpsc::{Sender, Receiver};

use futures::{future::FutureExt, pin_mut, select};

use crate::message::Message;

pub async fn read_stream(stream: &UnixStream) -> Result<String, Error> {
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
            return Ok(value[..x+1].to_string());
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

pub async fn read_async(src: &mut(impl AsyncRead + Unpin)) -> Result<String, Error> {
  let mut value: String = String::new();
  loop {
    let mut data = vec![0; 1024];
    match src.read(&mut data).await {
      Ok(n) => {
        let v = String::from_utf8(data[..n].to_vec()).unwrap();
        value.push_str(&v);
        if let Some(x) = value.find("\n") {
          return Ok(value[..x+1].to_string());
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

pub async fn write_stream(stream: &UnixStream, data: String) -> Result<(), Error> {
  let mut value: &[u8] = data.as_bytes();
  loop {
    let ready = stream.ready(Interest::WRITABLE).await?;
    if ready.is_writable() {
      match stream.try_write(value) {
        Ok(n) => {
          if n == value.len() {
            return Ok(());
          }else{
            value = &value[n..];
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

pub async fn recv_channel(mut rx: Receiver<String>) -> Result<String, Error> {
  match rx.recv().await {
    Some(data) => {
      return Ok(data);
    }
    _ => {
      return Err(Error::new(io::ErrorKind::UnexpectedEof, "Nothing to receive"));
    }
  }
}

pub async fn send_channel(tx: Sender<Message>, msg: Message) {
  match tx.send(msg).await {
    Ok(_) => {},
    Err(err) => {
      println!("*** Could not send: {}", err);
    }
  }
}

pub async fn read_recv(stream: &UnixStream, rx: Receiver<String>) -> Result<String, Error> {
  let read = read_stream(stream).fuse();
  let recv = recv_channel(rx).fuse();
  pin_mut!(read, recv);
  select! {
    recv_res = recv => {
      return recv_res;
    },
    read_res = read => {
      return read_res;
    },
  }
}

pub async fn read_either(a: &mut(impl AsyncRead + Unpin), b: &mut(impl AsyncRead + Unpin)) -> Result<String, Error> {
  let fa = read_async(a).fuse();
  let fb = read_async(b).fuse();
  pin_mut!(fa, fb);
  select! {
    res_a = fa => {
      return res_a;
    },
    res_b = fb => {
      return res_b;
    },
  }
}
