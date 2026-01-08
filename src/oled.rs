use arduino_hal::prelude::*;
use arduino_hal::I2c;
use arduino_hal::i2c::Error as I2cError;

use crate::device::AlarmFrequency;

// SSD1306 OLED I2C Address
const OLED_ADDR: u8 = 0x3C;

static mut OLED_BUFFER: [u8; 1024] = [0; 1024];

pub fn init_oled(i2c: &mut I2c) -> Result<(), I2cError> {
    let init_cmds = [
        0xAE, // Display OFF
        0xD5, 0x80, // Set display clock
        0xA8, 0x3F, // Set multiplex (HEIGHT-1): 0x3F for 64 rows
        0xD3, 0x00, // Set display offset
        0x40, // Set start line
        0x8D, 0x14, // Charge pump
        0x20, 0x00, // Memory mode
        0xA1, // Set segment remap
        0xC8, // COM scan direction
        0xDA, 0x12, // COM hardware config
        0x81, 0x01, // Set contrast
        0xD9, 0x42, // Set precharge
        0xDB, 0x40, // Set VCOMH
        0xA4, // Display all on resume
        0xA6, // Normal display (not inverted)
        0xAF, // Display ON
    ];

    for &cmd in &init_cmds {
        i2c.write(OLED_ADDR, &[0x00, cmd])?;
    }

    Ok(())
}

pub fn display_frequency(i2c: &mut I2c, frequency: &AlarmFrequency) -> Result<(), I2cError> {
    clear_top(i2c)?;

    match frequency {
        AlarmFrequency::On => {
            display_seven_segment_number(i2c, 14, 8, 3, 30, 8 * 6, 0);
            display_seven_segment_number(i2c, 14, 8, 3, 40, 8 * 6, 10);
        }
        AlarmFrequency::Off => {
            display_seven_segment_number(i2c, 14, 8, 3, 30, 8 * 6, 0);
            display_seven_segment_number(i2c, 14, 8, 3, 40, 8 * 6, 13);
            display_seven_segment_number(i2c, 14, 8, 3, 50, 8 * 6, 13);
        }
        AlarmFrequency::Once => {
            display_seven_segment_number(i2c, 14, 8, 3, 30, 8 * 6, 0);
            display_seven_segment_number(i2c, 14, 8, 3, 40, 8 * 6, 10);
            display_seven_segment_number(i2c, 14, 8, 3, 50, 8 * 6, 11);
            display_seven_segment_number(i2c, 14, 8, 3, 60, 8 * 6, 12);
        }
    }

    Ok(())
}

pub fn clear_top(i2c: &mut I2c) -> Result<(), I2cError> {
        for i in 0..256 {
            unsafe {
                OLED_BUFFER[i] = 0;
            }
        }
        i2c.write(OLED_ADDR, &[0x00, 0x21, 0, 127])?; // column range
        i2c.write(OLED_ADDR, &[0x00, 0x22, 0, 1])?;   // page range

        for _ in 0..256 {
            i2c.write(OLED_ADDR, &[0x40, 0])?;
        }

        Ok(())
}

pub fn display_date(i2c: &mut I2c, date: u8, month: u8, year: u8) -> Result<(), I2cError> {
    display_seven_segment_number(i2c, 14, 8, 3, 20, 8 * 6, date / 10);
    display_seven_segment_number(i2c, 14, 8, 3, 30, 8 * 6, date % 10);

    display_seven_segment_number(i2c, 14, 8, 3, 50, 8 * 6, month / 10);
    display_seven_segment_number(i2c, 14, 8, 3, 60, 8 * 6, month % 10);

    display_seven_segment_number(i2c, 14, 8, 3, 90, 8 * 6, year / 10);
    display_seven_segment_number(i2c, 14, 8, 3, 100, 8 * 6, year % 10);

    Ok(())
}

pub fn display_time(i2c: &mut I2c, hours: u8, minutes: u8, seconds: u8, quick_mode: bool) -> Result<(), I2cError> {
    if (seconds == 0 && minutes == 0) || !quick_mode {
        display_seven_segment_number(i2c, 47, 20, 7,   0, 0, hours / 10);
        display_seven_segment_number(i2c, 47, 20, 7,  25, 0, hours % 10);
    }

    if (seconds == 0) || (seconds == 1) || !quick_mode {
        display_seven_segment_number(i2c, 47, 20, 7,  60, 0, minutes / 10);
        display_seven_segment_number(i2c, 47, 20, 7,  85, 0, minutes % 10);
    }

    display_seven_segment_number(i2c, 7, 5, 1, 110, 0, seconds / 10);
    display_seven_segment_number(i2c, 7, 5, 1, 117, 0, seconds % 10);

    Ok(())
}

const OLED_HEIGHT: u8 = 64;
const OLED_WIDTH: u8 = 128;

