use std::error::Error;

use bit_set::BitSet;
use qrcode::{
    render::{Canvas, Pixel},
    QrCode,
};

use crate::ui::{
    citro2d::{color32, Image, Luminance4, RenderTarget, Scene2d},
    LogicImgPool, OpaqueImg, Screen, Ui,
};

#[derive(Clone, Copy)]
struct MyPixel(());

impl Pixel for MyPixel {
    type Canvas = MyCanvas;
    type Image = MyCanvas;

    fn default_unit_size() -> (u32, u32) {
        (1, 1)
    }

    fn default_color(_color: qrcode::Color) -> Self {
        Self(())
    }
}

struct MyCanvas {
    bits: BitSet,
    width: u32,
    height: u32,
}

impl Canvas for MyCanvas {
    type Pixel = MyPixel;
    type Image = Self;

    fn new(width: u32, height: u32, _dark_pixel: Self::Pixel, _light_pixel: Self::Pixel) -> Self {
        let size = (width * height) as usize;
        let bits = BitSet::with_capacity(size);
        Self {
            bits,
            width,
            height,
        }
    }

    fn draw_dark_pixel(&mut self, x: u32, y: u32) {
        let index = y * self.width + x;
        self.bits.insert(index as _);
    }

    fn into_image(self) -> Self::Image {
        self
    }
}

pub struct QrScreen {
    qr_code: OpaqueImg,
    width: u16,
    height: u16,
}

impl QrScreen {
    pub fn new(data: &[u8], pool: LogicImgPool) -> Result<Self, Box<dyn Error>> {
        let qr = QrCode::new(data)?;
        let image = qr.render::<MyPixel>().build();
        let width = image.width as u16;
        let height = image.height as u16;
        let qr_code = pool.alloc(move |c2d| {
            Image::build::<Luminance4, _>(c2d, width, height, |texture| {
                // no filtering, so the qr code is crisp
                texture.set_filter(false);
                let mut i = 0;
                for y in 0..height {
                    for x in 0..width {
                        // SAFETY: for loops keep us in range
                        unsafe {
                            texture.set_unchecked(
                                x,
                                y,
                                if image.bits.contains(i) { 0 } else { 15 },
                            );
                        }
                        i += 1;
                    }
                }
            })
        });
        Ok(Self {
            qr_code,
            width,
            height,
        })
    }
}

impl Screen for QrScreen {
    fn draw<'gfx: 'screen, 'screen>(
        &self,
        ui: &Ui<'gfx, 'screen>,
        target: &RenderTarget<'gfx, 'screen>,
        ctx: &Scene2d,
    ) {
        let x = 200.0 - f32::from(self.width);
        let y = 120.0 - f32::from(self.height);
        target.clear(color32(0, 0, 0, 255));
        ui.draw_opaque_img(&self.qr_code, ctx, x, y, 2.0, 2.0);
    }
}
