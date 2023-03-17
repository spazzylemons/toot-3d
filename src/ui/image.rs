use std::{error::Error, io::Cursor};

use crate::net::Client;

use super::{
    citro2d::{Image, RGBA8},
    LogicImgPool, OpaqueImg,
};

pub fn download_image(
    client: &Client,
    pool: &LogicImgPool,
    url: &str,
    max_scale: Option<u16>,
) -> Result<(u16, u16, OpaqueImg), Box<dyn Error>> {
    // download the image from the web
    // TODO would be nice if we could stream instead of buffering the entire image
    let buffer = client.get(url)?;
    let mut reader = image::io::Reader::new(Cursor::new(&buffer));

    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(1024);
    limits.max_image_height = Some(1024);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let mut img = reader.with_guessed_format()?.decode()?;
    // if custom scale requested, use that
    if let Some(max_scale) = max_scale {
        let max_scale = u32::from(max_scale);
        if img.width() > max_scale || img.height() > max_scale {
            img = img.resize(max_scale, max_scale, image::imageops::FilterType::Triangle);
        }
    }
    let img = img.to_rgba8();

    let width = img.width() as u16;
    let height = img.height() as u16;
    let result = pool.alloc(move |c2d| {
        // TODO don't use RGBA8 if not necessary -
        // use rgb565 if there's no alpha, for instance
        Image::build::<RGBA8, _>(c2d, width, height, |tex| {
            let mut pixels = img.pixels();
            for y in 0..height {
                for x in 0..width {
                    let color = u32::from_be_bytes(pixels.next().unwrap().0);
                    unsafe {
                        tex.set_unchecked(x, y, color);
                    }
                }
            }
        })
    });

    Ok((width, height, result))
}
