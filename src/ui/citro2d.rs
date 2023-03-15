use std::{
    cell::RefMut,
    error::Error,
    ffi::CString,
    fmt::Display,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
};

use ctru::{gfx::Screen, prelude::Gfx, services::cfgu::Cfgu};

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
mod c {
    include!(concat!(env!("OUT_DIR"), "/citro2d.rs"));
}

/// There was not enough resources to complete the operation.
#[derive(Debug)]
pub struct C2dMemError;

impl Display for C2dMemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not enough resources for GPU operation")
    }
}

impl Error for C2dMemError {}

/// The handle to the Citro2D instance.
pub struct Citro2d(Gfx);

/// Ensures we don't make multiple Citro2D instances.
static CITRO_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Ensures we don't create multiple frames at once.
static FRAME_LOCK: Mutex<()> = Mutex::new(());

impl Citro2d {
    pub fn new(gfx: Gfx) -> Result<Self, C2dMemError> {
        // check count, only initialize if zero
        if CITRO_COUNT.fetch_add(1, Ordering::SeqCst) == 0 {
            // initialize citro2d
            unsafe {
                if !c::C3D_Init(c::C3D_DEFAULT_CMDBUF_SIZE as _) {
                    return Err(C2dMemError);
                }
                if !c::C2D_Init(c::C2D_DEFAULT_MAX_OBJECTS as _) {
                    c::C3D_Fini();
                    return Err(C2dMemError);
                }
                c::C2D_Prepare();
            }
        }
        Ok(Self(gfx))
    }

    pub fn begin_frame(&self) -> Frame<'_> {
        let lock = FRAME_LOCK.lock().unwrap();
        unsafe {
            // TODO handle return value
            c::C3D_FrameBegin(c::C3D_FRAME_SYNCDRAW as _);
        }
        Frame {
            _lock: lock,
            _phantom: PhantomData,
        }
    }

    pub fn gfx(&self) -> &Gfx {
        &self.0
    }
}

impl Drop for Citro2d {
    fn drop(&mut self) {
        // check count, only initialize if we hit zero
        if CITRO_COUNT.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe {
                c::C2D_Fini();
                c::C3D_Fini();
            }
        }
    }
}

pub struct Frame<'gfx> {
    /// Ensures one frame at a time.
    _lock: MutexGuard<'static, ()>,
    /// Locks us to citro2d reference.
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx> Drop for Frame<'gfx> {
    fn drop(&mut self) {
        unsafe {
            c::C3D_FrameEnd(0);
        }
    }
}

/// A render target to render a screen to.
pub struct RenderTarget<'gfx, 'screen> {
    /// Target handle.
    target: *mut c::C3D_RenderTarget,
    /// Keeps ownership of screen.
    _screen: RefMut<'screen, dyn Screen>,
    /// Locks us to c2d reference
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx, 'screen> RenderTarget<'gfx, 'screen> {
    pub fn new_2d(
        _c2d: &'gfx Citro2d,
        screen: RefMut<'screen, dyn Screen>,
    ) -> Result<Self, C2dMemError> {
        let target = unsafe {
            c::C2D_CreateScreenTarget(
                screen.as_raw(),
                match screen.side() {
                    ctru::gfx::Side::Left => c::gfx3dSide_t_GFX_LEFT,
                    ctru::gfx::Side::Right => c::gfx3dSide_t_GFX_RIGHT,
                },
            )
        };
        if target.is_null() {
            Err(C2dMemError)
        } else {
            Ok(Self {
                target,
                _screen: screen,
                _phantom: PhantomData,
            })
        }
    }

    pub fn clear(&self, color: u32) {
        unsafe {
            c::C2D_TargetClear(self.target, color);
        }
    }

