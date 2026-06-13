use arduino_hal::{
    I2c,
    i2c::Error as I2cError,
    port::{
        A3, D11, Pin, mode::{Input, Output, PullUp}
    }
};

use crate::{
    button_scanner::{BUTTON_MAX_VAL, ButtonScanner}, 
    clock::{clear_alarm_flag, disable_alarm, enable_alarm, read_time, set_alarm, set_time}, 
    display::{DisplayChar, DisplayController}
};

enum UpdateTimeState {
    Begin,
    UpdatingHour,
    UpdatingMinute,
    Complete,
}

#[derive(Copy, Clone)]
enum MenuState {
    ExitMenu,
    UpdateTime,
    UpdateAlarmTime,
    UpdateAlarmFrequency
}

enum DeviceState {
    Running,
    UpdateTime(UpdateTimeState),
    UpdateAlarm(UpdateTimeState),
    UpdateAlarmFrequency,
    //ScreenSaver,
    Alarm,
    Menu(MenuState)
}

pub enum AlarmFrequency {
    On,
    Off,
    Once,
}

pub enum DeviceError {
    I2cError(I2cError),
    PeripheralError,
}

impl From<I2cError> for DeviceError {
    fn from(value: I2cError) -> Self {
        DeviceError::I2cError(value)
    }
}

// stored first 4 bits for the first digits, last 4 bits for the second
//struct Time {
//    hours: u8, 
//    minutes: u8
//}

pub struct Device {
    state: DeviceState,
    i2c: I2c,
    tick_counter: u32,
    //alarm_time: Time,
    //current_time: Time,
    minutes: u8,
    hours: u8,
    alarm_minutes: u8,
    alarm_hours: u8,
    frequency: AlarmFrequency,
    display_controller: DisplayController,
    button_scanner: ButtonScanner,
    piezo_pin: Pin<Output, D11>,           // PB3
    alarm_pin: Pin<Input<PullUp>, A3>,     // PC3

}

impl Device {
    pub fn init() -> Result<Self, DeviceError> {
        let dp = match arduino_hal::Peripherals::take() {
            None => return Err(DeviceError::PeripheralError),
            Some(dp) => dp,
        };
        let pins = arduino_hal::pins!(dp);
        let mut i2c = I2c::new(
            dp.TWI,
            pins.a4.into_pull_up_input(),  // SDA
            pins.a5.into_pull_up_input(),  // SCL
            50000, 
        );
        let mut piezo_pin: Pin<Output, D11> = pins.d11.into_output();
        let alarm_pin: Pin<Input<PullUp>, A3> = pins.a3.into_pull_up_input(); 
        let display_controller = DisplayController::new(
            pins.d0.into_output(),
            pins.d1.into_output(),
            pins.d2.into_output(),
            pins.d3.into_output(),
            pins.d4.into_output(),
            pins.d5.into_output(),
            pins.d6.into_output(),
        );
        let button_scanner = ButtonScanner::new(
            pins.d10.into_output(),
            pins.d7.into_floating_input(),
            pins.d8.into_floating_input(),
            pins.d9.into_floating_input(),
        );

        piezo_pin.set_low();

        set_time(&mut i2c, 0, 0, 0, 0, 01, 01, 26)?;
        arduino_hal::delay_ms(1);
        clear_alarm_flag(&mut i2c)?;
        arduino_hal::delay_ms(1);
        disable_alarm(&mut i2c)?;

        Ok(Self {
            state: DeviceState::Running,
            tick_counter: 0,
            hours: 0,
            minutes: 0,
            alarm_hours: 0,
            alarm_minutes: 0,
            frequency: AlarmFrequency::Off,
            i2c,
            piezo_pin,
            alarm_pin,
            display_controller,
            button_scanner
        })
    }

    pub fn main_loop(&mut self) -> Result<(), DeviceError> {
        loop {
            self.bump_tick_counter();

            self.display_tick();

            self.button_scanner.button_scan();

            self.tick()?;

            self.read_time();

            self.play_button_press_noises();
        }
    }

    fn tick(&mut self) -> Result<(), DeviceError> {
        match &self.state {
            DeviceState::Running => self.running_state_tick(),
            DeviceState::Alarm => self.alarm_state_tick(),
            DeviceState::UpdateTime(_) => self.update_time_tick(),
            DeviceState::UpdateAlarm(_) => self.update_alarm_tick(),
            DeviceState::Menu(menu_state) => self.menu_tick(*menu_state),
            DeviceState::UpdateAlarmFrequency => self.update_alarm_frequency_tick()
            //DeviceState::ScreenSaver => todo!(),
        }
    }

