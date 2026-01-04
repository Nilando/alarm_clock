use arduino_hal::{
    Adc, 
    I2c,
    port::{
        A3, D2, D9, D10, D11, Pin, mode::{Analog, Floating, Input, Output}
    }
};

use crate::oled::display_time;
use super::oled::{init_oled, clear_oled};
use super::clock::{set_time, read_time};

enum UpdateTimeState {
    UpdatingHour,
    UpdatingMinute,
    UpdatingDay,
    UpdatingMonth,
    UpdatingYear,
}

enum UpdateAlarmState {
    UpdateTime,
    UpdateFrequency
}

enum DeviceState {
    Running,
    UpdateTime,
    UpdateAlarm,
    Alarm
}

enum AlarmFrequency {
    Daily,
    Weekdays,
    Once
}

struct AlarmTime {
    // time
    // frequency
}

use arduino_hal::i2c::Error as I2cError;

pub enum DeviceError {
    I2cError(I2cError)
}

impl From<I2cError> for DeviceError {
    fn from(value: I2cError) -> Self {
        DeviceError::I2cError(value)
    }
}

pub struct Device {
    state: DeviceState,
    alarm: Option<AlarmTime>,
    i2c: I2c,
    button_counter: u8,
    piezo_pin: Pin<Output, D2>,
    main_button: Pin<Input<Floating>, D10>,
    up_button: Pin<Input<Floating>, D11>,
    down_button: Pin<Input<Floating>, D9>,
}

impl Device {
    pub fn init() -> Result<Self, DeviceError> {
        let dp = arduino_hal::Peripherals::take().unwrap();
        let pins = arduino_hal::pins!(dp);
        let mut i2c = I2c::new(
            dp.TWI,
            pins.a4.into_pull_up_input(),  // SDA
            pins.a5.into_pull_up_input(),  // SCL
            50000, 
        );
        let mut piezo_pin: Pin<Output, D2> = pins.d2.into_output(); 
        let main_button: Pin<Input<Floating>, D10> = pins.d10; 
        let up_button: Pin<Input<Floating>, D11> = pins.d11; 
        let down_button: Pin<Input<Floating>, D9> = pins.d9; 

        piezo_pin.set_low();

        init_oled(&mut i2c)?;
        clear_oled(&mut i2c)?;
        set_time(&mut i2c, 21, 23, 0, 2, 29, 12, 25)?;
        display_time(&mut i2c, 21, 23, 0,  false)?;

        Ok(Self {
            state: DeviceState::Running,
            alarm: None,
            button_counter: 0,
            i2c,
            main_button,
            down_button,
            up_button,
            piezo_pin,
        })
    }

    pub fn main_loop(&mut self) -> Result<(), DeviceError> {
        loop {
            arduino_hal::delay_ms(990);

            self.main_tick()?;
        }
    }

    fn main_tick(&mut self) -> Result<(), DeviceError> {
        match &self.state {
            DeviceState::Running => {
                self.running_state_tick()
            }
            DeviceState::Alarm => {
                todo!()
            }
            DeviceState::UpdateTime => {
                let (hours, minutes, seconds, day, date, month, year) = self.update_time_loop()?;

                set_time(&mut self.i2c, hours, minutes, seconds, day, date, month, year)?;

                self.state = DeviceState::Running;

                Ok(())
            }
            DeviceState::UpdateAlarm => {
                let (hours, minutes, seconds, day, date, month, year) = self.update_time_loop()?;

                // TODO: SET ALARM

                self.state = DeviceState::Running;

                Ok(())
            }
        }
    }

    fn update_time_loop(&mut self) -> Result<(u8, u8, u8, u8, u8, u8, u8), DeviceError> {
        let mut state = UpdateTimeState::UpdatingHour;
        let (mut hours, mut minutes, day, date, month, year) = (0, 0, 1, 1, 1, 26);

        loop {
            display_time(&mut self.i2c, hours, minutes, 0, false)?;
            arduino_hal::delay_ms(20);

            match state {
                UpdateTimeState::UpdatingHour => {
                    if self.up_button.is_high() {
                        while self.up_button.is_high() {}
                        hours = (hours + 1) % 24;
                    }

                    if self.down_button.is_high() {
                        while self.down_button.is_high() {}
                        if hours == 0 {
                            hours = 23;
                        } else {
                            hours -= 1;
                        }
                    }

                    if self.main_button.is_high() {
                        while self.main_button.is_high() {}
                        state = UpdateTimeState::UpdatingMinute
                    }
                }
                UpdateTimeState::UpdatingMinute => {
                    if self.up_button.is_high() {
                        while self.up_button.is_high() {}
                        minutes = (minutes + 1) % 60;
                    }

                    if self.down_button.is_high() {
                        while self.down_button.is_high() {}
                        if minutes == 0 {
                            minutes = 59;
                        } else {
                            minutes -= 1;
                        }
                    }

                    if self.main_button.is_high() {
                        while self.main_button.is_high() {}
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok((hours, minutes, 0, day, date, month, year))
    }

    fn alarm_state_tick(&mut self) {
    }

    fn running_state_tick(&mut self) -> Result<(), DeviceError> {
        // TODO: check if alarm is set
        
        if self.main_button.is_high() {
            self.button_counter += 1;
            if self.button_counter >= 3 {
                while self.main_button.is_high() {}
                self.button_counter = 0;
                self.state = DeviceState::UpdateTime;
            }
        }

        let (hours, minutes, seconds, _day, _date, _month, _year) = read_time(&mut self.i2c)?;

        display_time(&mut self.i2c, hours, minutes, seconds, true)?;

        Ok(())
    }
}
