use arduino_hal::{
    I2c,
    i2c::Error as I2cError,
    port::{
        A3, D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, Pin, mode::{Floating, Input, Output, PullUp}
    }
};

use crate::clock::{clear_alarm_flag, disable_alarm, enable_alarm, set_alarm, set_time, read_time};

enum UpdateTimeState {
    Begin,
    UpdatingHour,
    UpdatingMinute,
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
    I2cError(I2cError),
    PeripheralError,
}

impl From<I2cError> for DeviceError {
    fn from(value: I2cError) -> Self {
        DeviceError::I2cError(value)
    }
}

enum CapPhase {
    Charge,
    Drain,
}

pub struct Device {
    state: DeviceState,
    i2c: I2c,
    tick_counter: u32,
    main_button_counter: u32,
    up_button_counter: u32,
    down_button_counter: u32,
    cap_phase: CapPhase,
    cap_button: u8,
    cap_count: u32,
    minutes: u8,
    hours: u8,
    alarm_minutes: u8,
    alarm_hours: u8,
    frequency: AlarmFrequency,
    data_pin: Pin<Output, D0>,             // PD0 - SER
    latch_pin: Pin<Output, D1>,            // PD1 - RCLK
    clock_pin: Pin<Output, D2>,            // PD2 - SRCLK
    digit1: Pin<Output, D3>,               // PD3
    digit2: Pin<Output, D4>,               // PD4
    digit3: Pin<Output, D5>,               // PB6
    digit4: Pin<Output, D6>,               // PB7
    piezo_pin: Pin<Output, D11>,           // PB3
    send_pin: Pin<Output, D10>,            // PB2 - cap sense send
    up_button: Pin<Input<Floating>, D7>,   // PD7 - cap sense receive
    main_button: Pin<Input<Floating>, D8>, // PB0 - cap sense receive
    down_button: Pin<Input<Floating>, D9>, // PB1 - cap sense receive
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
        let send_pin: Pin<Output, D10> = pins.d10.into_output();
        let up_button: Pin<Input<Floating>, D7> = pins.d7.into_floating_input();
        let main_button: Pin<Input<Floating>, D8> = pins.d8.into_floating_input();
        let down_button: Pin<Input<Floating>, D9> = pins.d9.into_floating_input();
        let alarm_pin: Pin<Input<PullUp>, A3> = pins.a3.into_pull_up_input(); 
        let data_pin: Pin<Output, D0> = pins.d0.into_output();
        let latch_pin: Pin<Output, D1> = pins.d1.into_output();
        let clock_pin: Pin<Output, D2> = pins.d2.into_output();
        let digit1: Pin<Output, D3> = pins.d3.into_output(); 
        let digit2: Pin<Output, D4> = pins.d4.into_output(); 
        let digit3: Pin<Output, D5> = pins.d5.into_output(); 
        let digit4: Pin<Output, D6> = pins.d6.into_output(); 

        piezo_pin.set_low();

        set_time(&mut i2c, 0, 0, 0, 0, 01, 01, 26)?;
        arduino_hal::delay_ms(1);
        clear_alarm_flag(&mut i2c)?;
        arduino_hal::delay_ms(1);
        disable_alarm(&mut i2c)?;

