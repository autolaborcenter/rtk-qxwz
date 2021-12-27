use std::str::FromStr;

#[derive(Default, Debug)]
pub struct Gpgga {
    pub utc: (u32, u8),
    pub latitude: (i64, u8),
    pub longitude: (i64, u8),
    pub status: GpggaStatus,
    pub satellite: u8,
    pub hdop: (u8, u8),
    pub altitude: (i32, u8),
    pub altitude_error: (i32, u8),
}

#[derive(Clone, Copy, Debug)]
pub enum GpggaStatus {
    无效解 = 0,
    单点解 = 1,
    伪距差分 = 2,
    PPS = 3,
    固定解 = 4,
    浮点解 = 5,
    航位推算 = 6,
    用户输入 = 7,
    PPP = 8,
}

pub enum GpggaParseError {
    WrongHead,
    LackOfField(&'static str),
    FailToParse(&'static str),
}

macro_rules! field {
    ($name:expr; $body:ident) => {
        if let Some(word) = $body.next() {
            if let Ok(val) = word.parse() {
                val
            } else {
                return Err(FailToParse($name));
            }
        } else {
            return Err(LackOfField($name));
        }
    };
    ($name:expr; $body:ident, $parse:expr) => {
        if let Some(word) = $body.next() {
            if let Some(val) = $parse(word) {
                val
            } else {
                return Err(FailToParse($name));
            }
        } else {
            return Err(LackOfField($name));
        }
    };
}

impl FromStr for Gpgga {
    type Err = GpggaParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use GpggaParseError::*;
        if let Some(body) = s.strip_prefix("$GPGGA,") {
            let mut body = body.split(',');
            let mut result = Self::default();
            // utc
            result.utc = field!("utc"; body, parse_fix);
            // latitude
            result.latitude = field!("latitude"; body, parse_fix);
            result.latitude.1 += 2;
            match body.next() {
                Some("N") => {}
                Some("S") => result.latitude.0 = -result.latitude.0,
                Some(_) => return Err(FailToParse("latitude_dir")),
                None => return Err(LackOfField("latitude_dir")),
            }
            // longitude
            result.longitude = field!("longitude"; body, parse_fix);
            result.longitude.1 += 2;
            match body.next() {
                Some("E") => {}
                Some("W") => result.longitude.0 = -result.longitude.0,
                Some(_) => return Err(FailToParse("longitude_dir")),
                None => return Err(LackOfField("longitude_dir")),
            }
            // status
            result.status = field!("status"; body);
            // satellite
            result.satellite = field!("satellite"; body);
            // hdop
            result.hdop = field!("hdop"; body, parse_fix);
            // altitude
            result.altitude = field!("altitude"; body, parse_fix);
            match body.next() {
                Some("M") => {}
                Some(_) => return Err(FailToParse("altitude_unit")),
                None => return Err(LackOfField("altitude_unit")),
            }
            // altitude_error
            result.altitude_error = field!("altitude_error"; body, parse_fix);
            match body.next() {
                Some("M") => {}
                Some(_) => return Err(FailToParse("altitude_error_unit")),
                None => return Err(LackOfField("altitude_error_unit")),
            }
            Ok(result)
        } else {
            Err(WrongHead)
        }
    }
}

impl Default for GpggaStatus {
    fn default() -> Self {
        Self::无效解
    }
}

impl FromStr for GpggaStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u8>() {
            Ok(0) => Ok(Self::无效解),
            Ok(1) => Ok(Self::单点解),
            Ok(2) => Ok(Self::伪距差分),
            Ok(3) => Ok(Self::PPS),
            Ok(4) => Ok(Self::固定解),
            Ok(5) => Ok(Self::浮点解),
            Ok(6) => Ok(Self::航位推算),
            Ok(7) => Ok(Self::用户输入),
            Ok(8) => Ok(Self::PPP),
            Ok(_) | Err(_) => Err(()),
        }
    }
}

fn parse_fix<T: FromStr>(word: &str) -> Option<(T, u8)> {
    word.split_once('.').and_then(|(a, b)| {
        let mut buffer = String::with_capacity(a.len() + b.len());
        buffer.extend(a.chars());
        buffer.extend(b.chars());
        Some((buffer.parse().ok()?, b.len() as u8))
    })
}
