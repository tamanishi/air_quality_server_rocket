use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MyBaseline {
    pub co2eq: u16,
    pub tvoc: u16,
}
