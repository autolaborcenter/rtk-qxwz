use async_std::{
    io::{prelude::BufReadExt, BufReader, ReadExt, WriteExt},
    net::TcpStream,
    task,
};
use driver::Driver;
use nmea::rebuild_nema;
use std::time::{Duration, Instant};

pub struct StreamToQXWZ(TcpStream);
pub struct GpggaSender(TcpStream);

// const ASK: &str = "\
// GET / HTTP/1.1\r\n\
// Accept: */*\r\n\
// \r\n";

macro_rules! AUTH_TEMPLATE {
    () => {
        "\
GET /AUTO HTTP/1.1\r\n\
Authorization: Basic {}\r\n\
\r\n"
    };
}

const AUTH: &str = "cXh3cWh6MDAzOjlmODcyMjA=";

impl GpggaSender {
    pub async fn send(&mut self, tail: &str, cs: u8) {
        let line = format!("{}\r\n", rebuild_nema("GPGGA", tail, cs));
        let _ = self.0.write_all(line.as_bytes()).await;
    }
}

impl StreamToQXWZ {
    pub fn get_sender(&self) -> GpggaSender {
        GpggaSender(self.0.clone())
    }
}

impl Driver for StreamToQXWZ {
    type Pacemaker = ();
    type Key = String;
    type Event = Vec<u8>;

    fn keys() -> Vec<Self::Key> {
        vec![AUTH.into()]
    }

    fn open_timeout() -> std::time::Duration {
        Duration::from_secs(1)
    }

    fn new(t: &Self::Key) -> Option<(Self::Pacemaker, Self)> {
        task::block_on(async move {
            let auth = format!(AUTH_TEMPLATE!(), t);
            let mut tcp = match TcpStream::connect("203.107.45.154:8002").await {
                Ok(tcp) => tcp,
                Err(_) => return None,
            };
            if let Err(_) = tcp.write_all(auth.as_bytes()).await {
                return None;
            }
            let mut line = String::new();
            let mut reader = BufReader::new(tcp);
            match reader.read_line(&mut line).await {
                Ok(_) => {
                    if line.as_str() == "ICY 200 OK" {
                        Some(((), Self(reader.into_inner())))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        })
    }

    fn join<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&mut Self, Option<(std::time::Instant, Self::Event)>) -> bool,
    {
        task::block_on(async move {
            let mut buf = [0u8; 1024];
            loop {
                match self.0.read(&mut buf).await {
                    Ok(0) | Err(_) => return false,
                    Ok(n) => {
                        // 如果回调指示不要继续阻塞，立即退出
                        if !f(self, Some((Instant::now(), buf[..n].to_vec()))) {
                            return true;
                        }
                    }
                }
            }
        })
    }
}

#[test]
fn assert_connect() {
    assert!(StreamToQXWZ::new(&AUTH.into()).is_some())
}