    // lifetime ensures scene context doesn't leave the function
    // frame argument requires a frame to be active
    pub fn scene_2d<F>(&self, _frame: &Frame<'gfx>, f: F)
    where
        F: FnOnce(&Scene2d),
    {
        unsafe {
            // no-inline wrapper for this function, as calling it inlined
            // makes the screen go sideways
            c::C2D_SceneBegin_NotInlined(self.target);
        }
        f(&Scene2d(()));
    }
}

impl<'gfx, 'screen> Drop for RenderTarget<'gfx, 'screen> {
    fn drop(&mut self) {
        unsafe {
            c::C3D_RenderTargetDelete(self.target);
        }
    }
}

pub struct Scene2d(());

impl Scene2d {
    #[inline]
    pub fn rect(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color0: u32,
        color1: u32,
        color2: u32,
        color3: u32,
    ) {
        unsafe {
            c::C2D_DrawRectangle(x, y, 0.5, w, h, color0, color1, color2, color3);
        }
    }

    #[inline]
    pub fn rect_solid(&self, x: f32, y: f32, w: f32, h: f32, color: u32) {
        self.rect(x, y, w, h, color, color, color, color);
    }
}

#[inline]
pub const fn color32(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_le_bytes([r, g, b, a])
}

/// A format of a texture.
/// This trait is marked as unsafe because it provides safe interfaces that
/// could result in undefined behavior if not implemented correctly. In
/// particular, the size of the texture must be valid in order to properly flush
/// the texture cache.
pub unsafe trait TextureFormat {
    /// The pixel type.
    type Pixel;
    /// The format enum.
    const FORMAT: c::GPU_TEXCOLOR;
    /// Set a pixel. Assumes that the texture coordinates are in range. Causes
    /// undefined behavior if not in range.
    unsafe fn set(data: *mut std::ffi::c_void, x: u16, y: u16, width: u16, pixel: Self::Pixel);
    /// Get the size of a texture in bytes given its dimensions.
    fn size(width: u16, height: u16) -> usize;
}

// Texture indexing code adapated from Citra source code

static MORTON_X: [u8; 8] = [0x00, 0x01, 0x04, 0x05, 0x10, 0x11, 0x14, 0x15];
static MORTON_Y: [u8; 8] = [0x00, 0x02, 0x08, 0x0a, 0x20, 0x22, 0x28, 0x2a];

fn morton_interleave(x: usize, y: usize) -> u8 {
    MORTON_X[x & 7] | MORTON_Y[y & 7]
}

fn morton_offset(x: usize, y: usize, bytes_per_pixel: usize) -> usize {
    let i = usize::from(morton_interleave(x, y));
    let offset = (x & !7) << 3;
    (i | offset) * bytes_per_pixel
}

fn buffer_offset(x: usize, y: usize, width: usize, nybbles_per_pixel: usize) -> usize {
    let bytes_per_pixel = (nybbles_per_pixel + 1) >> 1;
    let stride = bytes_per_pixel * width;
    morton_offset(x, y, bytes_per_pixel) + (y & !7) * stride
}

/// A 4-bit luminance texture format.
pub struct Luminance4;

unsafe impl TextureFormat for Luminance4 {
    type Pixel = u8;

    const FORMAT: c::GPU_TEXCOLOR = c::GPU_TEXCOLOR_GPU_L4;

    unsafe fn set(data: *mut std::ffi::c_void, x: u16, y: u16, width: u16, pixel: Self::Pixel) {
        let index = buffer_offset(x.into(), y.into(), width.into(), 1);
        let half = index & 1;
        let byte_ptr = (data as *mut u8).add(index >> 1);
        if half == 0 {
            *byte_ptr &= 0xf0;
            *byte_ptr |= pixel;
        } else {
            *byte_ptr &= 0x0f;
            *byte_ptr |= pixel << 4;
        }
    }

    fn size(width: u16, height: u16) -> usize {
        (usize::from(width) * usize::from(height)) >> 1
    }
}

/// A verified texture dimension.
pub struct TexDim(u16);

#[derive(Debug)]
pub struct TexDimError;

impl Display for TexDimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture dimension too large")
    }
}