        Ok(Self {
            state: DeviceState::Running,
            tick_counter: 0,
            main_button_counter: 0,
            up_button_counter: 0,
            down_button_counter: 0,
            cap_phase: CapPhase::Charge,
            cap_button: 0,
            cap_count: 0,
            hours: 0,
            minutes: 0,
            alarm_hours: 0,
            alarm_minutes: 0,
            frequency: AlarmFrequency::Off,
            i2c,
            send_pin,
            main_button,
            down_button,
            up_button,
            piezo_pin,
            alarm_pin,
            data_pin,
            latch_pin,
            clock_pin,
            digit1,
            digit2,
            digit3,
            digit4
        })
    }

    pub fn main_loop(&mut self) -> Result<(), DeviceError> {
        loop {
            self.bump_tick_counter();

            self.flash_display();

            self.button_scan();

            self.tick_dispatch()?;

            self.read_time();

            self.play_button_press_noises();
        }
    }

    fn tick_dispatch(&mut self) -> Result<(), DeviceError> {
        match &self.state {
            DeviceState::Running => self.running_state_tick(),
            DeviceState::Alarm => self.alarm_state_tick(),
            DeviceState::UpdateTime(_) => self.update_time_tick(),
            DeviceState::UpdateAlarm(_) => self.update_alarm_tick(),
        }
    }

    fn flash_display(&mut self) {
        match &self.state {
            DeviceState::Running => self.display_time_7seg(),
            DeviceState::Alarm | DeviceState::UpdateTime(_) => {
                if self.tick_counter % 4096 < 2048 {
                    self.display_time_7seg();
                }
            }
            DeviceState::UpdateAlarm(_) => {
                if self.tick_counter % 4096 < 2048 {
                    self.display_alarm_time_7seg();
                }
            }
        }
    }

    fn display_time_7seg(&mut self) {
        // un comment to easily test displaying numbers is working properly
        //self.hours = (self.tick_counter / 1_000) as u8;
        //self.minutes = (self.tick_counter / 1_000) as u8;
        self.display_number(
            self.hours / 10,
            self.hours % 10,
            self.minutes / 10,
            self.minutes % 10,
        );
    }

    fn display_alarm_time_7seg(&mut self) {
        self.display_number(
            self.alarm_hours / 10,
            self.alarm_hours % 10,
            self.alarm_minutes / 10,
            self.alarm_minutes % 10,
        );
    }

    fn display_number(&mut self, d1: u8, d2: u8, d3: u8, d4: u8) {
            self.push_byte_to_display(Self::digit_pattern(d1));

            self.digit1.set_high();
            arduino_hal::delay_us(10);
            self.digit1.set_low();

            self.push_byte_to_display(Self::digit_pattern(d2));

            self.digit2.set_high();
            arduino_hal::delay_us(10);
            self.digit2.set_low();

            self.push_byte_to_display(Self::digit_pattern(d3));

            self.digit3.set_high();
            arduino_hal::delay_us(10);
            self.digit3.set_low();

            self.push_byte_to_display(Self::digit_pattern(d4));

            self.digit4.set_high();
            arduino_hal::delay_us(10);
            self.digit4.set_low();
    }

    fn push_byte_to_display(&mut self, b: u8) {
        for i in 0..8 {
            if (b & (1 << i)) != 0 {
                self.data_pin.set_high();
            } else {
                self.data_pin.set_low();
            }

            self.clock_pin.set_high();

            self.data_pin.set_low();
            self.clock_pin.set_low();
        }
        self.latch_pin.set_high();
        self.latch_pin.set_low();
    }

    const A: u8 = 1 << 6;
    const B: u8 = 1 << 5;
    const C: u8 = 1 << 4;
    const D: u8 = 1 << 3;
    const E: u8 = 1 << 2;
    const F: u8 = 1 << 1;
    const G: u8 = 1 << 0;

    const ZERO: u8 = Self::A | Self::B | Self::C | Self::E | Self::F | Self::G;
    const ONE: u8 = Self::C | Self::F;
    const TWO: u8 = Self::A | Self::C | Self::D | Self::E | Self::G;
    const THREE: u8 = Self::A | Self::C | Self::D | Self::F | Self::G;
    const FOUR: u8 = Self::B | Self::C | Self::D | Self::F;
    const FIVE: u8 = Self::A | Self::B | Self::D | Self::F | Self::G;
    const SIX: u8 = Self::A | Self::B | Self::D | Self::E | Self::F | Self::G;
    const SEVEN: u8 = Self::C | Self::F | Self::A;
    const EIGHT: u8 = Self::A | Self::B | Self::C | Self::D | Self::E | Self::F | Self::G;
    const NINE: u8 = Self::A | Self::B | Self::C | Self::D | Self::F;

    fn digit_pattern(n: u8) -> u8 {
        match n {
            0 => Self::ZERO,
            1 => Self::ONE,
            2 => Self::TWO,
            3 => Self::THREE,
            4 => Self::FOUR,
            5 => Self::FIVE,
            6 => Self::SIX,
            7 => Self::SEVEN,
            8 => Self::EIGHT,
            9 => Self::NINE,
            _ => 0,
        }
    }

    fn read_time(&mut self) {
        match &self.state {
            DeviceState::UpdateTime(_) => {}
            _ => if self.tick_counter % 1024 == 0 {
                if let Ok((h, m, _, _, _, _, _)) = read_time(&mut self.i2c) {
                    self.hours = h;
                    self.minutes = m;
                }
            },
        }
    }

    fn play_button_press_noises(&mut self) {
        if self.main_button_counter > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 2 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }
        if self.down_button_counter > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 8 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }

        if self.up_button_counter > 0 {
          if  (self.tick_counter % 4096) < 2048  {
              if self.tick_counter % 32 == 0 {
                  self.piezo_pin.toggle();
              }
          }
        }
    }


    fn is_cap_button_low(&self) -> bool {
        match self.cap_button {
            0 => self.main_button.is_low(),
            1 => self.up_button.is_low(),
            2 => self.down_button.is_low(),
            _ => false,
        }
    }

    const BUTTON_MAX_VAL: u32 = 4;

    fn update_cap_button_counter(&mut self, is_pressed: bool) {
        match self.cap_button {
            0 => {
                if is_pressed {
                    if self.main_button_counter == 0 {
                        self.main_button_counter = Self::BUTTON_MAX_VAL;
                    } else {
                        self.main_button_counter = Self::BUTTON_MAX_VAL - 1;
                    }
                } else {
                    if self.main_button_counter != 0 {
                        self.main_button_counter -= 1;
                    }
                }
            }
            1 => {
                if is_pressed {
                    if self.up_button_counter == 0 {
                        self.up_button_counter = Self::BUTTON_MAX_VAL;
                    } else {
                        self.up_button_counter = Self::BUTTON_MAX_VAL - 1;
                    }
                } else {
                    if self.up_button_counter != 0 {
                        self.up_button_counter -= 1;
                    }
                }
            }
            2 => {
                if is_pressed {
                    if self.down_button_counter == 0 {
                        self.down_button_counter = Self::BUTTON_MAX_VAL;
                    } else {
                        self.down_button_counter = Self::BUTTON_MAX_VAL - 1;
                    }
                } else {
                    if self.down_button_counter != 0 {
                        self.down_button_counter -= 1;
                    }
                }
            }
            _ => {},
        }
    }

    const CAP_THRESHOLD: u32 = 10;
    const CAP_MAX_COUNT: u32 = 50;

    fn button_scan(&mut self) {
        match self.cap_phase {
            CapPhase::Drain => {
                if self.cap_count == 0 {
                    self.send_pin.set_low();

                    if self.main_button_counter == Self::BUTTON_MAX_VAL {
                        self.main_button_counter = Self::BUTTON_MAX_VAL - 1;
                    } else if self.up_button_counter == Self::BUTTON_MAX_VAL {
                        self.up_button_counter = Self::BUTTON_MAX_VAL - 1;
                    } else if self.down_button_counter == Self::BUTTON_MAX_VAL {
                        self.down_button_counter = Self::BUTTON_MAX_VAL - 1;
                    }
                }

                self.cap_count += 1;

                if self.cap_count == 500 {
                    self.cap_count = 0;
                    self.cap_phase = CapPhase::Charge;
                }
            }
            CapPhase::Charge => {
                if self.cap_count == 0 {
                    self.send_pin.set_high();
                }

                while self.is_cap_button_low() && self.cap_count < Self::CAP_MAX_COUNT && self.cap_count <= Self::CAP_THRESHOLD {
                    self.cap_count = self.cap_count.wrapping_add(1);
                }

                self.send_pin.set_low();

                self.update_cap_button_counter(self.cap_count > Self::CAP_THRESHOLD);

                self.cap_count = 0;
                self.cap_phase = CapPhase::Drain;
                self.cap_button = (self.cap_button + 1) % 3;
            }
        }
    }

    fn bump_tick_counter(&mut self) {
        self.tick_counter = (self.tick_counter + 1) % 1_000_000;
    }

    fn update_time_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateTime(UpdateTimeState::Begin) => {
                if self.main_button_counter == 0 && self.up_button_counter == 0 && self.down_button_counter == 0 {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingHour);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingHour) => {
                if self.up_button_counter == Self::BUTTON_MAX_VAL {
                    self.hours = (self.hours + 1) % 24;
                } else if self.down_button_counter == Self::BUTTON_MAX_VAL {
                    if self.hours == 0 {
                        self.hours = 23;
                    } else {
                        self.hours -= 1;
                    }
                }

                if self.main_button_counter == Self::BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute);
                }
            }
            DeviceState::UpdateTime(UpdateTimeState::UpdatingMinute) => {
                if self.up_button_counter == Self::BUTTON_MAX_VAL {
                    self.minutes = (self.minutes + 1) % 60;
                } else if self.down_button_counter == Self::BUTTON_MAX_VAL {
                    if self.minutes == 0 {
                        self.minutes = 59;
                    } else {
                        self.minutes -= 1;
                    }
                }

                if self.main_button_counter == Self::BUTTON_MAX_VAL {
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

    fn update_alarm_tick(&mut self) -> Result<(), DeviceError> {
        match self.state {
            DeviceState::UpdateAlarm(UpdateAlarmState::Begin) => {
                if self.main_button_counter == 0 && self.up_button_counter == 0 && self.down_button_counter == 0 {
                    //self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingFrequency);
                    self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingHour);
                    self.frequency = AlarmFrequency::On;
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingFrequency) => {
                if self.up_button_counter == Self::BUTTON_MAX_VAL || self.down_button_counter == Self::BUTTON_MAX_VAL {
                    match self.frequency {
                        AlarmFrequency::On => self.frequency = AlarmFrequency::Off,
                        AlarmFrequency::Off => self.frequency = AlarmFrequency::Once,
                        AlarmFrequency::Once => self.frequency = AlarmFrequency::On,
                    }
                }

                if self.main_button_counter == Self::BUTTON_MAX_VAL {
                    if let AlarmFrequency::Off = self.frequency {
                        self.state = DeviceState::UpdateAlarm(UpdateAlarmState::Complete);
                    } else {
                        self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingHour);
                    }
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingHour) => {
                if self.up_button_counter == Self::BUTTON_MAX_VAL {
                    self.alarm_hours = (self.alarm_hours + 1) % 24;
                } else if self.down_button_counter == Self::BUTTON_MAX_VAL {
                    if self.alarm_hours == 0 {
                        self.alarm_hours = 23;
                    } else {
                        self.alarm_hours -= 1;
                    }
                }

                if self.main_button_counter == Self::BUTTON_MAX_VAL {
                    self.state = DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingMinute);
                }
            }
            DeviceState::UpdateAlarm(UpdateAlarmState::UpdatingMinute) => {
                if self.up_button_counter == Self::BUTTON_MAX_VAL {
                    self.alarm_minutes = (self.alarm_minutes + 1) % 60;
                } else if self.down_button_counter == Self::BUTTON_MAX_VAL {
                    if self.alarm_minutes == 0 {
                        self.alarm_minutes = 59;
                    } else {
                        self.alarm_minutes -= 1;
                    }
                }

                if self.main_button_counter == Self::BUTTON_MAX_VAL {
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
        if self.main_button_counter == Self::BUTTON_MAX_VAL {
            self.state = DeviceState::Running;

            if let AlarmFrequency::Once = self.frequency {
                disable_alarm(&mut self.i2c)?;
            }

            clear_alarm_flag(&mut self.i2c)?;
            self.piezo_pin.set_low();
            return Ok(());
        }

        if (self.tick_counter % 8192) > 4096  {
           if self.tick_counter % 16 == 0 {
               self.piezo_pin.toggle();
           }
        }

        Ok(())
    }

    fn running_state_tick(&mut self) -> Result<(), DeviceError> {
        if self.alarm_pin.is_low() {
            self.state = DeviceState::Alarm;
            return Ok(());
        }

        // All three buttons pressed = enter time update mode
        if self.main_button_counter > 0 && self.up_button_counter > 0 && self.down_button_counter > 0 {
            self.state = DeviceState::UpdateTime(UpdateTimeState::Begin);
            return Ok(());
        }

        if self.main_button_counter == Self::BUTTON_MAX_VAL {
            self.state = DeviceState::UpdateAlarm(UpdateAlarmState::Begin);
            return Ok(());
        }

        Ok(())
    }
}
