use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::marker::PhantomData;
use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;

extern crate nalgebra_glm as glm;

struct FpsCounter {
    last_time: std::time::Instant,
    counter: u32
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter {
            last_time: std::time::Instant::now(),
            counter: 0
        }
    }

    fn update(&mut self) -> Option<u32> {
        self.counter += 1;
        match self.last_time.elapsed().as_millis() {
            s if s >= 1000 => {
                let counter = self.counter;
                self.counter = 0;
                self.last_time = std::time::Instant::now();
                Some(counter)
            },
            _ => None
        }
    }
}

struct TextureBuffer {
    buffer: Vec<u8>,
    size: (u32, u32),
    bytes_per_pixel: u32
}   

impl TextureBuffer {
    fn new(size: (u32, u32), bytes_per_pixel: u32) -> Self {
        TextureBuffer {
            buffer: vec![0; (size.0 * size.1 * bytes_per_pixel) as usize],
            size: size,
            bytes_per_pixel: bytes_per_pixel
        }
    }

    fn pitch(&self) -> usize {
        (self.size.0 * self.bytes_per_pixel) as usize
    }

    fn set(&mut self, point: (u32, u32), color: &[u8; 4]) {
        let index = (self.bytes_per_pixel * (point.1 * self.size.0 + point.0)) as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(color.as_ptr(),
                self.buffer.as_mut_ptr().offset(index as isize),
                std::mem::size_of_val(color));
        }
    }

    fn clear(&mut self, value: u8) {
        for v in &mut self.buffer {
            *v = value;
        }
    }
}

struct Camera {
    view: glm::Mat4,
    projection: glm::Mat4
}

