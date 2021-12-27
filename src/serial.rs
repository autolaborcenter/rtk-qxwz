use crate::nmea::Buffer;
use driver::Driver;
use serial_port::{Port, PortKey, SerialPort};
use std::{
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

const OPEN_TIMEOUT: Duration = Duration::from_millis(3000);
const LINE_RECEIVE_TIMEOUT: Duration = Duration::from_millis(2500);

pub struct RTKBoard {
    port: Arc<Port>,
    buf: Buffer<256>,
    last_time: Instant,
}

pub struct RTCMReceiver(Weak<Port>);

impl RTCMReceiver {
    pub fn receive(&self, buf: &[u8]) {
        if let Some(p) = self.0.upgrade() {
            let _ = p.write(buf);
        }
    }
}

impl RTKBoard {
    pub fn get_receiver(&self) -> RTCMReceiver {
        RTCMReceiver(Arc::downgrade(&self.port))
    }
}

impl Driver for RTKBoard {
    type Pacemaker = ();
    type Key = PortKey;
    type Event = String;

    fn keys() -> Vec<Self::Key> {
        Port::list().into_iter().map(|id| id.key).collect()
    }

    fn open_timeout() -> std::time::Duration {
        OPEN_TIMEOUT
    }

    fn new(t: &Self::Key) -> Option<(Self::Pacemaker, Self)> {
        Port::open(t, 115200, LINE_RECEIVE_TIMEOUT.as_millis() as u32)
            .ok()
            .map(|port| {
                (
                    (),
                    Self {
                        port: Arc::new(port),
                        buf: Buffer::new(),
                        last_time: Instant::now(),
                    },
                )
            })
    }

    fn join<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&mut Self, Option<(std::time::Instant, Self::Event)>) -> bool,
    {
        let mut time = Instant::now();
        loop {
            if let Some(line) = self.buf.parse() {
                time = self.last_time;
                let line = line.into();
                // 如果回调指示不要继续阻塞，立即退出
                if !f(self, Some((time, line))) {
                    return true;
                }
            }
            // 解析超时
            else if self.last_time > time + LINE_RECEIVE_TIMEOUT {
                return false;
            }
            // 接收
            else {
                let buf = self.buf.to_write();
                if let Some(n) = self.port.read(buf).filter(|n| *n > 0) {
                    self.last_time = Instant::now();
                    self.buf.extend(n);
                }
                // 接收失败
                else {
                    return false;
                }
            }
        }
    }
}
