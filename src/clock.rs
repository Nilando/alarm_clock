use arduino_hal::prelude::*;
use panic_halt as _;
use arduino_hal::{
    I2c,
    port::{
        A3, Pin, mode::{Input, PullUp}
    }
};

// DS3231 I2C Address
const DS3231_ADDR: u8 = 0x68;

// DS3231 Register Addresses
const REG_SECONDS: u8 = 0x00;
const _REG_MINUTES: u8 = 0x01;
const _REG_HOURS: u8 = 0x02;
const _REG_DAY: u8 = 0x03;
const _REG_DATE: u8 = 0x04;
const _REG_MONTH: u8 = 0x05;
const _REG_YEAR: u8 = 0x06;
const REG_ALARM: u8 = 0x07;
const REG_CONTROL: u8 = 0x0E;
const REG_STATUS: u8 = 0x0F;

// Convert BCD to decimal
fn bcd_to_dec(val: u8) -> u8 {
    (val / 16 * 10) + (val % 16)
}

// Convert decimal to BCD
fn dec_to_bcd(val: u8) -> u8 {
    (val / 10 * 16) + (val % 10)
}


pub struct ClockController {
    i2c: I2c,
    alarm_pin: Pin<Input<PullUp>, A3>,     // PC3
}

impl ClockController {
    pub fn new(
        i2c: I2c,
        alarm_pin: Pin<Input<PullUp>, A3>,     // PC3
    ) -> Self {
        Self {
            i2c,
            alarm_pin
        }
    }

    pub fn is_alarm_triggered(&self) -> bool {
        self.alarm_pin.is_low()
    }

    pub fn reset(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.set_time(0, 0, 0, 0, 01, 01, 26)?;
        arduino_hal::delay_ms(1);
        self.clear_alarm_flag()?;
        arduino_hal::delay_ms(1);
        self.disable_alarm()?;
        Ok(())
    }

    pub fn read_time(&mut self) -> Result<(u8, u8, u8, u8, u8, u8, u8), arduino_hal::i2c::Error> {
        let mut buffer = [0u8; 7];

        // Set register pointer to seconds register
        self.i2c.write(DS3231_ADDR, &[REG_SECONDS])?;

        // Small delay
        arduino_hal::delay_us(100);

        // Read 7 bytes (seconds through year)
        self.i2c.read(DS3231_ADDR, &mut buffer)?;

        let seconds = bcd_to_dec(buffer[0] & 0x7F);
        let minutes = bcd_to_dec(buffer[1] & 0x7F);
        let hours = bcd_to_dec(buffer[2] & 0x3F);
        let day = bcd_to_dec(buffer[3] & 0x07);
        let date = bcd_to_dec(buffer[4] & 0x3F);
        let month = bcd_to_dec(buffer[5] & 0x1F);
        let year = bcd_to_dec(buffer[6]);

        Ok((hours, minutes, seconds, day, date, month, year))
    }

    pub fn set_time(&mut self, hours: u8, minutes: u8, seconds: u8, day: u8, date: u8, month: u8, year: u8) -> Result<(), arduino_hal::i2c::Error> {
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

        self.i2c.write(DS3231_ADDR, &data)
    }

    pub fn set_alarm(&mut self, hours: u8, minutes: u8) -> Result<(), arduino_hal::i2c::Error> {
        let data = [
            REG_ALARM,
            0x00, // seconds
            dec_to_bcd(minutes),
            dec_to_bcd(hours),
            0x80, // date
        ];

        self.i2c.write(DS3231_ADDR, &data)
    }

    pub fn enable_alarm(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        let mut control = [0u8];

        self.i2c.write(DS3231_ADDR, &[REG_CONTROL])?;
        self.i2c.read(DS3231_ADDR, &mut control)?;

        control[0] |= 0b0000_0001; // enable alarm

        self.i2c.write(DS3231_ADDR, &[REG_CONTROL, control[0]])?;
        Ok(())
    }

    pub fn disable_alarm(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        let mut control = [0u8];

        self.i2c.write(DS3231_ADDR, &[REG_CONTROL])?;
        self.i2c.read(DS3231_ADDR, &mut control)?;

        control[0] &= !0b0000_0001; // disable alarm

        self.i2c.write(DS3231_ADDR, &[REG_CONTROL, control[0]])?;

        Ok(())
    }

    pub fn clear_alarm_flag(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        let mut status = [0u8];

        self.i2c.write(DS3231_ADDR, &[REG_STATUS])?;
        self.i2c.read(DS3231_ADDR, &mut status)?;

        status[0] &= !0b0000_0001; // clear A1F

        self.i2c.write(DS3231_ADDR, &[REG_STATUS, status[0]])?;
        Ok(())
    }
}

