#[macro_use]
extern crate structure;

use tokio_serial::{SerialPortBuilderExt, SerialPortBuilder, SerialPort, SerialStream};
use tokio;
use tokio_util;
use std::{sync::mpsc, borrow::BorrowMut};
use simplebgc::{self, ParamsQuery};
use anyhow::{Context as _, Result as _};
use futures::{StreamExt as _, SinkExt as _};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let baud_rate = 115_200;
    let port = "/dev/serial/by-id/usb-Silicon_Labs_CP2102_USB_to_UART_Bridge_Controller_0001-if00-port0";

    let serial_device = tokio_serial::new(port, baud_rate)
        .open_native_async()
        .with_context(|| format!("Failed to open PWM serial device {}", port))?;

    let framed = tokio_util::codec::Framed::new(serial_device, simplebgc::V2Codec::default());
    
    let (mut messages_tx, mut messages_rx) = framed.split();

    let time_ref = std::time::Instant::now();

    macro_rules! timestamp {
        () => {
            print!("{}\t", time_ref.elapsed().as_secs_f64())
        }
    }

    use tokio::time::timeout;
    use std::time::Duration;
    let (mut messages_tx, mut messages_rx, offset_yaw, offset_pitch) = 
    timeout(Duration::from_millis(1000), async move {
        messages_tx.send(
            simplebgc::OutgoingCommand::ReadParamsExt(ParamsQuery{profile_id: 0})
        ).await.unwrap();

        loop { 
            let msg = messages_rx.next().await.unwrap().unwrap();
            match msg {
                simplebgc::IncomingCommand::ReadParamsExt(cmd) => {
                    let (y,p,r) = (cmd.encoder_offset.yaw, cmd.encoder_offset.pitch, cmd.encoder_offset.roll);
                    timestamp!(); println!(" raw offsets y-p-r: {y}, {p}, {r}");
                    let offset_yaw = (cmd.encoder_offset.yaw as f64) / ((1 << 14) as f64);
                    let offset_pitch = (cmd.encoder_offset.pitch as f64) / ((1 << 14) as f64);

                    return Ok((messages_tx, messages_rx, offset_yaw, offset_pitch));
                }
                _ => {
                    timestamp!(); println!("got some other message while waiting for encoder offsets");
                }
            }
        }

        //ugly hack to force concrete error type, to make it compile
        if false {
            return  Err(()); 
        };
    }).await.expect("no response from gimbal (timeout)").expect("data not received");

    timestamp!(); println!("offsets: {offset_yaw}, {offset_pitch}");
    
    { //request realtime encoder data stream
        let payload_struct = structure!("<BHIxxxx?xxxxxxxxx");
        let data = payload_struct.pack(
            88, //CMD_ID = CMD_REALTIME_DATA_CUSTOM
            1, //INTERVAL_MS = 1, that is, each time the data is updated
            //#1 << 3, //FRAME_CAM_ANGLE[3]
            1 << 11, //ENCODER_RAW24[3]
            true //SYNC_TO_DATA
        ).unwrap();
        messages_tx.send(
            simplebgc::OutgoingCommand::RawMessage(simplebgc::RawMessage{
                typ: simplebgc::constants::CMD_DATA_STREAM_INTERVAL,
                payload: Bytes::from(data), 
            })
        ).await.unwrap();
    }

    loop { 
        let msg = messages_rx.next().await.unwrap().unwrap();
        match msg {
            simplebgc::IncomingCommand::RawMessage(msg) => {
                match msg.typ{
                    simplebgc::constants::CMD_REALTIME_DATA_CUSTOM  => {
                        timestamp!(); println!("data!"); // #FIXME: parse
                    }
                    _ => {
                        timestamp!(); println!("unknown message #{}", msg.typ);
                    }
                } 
            }
            msg => {
                timestamp!(); println!("got some other message: {msg:?}");
            }
        }
    }

    Ok(())
}
