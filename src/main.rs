use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::marker::PhantomData;

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

    fn set(&mut self, point: (u32, u32), color: &[u8; 3]) {
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

#[derive(Clone)]
struct Vertex {
    position: glm::Vec3
}

struct RenderContext<'a, V: Clone, VS: Fn(&mut V) -> glm::Vec4> {        
    draw_buffer: &'a mut TextureBuffer,
    vertex_shader: VS,
    phantom: PhantomData<V>
}

impl<'a, V: Clone, VS: Fn(&mut V) -> glm::Vec4> RenderContext<'a, V, VS> {
    fn new(draw_buffer: &'a mut TextureBuffer, vertex_shader: VS) -> Self {
        RenderContext {
            draw_buffer: draw_buffer,
            vertex_shader,
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
            if let [i1, i2, i3, rest @ ..] = current_indices {
                let mut p1 = positions[*i1];
                let mut p2 = positions[*i2];
                let mut p3 = positions[*i3];
                let v1 = &vertices[*i1];
                let v2 = &vertices[*i2];
                let v3 = &vertices[*i3];
                p1 /= p1.w;
                p2 /= p2.w;
                p3 /= p3.w;
                self.draw_triangle(
                    &self.transform_to_window_coordinates(&p1.xyz()), 
                    &self.transform_to_window_coordinates(&p2.xyz()), 
                    &self.transform_to_window_coordinates(&p3.xyz()), 
                    v1, v2, v3
                );
                current_indices = rest;
            } else {
                break;
            }
        }
    }
    
    fn draw_triangle(&mut self, 
        p1: &glm::Vec3, p2: &glm::Vec3, p3: &glm::Vec3,
        _v1: &V, _v2: &V, _v3: &V) {
        let mut p1 = p1;
        let mut p2 = p2;
        let mut p3 = p3;

        if p2.y < p1.y {
            std::mem::swap(&mut p1, &mut p2);
        }
        if p3.y < p2.y {
            std::mem::swap(&mut p2, &mut p3);
        }
        if p2.y < p1.y {
            std::mem::swap(&mut p1, &mut p2);
        }

        //natural flat top
        if p1.y == p2.y { 
            if p2.x < p1.x {
                std::mem::swap(&mut p1, &mut p2);
            }
            self.draw_flat_top_triangle(p1, p2, p3);
        }
        //natural flat bottom
        else if p2.y == p3.y {
            if p3.x < p2.x {
                std::mem::swap(&mut p2, &mut p3);
            }
            self.draw_flat_bottom_triangle(p1, p2, p3);
        }
        //general triangle
        else {
            let alpha = (p2.y - p1.y) / (p3.y - p1.y);
            let pi = p1 + (p3 - p1) * alpha;
            //major right
            if p2.x < pi.x {
                self.draw_flat_bottom_triangle(p1, p2, &pi);
                self.draw_flat_top_triangle(p2, &pi, p3);
            }
            //major left
            else {
                self.draw_flat_bottom_triangle(p1, &pi, p2);
                self.draw_flat_top_triangle(&pi, p2, p3);
            }
        }
    }

    fn draw_flat_top_triangle(&mut self, v1: &glm::Vec3, v2: &glm::Vec3, v3: &glm::Vec3) {
        let slope1 = (v3.x - v1.x) / (v3.y - v1.y);
        let slope2 = (v3.x - v2.x) / (v3.y - v2.y);

        let y_start = (v1.y - 0.5).ceil() as i32;
        let y_end = (v3.y - 0.5).ceil() as i32;
        
        for y in y_start..y_end {
            let x_start = (slope1 * (y as f32 + 0.5 - v1.y) + v1.x - 0.5).ceil() as i32;
            let x_end = (slope2 * (y as f32 + 0.5 - v2.y) + v2.x - 0.5).ceil() as i32;
            for x in x_start..x_end {
                self.pixel_shader((x, y));
            }
        }
    }

    fn draw_flat_bottom_triangle(&mut self, v1: &glm::Vec3, v2: &glm::Vec3, v3: &glm::Vec3) {
        let slope1 = (v2.x - v1.x) / (v2.y - v1.y);
        let slope2 = (v3.x - v1.x) / (v3.y - v1.y);
        
        let y_start = (v1.y - 0.5).ceil() as i32;
        let y_end = (v3.y - 0.5).ceil() as i32;
        
        for y in y_start..y_end {
            let x_start = (slope1 * (y as f32 + 0.5 - v1.y) + v1.x - 0.5).ceil() as i32;
            let x_end = (slope2 * (y as f32 + 0.5 - v1.y) + v1.x - 0.5).ceil() as i32;
            for x in x_start..x_end {
                self.pixel_shader((x, y));
            }
        }
    }

    fn pixel_shader(&mut self, point: (i32, i32)) {
        if point.0 >= 0 && point.1 >= 0 {
            let point = (point.0 as u32, point.1 as u32);
            if point.0 < self.draw_buffer.size.0 && point.1 < self.draw_buffer.size.1 {
                self.draw_buffer.set(point, &[255, 255, 255]);
            }
        }
    }

    fn transform_to_window_coordinates(&self, v: &glm::Vec3) -> glm::Vec3 {
        glm::vec3(
            (v.x + 1.0) * (self.draw_buffer.size.0 as f32 / 2.0),
            (v.y + 1.0) * (self.draw_buffer.size.1 as f32 / 2.0),
            v.z
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
        Vertex { position: glm::vec3(-1.0, -1.0, 1.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, 1.0) },
        Vertex { position: glm::vec3( 1.0,  1.0, 1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, 1.0) },
        
        Vertex { position: glm::vec3(1.0,  1.0,  1.0) },
        Vertex { position: glm::vec3(1.0,  1.0, -1.0) },
        Vertex { position: glm::vec3(1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3(1.0, -1.0,  1.0) },
    
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3( 1.0,  1.0, -1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, -1.0) },
    
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3(-1.0, -1.0,  1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0,  1.0) },
        Vertex { position: glm::vec3(-1.0,  1.0, -1.0) },
    
        Vertex { position: glm::vec3( 1.0, 1.0,  1.0) },
        Vertex { position: glm::vec3(-1.0, 1.0,  1.0) },
        Vertex { position: glm::vec3(-1.0, 1.0, -1.0) },
        Vertex { position: glm::vec3( 1.0, 1.0, -1.0) },
        
        Vertex { position: glm::vec3(-1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3( 1.0, -1.0, -1.0) },
        Vertex { position: glm::vec3( 1.0, -1.0,  1.0) },
        Vertex { position: glm::vec3(-1.0, -1.0,  1.0) }
    ];

    let cube_indices = [
        0,  1,  2,  0,  2,  3,
        4,  5,  6,  4,  6,  7,
        8,  9,  10, 8,  10, 11, 
        12, 13, 14, 12, 14, 15, 
        16, 17, 18, 16, 18, 19, 
        20, 21, 22, 20, 22, 23 as usize 
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

        angle += 0.006;
        let model = glm::translation(&glm::vec3(0.0, 0.0, 5.0)) * 
            glm::rotation(angle, &glm::vec3(0.0, 1.0, 0.0));
        let mvp = camera.projection * camera.view * model;
        let mut render_context = RenderContext::new(
            &mut texture_buffer, 
            |v: &mut Vertex| {
                let p = v.position;
                mvp * glm::vec4(p.x, p.y, p.z, 1.0)
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