    fn menu_tick(&mut self, menu_state: MenuState) -> Result<(), DeviceError> {
        match &menu_state {
            MenuState::UpdateAlarmTime => {
                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateAlarm(UpdateTimeState::Begin);
                } else if self.button_scanner.get_up() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::ExitMenu);
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateAlarmFrequency);
                }
            }
            MenuState::UpdateAlarmFrequency => {
                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateAlarmFrequency
                } else if self.button_scanner.get_up() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateAlarmTime);
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateTime);
                }
            }
            MenuState::UpdateTime => {
                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::Begin);
                } else if self.button_scanner.get_up() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateAlarmFrequency);
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::ExitMenu);
                }
            }
            MenuState::ExitMenu => {
                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::Running;
                } else if self.button_scanner.get_up() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateTime);
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL  {
                    self.state = DeviceState::Menu(MenuState::UpdateAlarmTime);
                }
            }
        }

        Ok(())
    }

    fn running_state_tick(&mut self) -> Result<(), DeviceError> {
        if self.alarm_pin.is_low() {
            self.state = DeviceState::Alarm;
            return Ok(());
        }

        if self.button_scanner.get_main() == BUTTON_MAX_VAL {
            self.state = DeviceState::Menu(MenuState::UpdateAlarmTime);
            return Ok(());
        }

        Ok(())
    }


    fn update_time_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateTime(UpdateTimeState::Begin) => {
                if self.button_scanner.get_main() == 0 && self.button_scanner.get_up() == 0 && self.button_scanner.get_down() == 0 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingHour);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingHour) => {
                if self.button_scanner.get_up() == BUTTON_MAX_VAL {
                    self.hours = (self.hours + 1) % 24;
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL {
                    if self.hours == 0 {
                        self.hours = 23;
                    } else {
                        self.hours -= 1;
                    }
                }

                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute) => {
                if self.button_scanner.get_up() == BUTTON_MAX_VAL {
                    self.minutes = (self.minutes + 1) % 60;
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL {
                    if self.minutes == 0 {
                        self.minutes = 59;
                    } else {
                        self.minutes -= 1;
                    }
                }

                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::Complete);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::Complete) => {
                set_time(&mut self.i2c, self.hours, self.minutes, 0, 0, 1, 1, 26)?;
                self.state = DeviceState::Running;
            }
            _ => {}
        }

        Ok(())
    }

    fn update_alarm_frequency_tick(&mut self) -> Result<(), DeviceError> {
        if self.button_scanner.get_up() == BUTTON_MAX_VAL || self.button_scanner.get_down() == BUTTON_MAX_VAL {
            match self.frequency {
                AlarmFrequency::On => self.frequency = AlarmFrequency::Off,
                AlarmFrequency::Off => self.frequency = AlarmFrequency::Once,
                AlarmFrequency::Once => self.frequency = AlarmFrequency::On,
            }
        } else if self.button_scanner.get_main() == BUTTON_MAX_VAL {
            self.state = DeviceState::Running;
        }

        Ok(())
    }

    fn update_alarm_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateAlarm(UpdateTimeState::Begin) => {
                if self.button_scanner.get_main() == 0 && self.button_scanner.get_up() == 0 && self.button_scanner.get_down() == 0 {
                    self.state = DeviceState::UpdateAlarm(UpdateTimeState::UpdatingHour);
                    self.frequency = AlarmFrequency::On;
                }
            }
            DeviceState::UpdateAlarm(UpdateTimeState::UpdatingHour) => {
                if self.button_scanner.get_up() == BUTTON_MAX_VAL {
                    self.alarm_hours = (self.alarm_hours + 1) % 24;
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL {
                    if self.alarm_hours == 0 {
                        self.alarm_hours = 23;
                    } else {
                        self.alarm_hours -= 1;
                    }
                }

                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateAlarm(UpdateTimeState::UpdatingMinute);
                }
            }
            DeviceState::UpdateAlarm(UpdateTimeState::UpdatingMinute) => {
                if self.button_scanner.get_up() == BUTTON_MAX_VAL {
                    self.alarm_minutes = (self.alarm_minutes + 1) % 60;
                } else if self.button_scanner.get_down() == BUTTON_MAX_VAL {
                    if self.alarm_minutes == 0 {
                        self.alarm_minutes = 59;
                    } else {
                        self.alarm_minutes -= 1;
                    }
                }

                if self.button_scanner.get_main() == BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateAlarm(UpdateTimeState::Complete);
                }
            }
            DeviceState::UpdateAlarm(UpdateTimeState::Complete) => {
                match self.frequency {
                    AlarmFrequency::Off => {
                        disable_alarm(&mut self.i2c)?;
                    }
                    AlarmFrequency::On | AlarmFrequency::Once => {
                        enable_alarm(&mut self.i2c)?;
                        arduino_hal::delay_ms(1);
                        set_alarm(&mut self.i2c, self.alarm_hours, self.alarm_minutes)?;
                        arduino_hal::delay_ms(1);
                        clear_alarm_flag(&mut self.i2c)?;
                    }
                }

                self.state = DeviceState::Running;
            }
            _ => {}
        }

        Ok(())
    }

    fn alarm_state_tick(&mut self) -> Result<(), DeviceError> {
        if self.button_scanner.get_main() == BUTTON_MAX_VAL {
            self.state = DeviceState::Running;

            if let AlarmFrequency::Once = self.frequency {
                disable_alarm(&mut self.i2c)?;
            }

            clear_alarm_flag(&mut self.i2c)?;
            self.piezo_pin.set_low();
            return Ok(());
        }

        if (self.tick_counter % 8192) > 4096  {
           if self.tick_counter % 32 == 0 {
               self.piezo_pin.toggle();
           }
        }

        Ok(())
    }

    fn display_tick(&mut self) {
        match &self.state {
            DeviceState::Running => self.display_time_7seg(),
            DeviceState::Menu(MenuState::ExitMenu) => {
                if self.tick_counter % 4096 < 2048 {
                    self.display_controller.display_chars([
                        DisplayChar::Dash,
                        DisplayChar::Dash,
                        DisplayChar::Dash,
                        DisplayChar::Dash
                    ]);
                }
            }
            DeviceState::Alarm | DeviceState::UpdateTime(_) | DeviceState::Menu(MenuState::UpdateTime) => {
                if self.tick_counter % 4096 < 2048 {
                    self.display_time_7seg();
                }
            }
            DeviceState::UpdateAlarm(_) | DeviceState::Menu(MenuState::UpdateAlarmTime) => {
                if self.tick_counter % 4096 < 2048 {
                    self.display_alarm_time_7seg();
                }
            }
            DeviceState::UpdateAlarmFrequency | DeviceState::Menu(MenuState::UpdateAlarmFrequency) => {
                if self.tick_counter % 4096 < 2048 {
                    match self.frequency {
                        AlarmFrequency::On => {
                            self.display_controller.display_chars([
                                DisplayChar::Dash,
                                DisplayChar::Blank,
                                DisplayChar::Blank,
                                DisplayChar::Blank
                            ]);
                        }
                        AlarmFrequency::Off => {
                            self.display_controller.display_chars([
                                DisplayChar::Dash,
                                DisplayChar::Dash,
                                DisplayChar::Blank,
                                DisplayChar::Blank
                            ]);
                        }
                        AlarmFrequency::Once => {
                            self.display_controller.display_chars([
                                DisplayChar::Dash,
                                DisplayChar::Dash,
                                DisplayChar::Dash,
                                DisplayChar::Blank
                            ]);
                        }
                    }
                }
            }
        }
    }

    fn display_time_7seg(&mut self) {
        self.display_controller.display_number(
            self.hours / 10,
            self.hours % 10,
            self.minutes / 10,
            self.minutes % 10,
        );
    }

    fn display_alarm_time_7seg(&mut self) {
        self.display_controller.display_number(
            self.alarm_hours / 10,
            self.alarm_hours % 10,
            self.alarm_minutes / 10,
            self.alarm_minutes % 10,
        );
    }

    fn read_time(&mut self) {
        match &self.state {
            DeviceState::UpdateTime(_) => {}
            _ => if self.tick_counter % 4096 == 0 {
                if let Ok((h, m, _, _, _, _, _)) = read_time(&mut self.i2c) {
                    self.hours = h;
                    self.minutes = m;
                }
            },
        }
    }

    fn play_button_press_noises(&mut self) {
        if self.button_scanner.get_main() > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 2 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }
        if self.button_scanner.get_down() > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 8 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }

        if self.button_scanner.get_up() > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 32 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }
    }

    fn bump_tick_counter(&mut self) {
        self.tick_counter = (self.tick_counter + 1) % (1 << 20);
    }
}
