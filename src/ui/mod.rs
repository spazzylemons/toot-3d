use std::{
    cell::RefMut,
    marker::PhantomData,
    sync::{Mutex, MutexGuard},
};

use ctru::{gfx::Screen, prelude::Gfx};

mod kbd;

pub use kbd::get_input;

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
mod c {
    include!(concat!(env!("OUT_DIR"), "/citro2d.rs"));
}

pub struct C2dGlobal<'gfx>(PhantomData<&'gfx ()>);

impl<'gfx> C2dGlobal<'gfx> {
    pub fn new(_gfx: &'gfx Gfx) -> Self {
        unsafe {
            c::C3D_Init(c::C3D_DEFAULT_CMDBUF_SIZE as _);
            c::C2D_Init(c::C2D_DEFAULT_MAX_OBJECTS as _);
            c::C2D_Prepare();
        }
        Self(PhantomData)
    }
}

impl<'gfx> Drop for C2dGlobal<'gfx> {
    fn drop(&mut self) {
        unsafe {
            c::C2D_Fini();
            c::C3D_Fini();
        }
    }
}

pub struct RenderTarget<'screen> {
    target: *mut c::C3D_RenderTarget,
    _screen: RefMut<'screen, dyn Screen>,
}

impl<'screen> RenderTarget<'screen> {
    pub fn new_2d<'gfx>(_global: &C2dGlobal<'gfx>, screen: RefMut<'screen, dyn Screen>) -> Self {
        let target = unsafe {
            c::C2D_CreateScreenTarget(
                screen.as_raw(),
                match screen.side() {
                    ctru::gfx::Side::Left => c::gfx3dSide_t_GFX_LEFT,
                    ctru::gfx::Side::Right => c::gfx3dSide_t_GFX_RIGHT,
                },
            )
        };

        Self {
            target,
            _screen: screen,
        }
    }

    pub fn clear(&self, color: u32) {
        unsafe {
            c::C2D_TargetClear(self.target, color);
        }
    }

    pub fn scene_2d<F>(&self, _frame: Frame, f: F)
    // function call requires that 2d render context is:
    //   1. only used within this function
    //   2. does not leave this function because it must be returned
    // additionally, by passing in the frame, it requires:
    //   3. a frame must begin to allow drawing
    //   4. no other calls to scene_2d can occur, since only one Frame can exist at a time
    where
        F: FnOnce(Scene2d) -> Scene2d,
    {
        unsafe {
            // no-inline wrapper for this function, as calling it inlined
            // makes the screen go sideways
            c::C2D_SceneBegin_NotInlined(self.target);
        }
        f(Scene2d(()));
    }
}

impl<'screen> Drop for RenderTarget<'screen> {
    fn drop(&mut self) {
        unsafe {
            c::C3D_RenderTargetDelete(self.target);
        }
    }
}

pub struct Scene2d(());

impl Scene2d {
    pub fn rect(
        &self,
        x: f32,
        y: f32,
        z: f32,
        w: f32,
        h: f32,
        color0: u32,
        color1: u32,
        color2: u32,
        color3: u32,
    ) {
        unsafe {
            c::C2D_DrawRectangle(x, y, z, w, h, color0, color1, color2, color3);
        }
    }

    pub fn rect_solid(&self, x: f32, y: f32, z: f32, w: f32, h: f32, color: u32) {
        self.rect(x, y, z, w, h, color, color, color, color);
    }
}

// prevents us from creating multiple frames at a time
static FRAME_LOCK: Mutex<()> = Mutex::new(());

pub struct Frame(MutexGuard<'static, ()>);

impl Frame {
    pub fn new() -> Self {
        let lock = FRAME_LOCK.lock().unwrap();
        unsafe {
            c::C3D_FrameBegin(c::C3D_FRAME_SYNCDRAW as _);
        }
        Self(lock)
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            c::C3D_FrameEnd(0);
        }
    }
}

#[inline]
pub const fn color32(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_le_bytes([r, g, b, a])
}
