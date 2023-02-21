use std::{ffi::CString, ptr, str::from_utf8};

use egui::Modifiers;
use glutin::{prelude::{GlConfig, GlDisplay, NotCurrentGlContextSurfaceAccessor}, display::GetGlDisplay, surface::{SurfaceAttributesBuilder, Surface, WindowSurface, GlSurface}, context::PossiblyCurrentContext};
use raw_window_handle::HasRawWindowHandle;
use winit::{event::{Event, WindowEvent, ElementState, VirtualKeyCode}, event_loop::ControlFlow};

use self::egui_gfx::*;

mod egui_gfx;

pub struct GlutinState {
    pub window: winit::window::Window,
    pub gl_ctx: PossiblyCurrentContext,
    pub gl_display: glutin::display::Display,
    pub gl_surface: Surface<WindowSurface>,
}

pub struct Graphics {
    pub glutin_state: GlutinState,
    pub egui_state: EguiState,
}

impl Graphics {
    pub fn setup(el: &winit::event_loop::EventLoop<()>, window_size: (u32, u32)) -> Self {
        let wb = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(window_size.0, window_size.1))
        .with_title("Yowu CatCaller");

        let (window, gl_config) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(wb))
        .build(&el, <_>::default(), |configs| {
            configs
                .filter(|c| c.srgb_capable())
                .max_by_key(|c| c.num_samples())
                .unwrap()
        }).unwrap();

        let window = window.unwrap();
        let raw_window_handle = window.raw_window_handle();
        let gl_display = gl_config.display();

        let context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_profile(glutin::context::GlProfile::Core)
            .with_context_api(glutin::context::ContextApi::OpenGl(Some(
                glutin::context::Version::new(4, 5),
            )))
            .build(Some(raw_window_handle));

        let (gl_surface, gl_ctx) = {
            let attrs = SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
                raw_window_handle,
                std::num::NonZeroU32::new(window_size.0).unwrap(),
                std::num::NonZeroU32::new(window_size.1).unwrap(),
            );

            let surface = unsafe { match gl_display.create_window_surface(&gl_config, &attrs) {
                Ok(o) => o,
                Err(_) => todo!("omg"),
            }};

            let context = unsafe { match gl_display.create_context(&gl_config, &context_attributes) {
                Ok(o) => o,
                Err(_) => todo!("omg"),
            }}.make_current(&surface).unwrap();
            (surface, context)
        };

        gl::load_with(|symbol| (gl_display.get_proc_address(&CString::new(symbol).unwrap()) as _));

        unsafe {
            gl::Enable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::CULL_FACE);
        }

        let (vao_e, vbo_e) = setup_vertex_arrays_egui();
        let vert_e = include_str!("shader_egui.vert");
        let frag_e = include_str!("shader_egui.frag");

        Self {
            glutin_state: GlutinState {
                window: window,
                gl_ctx: gl_ctx,
                gl_display: gl_display,
                gl_surface: gl_surface,
            },

            egui_state: EguiState{
                // windowed_context: windowed_context,
                ctx: egui::Context::default(),
                pos_in_points: None,
                raw_input: egui::RawInput::default(),
                vao: vao_e,
                vbo: vbo_e,
                tex: setup_texture_egui(),
                shader: create_program(vert_e, frag_e),
                buffer_size: 0,
                window_size: window_size,
            },
        }
    }

    pub fn paint(&mut self) {
        unsafe {
            gl::ClearColor(0.0, 0.1, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::STENCIL_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        paint_egui(&mut self.egui_state);

        self.glutin_state.gl_surface.swap_buffers(&self.glutin_state.gl_ctx).unwrap();
    }
}

fn create_program(vertex_src: &str, fragment_src: &str) -> u32 {
    let vertex_handler = compile_shader(vertex_src, gl::VERTEX_SHADER);
    let fragment_handler = compile_shader(fragment_src, gl::FRAGMENT_SHADER);

    unsafe {
        let program_id = gl::CreateProgram();
        gl::AttachShader(program_id, vertex_handler);
        gl::AttachShader(program_id, fragment_handler);

        gl::LinkProgram(program_id);
        gl::UseProgram(program_id);

        gl::DeleteShader(vertex_handler);
        gl::DeleteShader(fragment_handler);

        program_id
    }
}

fn compile_shader(source: &str, shader_type: u32) -> u32 {
    unsafe {
        let shader_handler = gl::CreateShader(shader_type);
        let c_str = CString::new(source.as_bytes()).unwrap();
        gl::ShaderSource(shader_handler, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader_handler);

        let mut success = i32::from(gl::FALSE);

        gl::GetShaderiv(shader_handler, gl::COMPILE_STATUS, &mut success);
        if success != i32::from(gl::TRUE) {
            let mut len = 0;
            gl::GetShaderiv(shader_handler, gl::INFO_LOG_LENGTH, &mut len);

            let mut info_log = vec![0; len as usize];
            gl::GetShaderInfoLog(shader_handler, len, ptr::null_mut(), info_log.as_mut_ptr() as *mut i8);
            println!("Shader compilation failed: {}", from_utf8(&info_log).unwrap());
            std::process::exit(-1);
        }

        shader_handler
    }
}

pub fn event_handling(event: Event<()>, control_flow: &mut ControlFlow, graphics_state: &mut Graphics) {
    match event {
        Event::LoopDestroyed => {}

        Event::WindowEvent{event, ..} => {
            match event {
                WindowEvent::ReceivedCharacter(ch) => {
                    if is_printable_char(ch) && !graphics_state.egui_state.raw_input.modifiers.ctrl {
                        graphics_state.egui_state.raw_input.events.push(egui::Event::Text(ch.to_string()));
                    }
                }

                WindowEvent::KeyboardInput{input, ..} => {
                    // ui_state.request_redraw = 2;

                    if let Some(keycode) = input.virtual_keycode {
                        let pressed = input.state == ElementState::Pressed;

                        if matches!(keycode, VirtualKeyCode::LAlt | VirtualKeyCode::RAlt) {
                            graphics_state.egui_state.raw_input.modifiers.alt = pressed;
                        }

                        if matches!(keycode, VirtualKeyCode::LControl | VirtualKeyCode::RControl) {
                            graphics_state.egui_state.raw_input.modifiers.ctrl = pressed;
                        }

                        if matches!(keycode, VirtualKeyCode::LShift | VirtualKeyCode::RShift) {
                            graphics_state.egui_state.raw_input.modifiers.shift = pressed;
                        }

                        if let Some(key) = translate_virtual_key_code(keycode) {
                            graphics_state.egui_state.raw_input.events.push(
                                egui::Event::Key{
                                    key,
                                    pressed,
                                    modifiers: graphics_state.egui_state.raw_input.modifiers,
                                    repeat: false,
                                }
                            );
                        }

                        if keycode == VirtualKeyCode::Escape && pressed {
                            *control_flow = ControlFlow::Exit
                        }
                    }
                }

                WindowEvent::CursorMoved{position, ..} => {
                    // ui_state.request_redraw = 2;

                    let pos_in_points_temp = egui::pos2(
                        position.x as f32 / 1.0,
                        position.y as f32 / 1.0,
                    );
                    graphics_state.egui_state.pos_in_points = Some(pos_in_points_temp);

                    graphics_state.egui_state.raw_input.events.push(egui::Event::PointerMoved(pos_in_points_temp));
                }

                WindowEvent::MouseInput{state, button, ..} => {
                    // ui_state.request_redraw = 2;

                    if let Some(pos_in_points_temp) = graphics_state.egui_state.pos_in_points {
                        if let Some(button) = match button {
                            winit::event::MouseButton::Left => Some(egui::PointerButton::Primary),
                            winit::event::MouseButton::Right => Some(egui::PointerButton::Secondary),
                            winit::event::MouseButton::Middle => Some(egui::PointerButton::Middle),
                            _ => None,
                        }
                        {
                            graphics_state.egui_state.raw_input.events.push(
                                egui::Event::PointerButton{
                                    pos: pos_in_points_temp,
                                    button,
                                    pressed: match state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    },
                                    modifiers: Modifiers::default(),
                                }
                            );
                        }
                    }
                }

                WindowEvent::MouseWheel {delta, ..} => {
                    // ui_state.request_redraw = 2;

                    if let winit::event::MouseScrollDelta::LineDelta(_, y) = delta {
                        graphics_state.egui_state.raw_input.events.push(
                            egui::Event::Scroll(egui::vec2(0.0, y * 35.0))
                        );
                    }
                }

                WindowEvent::Resized(physical_size) => {
                    // ui_state.request_redraw = 2;

                    if physical_size.width != 0 && physical_size.height != 0 {
                        graphics_state.glutin_state.gl_surface.resize(
                            &graphics_state.glutin_state.gl_ctx,
                            std::num::NonZeroU32::new(physical_size.width).unwrap(),
                            std::num::NonZeroU32::new(physical_size.height).unwrap(),
                        );
                        graphics_state.egui_state.window_size = (physical_size.width, physical_size.height);
    
                        unsafe {
                            gl::Viewport(0, 0, physical_size.width as i32, physical_size.height as i32);
                            gl::ProgramUniform2f(graphics_state.egui_state.shader, 3, physical_size.width as f32, physical_size.height as f32);
                        };
                    }
                }

                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }

                _ => ()
            }
        }

        _ => ()
    }
}

fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{E000}' ..= '\u{F8FF}').contains(&chr)
        || ('\u{F0000}' ..= '\u{FFFFD}').contains(&chr)
        || ('\u{100000}' ..= '\u{10FFFD}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

pub fn translate_virtual_key_code(key: VirtualKeyCode) -> Option<egui::Key> {
    use VirtualKeyCode::*;
    use egui::Key;

    Some(
        match key {
            Down => Key::ArrowDown,
            Left => Key::ArrowLeft,
            Right => Key::ArrowRight,
            Up => Key::ArrowUp,

            Escape => Key::Escape,
            Tab => Key::Tab,
            Back => Key::Backspace,
            Return => Key::Enter,
            Space => Key::Space,

            Insert => Key::Insert,
            Delete => Key::Delete,
            Home => Key::Home,
            End => Key::End,
            PageUp => Key::PageUp,
            PageDown => Key::PageDown,

            Key0 | Numpad0 => Key::Num0,
            Key1 | Numpad1 => Key::Num1,
            Key2 | Numpad2 => Key::Num2,
            Key3 | Numpad3 => Key::Num3,
            Key4 | Numpad4 => Key::Num4,
            Key5 | Numpad5 => Key::Num5,
            Key6 | Numpad6 => Key::Num6,
            Key7 | Numpad7 => Key::Num7,
            Key8 | Numpad8 => Key::Num8,
            Key9 | Numpad9 => Key::Num9,

            A => Key::A,
            B => Key::B,
            C => Key::C,
            D => Key::D,
            E => Key::E,
            F => Key::F,
            G => Key::G,
            H => Key::H,
            I => Key::I,
            J => Key::J,
            K => Key::K,
            L => Key::L,
            M => Key::M,
            N => Key::N,
            O => Key::O,
            P => Key::P,
            Q => Key::Q,
            R => Key::R,
            S => Key::S,
            T => Key::T,
            U => Key::U,
            V => Key::V,
            W => Key::W,
            X => Key::X,
            Y => Key::Y,
            Z => Key::Z,

            _ => return None,
        }
    )
}
