use sdl2::rect::Rect;
use std::cmp::min;

pub const BATCH_WIDTH : i32 = 128;
pub const BATCH_HEIGHT : i32 = 128;

pub struct Batch {
    pub rect: Rect,
    pub pixels: [u8; (BATCH_WIDTH*BATCH_HEIGHT*3) as usize],
}

#[derive(Debug, Clone)]
struct RectGenerator {
    screen_width: i32,
    screen_height: i32,
    x : i32,
    y : i32,
}

pub fn screen_rects(width: i32, height: i32) -> RectGenerator {
    RectGenerator {
        screen_width: width,
        screen_height: height,
        x: 0,
        y: 0,
    }
}

impl Iterator for RectGenerator {
    fn next(&mut self) -> Option<Rect> {
        if self.y >= self.screen_height {
            return None
        }

        let width = min(BATCH_WIDTH, self.screen_width - self.x);
        let height = min(BATCH_HEIGHT, self.screen_height - self.y);
        let res = Rect::new(self.x, self.y, width, height);
        
        self.x += BATCH_WIDTH;
        if self.x >= self.screen_width {
            self.x = 0;
            self.y += BATCH_HEIGHT;
        }
        
        Some(res)
    }

    type Item = Rect;
}