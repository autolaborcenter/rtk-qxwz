use async_std::{
    net::UdpSocket,
    sync::{Arc, Mutex},
    task,
};
use driver::{SupervisorEventForSingle::*, SupervisorForSingle};
use gnss::{Enu, LocalReference, WGS84};
use monitor_tool::{palette, rgba, vertex, Encoder};
use rtk_qxwz::{
    AuthFile, Gpgga, GpggaParseError::*, GpggaSender, GpggaStatus::*, QXWZService, RTCMReceiver,
    RTKBoard,
};
use std::time::Duration;

macro_rules! float {
    ($pair:expr) => {
        $pair.0 as f64 * 0.1f64.powi($pair.1 as i32)
    };
}

fn main() {
    let sender: Arc<Mutex<Option<GpggaSender>>> = Arc::new(Mutex::new(None));
    let receiver: Arc<Mutex<Option<RTCMReceiver>>> = Arc::new(Mutex::new(None));
    {
        let sender = sender.clone();
        let receiver = receiver.clone();
        task::spawn_blocking(move || {
            SupervisorForSingle::<QXWZService<AuthFile>>::default().join(|e| {
                task::block_on(async {
                    match e {
                        Connected(_, stream) => {
                            eprintln!("qxwz connected");
                            *sender.lock().await = Some(stream.get_sender());
                        }
                        Disconnected => {
                            eprintln!("qxwz disconnected");
                            *sender.lock().await = None;
                        }
                        Event(_, Some((_, buf))) => {
                            if let Some(ref mut receiver) = *receiver.lock().await {
                                receiver.receive(buf.as_slice());
                            }
                        }
                        Event(_, None) => {}
                        ConnectFailed => {
                            eprintln!("qxwz connect failed");
                            task::sleep(Duration::from_secs(3)).await;
                        }
                    }
                });
                true
            });
        });
    }

    let reference = LocalReference::from(WGS84 {
        latitude: 39.595678,
        longitude: 116.196329,
        altitude: 40.00,
    });
    let socket = Arc::new(task::block_on(UdpSocket::bind("0.0.0.0:0")).unwrap());
    let _ = task::block_on(socket.connect("127.0.0.1:12345"));
    send_config(socket.clone(), Duration::from_secs(3));

    SupervisorForSingle::<RTKBoard>::default().join(|e| {
        task::block_on(async {
            match e {
                Connected(port, board) => {
                    eprintln!("Port = COM{}", port);
                    *receiver.lock().await = Some(board.get_receiver());
                }
                Disconnected => {
                    eprintln!("Serial disconnected.");
                    *receiver.lock().await = None;
                }
                Event(_, Some((_, line))) => match line.parse::<Gpgga>() {
                    Ok(gpgga) => {
                        if let Some(ref mut sender) = *sender.lock().await {
                            sender.send(&line).await;
                        }
                        println!("{:?}", gpgga);
                        let enu = reference.wgs84_to_enu(WGS84 {
                            latitude: float!(gpgga.latitude),
                            longitude: float!(gpgga.longitude),
                            altitude: float!(gpgga.altitude),
                        });
                        match gpgga.status {
                            无效解 | 用户输入 | 航位推算 | PPS | PPP => {}
                            单点解 => paint(&socket, 0, enu).await,
                            伪距差分 => paint(&socket, 1, enu).await,
                            浮点解 => paint(&socket, 2, enu).await,
                            固定解 => paint(&socket, 3, enu).await,
                        }
                    }
                    Err(WrongHead) => {}
                    Err(_) => {
                        if let Some(ref mut sender) = *sender.lock().await {
                            sender.send(&line).await;
                        }
                    }
                },
                Event(_, None) => {}
                ConnectFailed => {
                    eprintln!("Serial failed to connect.");
                    task::sleep(Duration::from_secs(1)).await;
                }
            }
        });
        true
    });
}

fn send_config(socket: Arc<UdpSocket>, period: Duration) {
    let clear = Encoder::with(|encoder| encoder.topic("enu").clear());
    let topic = Encoder::with(|encoder| {
        encoder.config_topic(
            "enu",
            36000,
            500,
            &[
                (0, rgba!(GRAY; 0.25)),
                (1, rgba!(RED; 0.25)),
                (2, rgba!(YELLOW; 0.5)),
                (3, rgba!(GREEN; 0.5)),
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

#[inline]
async fn paint(socket: &Arc<UdpSocket>, level: u8, enu: Enu) {
    let _ = socket
        .send(&Encoder::with(|encoder| {
            encoder.topic("enu").push(vertex!(level; enu.e, enu.n; 64));
        }))
        .await;
}
