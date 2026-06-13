use arduino_hal::{
    port::{
        D0, D1, D2, D3, D4, D5, D6, Pin, mode::Output
    }
};

#[derive(Copy, Clone)]
pub enum DisplayChar {
    Blank,
    E,
    Dash
}

impl DisplayChar {
    fn as_bit_pattern(self) -> u8 {
        match self {
            DisplayChar::E => DisplayController::A | DisplayController::B | DisplayController::D | DisplayController::E | DisplayController::G,
            DisplayChar::Dash => DisplayController::D,
            DisplayChar::Blank => 0
        }
    }
}


pub struct DisplayController {
    data_pin: Pin<Output, D0>,             // PD0 - SER
    latch_pin: Pin<Output, D1>,            // PD1 - RCLK
    clock_pin: Pin<Output, D2>,            // PD2 - SRCLK
    digit1: Pin<Output, D3>,               // PD3
    digit2: Pin<Output, D4>,               // PD4
    digit3: Pin<Output, D5>,               // PB6
    digit4: Pin<Output, D6>,               // PB7
}

impl DisplayController {
    //   -----     <--- A
    //  |  <- | ------- B
    //  |     | <------ C
    //   -----     <--- D
    //  |  <- | ------- E
    //  |     | <------ F
    //   -----     <--- G
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

    pub fn new(
            data_pin: Pin<Output, D0>,        
            latch_pin: Pin<Output, D1>,        
            clock_pin: Pin<Output, D2>,         
            digit1: Pin<Output, D3>,             
            digit2: Pin<Output, D4>,              
            digit3: Pin<Output, D5>,               
            digit4: Pin<Output, D6>,               
        ) -> Self {

        Self {
            data_pin,
            latch_pin,
            clock_pin,
            digit1,
            digit2,
            digit3,
            digit4
        }
    }

    pub fn display_chars(&mut self, chars: [DisplayChar; 4]) {
        self.display_bit_patterns(
            chars[0].as_bit_pattern(),
            chars[1].as_bit_pattern(),
            chars[2].as_bit_pattern(),
            chars[3].as_bit_pattern(),
        );
    }

    pub fn display_number(&mut self, d1: u8, d2: u8, d3: u8, d4: u8) {
        self.display_bit_patterns(
            Self::digit_pattern(d1), 
            Self::digit_pattern(d2), 
            Self::digit_pattern(d3), 
            Self::digit_pattern(d4), 
        );
    }

    fn display_bit_patterns(&mut self, d1: u8, d2: u8, d3: u8, d4: u8) {
            self.push_byte_to_display(d1);

            self.digit1.set_high();
            arduino_hal::delay_us(10);
            self.digit1.set_low();

            self.push_byte_to_display(d2);

            self.digit2.set_high();
            arduino_hal::delay_us(10);
            self.digit2.set_low();

            self.push_byte_to_display(d3);

            self.digit3.set_high();
            arduino_hal::delay_us(10);
            self.digit3.set_low();

            self.push_byte_to_display(d4);

            self.digit4.set_high();
            arduino_hal::delay_us(10);
            self.digit4.set_low();
    }


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
}

