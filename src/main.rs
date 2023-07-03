pub(crate) mod custom_messages;
use custom_messages::{i24};

use tokio_serial::{SerialPortBuilderExt,};
use tokio;
use tokio_util;
use simplebgc::{self, ParamsQuery, Payload};
use anyhow::{Context as _, Result as _};
use futures::{StreamExt as _, SinkExt as _};

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
        let mut msg_data = custom_messages::RequestStreamInterval_Custom::default();
        msg_data.interval = 1;
        msg_data.realtime_data_custom_flags = 1 << 11; //ENCODER_RAW24[3]
        msg_data.sync_to_data = true;

        messages_tx.send(
            simplebgc::OutgoingCommand::RawMessage(simplebgc::RawMessage{
                typ: simplebgc::constants::CMD_DATA_STREAM_INTERVAL,
                payload: Payload::to_bytes(&msg_data), 
            })
        ).await.unwrap();
    }

    loop { 
        let msg = messages_rx.next().await.unwrap().unwrap();
        match msg {
            simplebgc::IncomingCommand::RawMessage(msg) => {
                match msg.typ{
                    simplebgc::constants::CMD_REALTIME_DATA_CUSTOM  => {
                        let msg_data: custom_messages::RealTimeDataCustom_Encoders = Payload::from_bytes(msg.payload).unwrap();
                        let (i24(roll), i24(pitch), i24(yaw)) = (msg_data.encoder_raw24.roll, msg_data.encoder_raw24.pitch, msg_data.encoder_raw24.yaw);
                        //convert to fractions of turn
                        let (roll, pitch, yaw) = (roll as f64 / (1<<24) as f64, pitch as f64 / (1<<24) as f64, yaw as f64 / (1<<24) as f64);
                        //subtract offsets
                        let (roll, pitch, yaw) = (roll, pitch - offset_pitch, yaw - offset_yaw);
                        // to degrees
                        let (roll,pitch,yaw) = (roll * 360.0, pitch * 360.0, yaw * 360.0);
                        timestamp!(); println!("{}\t{}\t", yaw, pitch);
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
