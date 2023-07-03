use simplebgc::{payload_rpy, Payload, PayloadParseError};
use simplebgc_derive::{BgcPayload};
use bytes::{BufMut, Bytes, BytesMut};

///representation of 24-bit signed integer, for encoder data sbgc messages
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct i24 (pub i32);

impl simplebgc::Payload for i24 {
    fn from_bytes(b: bytes::Bytes) -> Result<Self, simplebgc::PayloadParseError>
        where
            Self: Sized 
    {
        assert_eq!(b.len(), 3);
        let (b0, b1, b2) = (b[0], b[1], b[2]);
        let mut value = b0 as i32 + ((b1 as i32) << 8) + ((b2 as i32) << 16);
        if value >= 1<<23 {
            value -= 1<<24;
        }
        return Ok(i24(value));
    }

    fn to_bytes(&self) -> bytes::Bytes
        where
            Self: Sized 
    {
        let i24(mut val) = self;
        //reinterpret_cast to unsigned24:
        if val < 0 {
            val += 1<<24;
        }
        assert!(val >= 0);
        assert!(val < 1<<24);
        //make the bytes from unsigned
        return Bytes::copy_from_slice(&[
            (val & 0xFF) as u8,
            (val >> 8 & 0xFF) as u8,
            (val >> 16 & 0xFF) as u8,
        ]);
        
    }
}

//copied to here, because using the one 
//from sbgc causes "error[E0117]: only traits 
//defined in the current crate can be implemented for 
//types defined outside of the crate"
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollPitchYaw<T> {
    pub roll: T,
    pub pitch: T,
    pub yaw: T,
}

payload_rpy!(i24, 3);

#[derive(BgcPayload, Clone, Debug, PartialEq)]
pub struct RealTimeDataCustom_Encoders {
    #[kind(raw)]
    pub timestamp_ms: u16,

    #[kind(payload)]
    #[size(9)]
    pub encoder_raw24: RollPitchYaw<i24>,
}

#[derive(BgcPayload, Clone, Debug, PartialEq)]
pub struct RequestStreamInterval_Custom {
    #[kind(raw)]
    pub cmd_id: u8,

    ///milliseconds or sample ratedivisor, depending on sync_to_data
    #[kind(raw)]
    pub interval:  u16,

    #[kind(raw)]
    pub realtime_data_custom_flags: u32,

    #[kind(raw)]
    pub padding0: u32,

    #[kind(raw)]
    #[format(u8)]
    pub sync_to_data: bool,
    
    #[kind(raw)]
    pub padding1: [u8; 9],
}

impl Default for RequestStreamInterval_Custom {
    fn default() -> Self {
        return RequestStreamInterval_Custom { 
            cmd_id: simplebgc::constants::CMD_REALTIME_DATA_CUSTOM, 
            interval: 1, 
            realtime_data_custom_flags: 0, 
            padding0: 0, 
            sync_to_data: true, 
            padding1: [0; 9], 
        }
    }
}