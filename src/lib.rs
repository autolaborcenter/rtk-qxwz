mod gpgga;
mod network;
mod nmea;
mod serial;

pub use base64::encode as encode_base64;
pub use gpgga::{Gpgga, GpggaParseError, GpggaStatus};
pub use network::{AuthFile, GpggaSender, QXWZAccount, QXWZService};
pub use serial::{RTCMReceiver, RTKBoard};

#[cfg(feature = "display")]
pub extern crate monitor_tool;

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