impl Error for TexDimError {}

impl TexDim {
    // maximum size permitted by citro2d
    const MAX: u16 = 1024;

    pub fn to_fit(dim: u16) -> Result<Self, TexDimError> {
        if dim < 8 {
            return Ok(Self(8));
        }
        let log2 = dim.ilog2();
        let result = if 1 << log2 == dim {
            dim
        } else {
            1 << (log2 + 1)
        };
        if result > Self::MAX {
            return Err(TexDimError);
        }
        Ok(Self(result))
    }
}

impl Deref for TexDim {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A formatted texture.
pub struct Texture<'gfx, T: TextureFormat> {
    /// The underlying texture. This is not public, as overwriting it could
    /// change the texture format, causing undefined behavior.
    any: AnyTexture<'gfx>,
    _phantom: PhantomData<T>,
}

impl<'gfx, T: TextureFormat> Texture<'gfx, T> {
    pub fn new(c2d: &'gfx Citro2d, width: TexDim, height: TexDim) -> Result<Self, C2dMemError> {
        Ok(Self {
            any: AnyTexture::new(c2d, width, height, T::FORMAT)?,
            _phantom: PhantomData,
        })
    }

    /// Set a pixel. This is unsafe because the range of the coordinate is not checked.
    pub unsafe fn set_unchecked(&self, x: u16, y: u16, pixel: T::Pixel) {
        T::set(self.any.data_ptr(), x, y, self.any.width(), pixel);
    }

    /// Flush the GPU cache for this texture. Only valid if not a cubemap.
    pub fn flush_cache(&self) {
        let ptr = self.any.data_ptr();
        // SAFETY: TextureFormat is an unsafe trait, so we rely on it to safely
        // tell us how many bytes a texture takes.
        unsafe {
            c::GSPGPU_FlushDataCache(ptr, T::size(self.any.width(), self.any.height()) as _);
        }
    }

    #[inline]
    pub fn set_filter(&mut self, filter: bool) {
        self.any.set_filter(filter);
    }
}

/// A format-agnostic texture reference.
pub struct AnyTexture<'gfx> {
    /// The wrapped texture.
    tex: c::C3D_Tex,
    /// Locks us to citro2d reference
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx> AnyTexture<'gfx> {
    pub fn new(
        _c2d: &'gfx Citro2d,
        width: TexDim,
        height: TexDim,
        format: c::GPU_TEXCOLOR,
    ) -> Result<Self, C2dMemError> {
        // TODO handle potential error
        let mut tex = unsafe {
            let mut tex = MaybeUninit::uninit();
            if !c::C3D_TexInit_NotInlined(tex.as_mut_ptr(), *width, *height, format) {
                return Err(C2dMemError);
            }
            tex.assume_init()
        };
        // clamp textures, as we don't plan to loop any of them
        unsafe {
            c::C3D_TexSetWrap_NotInlined(
                &mut tex,
                c::GPU_TEXTURE_WRAP_PARAM_GPU_CLAMP_TO_BORDER,
                c::GPU_TEXTURE_WRAP_PARAM_GPU_CLAMP_TO_BORDER,
            );
        }
        Ok(Self {
            tex,
            _phantom: PhantomData,
        })
    }

    pub fn set_filter(&mut self, filter: bool) {
        let filter = if filter {
            c::GPU_TEXTURE_FILTER_PARAM_GPU_LINEAR
        } else {
            c::GPU_TEXTURE_FILTER_PARAM_GPU_NEAREST
        };
        unsafe {
            c::C3D_TexSetFilter_NotInlined(&mut self.tex, filter, filter);
        }
    }

    pub fn width(&self) -> u16 {
        // SAFETY: This union exists only as a convenience to group width and height
        // as a single integer. Both of its variants are always valid.
        unsafe { self.tex.__bindgen_anon_2.__bindgen_anon_1.width }
    }

    pub fn height(&self) -> u16 {
        // SAFETY: see above
        unsafe { self.tex.__bindgen_anon_2.__bindgen_anon_1.height }
    }

    /// Get a pointer to the texture bitmap.
    pub fn data_ptr(&self) -> *mut std::ffi::c_void {
        // SAFETY: we never make a cubemap, so this union variant is always valid
        unsafe { self.tex.__bindgen_anon_1.data }
    }
}

