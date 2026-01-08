use arduino_hal::{
    I2c,
    i2c::Error as I2cError,
    port::{
        D2, D9, D10, D11, D12, Pin, mode::{Floating, Input, Output, PullUp}
    }
};

use crate::{clock::{clear_alarm_flag, disable_alarm, enable_alarm, set_alarm}, oled::{clear_top, display_date, display_frequency, display_time}};
use super::oled::{init_oled, clear_oled};
use super::clock::{set_time, read_time};

enum UpdateTimeState {
    Begin,
    UpdatingHour,
    UpdatingMinute,
    UpdatingDate,
    UpdatingMonth,
    UpdatingYear,
    Complete,
}

enum UpdateAlarmState {
    Begin,
    UpdatingFrequency,
    UpdatingHour,
    UpdatingMinute,
    Complete,
}

enum DeviceState {
    Running,
    UpdateTime(UpdateTimeState),
    UpdateAlarm(UpdateAlarmState),
    Alarm
}

pub enum AlarmFrequency {
    On,
    Off,
    Once,
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
    piezo_pin: Pin<Output, D2>,
    main_button: Pin<Input<Floating>, D10>,
    up_button: Pin<Input<Floating>, D11>,
    down_button: Pin<Input<Floating>, D9>,
    alarm_pin: Pin<Input<PullUp>, D12>,
    tick_counter: u16,
    main_button_counter: u16,
    up_button_counter: u16,
    down_button_counter: u16,
    minutes: u8,
    hours: u8,
    date: u8,
    month: u8,
    year: u8,
    alarm_minutes: u8,
    alarm_hours: u8,
    frequency: AlarmFrequency,
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

        arduino_hal::delay_ms(50);
        init_oled(&mut i2c)?;
        arduino_hal::delay_ms(50);
        clear_oled(&mut i2c)?;
        arduino_hal::delay_ms(50);
        set_time(&mut i2c, 0, 0, 0, 0, 01, 01, 26)?;
        arduino_hal::delay_ms(100);
        display_time(&mut i2c, 0, 0, 0,  false)?;
        arduino_hal::delay_ms(50);
        display_date(&mut i2c, 01, 01, 26)?;
        arduino_hal::delay_ms(50);
        clear_alarm_flag(&mut i2c)?;
        arduino_hal::delay_ms(50);
        disable_alarm(&mut i2c)?;

        Ok(Self {
            state: DeviceState::Running,
            tick_counter: 0,
            main_button_counter: 0,
            up_button_counter: 0,
            down_button_counter: 0,
            date: 1,
            month: 1,
            year: 26,
            hours: 0,
            minutes: 0,
            alarm_hours: 0,
            alarm_minutes: 0,
            frequency: AlarmFrequency::Off,
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
            arduino_hal::delay_ms(1);

            self.bump_tick_counter();

            self.button_scan();

            self.main_tick()?;
        }
    }

    fn button_scan(&mut self) {
        if self.main_button.is_high() {
            self.main_button_counter = (self.main_button_counter + 1) % 10_000;
        } else {
            self.main_button_counter = 0;
        }

        if self.up_button.is_high() {
            self.up_button_counter = (self.up_button_counter + 1) % 10_000;
        } else {
            self.up_button_counter = 0;
        }

        if self.down_button.is_high() {
            self.down_button_counter = (self.down_button_counter + 1) % 10_000;
        } else {
            self.down_button_counter = 0;
        }
    }

    fn bump_tick_counter(&mut self) {
        self.tick_counter = (self.tick_counter + 1) % 10_000;
    }

    fn main_tick(&mut self) -> Result<(), DeviceError> {
        match &self.state {
            DeviceState::Running => self.running_state_tick(),
            DeviceState::Alarm => self.alarm_state_tick(),
            DeviceState::UpdateTime(_) => self.update_time_tick(),
            DeviceState::UpdateAlarm(_) => self.update_alarm_tick(),
        }
    }

    fn update_time_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateTime(UpdateTimeState::Begin) => {
                if self.tick_counter % 500 == 0 {
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                } else if self.tick_counter % 250 == 0 {
                    clear_oled(&mut self.i2c)?;
                }

                if self.main_button.is_low() && self.up_button.is_low() && self.down_button.is_low() {
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingHour);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingHour) => {
                if (self.up_button_counter % 100) == 1 {
                    self.hours = (self.hours + 1) % 24;
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                } else if (self.down_button_counter % 100) == 1 {
                    if self.hours == 0 {
                        self.hours = 23;
                    } else {
                        self.hours -= 1;
                    }
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute) => {
                if self.up_button_counter % 100 == 1 {
                    self.minutes = (self.minutes + 1) % 60;
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                } else if self.down_button_counter % 100 == 1 {
                    if self.minutes == 0 {
                        self.minutes = 59;
                    } else {
                        self.minutes -= 1;
                    }
                    display_time(&mut self.i2c, self.hours, self.minutes, 0, false)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingDate);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingDate) => {
                if self.up_button_counter % 100 == 1 {
                    if self.date == 31 {
                        self.date = 1;
                    } else {
                        self.date += 1;
                    }
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                } else if self.down_button_counter % 100 == 1 {
                    if self.date == 1 {
                        self.date = 31;
                    } else {
                        self.date -= 1;
                    }
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingMonth);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingMonth) => {
                if self.up_button_counter % 100 == 1 {
                    if self.month == 12 {
                        self.month = 1;
                    } else {
                        self.month += 1;
                    }
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                } else if self.down_button_counter % 100 == 1 {
                    if self.month == 1 {
                        self.month = 12;
                    } else {
                        self.month -= 1;
                    }
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingYear);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingYear) => {
                if self.up_button_counter % 100 == 1 {
                    self.year = (self.year + 1) % 100;
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                } else if self.down_button_counter % 100 == 1 {
                    if self.year == 0 {
                        self.year = 99;
                    } else {
                        self.year -= 1;
                    }
                    display_date(&mut self.i2c, self.date, self.month, self.year)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::Complete);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::Complete) => {
                set_time(&mut self.i2c, self.hours, self.minutes, 0, 0, self.date, self.month, self.year)?;
                self.state = DeviceState::Running;
            }
            _ => {}
        }

