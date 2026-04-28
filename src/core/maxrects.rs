use crate::core::types::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertResult {
    pub index: usize,
    pub rect: Rect,
    pub rotated: bool,
}

#[derive(Debug, Clone)]
pub struct MaxRectsPacker {
    pub width: u32,
    pub height: u32,
    pub free_rects: Vec<Rect>,
    pub used_rects: Vec<Rect>,
}

impl MaxRectsPacker {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Atlas dimensions must be positive");
        Self {
            width,
            height,
            free_rects: vec![Rect {
                x: 0,
                y: 0,
                w: width,
                h: height,
            }],
            used_rects: Vec::new(),
        }
    }

    pub fn insert(
        &mut self,
        index: usize,
        width: u32,
        height: u32,
        allow_rotation: bool,
    ) -> Option<InsertResult> {
        let (rect, rotated) = self.find_best_position(width, height, allow_rotation)?;
        self.place(rect);
        Some(InsertResult {
            index,
            rect,
            rotated,
        })
    }

    fn find_best_position(
        &self,
        width: u32,
        height: u32,
        allow_rotation: bool,
    ) -> Option<(Rect, bool)> {
        let mut best_rect = None;
        let mut best_rotated = false;
        let mut best_area_score = u64::MAX;
        let mut best_short_side = u32::MAX;

        for free in &self.free_rects {
            let candidates = if allow_rotation && width != height {
                vec![(width, height, false), (height, width, true)]
            } else {
                vec![(width, height, false)]
            };

            for (candidate_w, candidate_h, rotated) in candidates {
                if candidate_w > free.w || candidate_h > free.h {
                    continue;
                }

                let leftover_area = free.area() - candidate_w as u64 * candidate_h as u64;
                let short_side = (free.w - candidate_w).min(free.h - candidate_h);
                if leftover_area < best_area_score
                    || (leftover_area == best_area_score && short_side < best_short_side)
                {
                    best_rect = Some(Rect {
                        x: free.x,
                        y: free.y,
                        w: candidate_w,
                        h: candidate_h,
                    });
                    best_rotated = rotated;
                    best_area_score = leftover_area;
                    best_short_side = short_side;
                }
            }
        }

        best_rect.map(|rect| (rect, best_rotated))
    }

    fn place(&mut self, rect: Rect) {
        let mut next_free = Vec::new();
        for free in &self.free_rects {
            if !free.intersects(rect) {
                next_free.push(*free);
                continue;
            }
            next_free.extend(split_free_rect(*free, rect));
        }

        self.free_rects = prune_free_rects(&next_free);
        self.used_rects.push(rect);
    }
}

pub fn split_free_rect(free: Rect, used: Rect) -> Vec<Rect> {
    if !free.intersects(used) {
        return vec![free];
    }

    let mut result = Vec::new();

    if used.x < free.right() && used.right() > free.x {
        if used.y > free.y && used.y < free.bottom() {
            result.push(Rect {
                x: free.x,
                y: free.y,
                w: free.w,
                h: used.y - free.y,
            });
        }
        if used.bottom() < free.bottom() {
            result.push(Rect {
                x: free.x,
                y: used.bottom(),
                w: free.w,
                h: free.bottom() - used.bottom(),
            });
        }
    }

    if used.y < free.bottom() && used.bottom() > free.y {
        if used.x > free.x && used.x < free.right() {
            result.push(Rect {
                x: free.x,
                y: free.y,
                w: used.x - free.x,
                h: free.h,
            });
        }
        if used.right() < free.right() {
            result.push(Rect {
                x: used.right(),
                y: free.y,
                w: free.right() - used.right(),
                h: free.h,
            });
        }
    }

    result
        .into_iter()
        .filter(|rect| rect.w > 0 && rect.h > 0)
        .collect()
}

pub fn prune_free_rects(rects: &[Rect]) -> Vec<Rect> {
    let mut pruned = Vec::new();

    'outer: for (i, rect) in rects.iter().enumerate() {
        for (j, other) in rects.iter().enumerate() {
            if i != j && other.contains(*rect) {
                continue 'outer;
            }
        }
        if !pruned.contains(rect) {
            pruned.push(*rect);
        }
    }

    pruned
}
