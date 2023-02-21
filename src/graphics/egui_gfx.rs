// https://github.com/h3r2tic/egui-glutin-gl

use std::{ptr, ffi::c_void};

use egui::RawInput;

pub struct EguiState {
    pub ctx: egui::Context,
    pub pos_in_points: Option<egui::Pos2>,
    pub raw_input: RawInput,

    pub vao: u32,
    pub vbo: u32,
    pub tex: u32,
    pub shader: u32,

    pub buffer_size: u32,

    pub window_size: (u32, u32),
}

pub fn setup_vertex_arrays_egui() -> (u32, u32) {
    let (mut vao, mut vbo) = (0, 0);

    unsafe {
        gl::CreateBuffers(1, &mut vbo);
        gl::CreateVertexArrays(1, &mut vao);

        gl::VertexArrayElementBuffer(vao, vbo);

        gl::EnableVertexArrayAttrib(vao, 0);
        gl::EnableVertexArrayAttrib(vao, 1);
        gl::EnableVertexArrayAttrib(vao, 2);

        gl::VertexArrayAttribFormat( //vertex
            vao,
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            0 * 4,
        );

        gl::VertexArrayAttribFormat( //uv
            vao,
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            2 * 4,
        );

        gl::VertexArrayAttribFormat( //color
            vao,
            2,
            4,
            gl::UNSIGNED_BYTE,
            gl::FALSE,
            4 * 4,
        );

        gl::VertexArrayAttribBinding(vao, 0, 0);
        gl::VertexArrayAttribBinding(vao, 1, 0);
        gl::VertexArrayAttribBinding(vao, 2, 0);
    }

    (vao, vbo)
}

pub fn setup_texture_egui() -> u32 {
    let mut tex_e = 0;

    unsafe {
        gl::CreateTextures(gl::TEXTURE_2D, 1, &mut tex_e);
        gl::TextureParameteri(tex_e, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    }

    tex_e
}

pub fn paint_egui(egui_state: &mut EguiState) {
    let full_output = egui_state.ctx.end_frame();
    let clipped_primitives = egui_state.ctx.tessellate(full_output.shapes); // create triangles to paint
    // my_integration.set_cursor_icon(output.cursor_icon);

    update_textures(full_output.textures_delta.set, egui_state.tex);

    unsafe {
        gl::Enable(gl::SCISSOR_TEST);
        gl::Scissor(0, 0, egui_state.window_size.0 as i32, egui_state.window_size.1 as i32);

        gl::BindVertexArray(egui_state.vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, egui_state.vbo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, egui_state.vbo);
        gl::UseProgram(egui_state.shader);
        gl::BindTextureUnit(0, egui_state.tex);
        gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
    }

    for egui::ClippedPrimitive{clip_rect, primitive} in &clipped_primitives {
        unsafe {
            let x      = (clip_rect.min.x as i32).clamp(0, egui_state.window_size.0 as i32);
            let y      = (clip_rect.min.y as i32).clamp(0, egui_state.window_size.1 as i32);
            let width  = (clip_rect.max.x as i32).clamp(x, egui_state.window_size.0 as i32);
            let height = (clip_rect.max.y as i32).clamp(y, egui_state.window_size.1 as i32);
            gl::Scissor(x, egui_state.window_size.1 as i32 - height, width - x, height - y);

            let mesh = match &primitive {
                egui::epaint::Primitive::Mesh(mesh2) => mesh2,
                egui::epaint::Primitive::Callback(_) => todo!(),
            };

            let buffer_size = ((mesh.indices.len() + (mesh.vertices.len() * 5)) * 4) as u32;

            if egui_state.buffer_size < buffer_size {
                gl::NamedBufferData(
                    egui_state.vbo,
                    buffer_size as isize,
                    ptr::null(),
                    gl::DYNAMIC_DRAW,
                );

                egui_state.buffer_size = buffer_size;
            }

            gl::NamedBufferSubData(
                egui_state.vbo,
                0,
                mesh.indices.len() as isize * 4,
                mesh.indices.as_ptr() as *const c_void,
            );

            gl::NamedBufferSubData(
                egui_state.vbo,
                mesh.indices.len() as isize * 4,
                mesh.vertices.len() as isize * 5 * 4,
                mesh.vertices.as_ptr() as *const c_void,
            );

            gl::VertexArrayVertexBuffer(
                egui_state.vao,
                0,
                egui_state.vbo,
                mesh.indices.len() as isize * 4,
                5 * 4,
            );

            gl::DrawElements(gl::TRIANGLES, mesh.indices.len() as i32, gl::UNSIGNED_INT, ptr::null::<c_void>());
        }
    }

    unsafe{ gl::Disable(gl::SCISSOR_TEST); }

    for &id in &full_output.textures_delta.free {
        todo!();
    }
}

pub fn update_textures(tex_set: Vec<(egui::TextureId, egui::epaint::ImageDelta)>, tex_e: u32) {
    for (id, image_delta) in &tex_set {
        let pixels: Vec<(u8, u8, u8, u8)> = match &image_delta.image {
            egui::ImageData::Color(image) => {
                image.pixels.iter().map(|color| color.to_tuple()).collect()
            }

            egui::ImageData::Font(image) => {
                let gamma = 1.0;
                image.srgba_pixels(Some(gamma)).map(|color| color.to_tuple()).collect()
            }
        };

        let width = image_delta.image.width();
        let height = image_delta.image.height();

        if let Some(pos) = image_delta.pos {
            update_texture_egui(tex_e, pixels, pos[0], pos[1], width, height)
        } else {
            upload_texture_egui(tex_e, pixels, width, height);
        }
    }
}

pub fn update_texture_egui(tex_e: u32, pix: Vec<(u8, u8, u8, u8)>, x: usize, y: usize, width: usize, height: usize) {
    unsafe {
        gl::TextureSubImage2D(
            tex_e,
            0,
            x as i32,
            y as i32,
            width as i32,
            height as i32,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pix.as_ptr() as *const c_void,
        );
    }
}

pub fn upload_texture_egui(tex_e: u32, pix: Vec<(u8, u8, u8, u8)>, width: usize, height: usize) {
    unsafe {
        gl::TextureStorage2D(
            tex_e,
            1,
            gl::RGBA8,
            width as i32,
            height as i32,
        );

        update_texture_egui(tex_e, pix, 0, 0, width, height);
    }
}
