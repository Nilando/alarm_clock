use arduino_hal::{
    port::{
        D7, D8, D9, D10, Pin, mode::{Floating, Input, Output}
    }
};

pub const BUTTON_MAX_VAL: u32 = 4;

enum CapPhase {
    Charge,
    Drain,
}

pub struct ButtonScanner {
    send_pin: Pin<Output, D10>,            // PB2 - cap sense send
    up_button: Pin<Input<Floating>, D7>,   // PD7 - cap sense receive
    main_button: Pin<Input<Floating>, D8>, // PB0 - cap sense receive
    down_button: Pin<Input<Floating>, D9>, // PB1 - cap sense receive
    main_button_counter: u32,
    up_button_counter: u32,
    down_button_counter: u32,
    cap_phase: CapPhase,
    cap_button: u8,
    cap_count: u32,
}

impl ButtonScanner {
    const CAP_THRESHOLD: u32 = 8;

    pub fn new(
        send_pin: Pin<Output, D10>,            
        up_button: Pin<Input<Floating>, D7>,  
        main_button: Pin<Input<Floating>, D8>,
        down_button: Pin<Input<Floating>, D9>,
    ) -> Self {
        Self {
            send_pin,
            up_button,
            main_button,
            down_button,
            main_button_counter: 0,
            up_button_counter: 0,
            down_button_counter: 0,
            cap_phase: CapPhase::Charge,
            cap_button: 0,
            cap_count: 0,
        }
    }

    pub fn get_main(&self) -> u32 {
        self.main_button_counter
    }

    pub fn get_up(&self) -> u32 {
        self.up_button_counter
    }

    pub fn get_down(&self) -> u32 {
        self.down_button_counter
    }

    pub fn button_scan(&mut self) {
        match self.cap_phase {
            CapPhase::Drain => {
                if self.cap_count == 0 {
                    self.send_pin.set_low();

                    if self.main_button_counter == BUTTON_MAX_VAL {
                        self.main_button_counter = BUTTON_MAX_VAL - 1;
                    } else if self.up_button_counter == BUTTON_MAX_VAL {
                        self.up_button_counter = BUTTON_MAX_VAL - 1;
                    } else if self.down_button_counter == BUTTON_MAX_VAL {
                        self.down_button_counter = BUTTON_MAX_VAL - 1;
                    }
                }

                self.cap_count += 1;

                if self.cap_count == 500 {
                    self.cap_count = 0;
                    self.cap_phase = CapPhase::Charge;
                }
            }
            CapPhase::Charge => {
                self.send_pin.set_high();

                while self.cap_count <= Self::CAP_THRESHOLD && self.is_cap_button_low() {
                    self.cap_count += 1;
                }

                self.send_pin.set_low();

                self.update_cap_button_counter(self.cap_count > Self::CAP_THRESHOLD);

                self.cap_count = 0;
                self.cap_phase = CapPhase::Drain;

                self.cap_button = (self.cap_button + 1) % 3;
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

    fn update_cap_button_counter(&mut self, is_pressed: bool) {
        match self.cap_button {
            0 => {
                if is_pressed {
                    if self.main_button_counter == 0 {
                        self.main_button_counter = BUTTON_MAX_VAL;
                    } else {
                        self.main_button_counter = BUTTON_MAX_VAL - 1;
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
                        self.up_button_counter = BUTTON_MAX_VAL;
                    } else {
                        self.up_button_counter = BUTTON_MAX_VAL - 1;
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
                        self.down_button_counter = BUTTON_MAX_VAL;
                    } else {
                        self.down_button_counter = BUTTON_MAX_VAL - 1;
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
}
