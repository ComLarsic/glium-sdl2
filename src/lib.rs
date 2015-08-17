//! An SDL2 backend for [Glium](https://github.com/tomaka/glium) - a high-level
//! OpenGL wrapper for the Rust language.
//!
//! # Example
//! ```no_run
//! # #[macro_use] extern crate glium;
//! # extern crate glium_sdl2;
//! # extern crate sdl2;
//! # fn main() {
//! use glium_sdl2::DisplayBuild;
//!
//! let sdl_context = sdl2::init().unwrap();
//! let video_subsystem = sdl_context.video().unwrap();
//!
//! let display = video_subsystem.window("My window", 800, 600)
//!     .resizable()
//!     .build_glium()
//!     .unwrap();
//!
//! let mut running = true;
//! let mut event_pump = sdl_context.event_pump().unwrap();
//!
//! while running {
//!     let mut target = display.draw();
//!     // do drawing here...
//!     target.finish().unwrap();
//!
//!     // Event loop: includes all windows
//!
//!     for event in event_pump.poll_iter() {
//!         use sdl2::event::Event;
//!
//!         match event {
//!             Event::Quit { .. } => {
//!                 running = false;
//!             },
//!             _ => ()
//!         }
//!     }
//! }
//! # }
//! ```

extern crate libc;

extern crate glium;
extern crate sdl2;

use std::mem;
use std::cell::UnsafeCell;
use std::ops::Deref;
use std::rc::Rc;

use glium::SwapBuffersError;
use glium::backend::{Backend, Context, Facade};
use sdl2::SdlResult;
use sdl2::VideoSubsystem;
use sdl2::video::{Window, WindowRef};

pub type Display = SDL2Facade;

/// Facade implementation for an SDL2 window.
#[derive(Clone)]
pub struct SDL2Facade {
    // contains everything related to the current context and its state
    context: Rc<Context>,

    backend: Rc<SDL2WindowBackend>,
}

impl Facade for SDL2Facade {
    fn get_context(&self) -> &Rc<Context> { &self.context }
}

impl Deref for SDL2Facade {
    type Target = Context;

    fn deref(&self) -> &Context { &self.context }
}

impl SDL2Facade {
    pub fn window(&self) -> &WindowRef {
        self.backend.window()
    }

    pub fn window_mut(&mut self) -> &mut WindowRef {
        self.backend.window_mut()
    }

    /// Start drawing on the backbuffer.
    ///
    /// This function returns a `Frame`, which can be used to draw on it.
    /// When the `Frame` is destroyed, the buffers are swapped.
    ///
    /// Note that destroying a `Frame` is immediate, even if vsync is enabled.
    pub fn draw(&self) -> glium::Frame {
        glium::Frame::new(self.context.clone(), self.backend.get_framebuffer_dimensions())
    }
}

/// An object that can build a facade object.
///
/// This trait is different from `glium::DisplayBuild` because Rust doesn't allow trait
/// implementations on types from external crates, unless the trait is in the same crate as the impl.
pub trait DisplayBuild {
    /// Build a context and a facade to draw on it.
    fn build_glium(self) -> Result<SDL2Facade, glium::GliumCreationError<sdl2::ErrorMessage>>;

    /// Build a context and a facade to draw on it
    ///
    /// This function does the same as `build_glium`, except that the resulting context will assume
    /// that the current OpenGL context will never change.
    unsafe fn build_glium_unchecked(self) -> Result<SDL2Facade, glium::GliumCreationError<sdl2::ErrorMessage>>;
}

impl<'a> DisplayBuild for &'a mut sdl2::video::WindowBuilder {
    fn build_glium(self) -> Result<SDL2Facade, glium::GliumCreationError<sdl2::ErrorMessage>> {
        let backend = Rc::new(try!(SDL2WindowBackend::new(self)));
        let context = try!(unsafe { Context::new(backend.clone(), true) });

        let display = SDL2Facade {
            context: context,
            backend: backend
        };

        Ok(display)
    }

    unsafe fn build_glium_unchecked(self) -> Result<SDL2Facade, glium::GliumCreationError<sdl2::ErrorMessage>> {
        let backend = Rc::new(try!(SDL2WindowBackend::new(self)));
        let context = try!(Context::new(backend.clone(), false));

        let display = SDL2Facade {
            context: context,
            backend: backend
        };

        Ok(display)
    }
}

pub struct SDL2WindowBackend {
    window: UnsafeCell<sdl2::video::Window>,
    context: sdl2::video::GLContext
}

impl SDL2WindowBackend {
    fn subsystem(&self) -> &VideoSubsystem {
        let ptr = self.window.get();
        let window: &Window = unsafe { mem::transmute(ptr) };
        window.subsystem()
    }

    fn window(&self) -> &WindowRef {
        let ptr = self.window.get();
        let window: &Window = unsafe { mem::transmute(ptr) };
        window
    }

    fn window_mut(&self) -> &mut WindowRef {
        let ptr = self.window.get();
        let window: &mut Window = unsafe { mem::transmute(ptr) };
        window
    }

    pub fn new(window_builder: &mut sdl2::video::WindowBuilder) -> SdlResult<SDL2WindowBackend> {
        let window = try!(window_builder.opengl().build());
        let context = try!(window.gl_create_context());

        Ok(SDL2WindowBackend {
            window: UnsafeCell::new(window),
            context: context
        })
    }
}

unsafe impl Backend for SDL2WindowBackend {
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        self.window().gl_swap_window();

        // AFAIK, SDL or `SDL_GL_SwapWindow` doesn't have any way to detect context loss.
        // TODO: Find out if context loss is an issue in SDL2 (especially for the Android port).

        Ok(())
    }

    unsafe fn get_proc_address(&self, symbol: &str) -> *const libc::c_void {
        // Assumes the appropriate context for the window has been set before this call.

        self.subsystem().gl_get_proc_address(symbol)
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        let (width, height) = self.window().drawable_size();
        (width as u32, height as u32)
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    unsafe fn make_current(&self) {
        self.window().gl_make_current(&self.context).unwrap()
    }
}
