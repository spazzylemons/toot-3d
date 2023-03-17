use std::{error::Error, mem::MaybeUninit, num::NonZeroUsize, pin::Pin, rc::Rc};

use lru::LruCache;
use unicode_linebreak::{linebreaks, BreakOpportunity};

use super::citro2d::{AnyTexture, Citro2d, Image, Scene2d, TexDim};

struct Glyph<'gfx> {
    /// The glyph image.
    image: Image<'gfx>,
    /// The width of the glyph.
    x_advance: f32,
}

pub struct TextRenderer<'gfx> {
    /// pre-calculated sheets
    sheets: Vec<Pin<Rc<AnyTexture<'gfx>>>>,
    /// pre-calculated height
    height: u8,
    /// pre-calculated scale
    scale: f32,
    /// Cache of recently used glyphs.
    cache: LruCache<char, Glyph<'gfx>>,
}

fn get_shared_font() -> ctru::Result<&'static ctru_sys::CFNT_s> {
    extern "C" {
        static mut g_sharedFont: *mut ctru_sys::CFNT_s;
    }

    Ok(unsafe {
        if g_sharedFont.is_null() {
            ctru::error::ResultCode(ctru_sys::fontEnsureMapped())?;
        }

        &*g_sharedFont
    })
}

impl<'gfx> TextRenderer<'gfx> {
    pub fn new(c2d: &'gfx Citro2d) -> Result<Self, Box<dyn Error>> {
        let font = get_shared_font()?;

        let info = unsafe { &*font.finf.tglp };
        let height = info.cellHeight;
        // based on c2d code
        let scale = 30.0 / f32::from(height);
        let mut sheets = vec![];
        let sheet_size = info.sheetSize as usize;
        for i in 0..info.nSheets {
            let mut tex = unsafe {
                let data = info.sheetData.add(usize::from(i) * sheet_size);
                AnyTexture::raw(
                    c2d,
                    &mut *std::ptr::slice_from_raw_parts_mut(data, sheet_size),
                    TexDim::assume_valid(info.sheetWidth),
                    TexDim::assume_valid(info.sheetHeight),
                    u32::from(info.sheetFmt),
                )
            };
            tex.set_filter(true);
            sheets.push(Rc::pin(tex));
        }
        // 128 is more than enough for all of ASCII, so it's probably a good
        // cache size
        let cache = LruCache::new(NonZeroUsize::new(2).unwrap());

        Ok(Self {
            sheets,
            height,
            scale,
            cache,
        })
    }

    fn get_glyph(&mut self, c: char) -> &'_ Glyph<'gfx> {
        // check the cache first
        if !self.cache.contains(&c) {
            // not in the cache, put it in
            let pos = unsafe {
                let index =
                    ctru_sys::fontGlyphIndexFromCodePoint(std::ptr::null_mut(), u32::from(c));
                let mut pos = MaybeUninit::uninit();
                ctru_sys::fontCalcGlyphPos(
                    pos.as_mut_ptr(),
                    std::ptr::null_mut(),
                    index,
                    0,
                    1.0,
                    1.0,
                );
                pos.assume_init()
            };
            let texture = self.sheets[pos.sheetIndex as usize].clone();
            let image = Image::new_texcoord(
                texture,
                pos.width as _,
                self.height as _,
                pos.texcoord.left,
                pos.texcoord.top,
                pos.texcoord.right,
                pos.texcoord.bottom,
            );
            let glyph = Glyph {
                image,
                x_advance: pos.xAdvance,
            };
            self.cache.put(c, glyph);
        }
        self.cache.get(&c).unwrap()
    }

    pub fn print(&mut self, ctx: &Scene2d, line: &str, mut x: f32, y: f32, scale: f32, color: u32) {
        // avoid printing line if it won't be visible
        if y + f32::from(self.height) < 0.0 || y > 240.0 {
            return;
        }
        // otherwise, print it
        for c in line.chars() {
            let glyph = self.get_glyph(c);
            glyph.image.draw_tint(ctx, x, y, scale, scale, color);
            x += glyph.x_advance * self.scale * scale;
        }
    }

    fn text_width(&mut self, word: &str, scale: f32) -> f32 {
        let mut result = 0.0;
        for c in word.chars() {
            let glyph = self.get_glyph(c);
            result += glyph.x_advance * self.scale * scale;
        }
        result
    }

    fn create_lines(&mut self, text: &str, width: f32, scale: f32) -> Vec<String> {
        let mut words = vec![];
        let mut lines = vec![];
        let mut pos = 0.0;
        let mut remaining = text;
        let mut index_offset = 0;
        for (index, rule) in
            linebreaks(text).chain([(text.len(), BreakOpportunity::Mandatory)].into_iter())
        {
            let (word, r) = remaining.split_at(index - index_offset);
            index_offset = index;
            remaining = r;
            let word = word.replace('\n', "");
            let word_width = self.text_width(&word, scale);
            let mut pushed = false;
            if pos + word_width > width {
                lines.push(words.concat());
                words.clear();
                pos = 0.0;
                pushed = true;
            }
            words.push(word);
            pos += word_width;
            if !pushed && rule == BreakOpportunity::Mandatory {
                lines.push(words.concat());
                words.clear();
                pos = 0.0;
            }
        }
        return lines;
    }
}

pub struct TextLines {
    lines: Vec<String>,
    height: f32,
    scale: f32,
}

impl TextLines {
    pub fn new<'gfx>(
        text: &str,
        renderer: &mut TextRenderer<'gfx>,
        width: f32,
        scale: f32,
    ) -> Self {
        let lines = renderer.create_lines(text, width, scale);
        let height = (lines.len() as f32) * (renderer.height as f32) * scale;
        Self {
            lines,
            height,
            scale,
        }
    }

    pub fn render<'gfx>(
        &self,
        renderer: &mut TextRenderer<'gfx>,
        ctx: &Scene2d,
        x: f32,
        mut y: f32,
        color: u32,
    ) {
        for line in &self.lines {
            renderer.print(&ctx, &line, x, y, self.scale, color);
            y += (renderer.height as f32) * self.scale;
        }
    }

    pub fn height(&self) -> f32 {
        self.height
    }
}
