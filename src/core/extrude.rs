use image::{Rgba, RgbaImage};

pub fn extrude_image(image: &RgbaImage, amount: u32) -> RgbaImage {
    if amount == 0 {
        return image.clone();
    }

    let width = image.width();
    let height = image.height();
    let out_width = width + amount * 2;
    let out_height = height + amount * 2;
    let mut out = RgbaImage::from_pixel(out_width, out_height, Rgba([0, 0, 0, 0]));

    for y in 0..out_height {
        for x in 0..out_width {
            let source_x = x.saturating_sub(amount).min(width - 1);
            let source_y = y.saturating_sub(amount).min(height - 1);
            out.put_pixel(x, y, *image.get_pixel(source_x, source_y));
        }
    }

    out
}