pub fn clear_oled(i2c: &mut I2c) -> Result<(), I2cError> {
    // Set full column/page address
    i2c.write(OLED_ADDR, &[0x00, 0x21, 0, 127])?; // column range
    i2c.write(OLED_ADDR, &[0x00, 0x22, 0, 7])?;   // page range

    unsafe {
        for i in 0..1024 {
            OLED_BUFFER[i] = 0;
        }
        i2c.write(OLED_ADDR, &[0x00, 0x21, 0, 127])?; // column range
        i2c.write(OLED_ADDR, &[0x00, 0x22, 0, 1])?;   // page range

        for _ in 0..256 {
            i2c.write(OLED_ADDR, &[0x40, 0])?;
        }

        arduino_hal::delay_ms(10);

        i2c.write(OLED_ADDR, &[0x00, 0x21, 0, 127])?; // column range
        i2c.write(OLED_ADDR, &[0x00, 0x22, 2, 7])?;   // page range

        for _ in 256..1024 {
            i2c.write(OLED_ADDR, &[0x40, 0])?;
        }
    }

    Ok(())
}

fn display_seven_segment_number(
    i2c: &mut I2c,
    height: u8,
    width: u8,
    thickness: u8,
    x: u8,
    y: u8,
    number: u8,
) {
    let start_page = 7 - ((y + height) / 8);
    let end_page = 7 - (y / 8);
    let start_col = x;
    let end_col = x + width;
    for page in start_page..=end_page {
        for col in start_col..=end_col {
            let idx = (page as usize) * 128 + col as usize;

            unsafe {
                OLED_BUFFER[idx] = 0;
            }
        }
    }

    // o n c e f
    // 7-segment mapping (segments: 0 to 6)
    let segments_map: [u8; 14] = [
        0b00111111, // 0 & o
        0b00011000, // 1
        0b01101110, // 2
        0b01111100, // 3
        0b01011001, // 4
        0b01110101, // 5
        0b01110111, // 6
        0b00011100, // 7
        0b01111111, // 8
        0b01111101, // 9
                    
        0b00011111, // n
        0b00100111, // c
        0b01100111, // e
        0b01000111, // f
    ];

    let segments = segments_map[number as usize];


    // Segment coordinates
    if segments & 0b00000001 != 0 {
        draw_vertical(x, y + (height / 2), height / 2, thickness); // segment 0 (top-left)
    }
    if segments & 0b00000010 != 0 {
        draw_vertical(x, y, height / 2, thickness); // segment 1 (bottom-left)
    }
    if segments & 0b00000100 != 0 {
        draw_horizontal(x, y + height - thickness, width, thickness); // segment 2 (top)
    }
    if segments & 0b00001000 != 0 {
        draw_vertical(x + width - thickness, y + (height / 2), height / 2, thickness); // segment 3 (top-right)
    }
    if segments & 0b00010000 != 0 {
        draw_vertical(x + width - thickness, y, height / 2, thickness); // segment 4 (bottom-right)
    }
    if segments & 0b00100000 != 0 {
        draw_horizontal(x, y, width, thickness); // segment 5 (bottom)
    }
    if segments & 0b01000000 != 0 {
        draw_horizontal(x, y + (height / 2) - (thickness / 2), width, thickness); // segment 6 (middle)
    }

    i2c.write(OLED_ADDR, &[0x00, 0x21, start_col, end_col]); // column range
    i2c.write(OLED_ADDR, &[0x00, 0x22, start_page, end_page]);   // page range
                                                                 
    for page in start_page..=end_page {
        for col in start_col..=end_col {
            let idx = (page as usize) * 128 + col as usize;

            unsafe {
                i2c.write(OLED_ADDR, &[0x40, OLED_BUFFER[idx]]);
            }
        }
    }
}

// the below logic is inefficient/could be optimized, but also its working plenty fast for this use
// case.

fn set_pixel(bx: u8, by: u8) {
        if bx >= OLED_WIDTH || by >= OLED_HEIGHT {
            return;
        }
        let page = 7 - (by / 8);
        let bit = 0x80 >> (by % 8);
        let idx = (page as usize) * 128 + bx as usize;
        
        unsafe {
            OLED_BUFFER[idx] |= bit;
        }
}

// Draw horizontal line
fn draw_horizontal(x0: u8, y0: u8, w: u8, thickness: u8) {
    for t in 0..thickness {
        for dx in 0..w {
            set_pixel(x0 + dx, y0 + t);
        }
    }

    // OPTIMIZE: for each byte 
    // if its the last byte, or first byte to special calculations, else just print a full thing of
    // 1s
}

// Draw vertical line
fn draw_vertical(x0: u8, y0: u8, h: u8, thickness: u8) {
    for t in 0..thickness {
        for dy in 0..h {
            set_pixel(x0 + t, y0 + dy);
        }
    }

    // OPTIMIZE: for each byte 
    // calculate what the byte looks like, then copy it across for every byte
}