impl<'gfx> Drop for AnyTexture<'gfx> {
    fn drop(&mut self) {
        unsafe {
            c::C3D_TexDelete(&mut self.tex);
        }
    }
}

/// A 2D image.
pub struct Image<'gfx> {
    /// Wrapped type
    image: c::C2D_Image,
    // reference to Citro2d
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx> Image<'gfx> {
    pub fn new(texture: AnyTexture<'gfx>, width: u16, height: u16) -> Self {
        // NOTE - by taking ownership of the texture, we can't do aliasing
        // maybe we could use an Rc type if necessary, but we'd need to cast a
        // const pointer to a mut pointer
        let tex = Box::new(texture.tex);
        let subtex = Box::new(c::Tex3DS_SubTexture {
            width,
            height,
            left: 0.0,
            top: 1.0,
            right: f32::from(width) / f32::from(texture.width()),
            bottom: 1.0 - (f32::from(height) / f32::from(texture.height())),
        });
        // use leak to pass ownership to image type, we'll reclaim it later to drop it
        let image = c::C2D_Image {
            tex: Box::leak(tex),
            subtex: Box::leak(subtex),
        };
        Self {
            image,
            _phantom: PhantomData,
        }
    }

    /// Helper function to build texture and turn into image.
    pub fn build<T, F>(
        c2d: &'gfx Citro2d,
        width: u16,
        height: u16,
        f: F,
    ) -> Result<Self, Box<dyn Error>>
    where
        T: TextureFormat,
        F: FnOnce(&mut Texture<'gfx, T>),
    {
        // create texture
        let mut texture = Texture::new(c2d, TexDim::to_fit(width)?, TexDim::to_fit(height)?)?;
        // initialize it
        f(&mut texture);
        // flush cache automatically
        texture.flush_cache();
        Ok(Self::new(texture.any, width, height))
    }

    pub fn draw(&self, _ctx: &Scene2d, x: f32, y: f32, scale_x: f32, scale_y: f32) {
        unsafe {
            c::C2D_DrawImageAt_NotInlined(
                self.image,
                x,
                y,
                0.5,
                std::ptr::null(),
                scale_x,
                scale_y,
            );
        }
    }
}

impl<'gfx> Drop for Image<'gfx> {
    fn drop(&mut self) {
        // SAFETY: the TexDelete() call drops the C texture struct, which must
        // be done. The from_raw() calls allow us to drop data that we
        // previously allocated in Boxes, and stored using Box::leak(). Since we
        // own the pointers, and they come from Boxes, they are safe to wrap in
        // Boxes.
        unsafe {
            c::C3D_TexDelete(self.image.tex);
            drop(Box::from_raw(self.image.tex));
            drop(Box::from_raw(
                self.image.subtex as *mut c::Tex3DS_SubTexture,
            ));
        }
    }
}

pub struct Font<'gfx> {
    font: c::C2D_Font,
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx> Font<'gfx> {
    pub fn new(_c2d: &'gfx Citro2d) -> ctru::Result<Option<Self>> {
        // select font from system region - hopefully correct?
        let cfgu = Cfgu::init()?;
        let region = cfgu.get_region()?;
        let font = unsafe { c::C2D_FontLoadSystem(region as _) };
        // null means system font
        if font.is_null() {
            Ok(None)
        } else {
            Ok(Some(Self {
                font,
                _phantom: PhantomData,
            }))
        }
    }
}

