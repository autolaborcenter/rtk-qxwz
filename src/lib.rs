mod network;
mod serial;

pub use base64::encode as encode_base64;
pub use network::{AuthFile, GpggaSender, QXWZAccount, QXWZService};
pub use serial::{RTCMReceiver, RTKBoard};

pub extern crate nmea;

#[cfg(feature = "display")]
pub extern crate monitor_tool;

/// 预封装，使用更方便
pub mod prefab {
    use super::*;
    use async_std::{
        sync::{Arc, Mutex},
        task,
    };
    use driver::{SupervisorEventForSingle::*, SupervisorForSingle};
    use std::time::Duration;

    const RETRY: Duration = Duration::from_secs(3);

    /// 启动千寻位置服务
    pub fn spawn_qxwz<T: QXWZAccount>(
        sender: Arc<Mutex<Option<GpggaSender>>>,
        receiver: Arc<Mutex<Option<RTCMReceiver>>>,
    ) {
        task::spawn_blocking(move || {
            SupervisorForSingle::<QXWZService<T>>::default().join(|e| {
                match e {
                    Connected(_, stream) => {
                        *task::block_on(sender.lock()) = Some(stream.get_sender());
                    }
                    Disconnected => {
                        *task::block_on(sender.lock()) = None;
                    }
                    Event(_, Some((_, buf))) => {
                        if let Some(ref mut receiver) = *task::block_on(receiver.lock()) {
                            receiver.receive(buf.as_slice());
                        }
                    }
                    Event(_, None) => {}
                    ConnectFailed => task::block_on(task::sleep(RETRY)),
                }
                true
            });
        });
    }
}

#[cfg(feature = "display")]
pub mod display {
    use gnss::Enu;
    use monitor_tool::{vertex, Vertex};
    use nmea::gpfpd::{RtkStatus::*, Status, SystemStatus::*};

    pub const ENU_TOPIC: &str = "enu";

    pub fn vertex(status: Status, enu: Enu, dir: f32) -> Vertex {
        let Status(a, b) = status;
        let level = match (a, b) {
            (纯惯性, _) => 1,
            (RTK, Gps1Bd | 双模) => 2,
            (RTK, RTK浮点解) => 3,
            (RTK, RTK固定解) => 4,
            (差分定向, Gps1Bd | 双模) => 5,
            (差分定向, RTK浮点解) => 6,
            (差分定向, RTK固定解) => 7,
            _ => 0,
        };
        vertex!(level; enu.e as f32, enu.n as f32; Arrow, dir; 64)
    }
}

// 配置星网宇达
// use nmea::cmd::Body::*;
// driver.send(&Set("coordinate,-x,y,-z".into()));
// driver.send(&Set("leverarm,gnss,0,0.28,0".into()));
// driver.send(&Set("headoffset,180".into()));
// driver.send(&Undefined("output".into(), "com0,Null".into()));
// driver.send(&Undefined("output".into(), "com1,Null".into()));
// driver.send(&Undefined("output".into(), "com1,gpgga,1".into()));
// driver.send(&Undefined("output".into(), "com1,gpfpd,0.1".into()));
// driver.send(&Undefined("save".into(), "config".into()));
