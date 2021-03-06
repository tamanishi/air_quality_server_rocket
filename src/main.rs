#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod air_quality;
mod baseline;

use std::io::prelude::*;
use std::fs::File;
use std::sync::Mutex;

use chrono::Local;
use linux_embedded_hal::{Delay, I2cdev};
use rocket::State;
use rocket_contrib::json::Json;
use sgp30::{Sgp30, Humidity, Baseline};
use air_quality::AirQuality;
use baseline::MyBaseline;

fn save_baseline(sgp30: &mut Sgp30<I2cdev, Delay>) {
    let base_line: Baseline = sgp30.get_baseline().unwrap();
    let my_base_line = MyBaseline {
        co2eq: base_line.co2eq,
        tvoc: base_line.tvoc,
    };

    let base_line_json = serde_json::to_string(&my_base_line).unwrap();
    println!("read baseline: {:?}", base_line);

    let mut file = File::create("baseline.json").unwrap();
    file.write_all(&base_line_json.as_bytes()).unwrap();
}

fn rel_humidity_to_abs_humidity(temp: f32, rel_humidty: f32) -> f32 {
    // see https://komoriss.com/relative-humidity-volumetric-humidity/
    let e = 6.1078 * 10f32.powf((7.5 * temp) / (temp + 273.3));
    let a = (217.0 * e) / (temp + 273.15);
    a * (rel_humidty / 100.0)
}

fn separate_humidity_value(num: f32) -> [u8; 2] {
    // see https://note.cman.jp/convert/bit/
    let mut value: [u8; 2] = [0, 0];
    let mut temp = num;

    for digit in 0..8 {
        let i = temp as i32;
        let f = temp as f32;

        if digit == 0 {
            value[0] = i as u8;
        }

        temp = (f - (i as f32)) * 2.0;

        if temp >= 1.0 {
            value[1] |= 1;
        }

        if digit < 7 {
            value[1] <<= 1;
        }
    }

    value
}

#[get("/measure?<temp>&<humidity>")]
fn measure(mutex: State<Mutex<Sgp30<I2cdev, Delay>>>, temp: Option<String>, humidity: Option<String>) -> Json<AirQuality> {
    let mut sgp30 = mutex.lock().unwrap();

    save_baseline(&mut sgp30);

    if let (Some(temp), Some(humidity)) = (temp, humidity) {
        // calc absolute humidity
        let abs_humidity = rel_humidity_to_abs_humidity(temp.parse::<f32>().unwrap(), humidity.parse::<f32>().unwrap());
        // separate absolute humidity to integer part and fractional part
        let humidity_array = separate_humidity_value(abs_humidity);
        // tell absolute humidity to sensor
        let humidity = Humidity::new(humidity_array[0], humidity_array[1]).unwrap();
        sgp30.set_humidity(Some(&humidity)).unwrap(); 
    }

    let air_quality = sgp30.measure().unwrap();
    let raw_signals = sgp30.measure_raw_signals().unwrap();

    Json(AirQuality {
        timestamp: Local::now(),
        co2: air_quality.co2eq_ppm,
        tvoc: air_quality.tvoc_ppb,
        h2: raw_signals.h2,
        etha: raw_signals.ethanol,
    })
}

fn rocket() -> rocket::Rocket {
    let dev = I2cdev::new("/dev/i2c-1").unwrap();
    let address = 0x58;
    let mut sgp30 = Sgp30::new(dev, address, Delay);

    println!("Initializing Sgp30 ...");

    sgp30.init().unwrap();

    let file = File::open("baseline.json");

    if let  Ok(file) = file {
        let hoge: MyBaseline = serde_json::from_reader(file).unwrap();
        let base_line_to_load = Baseline {
            co2eq: hoge.co2eq,
            tvoc: hoge.tvoc,
        };
        println!("load baseline: {:?}", base_line_to_load);
        sgp30.set_baseline(&base_line_to_load).unwrap();
    }

    rocket::ignite()
        .mount("/", routes![measure])
        .manage(Mutex::new(sgp30))
}

fn main() {
    rocket().launch();
}
