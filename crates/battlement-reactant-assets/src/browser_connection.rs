use std::{
  io::ErrorKind,
  net::{Ipv4Addr, SocketAddr, TcpStream},
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use battlement_tooling::build_control::BuildControl;
use serde_json::{Value, json};
use tungstenite::{Error, HandshakeError, Message, WebSocket, client::IntoClientRequest};

pub(crate) struct Protocol<'a> {
  socket: WebSocket<TcpStream>,
  next_id: u64,
  control: BuildControl<'a>,
}

impl<'a> Protocol<'a> {
  pub(crate) fn connect(endpoint: &str, control: BuildControl<'a>) -> Result<Self> {
    control.check()?;
    let request = endpoint.into_client_request()?;
    let port = request
      .uri()
      .port_u16()
      .context("browser endpoint omitted its port")?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let stream = TcpStream::connect_timeout(&address, Duration::from_millis(100))?;
    stream.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut handshake = tungstenite::client(request, stream);
    let socket = loop {
      control.check()?;
      match handshake {
        Ok((socket, _)) => break socket,
        Err(HandshakeError::Interrupted(pending)) => {
          if Instant::now() >= deadline {
            bail!("browser protocol handshake did not complete within 10 seconds");
          }
          thread::sleep(Duration::from_millis(10));
          handshake = pending.handshake();
        }
        Err(HandshakeError::Failure(error)) => return Err(error.into()),
      }
    };
    Ok(Self {
      socket,
      next_id: 1,
      control,
    })
  }

  pub(crate) fn command(
    &mut self,
    method: &str,
    params: Value,
    session: Option<&str>,
  ) -> Result<Value> {
    self.control.check()?;
    let id = self.next_id;
    self.next_id += 1;
    let mut request = json!({"id": id, "method": method, "params": params});
    if let Some(session) = session {
      request["sessionId"] = Value::String(session.to_owned());
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sent = self.socket.send(Message::Text(request.to_string().into()));
    loop {
      self.wait_for_io(deadline, method)?;
      match sent {
        Ok(()) => break,
        Err(Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
          thread::sleep(Duration::from_millis(10));
          sent = self.socket.flush();
        }
        Err(error) => return Err(error.into()),
      }
    }
    loop {
      self.wait_for_io(deadline, method)?;
      let message = match self.socket.read() {
        Err(Error::Io(error))
          if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
        {
          thread::sleep(Duration::from_millis(10));
          continue;
        }
        result => result?,
      };
      let Message::Text(text) = message else {
        continue;
      };
      let response: Value = serde_json::from_str(&text)?;
      if response.get("id").and_then(Value::as_u64) != Some(id) {
        continue;
      }
      if let Some(error) = response.get("error") {
        bail!("browser protocol {method} failed: {error}");
      }
      return response
        .get("result")
        .cloned()
        .with_context(|| format!("browser protocol {method} omitted its result"));
    }
  }

  fn wait_for_io(&self, deadline: Instant, method: &str) -> Result<()> {
    self.control.check()?;
    if Instant::now() >= deadline {
      bail!("browser protocol {method} did not respond within 10 seconds");
    }
    Ok(())
  }
}
