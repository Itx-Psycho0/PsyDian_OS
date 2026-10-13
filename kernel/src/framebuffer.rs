//! Framebuffer text console.
//!
//! `bootloader` 0.11 (BIOS) puts the screen into a VESA *graphics* mode, so the
//! old VGA text buffer at physical address 0xb8000 is neither displayed nor
//! mapped into the kernel's page tables. Text must be drawn pixel by pixel into
//! the framebuffer the bootloader hands us in `BootInfo::framebuffer`.

use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use core::{fmt, ptr};
use noto_sans_mono_bitmap::{
    FontWeight, RasterHeight, RasterizedChar, get_raster, get_raster_width,
};
use spin::Mutex;

/// Extra vertical space between text lines, in pixels.
const LINE_SPACING: usize = 2;
/// Extra horizontal space between characters, in pixels.
const LETTER_SPACING: usize = 0;
/// Padding kept free around the whole text area, in pixels.
const BORDER_PADDING: usize = 4;

const FONT_WEIGHT: FontWeight = FontWeight::Regular;
const CHAR_HEIGHT: RasterHeight = RasterHeight::Size16;
const CHAR_WIDTH: usize = get_raster_width(FONT_WEIGHT, CHAR_HEIGHT);
const BACKUP_CHAR: char = '?';

/// The global console. `None` until [`init`] is called.
pub static WRITER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);

/// Takes over the bootloader's framebuffer, clears it and starts the console.
pub fn init(framebuffer: &'static mut FrameBuffer) {
    let info = framebuffer.info();
    let writer = FrameBufferWriter::new(framebuffer.buffer_mut(), info);
    *WRITER.lock() = Some(writer);
}

pub struct FrameBufferWriter {
    buffer: &'static mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
}

impl FrameBufferWriter {
    pub fn new(buffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut writer = Self {
            buffer,
            info,
            x: BORDER_PADDING,
            y: BORDER_PADDING,
        };
        writer.clear();
        writer
    }

    /// Blanks the whole screen (this also removes the bootloader's own log text).
    pub fn clear(&mut self) {
        self.x = BORDER_PADDING;
        self.y = BORDER_PADDING;
        self.buffer.fill(0);
    }

    fn line_height() -> usize {
        CHAR_HEIGHT.val() + LINE_SPACING
    }

    fn newline(&mut self) {
        self.y += Self::line_height();
        self.carriage_return();
        // If the next line would not fit on screen, scroll the contents up.
        while self.y + CHAR_HEIGHT.val() + BORDER_PADDING > self.info.height {
            if !self.scroll_up_one_line() {
                break; // screen too small to scroll; stop instead of looping forever
            }
        }
    }

    fn carriage_return(&mut self) {
        self.x = BORDER_PADDING;
    }

    /// Moves every pixel row up by one text line and blanks the freed bottom line.
    /// Returns `false` if scrolling was impossible (so callers can't loop forever).
    fn scroll_up_one_line(&mut self) -> bool {
        let line = Self::line_height();
        let row_bytes = self.info.stride * self.info.bytes_per_pixel;
        let shift = line * row_bytes;
        let used = (self.info.height * row_bytes).min(self.buffer.len());
        if self.y < line || shift >= used {
            return false;
        }
        self.buffer.copy_within(shift..used, 0);
        self.buffer[used - shift..used].fill(0);
        self.y -= line;
        true
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let next_x = self.x + CHAR_WIDTH;
                if next_x + BORDER_PADDING > self.info.width {
                    self.newline();
                }
                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    fn write_rendered_char(&mut self, rendered: RasterizedChar) {
        for (dy, row) in rendered.raster().iter().enumerate() {
            for (dx, &intensity) in row.iter().enumerate() {
                self.write_pixel(self.x + dx, self.y + dy, intensity);
            }
        }
        self.x += rendered.width() + LETTER_SPACING;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }
        // Yellow text on black (matches the old VGA colour scheme).
        let color: [u8; 4] = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity, intensity, 0, 0],
            PixelFormat::Bgr => [0, intensity, intensity, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            // Unknown layouts: draw nothing rather than corrupt the screen.
            _ => return,
        };
        let bytes_per_pixel = self.info.bytes_per_pixel.min(color.len());
        let offset = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        if offset + bytes_per_pixel > self.buffer.len() {
            return;
        }
        for (i, &byte) in color[..bytes_per_pixel].iter().enumerate() {
            // Volatile: this memory is read by the display device, not by us,
            // so the compiler must not optimise the writes away.
            unsafe { ptr::write_volatile(&mut self.buffer[offset + i], byte) };
        }
    }
}

fn get_char_raster(c: char) -> RasterizedChar {
    get_raster(c, FONT_WEIGHT, CHAR_HEIGHT)
        .or_else(|| get_raster(BACKUP_CHAR, FONT_WEIGHT, CHAR_HEIGHT))
        .expect("backup char '?' must exist in the font")
}

impl fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