impl<'gfx> Drop for Font<'gfx> {
    fn drop(&mut self) {
        unsafe {
            c::C2D_FontFree(self.font);
        }
    }
}

pub struct TextBuf<'gfx> {
    buf: c::C2D_TextBuf,
    _phantom: PhantomData<&'gfx ()>,
}

impl<'gfx> TextBuf<'gfx> {
    pub fn new(_c2d: &'gfx Citro2d, size: usize) -> Result<Self, C2dMemError> {
        let buf = unsafe { c::C2D_TextBufNew(size) };
        if buf.is_null() {
            Err(C2dMemError)
        } else {
            Ok(Self {
                buf,
                _phantom: PhantomData,
            })
        }
    }

    pub fn clear(&self) {
        unsafe {
            c::C2D_TextBufClear(self.buf);
        }
    }
}

impl<'gfx> Drop for TextBuf<'gfx> {
    fn drop(&mut self) {
        unsafe {
            c::C2D_TextBufDelete(self.buf);
        }
    }
}

#[derive(Default)]
pub enum TextAlign {
    #[default]
    Left,
    Right,
    Center,
    Justified,
}

#[derive(Default)]
pub struct TextConfig {
    pub baseline: bool,
    pub color: Option<u32>,
    pub align: TextAlign,
    pub wrap_width: Option<f32>,
}

impl TextConfig {
    fn to_flags(&self) -> u32 {
        let mut result = 0;

        if self.baseline {
            result |= 1 << 0;
        }

        if self.color.is_some() {
            result |= 1 << 1;
        }

        result |= match self.align {
            TextAlign::Left => 0 << 2,
            TextAlign::Right => 1 << 2,
            TextAlign::Center => 2 << 2,
            TextAlign::Justified => 3 << 2,
        };

        if self.wrap_width.is_some() {
            result |= 1 << 4;
        }

        result
    }
}

pub struct Text<'gfx, 'buf, 'font> {
    text: c::C2D_Text,
    _phantom: PhantomData<(&'gfx (), &'buf (), &'font ())>,
}

impl<'gfx, 'buf, 'font> Text<'gfx, 'buf, 'font> {
    pub fn parse(
        font: Option<&'font Font<'gfx>>,
        buf: &'buf TextBuf<'gfx>,
        str: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let str = CString::new(str)?;
        let text = unsafe {
            let mut text = MaybeUninit::uninit();
            // docs say that this can fail, but afaict it can't
            if c::C2D_TextFontParse(
                text.as_mut_ptr(),
                match font {
                    Some(font) => font.font,
                    None => std::ptr::null_mut(),
                },
                buf.buf,
                str.as_ptr(),
            )
            .is_null()
            {
                panic!("C2D_TextFontParse() failed");
            }
            text.assume_init()
        };
        Ok(Self {
            text,
            _phantom: PhantomData,
        })
    }

    #[inline]
    pub fn optimize(&self) {
        unsafe {
            c::C2D_TextOptimize(&self.text);
        }
    }

    pub fn draw(&self, _ctx: &Scene2d, config: &TextConfig, x: f32, y: f32, scale: f32) {
        let flags = config.to_flags();
        unsafe {
            if let Some(width) = config.wrap_width {
                if let Some(color) = config.color {
                    c::C2D_DrawText(
                        &self.text,
                        flags,
                        x,
                        y,
                        0.5,
                        scale,
                        scale,
                        color,
                        width as std::ffi::c_double,
                    );
                } else {
                    c::C2D_DrawText(
                        &self.text,
                        flags,
                        x,
                        y,
                        0.5,
                        scale,
                        scale,
                        width as std::ffi::c_double,
                    );
                }
            } else if let Some(color) = config.color {
                c::C2D_DrawText(&self.text, flags, x, y, 0.5, scale, scale, color);
            } else {
                c::C2D_DrawText(&self.text, flags, x, y, 0.5, scale, scale);
            }
        }
    }
}
