use sdl2::rect::Rect;
use std::cmp::min;

pub const BATCH_WIDTH: usize = 128;
pub const BATCH_HEIGHT: usize = 128;

pub struct Batch {
    pub rect: Rect,
    pub pixels: [u8; (BATCH_WIDTH * BATCH_HEIGHT * 3) as usize],
}

#[derive(Debug, Clone)]
pub struct RectGenerator {
    screen_width: i32,
    screen_height: i32,
    x: i32,
    y: i32,
}

pub fn screen_rects(width: u32, height: u32) -> RectGenerator {
    RectGenerator {
        screen_width: width as i32,
        screen_height: height as i32,
        x: 0,
        y: 0,
    }
}

impl Iterator for RectGenerator {
    fn next(&mut self) -> Option<Rect> {
        if self.y >= self.screen_height {
            return None;
        }

        let width = min(BATCH_WIDTH as i32, self.screen_width - self.x) as u32;
        let height = min(BATCH_HEIGHT as i32, self.screen_height - self.y) as u32;
        let res = Rect::new_unwrap(self.x, self.y, width, height);

        self.x += BATCH_WIDTH as i32;
        if self.x >= self.screen_width {
            self.x = 0;
            self.y += BATCH_HEIGHT as i32;
        }

        Some(res)
    }

    type Item = Rect;
}
