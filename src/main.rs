#![feature(slice_as_chunks)]

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::mpsc::RecvError,
    thread,
    time::Duration,
};

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

    let proxy = event_loop.create_proxy();

    thread::spawn(move || loop {
        proxy.send_event(UserEvent::CursorFlash(true)).unwrap();
        thread::sleep(Duration::from_millis(500));

        proxy.send_event(UserEvent::CursorFlash(false)).unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    let (stdin_send, stdin_recv) = std::sync::mpsc::channel();

    {
        let mut command = Command::new("cmd.exe")
            .args(["/U", "/Q"])
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        thread::spawn({
            let mut stdin = command.stdin.take().unwrap();

            move || -> Result<(), RecvError> {
                let mut command = String::new();

                loop {
                    let char = stdin_recv.recv()?;

                    match char {
                        '\u{8}' => {
                            command.pop();
                        }
                        _ => command.push(char),
                    }

                    if char == '\n' {
                        stdin.write_all(command.as_bytes()).unwrap();
                        command.clear();
                    }
                }
            }
        });

        thread::spawn({
            let mut stdout = command.stdout.take().unwrap();
            let proxy = event_loop.create_proxy();

            move || {
                let mut buffer = [0; 1028];
                loop {
                    let amount_read = stdout.read(&mut buffer).unwrap();

                    let string = String::from_utf8(buffer[..amount_read].to_vec());

                    match string {
                        Ok(string) => proxy
                            .send_event(UserEvent::String(string, [255, 255, 255]))
                            .unwrap(),
                        Err(utf8_error) => proxy
                            .send_event(UserEvent::String(
                                format!(
                                    "KERNEL ERROR: encountered a non-utf8 string: {utf8_error}"
                                ),
                                [240, 80, 40],
                            ))
                            .unwrap(),
                    }
                }
            }
        });

        thread::spawn({
            let mut stderr = command.stderr.take().unwrap();
            let proxy = event_loop.create_proxy();

            move || {
                let mut buffer = [0; 1028];

                loop {
                    let amount_read = stderr.read(&mut buffer).unwrap();

                    let string = String::from_utf8(buffer[..amount_read].to_vec()).unwrap();

                    proxy
                        .send_event(UserEvent::String(string, [255, 150, 150]))
                        .unwrap();
                }
            }
        });
    };

    #[derive(Debug)]
    enum UserEvent {
        CursorFlash(bool),
        String(String, [u8; 3]),
    }

    event_loop.run(move |event, _window, control_flow| match event {
        Event::UserEvent(event) => {
            match event {
                UserEvent::CursorFlash(true) => terminal.cursor_on(),
                UserEvent::CursorFlash(false) => terminal.cursor_off(),
                UserEvent::String(string, color) => terminal.put_string(&string, color),
            }

            window.request_redraw();
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(PhysicalSize { height, width }) => {
                terminal.resize_surface(width, height);
            }
            WindowEvent::ReceivedCharacter(char) => {
                terminal.put_char(char, [255, 200, 100]);
                let send_result = if char == '\r' {
                    stdin_send.send('\n')
                } else {
                    stdin_send.send(char)
                };

                if send_result.is_err() {
                    *control_flow = ControlFlow::Exit;
                }

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
