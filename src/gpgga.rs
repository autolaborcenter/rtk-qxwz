use std::str::FromStr;

#[derive(Default, Debug)]
pub struct Gpgga {
    pub utc: f32,
    pub latitude: f64,
    pub longitude: f64,
    pub status: GpggaStatus,
    pub satellite: u8,
    pub hdop: f32,
    pub altitude: f64,
    pub altitude_error: f64,
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
            result.utc = field!("utc"; body);
            // latitude
            result.latitude = field!("latitude"; body, parse_degree);
            match body.next() {
                Some("N") => {}
                Some("S") => result.latitude = -result.latitude,
                Some(_) => return Err(FailToParse("latitude_dir")),
                None => return Err(LackOfField("latitude_dir")),
            }
            // longitude
            result.longitude = field!("longitude"; body, parse_degree);
            match body.next() {
                Some("E") => {}
                Some("W") => result.longitude = -result.longitude,
                Some(_) => return Err(FailToParse("longitude_dir")),
                None => return Err(LackOfField("longitude_dir")),
            }
            // status
            result.status = field!("status"; body);
            // satellite
            result.satellite = field!("satellite"; body);
            // hdop
            result.hdop = field!("hdop"; body);
            // altitude
            result.altitude = field!("altitude"; body);
            match body.next() {
                Some("M") => {}
                Some(_) => return Err(FailToParse("altitude_unit")),
                None => return Err(LackOfField("altitude_unit")),
            }
            // altitude_error
            result.altitude_error = field!("altitude_error"; body);
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

/// 度分格式转十进制度
fn parse_degree(word: &str) -> Option<f64> {
    let split = word.find('.')? - 2;
    let degrees = &word[..split].parse::<f64>().ok()?;
    let minutes = &word[split..].parse::<f64>().ok()?;
    Some(degrees + minutes / 60.0)
}
