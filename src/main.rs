extern crate sdl2;
extern crate image;

use sdl2::render::Renderer;
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::cmp::{min, max};
use std::io::Write;

use batches::{Batch, screen_rects, BATCH_WIDTH, BATCH_HEIGHT};

mod batches;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const ASPECT: f32 = WIDTH as f32 / HEIGHT as f32;

fn main() {
    let ctx = sdl2::init().unwrap();
    let video_ctx = ctx.video().unwrap();
    let mut event_pump = ctx.event_pump().unwrap();

    let window =
        match video_ctx.window("Julia", WIDTH, HEIGHT).position_centered().opengl().build() {
            Ok(window) => window,
            Err(error) => panic!("Failed to create window: {}", error),
        };

    let mut renderer = match window.renderer().build() {
        Ok(renderer) => renderer,
        Err(error) => panic!("Failed to create renderer: {}", error),
    };

    let mut texture =
        renderer.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, (WIDTH, HEIGHT))
            .unwrap();

    let (sender, receiver) = channel();

    let mut data = Data {
        x: 0.4,
        y: -0.4,
        n: 4,
        xoff: 0.002,
        yoff: 0.002,
    };

    let mut image_id = 0;

    set_title(&mut renderer, &data);

    'all: loop {
        let mut num_rects = 0;
        for rect in screen_rects(WIDTH, HEIGHT) {
            num_rects += 1;
            let dat = data.clone();
            let send = sender.clone();
            let _ = thread::Builder::new().name("Worker".into()).spawn(move || {
                calc_batch(rect, send, dat);
            });
        }

        // data.xoff *= -0.8;
        // if n % 8 == 0 {
        //     data.yoff *= -0.8;
        //     data.xoff = 0.002;
        // }
        for _ in 0..num_rects {
            let x = *receiver.recv().unwrap();
            match texture.update(Some(x.rect), &x.pixels, batches::BATCH_WIDTH * 3) {
                Ok(())   => (),
                Err(msg) => panic!("Error updating texture: {}", msg),
            }
        }

        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        texture.set_alpha_mod(255);
        renderer.copy(&texture, None, None);
        renderer.present();
        // Copy the texture into both buffers
        renderer.copy(&texture, None, None);

        let mut ready_to_break = true;
        let mut next_event: Option<Event> = None;
        'events: loop {
            // Allow the Event::None handler to insert a different next event
            // This lets us do a sleeping wait instead of a busy wait
            let event = next_event.or_else(|| event_pump.poll_event());

            match event {
                Some(Event::Quit { .. }) => break 'all,
                Some(Event::MouseMotion { x, y, .. }) => {
                    let (newx, newy) = map_pixel(x, y);
                    data.x = newx;
                    data.y = newy;
                    ready_to_break = true;
                }
                Some(Event::KeyDown { keycode: Some(Keycode::Up), .. }) => {
                    data.n = min(data.n as u32 + 1, 255) as u8;
                    set_title(&mut renderer, &data);
                    ready_to_break = true;
                }
                Some(Event::KeyDown { keycode: Some(Keycode::Down), .. }) => {
                    data.n = max(data.n - 1, 1);
                    set_title(&mut renderer, &data);
                    ready_to_break = true;
                }
                Some(Event::KeyDown { keycode: Some(Keycode::Space), .. }) => {
                    let dat = data.clone();
                    let _ = thread::Builder::new().name("Save Image".into()).spawn(move || {
                        let fname = format!("image{:04}.png", image_id);
                        if let Err(e) = render_to_image(&fname, dat) {
                            let _ = writeln!(std::io::stderr(), "Error saving image {}: {:?}", fname, e);
                        }
                    });
                    image_id += 1;
                }
                None => {
                    if ready_to_break {
                        break 'events;
                    }
                    next_event = Some(event_pump.wait_event());
                    continue 'events;
                }
                _ => (),
            }
            next_event = None;
        }
    }
}

fn set_title(renderer: &mut Renderer, data: &Data) {
    renderer.window_mut()
        .unwrap()
        .set_title(&format!("Julia ({} iterations)", data.n));
}

#[derive(Clone)]
struct Data {
    x: f32,
    y: f32,
    xoff: f32,
    yoff: f32,
    n: u8,
}

const SCALE: f32 = 1.25;
fn map_pixel(x: i32, y: i32) -> (f32, f32) {
    let newx = (x as f32 / (WIDTH as f32 / 2.0) - 1.0) * SCALE * ASPECT;
    let newy = (y as f32 / (HEIGHT as f32 / 2.0) - 1.0) * SCALE;
    (newx, newy)
}

fn calc_batch(rect: Rect, sender: Sender<Box<Batch>>, data: Data) {
    let mut p = [0u8; (BATCH_WIDTH * BATCH_HEIGHT * 3) as usize];
    for y in 0..rect.height() as i32 {
        for x in 0..rect.width() as i32 {
            let index = (x as usize * 3 + y as usize * BATCH_WIDTH * 3) as usize;
            let (newx, newy) = map_pixel(x + rect.x(), y + rect.y());
            let (r, g, b) = color(julia(newx + data.xoff, newy + data.yoff, &data));
            p[index] = r;
            p[index + 1] = g;
            p[index + 2] = b;
        }
    }

    match sender.send(Box::new(Batch {
        rect: rect,
        pixels: p,
    })) {
        Ok(_) => (),
        Err(x) => panic!("send error {}", x),
    }
}

fn color(x: u8) -> (u8, u8, u8) {
    if x == 255 { (0, 0, 0) } else { (255, x, x / 2) }
}

fn cmpsqr(a: f32, b: f32) -> (f32, f32) {
    let real = a * a - b * b;
    let imag = a * b * 2.0;
    (real, imag)
}

fn julia(mut a: f32, mut b: f32, data: &Data) -> u8 {
    let mut i = 0u8;
    for _ in 0..data.n {
        // a + bi
        // (a + bi)^2 = a*a + 2*a*bi - b*b
        let (x, y) = cmpsqr(a, b);
        a = x + data.x;
        b = y + data.y;
        if (a * a + b * b) > 4.0 {
            return i;
        }
        i += (255 / data.n) as u8;
    }
    255
}

fn render_to_image(fname: &str, data: Data) -> std::io::Result<()>{
    const HEIGHT: usize = 4096;
    const WIDTH: usize = HEIGHT * 110 / 85;
    const ASPECT: f32 = WIDTH as f32 / HEIGHT as f32;
    fn map_pixel(x: usize, y: usize) -> (f32, f32) {
        let newx = (x as f32 / (WIDTH as f32 / 2.0) - 1.0) * SCALE * ASPECT;
        let newy = (y as f32 / (HEIGHT as f32 / 2.0) - 1.0) * SCALE;
        (newx, newy)
    }

    let mut buffer = vec![0; WIDTH * HEIGHT * 3];
    for (y, row) in buffer.chunks_mut(WIDTH * 3).enumerate() {
        for (x, pixel) in row.chunks_mut(3).enumerate() {
            let (real, imag) = map_pixel(x, y);
            let (r, g, b) = color(julia(real, imag, &data));
            pixel[0] = r; pixel[1] = g; pixel[2] = b;
        }
    }
    image::save_buffer(fname, &buffer, WIDTH as u32, HEIGHT as u32,
                       image::ColorType::RGB(8))
}
