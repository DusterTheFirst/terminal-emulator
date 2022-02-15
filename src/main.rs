#![feature(slice_as_chunks)]

use std::{thread, time::Duration};

use pixels::{wgpu::Color, Pixels, SurfaceTexture};
use terminal::{Terminal, HEIGHT, WIDTH};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

mod terminal;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event();

    let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_min_inner_size(PhysicalSize::new(WIDTH as f64, HEIGHT as f64))
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

    terminal.put_string("Hello there!\n", [255, 20, 255]);

    let proxy = event_loop.create_proxy();

    thread::spawn(move || loop {
        proxy.send_event(UserEvent::CursorFlash(true)).unwrap();
        thread::sleep(Duration::from_millis(500));

        proxy.send_event(UserEvent::CursorFlash(false)).unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    let proxy = event_loop.create_proxy();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(100));
        proxy.send_event(UserEvent::String("hello".into())).unwrap();
    });

    #[derive(Debug)]
    enum UserEvent {
        CursorFlash(bool),
        String(String),
    }

    event_loop.run(move |event, _window, control_flow| match event {
        Event::UserEvent(event) => {
            match event {
                UserEvent::CursorFlash(true) => terminal.cursor_on(),
                UserEvent::CursorFlash(false) => terminal.cursor_off(),
                UserEvent::String(string) => terminal.put_string(&string, [255, 200, 100]),
            }

            window.request_redraw();
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(PhysicalSize { height, width }) => {
                terminal.resize_surface(width, height);
            }
            WindowEvent::ReceivedCharacter(char) => {
                terminal.put_char(char, [255, 255, 255]);
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(virtual_keycode) = input.virtual_keycode {
                    match (virtual_keycode, input.state) {
                        (VirtualKeyCode::Left, ElementState::Pressed) => todo!(),
                        (VirtualKeyCode::Right, ElementState::Pressed) => todo!(),
                        (VirtualKeyCode::F11, ElementState::Pressed) => {
                            if window.fullscreen().is_some() {
                                window.set_fullscreen(None)
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
            terminal.render().unwrap();
        }

        _ => (),
    });
}
