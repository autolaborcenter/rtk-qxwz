use async_std::{
    net::UdpSocket,
    sync::{Arc, Mutex},
    task,
};
use driver::{SupervisorEventForSingle::*, SupervisorForSingle};
use gnss::{LocalReference, WGS84};
use monitor_tool::{rgba, vertex, Encoder};
use nmea::gpfpd::{RtkStatus, Status, SystemStatus};
use rtk_qxwz::{GpggaSender, RTCMReceiver, RTKBoard, StreamToQXWZ};
use std::{f32::consts::FRAC_PI_2, time::Duration};

fn main() {
    let sender: Arc<Mutex<Option<GpggaSender>>> = Arc::new(Mutex::new(None));
    let receiver: Arc<Mutex<Option<RTCMReceiver>>> = Arc::new(Mutex::new(None));
    spawn_qxwz(sender.clone(), receiver.clone());

    const REFERENCE: WGS84 = WGS84 {
        latitude: 39.9926296,
        longitude: 116.3270623,
        altitude: 54.12,
    };
    let reference = LocalReference::from(REFERENCE);
    let socket = Arc::new(task::block_on(UdpSocket::bind("0.0.0.0:0")).unwrap());
    let _ = task::block_on(socket.connect("127.0.0.1:12345"));
    send_config(socket.clone(), Duration::from_secs(3));

    SupervisorForSingle::<RTKBoard>::default().join(|e| {
        match e {
            Connected(port, board) => {
                println!("port = COM{}", port);
                *task::block_on(receiver.lock()) = Some(board.get_receiver());
            }
            Disconnected => {
                *task::block_on(receiver.lock()) = None;
            }
            Event(_, Some((_, (line, cs)))) => {
                use nmea::NmeaLine::*;
                match line {
                    GPGGA(_, tail) => {
                        task::block_on(async {
                            if let Some(ref mut s) = *sender.lock().await {
                                s.send(tail.as_str(), cs).await;
                            }
                        });
                    }
                    GPFPD(body) => {
                        let wgs84 = WGS84 {
                            latitude: body.latitude as f64 * 1e-7,
                            longitude: body.longitude as f64 * 1e-7,
                            altitude: body.altitude as f64 * 1e-2,
                        };
                        let enu = reference.wgs84_to_enu(wgs84);
                        let dir = (body.heading as f32) * 1e-3;
                        println!(
                            "{:?} |({}, {})| {:?} | {}",
                            body.status, body.nsv1, body.nsv2, enu, dir
                        );
                        let vertex = vertex!(status_level(body.status); enu.e as f32, enu.n as f32; Arrow, FRAC_PI_2 - dir.to_radians(); 64);
                        let socket = socket.clone();
                        task::spawn(async move {
                            let _ = socket.send(&Encoder::with(|encoder| encoder.topic("enu").push(vertex))).await;
                        });
                    }
                    _ => println!("{:?}", line),
                }
            }
            Event(_, None) => {}
            ConnectFailed => {
                println!("serial failed to connect.");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        true
    });
}

fn spawn_qxwz(sender: Arc<Mutex<Option<GpggaSender>>>, receiver: Arc<Mutex<Option<RTCMReceiver>>>) {
    task::spawn_blocking(move || {
        SupervisorForSingle::<StreamToQXWZ>::default().join(|e| {
            match e {
                Connected(key, stream) => {
                    println!("key = {}", key);
                    *task::block_on(sender.lock()) = Some(stream.get_sender());
                }
                Disconnected => {
                    *task::block_on(sender.lock()) = None;
                }
                Event(_, Some((_, buf))) => {
                    println!("forward. len = {}", buf.len());
                    if let Some(ref mut r) = *task::block_on(receiver.lock()) {
                        r.receive(buf.as_slice());
                    }
                }
                Event(_, None) => {}
                ConnectFailed => {
                    println!("network failed to connect.");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
            true
        });
    });
}

fn send_config(socket: Arc<UdpSocket>, period: Duration) {
    let clear = Encoder::with(|encoder| encoder.topic("enu").clear());
    let topic = Encoder::with(|encoder| {
        encoder.config_topic(
            "enu",
            50000,
            500,
            &[
                (0, rgba!(GRAY; 0.2)),
                (1, rgba!(RED; 0.2)),
                (2, rgba!(ORANGE; 0.5)),
                (3, rgba!(YELLOW; 0.5)),
                (4, rgba!(GREEN; 0.5)),
                (5, rgba!(LIGHTBLUE; 0.5)),
                (6, rgba!(BLUE; 0.5)),
                (7, rgba!(VIOLET; 0.5)),
            ],
            |_| {},
        );
    });
    task::spawn(async move {
        let _ = socket.send(&clear).await;
        loop {
            let _ = socket.send(&topic).await;
            task::sleep(period).await;
        }
    });
}

fn status_level(status: Status) -> u8 {
    use RtkStatus::*;
    use SystemStatus::*;
    let Status(a, b) = status;
    match (a, b) {
        (纯惯性, _) => 1,
        (RTK, Gps1Bd | 双模) => 2,
        (RTK, RTK浮点解) => 3,
        (RTK, RTK固定解) => 4,
        (差分定向, Gps1Bd | 双模) => 5,
        (差分定向, RTK浮点解) => 6,
        (差分定向, RTK固定解) => 7,
        _ => 0,
    }
}
