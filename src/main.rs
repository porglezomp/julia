#![feature(box_patterns)]
#![feature(std_misc)]

extern crate sdl2;

use sdl2::video::{WindowPos, Window, OPENGL};
use sdl2::event::{Event, poll_event, wait_event};
use sdl2::rect::Rect;
use sdl2::keycode::KeyCode;
use std::sync::mpsc::{channel, Sender};
use std::thread::Thread;
use std::cmp::{min, max};

use batches::{Batch, screen_rects, BATCH_WIDTH, BATCH_HEIGHT};

mod batches;

const WIDTH : i32 = 1280;
const HEIGHT : i32 = 720;
const ASPECT : f32 = WIDTH as f32 / HEIGHT as f32;

fn main() {
    sdl2::init(sdl2::INIT_VIDEO);

    let window = Window::new("Julia",
                             WindowPos::PosCentered,
                             WindowPos::PosCentered,
                             WIDTH, HEIGHT, OPENGL)
                    .unwrap();

    let renderer = sdl2::render::Renderer::from_window(window,
                                                       sdl2::render::RenderDriverIndex::Auto,
                                                       sdl2::render::ACCELERATED)
                    .unwrap();

    let mut texture = renderer.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24,
                                                    (WIDTH, HEIGHT))
                    .unwrap();

    let mut drawer = renderer.drawer();

    let (sender, receiver) = channel();

    let mut data = Data { x: 0.0, y: 0.0, n: 12 };

    'all: loop {
        let mut num_rects = 0;
        for rect in screen_rects(WIDTH, HEIGHT) {
            num_rects += 1;
            let dat = data.clone();
            let send = sender.clone();
            Thread::spawn(move || {
                calc_batch(rect, send, dat);
            });
        }

        for _ in 0..num_rects {
            let box x = receiver.recv().unwrap();
            match texture.update(Some(x.rect), &x.pixels, batches::BATCH_WIDTH*3) {
                Ok(())   => (),
                Err(msg) => panic!("Error updating texture: {}", msg)
            }
        }

        drawer.copy(&texture, None, None);
        drawer.present();

        let mut ready_to_break = false;
        let mut next_event = None;
        'events: loop {
            // Allow the Event::None handler to insert a different next event
            // This lets us do a sleeping wait instead of a busy wait
            let event = match next_event {
                None    => poll_event(),
                Some(e) => e,
            };

            match event {
                Event::Quit{..} => break 'all,
                Event::MouseMotion{x, y, ..} => {
                    let (newx, newy) = map_pixel(x, y);
                    data.x = newx;
                    data.y = newy;
                    ready_to_break = true;
                }
                Event::KeyDown{keycode: KeyCode::Up, ..} => {
                    data.n = min(data.n + 1, 255);
                    ready_to_break = true;
                }
                Event::KeyDown{keycode: KeyCode::Down, ..} => {
                    data.n = max(data.n - 1, 1);
                    ready_to_break = true;
                }
                Event::None => {
                    if ready_to_break {
                        break 'events;
                    }
                    // Insert a new event for the event handler to use instead of polling
                    // This does a sleeping wait instead of a busy wait
                    next_event = Some(wait_event().unwrap());
                    continue 'events;
                }
                _ => (),
            }
            next_event = None;
        }
    }

    sdl2::quit();
}

#[derive(Clone)]
struct Data {
    x: f32,
    y: f32,
    n: u8,
}

const SCALE : f32 = 1.25;
fn map_pixel(x: i32, y: i32) -> (f32, f32) {
    let newx = (x as f32 / (WIDTH as f32 / 2.0) - 1.0) * SCALE * ASPECT;
    let newy = (y as f32 / (HEIGHT as f32 / 2.0) - 1.0) * SCALE;
    (newx, newy)
}

fn calc_batch(rect: Rect, sender: Sender<Box<Batch>>, data: Data) {
    let mut p = [0u8; (BATCH_WIDTH * BATCH_HEIGHT * 3) as usize];
    for y in 0..rect.h {
        for x in 0..rect.w {
            let index = (x*3 + y*BATCH_WIDTH*3) as usize;
            let (newx, newy) = map_pixel(x + rect.x, y + rect.y);
            p[index] = julia(newx, newy, &data);
        }
    }
    match sender.send(Box::new(Batch { rect: rect,
                                       pixels: p })) {
        Ok(_)  => (),
        Err(x) => panic!("send error {}", x),
    }
}

fn julia(mut a: f32, mut b: f32, data: &Data) -> u8 {
    let mut i = 0u8;
    for _ in 0..data.n {
        i += (255 / data.n) as u8;
        // a + bi
        // (a + bi)^2 = a*a + 2*a*bi - b*b
        let c = 2.0*a*b;
        a = a*a - b*b;
        b = c;

        // z_n = z_{n-1}^2 + c
        a += data.x;
        b += data.y;
        if (a*a + b*b) > 4.0 { return i; }
    }
    0
}