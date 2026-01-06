use arduino_hal::{
    I2c,
    i2c::Error as I2cError,
    port::{
        D2, D9, D10, D11, D12, Pin, mode::{Floating, Input, Output, PullUp}
    }
};

use crate::{clock::{clear_alarm_flag, set_alarm}, oled::{display_date, display_time, draw_test}};
use super::oled::{init_oled, clear_oled};
use super::clock::{set_time, read_time};

enum UpdateTimeState {
    UpdatingHour,
    UpdatingMinute,
    UpdatingDate,
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
    i2c: I2c,
    button_counter: u8,
    piezo_pin: Pin<Output, D2>,
    main_button: Pin<Input<Floating>, D10>,
    up_button: Pin<Input<Floating>, D11>,
    down_button: Pin<Input<Floating>, D9>,
    alarm_pin: Pin<Input<PullUp>, D12>
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
        let alarm_pin: Pin<Input<PullUp>, D12> = pins.d12.into_pull_up_input(); 

        piezo_pin.set_low();

        arduino_hal::delay_ms(100);
        init_oled(&mut i2c)?;
        arduino_hal::delay_ms(100);
        clear_oled(&mut i2c)?;
        arduino_hal::delay_ms(100);
        set_time(&mut i2c, 12, 51, 0, 2, 01, 01, 26)?;
        arduino_hal::delay_ms(100);
        set_alarm(&mut i2c, 0, 0)?;
        arduino_hal::delay_ms(100);
        display_time(&mut i2c, 12, 51, 0,  false)?;
        arduino_hal::delay_ms(100);
        display_date(&mut i2c, 01, 01, 26)?;

        Ok(Self {
            state: DeviceState::Running,
            button_counter: 0,
            i2c,
            main_button,
            down_button,
            up_button,
            piezo_pin,
            alarm_pin,
        })
    }

    pub fn main_loop(&mut self) -> Result<(), DeviceError> {
        loop {
            arduino_hal::delay_ms(100);

            self.main_tick()?;
        }
    }

    fn main_tick(&mut self) -> Result<(), DeviceError> {
        match &self.state {
            DeviceState::Running => {
                self.running_state_tick()
            }
            DeviceState::Alarm => {
                self.alarm_state_tick()
            }
            DeviceState::UpdateTime => {
                let (hours, minutes, seconds, day, date, month, year) = self.update_time_loop()?;

                set_time(&mut self.i2c, hours, minutes, seconds, day, date, month, year)?;
                display_date(&mut self.i2c, date, month, year)?;
                display_time(&mut self.i2c, hours, minutes, seconds, false)?;

                self.state = DeviceState::Running;

                Ok(())
            }
            DeviceState::UpdateAlarm => {
                let (hours, minutes, seconds, day, date, month, year) = self.update_time_loop()?;

                // you need hours and minutes
                
                // TODO: SET ALARM

                self.state = DeviceState::Running;

                Ok(())
            }
        }
    }

    fn update_time_loop(&mut self) -> Result<(u8, u8, u8, u8, u8, u8, u8), DeviceError> {
        let mut state = UpdateTimeState::UpdatingHour;
        let (mut hours, mut minutes, day, mut date, mut month, mut year) = (0, 0, 1, 1, 1, 26);

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
                        state = UpdateTimeState::UpdatingDate
                    }
                }
                UpdateTimeState::UpdatingDate => {
                    display_date(&mut self.i2c, date, month, year)?;
                    if self.up_button.is_high() {
                        while self.up_button.is_high() {}

                        if date == 31 {
                            date = 1;
                        } else {
                            date += 1;
                        }
                    }

                    if self.down_button.is_high() {
                        while self.down_button.is_high() {}
                        if date == 1 {
                            date = 31;
                        } else {
                            date -= 1;
                        }
                    }

                    if self.main_button.is_high() {
                        while self.main_button.is_high() {}
                        state = UpdateTimeState::UpdatingMonth
                    }
                }
                UpdateTimeState::UpdatingMonth => {
                    display_date(&mut self.i2c, date, month, year)?;
                    if self.up_button.is_high() {
                        while self.up_button.is_high() {}

                        if date == 12 {
                            date = 1;
                        } else {
                            date += 1;
                        }
                    }

                    if self.down_button.is_high() {
                        while self.down_button.is_high() {}
                        if month == 1 {
                            month = 12;
                        } else {
                            month -= 1;
                        }
                    }

                    if self.main_button.is_high() {
                        while self.main_button.is_high() {}
                        state = UpdateTimeState::UpdatingYear
                    }
                }
                UpdateTimeState::UpdatingYear => {
                    display_date(&mut self.i2c, date, month, year)?;
                    if self.up_button.is_high() {
                        while self.up_button.is_high() {}
                        year = (year + 1) % 100;
                    }

                    if self.down_button.is_high() {
                        while self.down_button.is_high() {}
                        if year == 0 {
                            year = 99;
                        } else {
                            year -= 1;
                        }
                    }

                    if self.main_button.is_high() {
                        while self.main_button.is_high() {}
                        break;
                    }
                }
            }
        }

        Ok((hours, minutes, 0, day, date, month, year))
    }

    fn alarm_state_tick(&mut self) -> Result<(), DeviceError> {
        self.piezo_pin.set_high();
        if self.main_button.is_high() {
            while self.main_button.is_high() {}
            self.state = DeviceState::Running;
            clear_alarm_flag(&mut self.i2c)?;
            self.piezo_pin.set_low();
            return Ok(());
        }

        let (hours, minutes, seconds, _day, date, month, year) = read_time(&mut self.i2c)?;

        display_time(&mut self.i2c, hours, minutes, seconds, true)?;

        if seconds == 0 {
            display_date(&mut self.i2c, date, month, year)?;
        }

        Ok(())
    }

    fn running_state_tick(&mut self) -> Result<(), DeviceError> {
        if self.alarm_pin.is_low() {
            self.state = DeviceState::Alarm;

            return Ok(());
        }
        
        if self.main_button.is_high() && self.up_button.is_high() && self.down_button.is_high() {
            while self.main_button.is_high() || self.up_button.is_high() || self.down_button.is_high() {}
            self.state = DeviceState::UpdateTime;
            return Ok(());
        }

        let (hours, minutes, seconds, _day, date, month, year) = read_time(&mut self.i2c)?;

        display_time(&mut self.i2c, hours, minutes, seconds, true)?;

        if seconds == 0 {
            display_date(&mut self.i2c, date, month, year)?;
        }

        Ok(())
    }
}
