use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct AirQuality {
    pub timestamp: DateTime<Local>,
    pub co2: u16,
    pub tvoc: u16,
    pub h2: u16,
    pub etha: u16,
}
