use noto_sans_mono_bitmap::{get_bitmap, get_bitmap_width, BitmapHeight, FontWeight};
use pixels::Pixels;

pub const COLUMNS: usize = 80;
pub const ROWS: usize = 30;

const CHAR_HEIGHT: BitmapHeight = BitmapHeight::Size18;
const CHAR_WIDTH: usize = get_bitmap_width(FontWeight::Regular, CHAR_HEIGHT);

pub const WIDTH: usize = CHAR_WIDTH * COLUMNS;
pub const HEIGHT: usize = CHAR_HEIGHT as usize * ROWS;

const PX_WIDTH: usize = 4;

const CHAR_WIDTH_PX: usize = CHAR_WIDTH * PX_WIDTH;
const WIDTH_PX: usize = WIDTH * PX_WIDTH;

#[derive(Debug)]
pub struct Terminal {
    row: usize,
    column: usize,
    pixels: Pixels,
}

impl Terminal {
    pub fn new(pixels: Pixels) -> Self {
        Self {
            row: 0,
            column: 0,
            pixels,
        }
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.pixels.resize_surface(width, height);
    }

    pub fn render(&mut self) -> Result<(), pixels::Error> {
        self.pixels.render()
    }

    pub fn cursor_off(&mut self) {
        let buffer = self.pixels.get_frame();

        clear_window(&mut create_window(buffer, self.column, self.row));
    }

    pub fn cursor_on(&mut self) {
        let buffer = self.pixels.get_frame();

        draw_char(
            &mut create_window(buffer, self.column, self.row),
            '_',
            [255, 255, 255],
        );
    }

    pub fn put_string(&mut self, string: &str, color: [u8; 3]) {
        for char in string.chars().filter(|char| *char != '\r') {
            self.put_char(char, color)
        }
    }

    pub fn put_char(&mut self, char: char, color: [u8; 3]) {
        let buffer = self.pixels.get_frame();

        match char {
            // NULL
            '\0' | '\t' => {}
            // Backspace
            '\u{0008}' => {
                if self.column == 0 && self.row == 0 {
                    return;
                }

                clear_window(&mut create_window(buffer, self.column, self.row));

                if self.column == 0 {
                    self.row -= 1;
                    self.column = COLUMNS;
                }

                self.column -= 1;

                clear_window(&mut create_window(buffer, self.column, self.row));
            }
            // Newline
            '\r' | '\n' => {
                clear_window(&mut create_window(buffer, self.column, self.row));

                self.row += 1;
                self.column = 0;
            }
            _ => {
                draw_char(
                    &mut create_window(buffer, self.column, self.row),
                    char,
                    color,
                );

                self.column += 1;
            }
        }

        if self.column >= COLUMNS {
            self.column = 0;
            self.row += 1;
        }

        if self.row >= ROWS {
            let start = WIDTH_PX * CHAR_HEIGHT as usize;
            let end = buffer.len() - start;

            buffer.copy_within(start.., 0);
            for pixel in buffer[end..].chunks_exact_mut(PX_WIDTH) {
                pixel.copy_from_slice(&[0, 0, 0, u8::MAX]);
            }

            self.row = ROWS - 1;
        }

        draw_char(
            &mut create_window(buffer, self.column, self.row),
            '_',
            [255, 255, 255],
        );
    }
}

type Pixel = [u8; PX_WIDTH];
type Row<'w> = &'w mut [Pixel; CHAR_WIDTH];
type Window<'w> = [Row<'w>; CHAR_HEIGHT as usize];

fn draw_char(window: &mut Window, char: char, [r, g, b]: [u8; 3]) {
    let bitmap = get_bitmap(char, FontWeight::Regular, CHAR_HEIGHT)
        .unwrap_or_else(|| {
            eprintln!("unsupported char: {char:?} Unicode: {:#X}", char as u32);

            get_bitmap('?', FontWeight::Regular, CHAR_HEIGHT).expect("question mark always exists")
        })
        .bitmap();

    for (bitmap_row, window_row) in bitmap.iter().zip(window.iter_mut()) {
        for (bitmap_pixel, window_pixel) in bitmap_row.iter().zip(window_row.iter_mut()) {
            let opacity = *bitmap_pixel as f64 / u8::MAX as f64;

            window_pixel.copy_from_slice(&[
                (r as f64 * opacity).round() as u8,
                (g as f64 * opacity).round() as u8,
                (b as f64 * opacity).round() as u8,
                u8::MAX,
            ])
        }
    }
}

fn clear_window(window: &mut Window) {
    for window_row in window.iter_mut() {
        for window_pixel in window_row.iter_mut() {
            window_pixel.copy_from_slice(&[0, 0, 0, u8::MAX])
        }
    }
}

fn create_window(buffer: &mut [u8], column: usize, row: usize) -> Window {
    let left = column * CHAR_WIDTH_PX;
    let right = left + CHAR_WIDTH_PX;

    let top = row * CHAR_HEIGHT as usize * WIDTH_PX;
    let bottom = top + CHAR_HEIGHT as usize * WIDTH_PX;

    let windows = buffer[top..bottom]
        .chunks_exact_mut(WIDTH_PX)
        .map(|row| {
            let (columns, remainder) = row[left..right].as_chunks_mut::<PX_WIDTH>();

            debug_assert_eq!(remainder.len(), 0);

            columns.try_into().expect("wrong column length")
        })
        .collect::<Vec<_>>();

    windows.try_into().expect("wrong row length")
}