impl Camera {
    fn new(aspect: f32, fovy: f32, near: f32, far: f32) -> Self {
        Camera {
            view: glm::identity(),
            projection: glm::perspective(aspect, fovy, near, far)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: glm::Vec3,
    uv: glm::Vec2
}

impl Add<Vertex> for Vertex {
    type Output = Vertex;
    fn add(self, rhs: Vertex) -> Self::Output {
        Vertex {
            position: self.position + rhs.position,
            uv: self.uv + rhs.uv
        }
    }
}

impl Sub<Vertex> for Vertex {
    type Output = Vertex;
    fn sub(self, rhs: Vertex) -> Self::Output {
        Vertex {
            position: self.position - rhs.position,
            uv: self.uv - rhs.uv
        }
    }
}

impl Mul<f32> for Vertex {
    type Output = Vertex;
    fn mul(self, rhs: f32) -> Self::Output {
        Vertex {
            position: self.position * rhs,
            uv: self.uv * rhs
        }
    } 
}

trait Linear: Copy + Add<Self, Output=Self> + Sub<Self, Output=Self> + Mul<f32, Output=Self> {}

impl Linear for Vertex {}

struct RenderContext<'a, V: Clone + Linear, 
    VS: Fn(&mut V) -> glm::Vec4, 
    PS: Fn(&V) -> [u8; 4]> {   
    cull_backface: bool,     
    target: &'a mut TextureBuffer,
    vertex_shader: VS,
    pixel_shader: PS,
    phantom: PhantomData<V>
}

impl<'a, V: Clone + Linear, 
    VS: Fn(&mut V) -> glm::Vec4, 
    PS: Fn(&V) -> [u8; 4]> RenderContext<'a, V, VS, PS> {
    fn new(cull_backface: bool, target: &'a mut TextureBuffer, vertex_shader: VS, pixel_shader: PS) -> Self {
        RenderContext {
            cull_backface,
            target,
            vertex_shader,
            pixel_shader,
            phantom: PhantomData
        }
    }

    fn draw_indexed_triangles(&mut self, indices: &[usize], vertices: &[V]) {
        let mut vertices = vertices.to_vec();
        let positions = vertices.
            iter_mut().
            map(&self.vertex_shader).
            collect::<Vec<_>>();
        let mut current_indices = indices;
        loop {
            if let [i0, i1, i2, ref rest @ ..] = *current_indices {
                current_indices = rest;
                let mut p0 = positions[i0];
                let mut p1 = positions[i1];
                let mut p2 = positions[i2];
                let v0 = &vertices[i0];
                let v1 = &vertices[i1];
                let v2 = &vertices[i2];
                p0 /= p0.w;
                p1 /= p1.w;
                p2 /= p2.w;
                if self.cull_backface {
                    let d0 = p2 - p0;
                    let d1 = p2 - p1;
                    if (d0.x * d1.y) - (d0.y * d1.x) < 0.0 {
                        continue;
                    }
                }
                self.draw_triangle(
                    &self.transform_to_target_coordinates(&p0), 
                    &self.transform_to_target_coordinates(&p1), 
                    &self.transform_to_target_coordinates(&p2), 
                    v0, v1, v2
                );
            } else {
                break;
            }
        }
    }
    
    fn draw_triangle(&mut self, 
        p0: &glm::Vec4, p1: &glm::Vec4, p2: &glm::Vec4,
        v0: &V, v1: &V, v2: &V) {
        let mut p0 = p0;
        let mut p1 = p1;
        let mut p2 = p2;
        let mut v0 = v0;
        let mut v1 = v1;
        let mut v2 = v2;

        if p1.y < p0.y {
            std::mem::swap(&mut p0, &mut p1);
            std::mem::swap(&mut v0, &mut v1);
        }
        if p2.y < p1.y {
            std::mem::swap(&mut p1, &mut p2);
            std::mem::swap(&mut v1, &mut v2);
        }
        if p1.y < p0.y {
            std::mem::swap(&mut p0, &mut p1);
            std::mem::swap(&mut v0, &mut v1);
        }

        //natural flat top
        if p0.y == p1.y { 
            if p1.x < p0.x {
                std::mem::swap(&mut p0, &mut p1);
                std::mem::swap(&mut v0, &mut v1);
            }
            self.draw_flat_top_triangle(p0, p1, p2, v0, v1, v2);
        }
        //natural flat bottom
        else if p1.y == p2.y {
            if p2.x < p1.x {
                std::mem::swap(&mut p1, &mut p2);
                std::mem::swap(&mut v1, &mut v2);
            }
            self.draw_flat_bottom_triangle(p0, p1, p2, v0, v1, v2);
        }
        //general triangle
        else {
            let alpha = (p1.y - p0.y) / (p2.y - p0.y);
            let pi = p0 + (p2 - p0) * alpha;
            let vi = *v0 + (*v2 - *v0) * alpha;
            //major right
            if p1.x < pi.x {
                self.draw_flat_bottom_triangle(p0, p1, &pi, v0, v1, &vi);
                self.draw_flat_top_triangle(p1, &pi, p2, v1, &vi, v2);
            }
            //major left
            else {
                self.draw_flat_bottom_triangle(p0, &pi, p1, v0, &vi, v1);
                self.draw_flat_top_triangle(&pi, p1, p2, &vi, v1, v2);
            }
        }
    }

    fn draw_flat_top_triangle(&mut self, 
        p0: &glm::Vec4, p1: &glm::Vec4, p2: &glm::Vec4,
        v0: &V, v1: &V, v2: &V) {

        let slope1 = (p2.x - p0.x) / (p2.y - p0.y);
        let slope2 = (p2.x - p1.x) / (p2.y - p1.y);

        self.draw_flat_triangle_common(p0, p1, p2, [(slope1, p0), (slope2, p1)], v0, v1, v2);
    }

    fn draw_flat_bottom_triangle(&mut self, 
        p0: &glm::Vec4, p1: &glm::Vec4, p2: &glm::Vec4,
        v0: &V, v1: &V, v2: &V) {

        let slope1 = (p1.x - p0.x) / (p1.y - p0.y);
        let slope2 = (p2.x - p0.x) / (p2.y - p0.y);

        self.draw_flat_triangle_common(p0, p1, p2, [(slope1, p0), (slope2, p0)], v0, v1, v2);
    }

    fn draw_flat_triangle_common(&mut self, 
        p0: &glm::Vec4, p1: &glm::Vec4, p2: &glm::Vec4, lines: [(f32, &glm::Vec4); 2],
        v0: &V, v1: &V, v2: &V) {
    
        let [(slope0, line_start0), 
            (slope1, line_start1)] = lines;
            
        let snap = |c: f32| {
            (c - 0.5).ceil()
        };

        let y_start = snap(p0.y).max(0.0) as i32;
        let y_end = snap(p2.y).min(self.target.size.1 as f32) as i32;
            
        for y in y_start..y_end {
            let px0 = slope0 * (y as f32 + 0.5 - line_start0.y) + line_start0.x;
            let px1 = slope1 * (y as f32 + 0.5 - line_start1.y) + line_start1.x;

            let x_start = snap(px0).max(0.0) as i32;
            let x_end = snap(px1).min(self.target.size.0 as f32) as i32;

            for x in x_start..x_end {
                let f = Self::barycentric_coordinates(
                    &glm::vec4(x as f32, y as f32, 0.0, 0.0), &p0, &p1, &p2
                );
                let interpolated = *v0 * f.0 + *v1 * f.1 + *v2 * f.2;
                let color = (self.pixel_shader)(&interpolated);
                self.target.set((x as u32, y as u32), &color);
            }
        }
    }

    fn barycentric_coordinates(p: &glm::Vec4, p0: &glm::Vec4, p1: &glm::Vec4, p2: &glm::Vec4) -> (f32, f32, f32) {
        let v0 = p1 - p0;
        let v1 = p2 - p0; 
        let v2 = p - p0;
        let d00 = glm::dot(&v0.xy(), &v0.xy());
        let d01 = glm::dot(&v0.xy(), &v1.xy());
        let d11 = glm::dot(&v1.xy(), &v1.xy());
        let d20 = glm::dot(&v2.xy(), &v0.xy());
        let d21 = glm::dot(&v2.xy(), &v1.xy());
        let denom = d00 * d11 - d01 * d01;
        let f1 = (d11 * d20 - d01 * d21) / denom;
        let f2 = (d00 * d21 - d01 * d20) / denom;
        let f0 = 1.0 - f1 - f2;
        (f0, f1, f2)
    }

    fn transform_to_target_coordinates(&self, v: &glm::Vec4) -> glm::Vec4 {
        glm::vec4(
            (v.x + 1.0) * (self.target.size.0 as f32 / 2.0),
            (v.y + 1.0) * (self.target.size.1 as f32 / 2.0),
            v.z,
            v.w
        )
    }

}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window("test_rasterization", 1024, 768)
        .position_centered()
        .build()
        .unwrap();

