#![no_std]
#![no_main]

use alarm_clock::Device;
use core::prelude::v1::Ok;

#[arduino_hal::entry]
fn main() -> ! {
    loop {
        let _ = match Device::init() {
            Ok(mut device) => device.main_loop(),
            Err(_) => { Ok(()) }
        };
    }
}
