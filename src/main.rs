use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::WindowCanvas;

extern crate nalgebra_glm as glm;
 
struct RenderContext<'a> {
    canvas: &'a mut WindowCanvas
}

impl<'a> RenderContext<'a> {
    fn new(canvas: &'a mut WindowCanvas) -> Self {
        RenderContext {
            canvas: canvas
        }
    }

    fn draw_indexed_triangles(&mut self, indices: &[usize], vertices: &[glm::Vec3]) {
        let mut current_indices = indices;
        loop {
            if let [i1, i2, i3, rest @ ..] = current_indices {
                self.draw_triangle(&vertices[*i1], &vertices[*i2], &vertices[*i3]);
                current_indices = rest;
            } else {
                break;
            }
        }
    }
    
    fn draw_triangle(&mut self, p1: &glm::Vec3, p2: &glm::Vec3, p3: &glm::Vec3) {
        let rotation = glm::rotation(std::f32::consts::PI / 4.0,
             &glm::vec3(0.0, 1.0, 0.0));
        let translation = glm::translation(&glm::vec3(0.0, 0.0, 5.0));
        let model = translation * rotation;
        let perspective = glm::perspective(
            4.0 / 3.0, 
            std::f32::consts::PI / 4.0,
            0.1,
            100.0
        );
        let mut p1 = (perspective * model) * glm::vec4(p1.x, p1.y, p1.z, 1.0);
        let mut p2 = (perspective * model) * glm::vec4(p2.x, p2.y, p2.z, 1.0);
        let mut p3 = (perspective * model) * glm::vec4(p3.x, p3.y, p3.z, 1.0);
        p1 /= p1.w;
        p2 /= p2.w;
        p3 /= p3.w;
        let window_size = self.canvas.output_size().unwrap();
        let points = vec![p1, p2, p3, p1].
            iter().map(|p| {
                sdl2::rect::Point::new(
                ((p.x + 1.0) * (window_size.0 as f32 / 2.0)) as i32,
                ((p.y + 1.0) * (window_size.1 as f32 / 2.0)) as i32
                )
            }).collect::<Vec<_>>();
        self.canvas.draw_lines(&*points).unwrap();
    }

}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window("test_rasterization", 1024, 768)
        .position_centered()
        .build()
        .unwrap();
 
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let cube_vertices = [
        glm::vec3(-1.0, -1.0, 1.0),
        glm::vec3( 1.0, -1.0, 1.0),
        glm::vec3( 1.0,  1.0, 1.0),
        glm::vec3(-1.0,  1.0, 1.0),
        
        glm::vec3(1.0,  1.0,  1.0),
        glm::vec3(1.0,  1.0, -1.0),
        glm::vec3(1.0, -1.0, -1.0),
        glm::vec3(1.0, -1.0,  1.0),
    
        glm::vec3(-1.0, -1.0, -1.0),
        glm::vec3( 1.0, -1.0, -1.0),
        glm::vec3( 1.0,  1.0, -1.0),
        glm::vec3(-1.0,  1.0, -1.0),
    
        glm::vec3(-1.0, -1.0, -1.0),
        glm::vec3(-1.0, -1.0,  1.0),
        glm::vec3(-1.0,  1.0,  1.0),
        glm::vec3(-1.0,  1.0, -1.0),
    
        glm::vec3( 1.0, 1.0,  1.0),
        glm::vec3(-1.0, 1.0,  1.0),
        glm::vec3(-1.0, 1.0, -1.0),
        glm::vec3( 1.0, 1.0, -1.0),
        
        glm::vec3(-1.0, -1.0, -1.0),
        glm::vec3( 1.0, -1.0, -1.0),
        glm::vec3( 1.0, -1.0,  1.0),
        glm::vec3(-1.0, -1.0,  1.0)
    ];

    let cube_indices = [
        0,  1,  2,  0,  2,  3,
        4,  5,  6,  4,  6,  7,
        8,  9,  10, 8,  10, 11, 
        12, 13, 14, 12, 14, 15, 
        16, 17, 18, 16, 18, 19, 
        20, 21, 22, 20, 22, 23 as usize 
    ];

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

        canvas.set_draw_color((255, 0, 0));

        let mut render_context = RenderContext::new(&mut canvas);
        render_context.draw_indexed_triangles(&cube_indices, &cube_vertices);

        canvas.present();
    }
}