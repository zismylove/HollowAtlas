use image::{Rgba, RgbaImage};

#[derive(Debug, Clone)]
pub struct TrimResult {
    pub image: RgbaImage,
    pub trim_x: u32,
    pub trim_y: u32,
    pub trim_width: u32,
    pub trim_height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub trimmed: bool,
    pub fully_transparent: bool,
}

pub fn trim_transparent(image: &RgbaImage, enabled: bool) -> TrimResult {
    let source_width = image.width();
    let source_height = image.height();

    if !enabled {
        return TrimResult {
            image: image.clone(),
            trim_x: 0,
            trim_y: 0,
            trim_width: source_width,
            trim_height: source_height,
            source_width,
            source_height,
            trimmed: false,
            fully_transparent: false,
        };
    }

    let mut min_x = source_width;
    let mut min_y = source_height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 0 {
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !found {
        return TrimResult {
            image: RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])),
            trim_x: 0,
            trim_y: 0,
            trim_width: 1,
            trim_height: 1,
            source_width,
            source_height,
            trimmed: true,
            fully_transparent: true,
        };
    }

    let trim_width = max_x - min_x + 1;
    let trim_height = max_y - min_y + 1;
    let mut cropped = RgbaImage::new(trim_width, trim_height);
    for y in 0..trim_height {
        for x in 0..trim_width {
            let pixel = *image.get_pixel(min_x + x, min_y + y);
            cropped.put_pixel(x, y, pixel);
        }
    }

    TrimResult {
        image: cropped,
        trim_x: min_x,
        trim_y: min_y,
        trim_width,
        trim_height,
        source_width,
        source_height,
        trimmed: min_x != 0
            || min_y != 0
            || trim_width != source_width
            || trim_height != source_height,
        fully_transparent: false,
    }
}
