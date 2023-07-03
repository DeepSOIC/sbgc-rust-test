# sbgc-rust-test
my sandbox for talking to a SimpleBGC gimbal with Rust

uses modified version of [simplebgc-rs](https://github.com/CUAir/simplebgc-rs) - https://github.com/DeepSOIC/simplebgc-rs/tree/drone-dev

# functionality

uses tokio and async.

1) fetches encoder offsets from the gimbal (CMD_READ_PARAMS_EXT)
2) enables streaming of encoder values (CMD_DATA_STREAM_INTERVAL), parses the incoming stream (CMD_REALTIME_DATA_CUSTOM) and converts the received encoder values into angles in degrees
3) moves the gimbal in speed mode for a little while, then homes it to neutral position (into frame-follow mode)

# issues

* on windows, encoder data comes in huge chunks each 2-ish seconds - pretty unusable. (On Ubuntu 20.04, the messages do actually come in promptly.)

# running

1) clone this.
2) clone https://github.com/DeepSOIC/simplebgc-rs/tree/drone-dev to a directory `../simplebgc-rs` . Make sure you're using `drone-dev` branch.
3) edit serial port name in main.rs
4) `cargo run`