        Ok(())
    }

    fn update_alarm_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateAlarm(UpdateAlarmState::Begin) => {
                if self.tick_counter % 500 == 0 {
                    display_frequency(&mut self.i2c, &self.frequency)?;
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                } else if self.tick_counter % 250 == 0 {
                    clear_oled(&mut self.i2c)?;
                }

                if self.main_button.is_low() && self.up_button.is_low() && self.down_button.is_low() {
                    display_frequency(&mut self.i2c, &self.frequency)?;
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                    self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingFrequency);
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingFrequency) => {
                if self.up_button_counter % 100 == 1 || self.down_button_counter % 100 == 1 {
                    match self.frequency {
                        AlarmFrequency::On => self.frequency = AlarmFrequency::Off,
                        AlarmFrequency::Off => self.frequency = AlarmFrequency::Once,
                        AlarmFrequency::Once => self.frequency = AlarmFrequency::On,
                    }
                    display_frequency(&mut self.i2c, &self.frequency)?;
                }

                if self.main_button_counter == 1 {
                    if let AlarmFrequency::Off = self.frequency {
                        self.state = DeviceState::UpdateAlarm(UpdateAlarmState::Complete);
                    } else {
                        self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingHour);
                    }
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingHour) => {
                if (self.up_button_counter % 100) == 1 {
                    self.alarm_hours = (self.alarm_hours + 1) % 24;
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                } else if (self.down_button_counter % 100) == 1 {
                    if self.alarm_hours == 0 {
                        self.alarm_hours = 23;
                    } else {
                        self.alarm_hours -= 1;
                    }
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingMinute);
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingMinute) => {
                if self.up_button_counter % 100 == 1 {
                    self.alarm_minutes = (self.alarm_minutes + 1) % 60;
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                } else if self.down_button_counter % 100 == 1 {
                    if self.alarm_minutes == 0 {
                        self.alarm_minutes = 59;
                    } else {
                        self.alarm_minutes -= 1;
                    }
                    display_time(&mut self.i2c, self.alarm_hours, self.alarm_minutes, 0, false)?;
                }

                if self.main_button_counter == 1 {
                    self.state = DeviceState::UpdateAlarm(UpdateAlarmState::Complete);
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::Complete) => {
                match self.frequency {
                    AlarmFrequency::Off => {
                        disable_alarm(&mut self.i2c)?;
                    }
                    AlarmFrequency::On | AlarmFrequency::Once => {
                        enable_alarm(&mut self.i2c)?;
                        arduino_hal::delay_ms(20);
                        set_alarm(&mut self.i2c, self.alarm_hours, self.alarm_minutes)?;
                        arduino_hal::delay_ms(20);
                        clear_alarm_flag(&mut self.i2c)?;
                    }
                }

                self.state = DeviceState::Running;

                let (hours, minutes, seconds, _day, date, month, year) = read_time(&mut self.i2c)?;

                clear_oled(&mut self.i2c)?;
                display_time(&mut self.i2c, hours, minutes, seconds, false)?;
                display_date(&mut self.i2c, date, month, year)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn alarm_state_tick(&mut self) -> Result<(), DeviceError> {
        self.piezo_pin.set_high();
        if self.main_button_counter == 1 {
            self.state = DeviceState::Running;

            if let AlarmFrequency::Once = self.frequency {
                disable_alarm(&mut self.i2c)?;
            }

            clear_alarm_flag(&mut self.i2c)?;
            self.piezo_pin.set_low();
            return Ok(());
        }

        if self.tick_counter % 1000 == 0 {
            let (hours, minutes, seconds, _day, date, month, year) = read_time(&mut self.i2c)?;

            display_time(&mut self.i2c, hours, minutes, seconds, true)?;

            if seconds == 0 {
                display_date(&mut self.i2c, date, month, year)?;
            }
        }

        Ok(())
    }

    fn running_state_tick(&mut self) -> Result<(), DeviceError> {
        if self.alarm_pin.is_low() {
            self.state = DeviceState::Alarm;

            return Ok(());
        }

        if self.main_button.is_high() && self.up_button.is_high() && self.down_button.is_high() {
            self.main_button_counter = 0;
            self.state = DeviceState::UpdateTime(UpdateTimeState::Begin);
            return Ok(());
        }
        
        if self.main_button_counter >= 3000 {
            self.state = DeviceState::UpdateAlarm(UpdateAlarmState::Begin);
            return Ok(());
        }

        if self.tick_counter % 1000 == 0 {
            let (hours, minutes, seconds, _day, date, month, year) = read_time(&mut self.i2c)?;

            display_time(&mut self.i2c, hours, minutes, seconds, true)?;

            if seconds == 0 {
                display_date(&mut self.i2c, date, month, year)?;
            }
        }

        Ok(())
    }
}
