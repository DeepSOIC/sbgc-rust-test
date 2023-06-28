use tokio_serial::SerialPortBuilderExt;
use tokio;
use tokio_util;
use std::sync::mpsc;
use simplebgc::{self, ParamsQuery};
use anyhow::{Context as _, Result as _};
use futures::{StreamExt as _, SinkExt as _};

enum FromGimbalMsg {
    Config(simplebgc::Params3Data), //FIXME: use Ext3
    EncoderData((f64, f64)), //yaw, pitch
    Other(simplebgc::IncomingCommand),
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let (incoming_messages_prod, incoming_messages_cons) = 
        mpsc::channel::<FromGimbalMsg>();
    {

        let baud_rate = 115_200;
        let port = "COM14";

        let serial_device = tokio_serial::new(port, baud_rate)
            .open_native_async()
            .with_context(|| format!("Failed to open PWM serial device {}", port))?;

        // let mut rx_task = tokio::spawn(async move {
        //     gimbal_rx_loop(
        //         incoming_messages_prod,
        //         serial_device,
        //     )
        //     .await;
        // });

        let framed = tokio_util::codec::Framed::new(serial_device, simplebgc::V2Codec);
        
        let (mut messages_tx, mut messages_rx) = framed.split();

        use tokio::time::timeout;
        use std::time::Duration;
        let (mut messages_tx, mut messages_rx, offset_yaw, offset_pitch) = 
        timeout(Duration::from_millis(1000), async move {
            messages_tx.send(
                simplebgc::OutgoingCommand::ReadParamsExt(ParamsQuery{profile_id: 0})
            ).await.unwrap();

            if false {
                return  Err("");
            };
            loop { 
                let msg = messages_rx.next().await.unwrap().unwrap();
                match msg {
                    simplebgc::IncomingCommand::ReadParamsExt(cmd) => {
                        
                        let offset_yaw = (cmd.encoder_offset.yaw as f64) / ((1 << 14) as f64);
                        let offset_pitch = (cmd.encoder_offset.pitch as f64) / ((1 << 14) as f64);

                        break Ok((messages_tx, messages_rx, offset_yaw, offset_pitch));
                    }
                    _ => {
                        println!("got some other message while waiting for encoder offsets");
                    }
                }
            }

            
        }).await.expect("no response from gimbal (timeout)").expect("data not received");

        println!("offsets: {offset_yaw}, {offset_pitch}");
    };
    Ok(())
}

async fn gimbal_rx_loop(
    mut incoming_messages_prod: mpsc::Sender<FromGimbalMsg>,
    serial_device: tokio_serial::SerialStream,
) -> Result<(), Box<dyn std::error::Error>> {

    Ok(())
}
