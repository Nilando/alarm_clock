use arduino_hal::prelude::*;
use arduino_hal::I2c;
use arduino_hal::i2c::Error as I2cError;

// SSD1306 OLED I2C Address
const OLED_ADDR: u8 = 0x3C;

// Simple 5x7 font data for digits 0-9 and colon
const FONT_5X7: [[u8; 5]; 11] = [
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // 0 
    [0x00, 0x42, 0x7F, 0x40, 0x00], // 1
    [0x42, 0x61, 0x51, 0x49, 0x46], // 2
    [0x21, 0x41, 0x45, 0x4B, 0x31], // 3
    [0x18, 0x14, 0x12, 0x7F, 0x10], // 4
    [0x27, 0x45, 0x45, 0x45, 0x39], // 5
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // 6
    [0x01, 0x71, 0x09, 0x05, 0x03], // 7
    [0x36, 0x49, 0x49, 0x49, 0x36], // 8
    [0x06, 0x49, 0x49, 0x29, 0x1E], // 9
    [0x00, 0x36, 0x36, 0x00, 0x00], // : (colon)
];

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
        0x81, 0xCF, // Set contrast
        0xD9, 0xF1, // Set precharge
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

pub fn clear_oled(i2c: &mut I2c) -> Result<(), I2cError> {
    i2c.write(OLED_ADDR, &[0x00, 0x21, 0, 127])?;
    // Set page address range 0-7
    i2c.write(OLED_ADDR, &[0x00, 0x22, 0, 7])?;

    // Clear all pages
    for _ in 0..1024 {
        i2c.write(OLED_ADDR, &[0x40, 0x00])?;
    }

    Ok(())
}

pub fn display_time(i2c: &mut I2c, hours: u8, minutes: u8, seconds: u8) -> Result<(), I2cError> {
    let time_str = [
        b'0' + (hours / 10),
        b'0' + (hours % 10),
        b':',
        b'0' + (minutes / 10),
        b'0' + (minutes % 10),
        b':',
        b'0' + (seconds / 10),
        b'0' + (seconds % 10),
    ];

    let mut x = 10u8;
    let page = 2; // Middle of screen

    for &c in &time_str {
        draw_char(i2c, c, x, page)?;

        x += 6;
    }

    Ok(())
}

fn draw_char(i2c: &mut I2c, c: u8, x: u8, page: u8) -> Result<(), I2cError> {
    let char_index = if c >= b'0' && c <= b'9' {
        (c - b'0') as usize
    } else if c == b':' {
        10
    } else {
        // TODO: turn this into a error
        return Ok(());
    };

    // set column address
    i2c.write(OLED_ADDR, &[0x00, 0x21, x, x + 5])?;
    // Set page address
    i2c.write(OLED_ADDR, &[0x00, 0x22, 2, 3])?;

    // Write character data
    for &byte in &FONT_5X7[char_index] {
        i2c.write(OLED_ADDR, &[0x40, byte])?;
    }

    // Add spacing
    i2c.write(OLED_ADDR, &[0x40, 0x00])?;

    for &byte in &FONT_5X7[char_index] {
        i2c.write(OLED_ADDR, &[0x40, byte])?;
    }
    // Add spacing
    i2c.write(OLED_ADDR, &[0x40, 0x00])?;

    Ok(())
}
