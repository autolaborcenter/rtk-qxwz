mod network;
mod serial;

pub use network::{GpggaSender, StreamToQXWZ};
pub use serial::{RTCMReceiver, RTKBoard};

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