    let window_size = window.size();
    let mut texture_buffer = TextureBuffer::new(window_size, 4);
    
    let mut angle = 0.0;
    let camera = Camera::new(
        window_size.0 as f32 / window_size.1 as f32,
        std::f32::consts::PI / 4.0,
        0.1,
        100.0
    );
 
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let cube_vertices = [
        Vertex { position: glm::vec3(-1.0, -1.0, 1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, 1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3( 1.0,  1.0, 1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, 1.0), uv: glm::vec2(0.0, 1.0) },
        
        Vertex { position: glm::vec3(1.0,  1.0,  1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3(1.0,  1.0, -1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3(1.0, -1.0, -1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3(1.0, -1.0,  1.0), uv: glm::vec2(0.0, 1.0) },
    
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, -1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3( 1.0,  1.0, -1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, -1.0), uv: glm::vec2(0.0, 1.0) },
    
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3(-1.0, -1.0,  1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3(-1.0,  1.0,  1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, -1.0), uv: glm::vec2(0.0, 1.0) },
    
        Vertex { position: glm::vec3( 1.0, 1.0,  1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3(-1.0, 1.0,  1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3(-1.0, 1.0, -1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3( 1.0, 1.0, -1.0), uv: glm::vec2(0.0, 1.0) },
        
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0), uv: glm::vec2(0.0, 0.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, -1.0), uv: glm::vec2(1.0, 0.0) },
        Vertex { position: glm::vec3( 1.0, -1.0,  1.0), uv: glm::vec2(1.0, 1.0) },
        Vertex { position: glm::vec3(-1.0, -1.0,  1.0), uv: glm::vec2(0.0, 1.0) }
    ];

    let cube_indices = [
        0,  2,  1,  0,  3,  2,
        4,  5,  6,  4,  6,  7,
        8,  9,  10, 8,  10, 11, 
        12, 14, 13, 12, 15, 14, 
        16, 17, 18, 16, 18, 19, 
        20, 22, 21, 20, 23, 22 
    ];

    let mut fps_counter = FpsCounter::new();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {   
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        texture_buffer.clear(0);

        angle += 0.01;
        let model = glm::translation(&glm::vec3(0.0, 0.0, 5.0)) * 
            glm::rotation(angle, &glm::vec3(0.0, 1.0, 0.0));
        let mvp = camera.projection * camera.view * model;
        let mut render_context = RenderContext::new(
            true,
            &mut texture_buffer, 
            |v: &mut Vertex| {
                let p = v.position;
                mvp * glm::vec4(p.x, p.y, p.z, 1.0)
            },
            |v: &Vertex| {
                [0, (v.uv.y * 255.0) as u8, (v.uv.x * 255.0) as u8, 255]
            }
        );
        render_context.draw_indexed_triangles(&cube_indices, &cube_vertices);

        let texture_creator = canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_target(texture_creator.default_pixel_format(),
                 window_size.0,
                 window_size.1)
            .unwrap();
        texture.update(None, &texture_buffer.buffer, 
            texture_buffer.pitch()).unwrap();

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        if let Some(fps) = fps_counter.update() {
            println!("Fps: {}", fps);
        }
    }
}