use async_std::{
    sync::{Arc, Mutex},
    task,
};
use driver::{SupervisorEventForSingle::*, SupervisorForSingle};
use nmea::rebuild_nema;
use rtk_qxwz::{GpggaSender, RTCMReceiver, RTKBoard, StreamToQXWZ};
use std::time::Duration;

fn main() {
    let sender: Arc<Mutex<Option<GpggaSender>>> = Arc::new(Mutex::new(None));
    let receiver: Arc<Mutex<Option<RTCMReceiver>>> = Arc::new(Mutex::new(None));
    {
        let sender = sender.clone();
        let receiver = receiver.clone();
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
    SupervisorForSingle::<RTKBoard>::default().join(|e| {
        match e {
            Connected(port, board) => {
                println!("port = COM{}", port);
                *task::block_on(receiver.lock()) = Some(board.get_receiver());
            }
            Disconnected => {
                *task::block_on(receiver.lock()) = None;
            }
            Event(_, Some((_, (tail, cs)))) => {
                println!("send: {}", rebuild_nema("GAGGA", tail.as_str(), cs));
                task::block_on(async {
                    if let Some(ref mut s) = *sender.lock().await {
                        s.send(tail.as_str(), cs).await;
                    }
                });
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
