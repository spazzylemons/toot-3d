use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::Cursor,
    sync::{Arc, Mutex},
};

use crate::net::retriever::{Method, Request, Retriever};

use super::{
    citro2d::{Image, RGBA8},
    LogicImgPool, OpaqueImg,
};

fn convert_image(
    pool: &LogicImgPool,
    buffer: &[u8],
    max_scale: Option<u16>,
) -> Result<(u16, u16, OpaqueImg), Box<dyn Error + Send + Sync>> {
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
                    unsafe {
                        let color = u32::from_be_bytes(pixels.next().unwrap_unchecked().0);
                        tex.set_unchecked(x, y, color);
                    }
                }
            }
        })
    });

    Ok((width, height, result))
}

pub struct WebImage {
    pub width: u16,
    pub height: u16,
    pub image: Mutex<OpaqueImg>,
    url: String,
}

/// A cached image.
pub struct CachedImage {
    /// The image.
    image: Arc<WebImage>,
    /// The cache.
    cache: Arc<WebImageCache>,
}

impl CachedImage {
    pub fn image(&self) -> &Arc<WebImage> {
        &self.image
    }
}

impl Drop for CachedImage {
    fn drop(&mut self) {
        // check refcount of image
        if Arc::strong_count(&self.image) == 2 {
            // two references means that the only references to this image are
            // 1. this CachedImage, and
            // 2. the entry in WebImageCache.
            // so, we should tell the cache to remove the entry, as no one else
            // will be pointing to it after we're dropped.
            self.cache.remove(&self.image.url);
        }
    }
}

/// Caches images from the web.
pub struct WebImageCache {
    /// Contains references to all web images in use. Wrapped to allow interior mutability.
    entries: Mutex<HashMap<String, Arc<WebImage>>>,
}

impl WebImageCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(
        self: &Arc<Self>,
        retriever: &Retriever,
        pool: &LogicImgPool,
        images: &[(&str, Option<u16>)],
    ) -> Result<Vec<CachedImage>, Box<dyn Error + Send + Sync>> {
        let mut requests = vec![];
        let mut request_info = vec![];
        let mut added_requests = HashSet::new();
        let mut entries = self.entries.lock().unwrap();
        for (url, max_scale) in images {
            // ensure each entry exists
            if !entries.contains_key(*url) && !added_requests.contains(*url) {
                let url_string = String::from(*url);
                requests.push(Request {
                    method: Method::Get,
                    url: url_string.clone(),
                });
                added_requests.insert(url_string);
                request_info.push((url, max_scale));
            }
        }
        let responses = retriever.request(requests);
        for (url, max_scale) in request_info {
            let response = responses.recv().unwrap()?;
            // add image
            let (width, height, image) = convert_image(pool, &response, *max_scale)?;
            let image = Arc::new(WebImage {
                width,
                height,
                image: Mutex::new(image),
                url: String::from(*url),
            });
            // store in cache
            entries.insert(String::from(*url), image);
        }
        // build result from reading cache
        let mut result = vec![];
        for (url, _) in images {
            let image = entries.get(*url).unwrap();
            // create cached image struct from this
            result.push(CachedImage {
                image: image.clone(),
                cache: self.clone(),
            });
        }
        Ok(result)
    }

    fn remove(&self, url: &str) {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(url);
    }
}
