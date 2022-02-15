#![feature(slice_as_chunks)]

use noto_sans_mono_bitmap::{get_bitmap, get_bitmap_width, BitmapHeight, FontWeight};
use pixels::{wgpu::Color, Pixels, SurfaceTexture};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    monitor::VideoMode,
    window::{Fullscreen, WindowBuilder},
};

const COLUMNS: usize = 80;
const ROWS: usize = 30;

const CHAR_HEIGHT: BitmapHeight = BitmapHeight::Size18;
const CHAR_WIDTH: usize = get_bitmap_width(FontWeight::Regular, CHAR_HEIGHT);

const WIDTH: usize = CHAR_WIDTH * COLUMNS;
const HEIGHT: usize = CHAR_HEIGHT as usize * ROWS;

const PX_WIDTH: usize = 4;

const CHAR_WIDTH_PX: usize = CHAR_WIDTH * PX_WIDTH;
const WIDTH_PX: usize = WIDTH * PX_WIDTH;

#[derive(Debug)]
struct Terminal {
    row: usize,
    column: usize,
    pixels: Pixels,
}

impl Terminal {
    fn new(pixels: Pixels) -> Self {
        Self {
            row: 0,
            column: 0,
            pixels,
        }
    }

    pub fn put_char(&mut self, char: char) {
        let buffer = self.pixels.get_frame();

        match char {
            // NULL
            '\0' => {}
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
                draw_char(&mut create_window(buffer, self.column, self.row), char);

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

        draw_char(&mut create_window(buffer, self.column, self.row), '_');
    }
}

type Pixel = [u8; PX_WIDTH];
type Row<'w> = &'w mut [Pixel; CHAR_WIDTH];
type Window<'w> = [Row<'w>; CHAR_HEIGHT as usize];

fn draw_char(window: &mut Window, char: char) {
    let bitmap = get_bitmap(char, FontWeight::Regular, BitmapHeight::Size18)
        .unwrap_or_else(|| panic!("unsupported char: {char}"))
        .bitmap();

    for (bitmap_row, window_row) in bitmap.iter().zip(window.iter_mut()) {
        for (bitmap_pixel, window_pixel) in bitmap_row.iter().zip(window_row.iter_mut()) {
            window_pixel.copy_from_slice(&[*bitmap_pixel, *bitmap_pixel, *bitmap_pixel, u8::MAX])
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

fn main() {
    let event_loop = EventLoop::new();

    let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let mut terminal = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);

        let mut pixels = Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture).unwrap();

        pixels.set_clear_color(Color::BLUE);
        // Clear frame
        for pixel in pixels.get_frame().chunks_exact_mut(4) {
            // TODO: color type
            pixel.copy_from_slice(&[0, 0, 0, u8::MAX]);
        }

        Terminal::new(pixels)
    };

    terminal.put_char('\0');

    event_loop.run(move |event, _window, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(PhysicalSize { height, width }) => {
                terminal.pixels.resize_surface(width, height);
            }
            WindowEvent::ReceivedCharacter(char) => {
                terminal.put_char(char);
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(virtual_keycode) = input.virtual_keycode {
                    match virtual_keycode {
                        VirtualKeyCode::Left => todo!(),
                        VirtualKeyCode::Right => todo!(),
                        VirtualKeyCode::F11 => {
                            dbg!("s");

                            if window.fullscreen().is_some() {
                                // FIXME:
                                // window.set_fullscreen(None)
                            } else {
                                let fullscreen_mode = {
                                    // Weirdly using .min() gives the biggest
                                    if let Some(video_mode) = window
                                        .current_monitor()
                                        .and_then(|monitor| monitor.video_modes().min())
                                    {
                                        Fullscreen::Exclusive(video_mode)
                                    } else {
                                        Fullscreen::Borderless(None)
                                    }
                                };

                                window.set_fullscreen(Some(fullscreen_mode))
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        },
        Event::RedrawRequested(_) => {
            terminal.pixels.render().unwrap();
        }

        _ => (),
    });
}
