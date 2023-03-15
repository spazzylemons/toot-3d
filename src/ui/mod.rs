pub mod citro2d;
mod kbd;
pub mod screen;

use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use bit_set::BitSet;
use ctru::services::{Apt, Hid};
pub use kbd::get_input;

use self::citro2d::{color32, Citro2d, Image, RenderTarget, Scene2d, Text, TextBuf, TextConfig};

pub struct Ui<'gfx, 'screen> {
    apt: Apt,
    hid: Hid,

    c2d: &'gfx Citro2d,
    receiver: UiMsgReceiver,

    target: RenderTarget<'gfx, 'screen>,
    buf: TextBuf<'gfx>,

    pool: HashMap<usize, Image<'gfx>>,
    screen: Box<dyn Screen>,
}

impl<'gfx: 'screen, 'screen> Ui<'gfx, 'screen> {
    pub fn new(c2d: &'gfx Citro2d, receiver: UiMsgReceiver) -> Result<Self, Box<dyn Error>> {
        let apt = Apt::init()?;
        let hid = Hid::init()?;

        let target = RenderTarget::new_2d(c2d, c2d.gfx().top_screen.borrow_mut())?;
        let buf = TextBuf::new(c2d, 4096)?;

        let pool = HashMap::new();
        let screen = Box::new(EmptyScreen);

        Ok(Self {
            apt,
            hid,
            c2d,
            receiver,
            target,
            buf,
            pool,
            screen,
        })
    }

    pub fn iteration(&mut self) -> bool {
        // if it's time to quit, then do so
        if !self.apt.main_loop() {
            return false;
        }
        // check for all new messages
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                UiMsg::LoadImage(id, func) => match func(self.c2d) {
                    Ok(img) => {
                        self.pool.insert(id, img);
                    }
                    Err(e) => {
                        println!("image load failed: {e}");
                    }
                },

                UiMsg::UnloadImage(id) => {
                    self.pool.remove(&id);
                }

                UiMsg::SetScreen(screen) => {
                    self.screen = screen;
                }

                UiMsg::Quit => return false,
            }
        }
        // update the screen
        self.hid.scan_input();
        self.screen.update(&self.hid);
        // render the screen
        let frame = self.c2d.begin_frame();
        self.target.scene_2d(&frame, |ctx| {
            self.screen.draw(&self, &self.target, ctx);
        });
        drop(frame);
        // wait for vblank
        self.c2d.gfx().wait_for_vblank();
        // continue running
        true
    }

    pub fn draw_opaque_img(
        &self,
        img: &OpaqueImg,
        ctx: &Scene2d,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
    ) {
        if let Some(img) = self.pool.get(&img.id) {
            img.draw(ctx, x, y, scale_x, scale_y);
        }
    }

    pub fn draw_text(
        &self,
        ctx: &Scene2d,
        config: &TextConfig,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
    ) {
        self.buf.clear();
        // if the text fails to render, we'll just not render anything
        if let Ok(text) = Text::parse(None, &self.buf, text) {
            text.draw(ctx, config, x, y, scale);
        }
    }
}

pub trait ImageLoader:
    Send + for<'gfx> FnOnce(&'gfx Citro2d) -> Result<Image<'gfx>, Box<dyn Error>>
{
}

impl<T> ImageLoader for T where
    T: Send + for<'gfx> FnOnce(&'gfx Citro2d) -> Result<Image<'gfx>, Box<dyn Error>>
{
}

/// Message sent to render thread by logic thread.
pub enum UiMsg {
    /// Load an image with the given ID by running the given function.
    LoadImage(usize, Box<dyn ImageLoader>),
    /// Unload the image with the given ID.
    UnloadImage(usize),
    /// Switch to a new screen.
    SetScreen(Box<dyn Screen>),
    /// Quit the application.
    Quit,
}

pub type UiMsgSender = std::sync::mpsc::Sender<UiMsg>;
pub type UiMsgReceiver = std::sync::mpsc::Receiver<UiMsg>;

/// Allocates images on the logic thread.
#[derive(Clone)]
pub struct LogicImgPool {
    sender: UiMsgSender,
    used_ids: Arc<Mutex<BitSet>>,
}

impl LogicImgPool {
    pub fn new(sender: UiMsgSender) -> Self {
        Self {
            sender,
            used_ids: Arc::new(Mutex::new(BitSet::new())),
        }
    }

    pub fn alloc_box(&self, f: Box<dyn ImageLoader>) -> OpaqueImg {
        let mut used_ids = self.used_ids.lock().unwrap();
        let mut id = 0;
        for i in 0.. {
            if !used_ids.contains(i) {
                used_ids.insert(i);
                id = i;
                break;
            }
        }
        self.sender.send(UiMsg::LoadImage(id, f)).unwrap();
        OpaqueImg {
            id,
            pool: self.clone(),
        }
    }

    pub fn alloc<F>(&self, f: F) -> OpaqueImg
    where
        F: ImageLoader + 'static,
    {
        self.alloc_box(Box::new(f))
    }

    fn dealloc(&self, id: usize) {
        self.used_ids.lock().unwrap().remove(id);
        // ignore send errors here, it means that the ui deallocated before us
        _ = self.sender.send(UiMsg::UnloadImage(id));
    }
}

/// Image object that can be shared between threads.
pub struct OpaqueImg {
    id: usize,
    pool: LogicImgPool,
}

impl Drop for OpaqueImg {
    fn drop(&mut self) {
        self.pool.dealloc(self.id);
    }
}

pub trait Screen: Send {
    fn update(&self, hid: &Hid) {
        _ = hid;
    }

    fn draw<'gfx: 'screen, 'screen>(
        &self,
        ui: &Ui<'gfx, 'screen>,
        target: &RenderTarget<'gfx, 'screen>,
        ctx: &Scene2d,
    );
}

pub struct EmptyScreen;

impl Screen for EmptyScreen {
    fn draw<'gfx: 'screen, 'screen>(
        &self,
        _ui: &Ui<'gfx, 'screen>,
        target: &RenderTarget<'gfx, 'screen>,
        _ctx: &Scene2d,
    ) {
        target.clear(color32(0, 0, 0, 255));
    }
}
