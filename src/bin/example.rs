use async_std::{
    net::UdpSocket,
    sync::{Arc, Mutex},
    task,
};
use driver::{SupervisorEventForSingle::*, SupervisorForSingle};
use gnss::{LocalReference, WGS84};
use nmea::NmeaLine::*;
use rtk_qxwz::{
    display::{vertex, ENU_TOPIC},
    monitor_tool::{palette, rgba, vertex, Encoder},
};
use rtk_qxwz::{AuthFile, GpggaSender, QXWZService, RTCMReceiver, RTKBoard};
use std::{f32::consts::FRAC_PI_2, time::Duration};

fn main() {
    let sender: Arc<Mutex<Option<GpggaSender>>> = Arc::new(Mutex::new(None));
    let receiver: Arc<Mutex<Option<RTCMReceiver>>> = Arc::new(Mutex::new(None));
    {
        let sender = sender.clone();
        let receiver = receiver.clone();
        task::spawn_blocking(move || {
            SupervisorForSingle::<QXWZService<AuthFile>>::default().join(|e| {
                match e {
                    Connected(_, stream) => {
                        println!("qxwz connected");
                        *task::block_on(sender.lock()) = Some(stream.get_sender());
                    }
                    Disconnected => {
                        println!("qxwz disconnected");
                        *task::block_on(sender.lock()) = None;
                    }
                    Event(_, Some((_, buf))) => {
                        if let Some(ref mut receiver) = *task::block_on(receiver.lock()) {
                            receiver.receive(buf.as_slice());
                        }
                    }
                    Event(_, None) => {}
                    ConnectFailed => {
                        println!("qxwz connect failed");
                        task::block_on(task::sleep(Duration::from_secs(3)));
                    }
                }
                true
            });
        });
    }

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
                println!("Port = COM{}", port);
                *task::block_on(receiver.lock()) = Some(board.get_receiver());
            }
            Disconnected => {
                println!("Serial disconnected.");
                *task::block_on(receiver.lock()) = None;
            }
            Event(_, Some((_, (line, cs)))) => match line {
                GPGGA(_, tail) => {
                    task::block_on(async {
                        if let Some(ref mut s) = *sender.lock().await {
                            s.send(tail.as_str(), cs).await;
                        }
                    });
                }
                GPHPD(body) => {
                    use nmea::gphpd::Status::*;

                    let wgs84 = WGS84 {
                        latitude: body.latitude as f64 * 1e-7,
                        longitude: body.longitude as f64 * 1e-7,
                        altitude: body.altitude as f64 * 1e-2,
                    };
                    let enu = reference.wgs84_to_enu(wgs84);
                    let dir = (body.heading as f32) * 1e-3;
                    let level = match body.status{
                        GPS定位 => 1,
                        GPS定向 => 2,
                        RTK定位 => 3,
                        RTK定向 => 4,
                        _ => 0,
                    };
                    let vertex = vertex!(level; enu.e as f32, enu.n as f32; Arrow, FRAC_PI_2 - dir.to_radians(); 64) ;
                    let packet = Encoder::with(|encoder| encoder.topic("gphpd").push(vertex));
                    let _ = task::block_on(socket.send(&packet));
                }
                GPFPD(body) => {
                    let wgs84 = WGS84 {
                        latitude: body.latitude as f64 * 1e-7,
                        longitude: body.longitude as f64 * 1e-7,
                        altitude: body.altitude as f64 * 1e-2,
                    };
                    let enu = reference.wgs84_to_enu(wgs84);
                    let dir = (body.heading as f32) * 1e-3;
                    let vertex = vertex(body.status, enu, FRAC_PI_2 - dir.to_radians());
                    let packet = Encoder::with(|encoder| encoder.topic(ENU_TOPIC).push(vertex));
                    let _ = task::block_on(socket.send(&packet));
                }
                _ => println!("{:?}", line),
            },
            Event(_, None) => {}
            ConnectFailed => {
                println!("Serial failed to connect.");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        true
    });
}

fn send_config(socket: Arc<UdpSocket>, period: Duration) {
    let clear = Encoder::with(|encoder| encoder.topic(ENU_TOPIC).clear());
    let topic = Encoder::with(|encoder| {
        encoder.config_topic(
            "gphpd",
            36000,
            500,
            &[
                (0, rgba!(GRAY; 0.2)),
                (1, rgba!(NAVY; 0.5)),
                (2, rgba!(BLUE; 0.5)),
                (3, rgba!(DEEPSKYBLUE; 0.5)),
                (4, rgba!(CYAN; 0.5)),
            ],
            |_| {},
        );
        encoder.config_topic(
            "gphpd",
            36000,
            500,
            &[
                (0, rgba!(GRAY; 0.2)),
                (1, rgba!(RED; 0.2)),
                (2, rgba!(ORANGE; 0.5)),
                (3, rgba!(YELLOW; 0.5)),
                (4, rgba!(GREEN; 0.5)),
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
