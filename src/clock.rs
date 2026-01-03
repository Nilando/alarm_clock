use arduino_hal::prelude::*;
use arduino_hal::I2c;
use panic_halt as _;

// DS3231 I2C Address
const DS3231_ADDR: u8 = 0x68;

// DS3231 Register Addresses
const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x01;
const REG_HOURS: u8 = 0x02;
const REG_DAY: u8 = 0x03;
const REG_DATE: u8 = 0x04;
const REG_MONTH: u8 = 0x05;
const REG_YEAR: u8 = 0x06;


// Convert BCD to decimal
fn bcd_to_dec(val: u8) -> u8 {
    (val / 16 * 10) + (val % 16)
}

// Convert decimal to BCD
fn dec_to_bcd(val: u8) -> u8 {
    (val / 10 * 16) + (val % 10)
}

// Read time from DS3231
pub fn read_time(i2c: &mut I2c) -> Result<(u8, u8, u8, u8, u8, u8, u8), arduino_hal::i2c::Error> {
    let mut buffer = [0u8; 7];

    // Set register pointer to seconds register
    i2c.write(DS3231_ADDR, &[REG_SECONDS])?;

    // Small delay
    arduino_hal::delay_us(100);

    // Read 7 bytes (seconds through year)
    i2c.read(DS3231_ADDR, &mut buffer)?;

    let seconds = bcd_to_dec(buffer[0] & 0x7F);
    let minutes = bcd_to_dec(buffer[1] & 0x7F);
    let hours = bcd_to_dec(buffer[2] & 0x3F);
    let day = bcd_to_dec(buffer[3] & 0x07);
    let date = bcd_to_dec(buffer[4] & 0x3F);
    let month = bcd_to_dec(buffer[5] & 0x1F);
    let year = bcd_to_dec(buffer[6]);

    Ok((hours, minutes, seconds, day, date, month, year))
}

// Set time on DS3231
pub fn set_time(i2c: &mut I2c, hours: u8, minutes: u8, seconds: u8, day: u8, date: u8, month: u8, year: u8) -> Result<(), arduino_hal::i2c::Error> {
    let data = [
        REG_SECONDS,
        dec_to_bcd(seconds),
        dec_to_bcd(minutes),
        dec_to_bcd(hours),
        dec_to_bcd(day),
        dec_to_bcd(date),
        dec_to_bcd(month),
        dec_to_bcd(year),
    ];

    i2c.write(DS3231_ADDR, &data)
}
