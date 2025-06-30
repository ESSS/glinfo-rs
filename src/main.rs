use std::error::Error;
use std::ffi::{CStr, CString};

use winit::application::ApplicationHandler;
use winit::event::{ WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

use glutin::config::{Config, ConfigTemplateBuilder, GetGlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext, Version,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};

use glutin_winit::{DisplayBuilder, GlWindow};


pub mod gl {
    #![allow(clippy::all)]
    #![allow(unsafe_code)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let event_loop  =EventLoop::new().unwrap();
    let template = ConfigTemplateBuilder::new();

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes()));

    let mut app = App::new(template, display_builder);
    event_loop.run_app(&mut app)?;

    println!("{:?}", app.gl_info);

    app.exit_state
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (driver, window, gl_config) = match &self.gl_display {
            // We just created the event loop, so initialize the display, pick the config, and
            // create the context.
            GlDisplayCreationState::Builder(display_builder) => {
                let (window, gl_config) = match display_builder.clone().build(
                    event_loop,
                    self.template.clone(),
                    gl_config_picker,
                ) {
                    Ok((window, gl_config)) => (window.unwrap(), gl_config),
                    Err(err) => {
                        self.exit_state = Err(err);
                        event_loop.exit();
                        return;
                    },
                };

                // Mark the display as initialized to not recreate it on resume, since the
                // display is valid until we explicitly destroy it.
                self.gl_display = GlDisplayCreationState::Init;

                // Create gl context.
                let (driver, context) = create_gl_context(&window, &gl_config);
                self.gl_context = Some(context.treat_as_possibly_current());

                (Some(driver), window, gl_config)
            },
            GlDisplayCreationState::Init => {
                // Pick the config which we already use for the context.
                panic!("OH NO");
                let gl_config = self.gl_context.as_ref().unwrap().config();
                match glutin_winit::finalize_window(event_loop, window_attributes(), &gl_config) {
                    Ok(window) => (None, window, gl_config),
                    Err(err) => {
                        self.exit_state = Err(err.into());
                        event_loop.exit();
                        return;
                    },
                }
            },
        };

        let attrs = window
            .build_surface_attributes(Default::default())
            .expect("Failed to build surface attributes");
        let gl_surface =
            unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };

        // The context needs to be current for the Renderer to set up shaders and
        // buffers. It also performs function loading, which needs a current context on
        // WGL.
        let gl_context = self.gl_context.as_ref().unwrap();
        gl_context.make_current(&gl_surface).unwrap();


        let gl_display = gl_config.display();

        let gl = gl::Gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });

        if let Some(lib_name) = driver {
            let mut gl_info = GLInfo {
                driver: lib_name.to_string(),
                vendor: "".to_string(),
                renderer: "".to_string(),
                version: "".to_string(),
                shading_language: "".to_string(),
            };
            if let Some(vendor) = get_gl_string(&gl, gl::VENDOR) {
                gl_info.vendor = vendor.to_string_lossy().into();
            }
            if let Some(renderer) = get_gl_string(&gl, gl::RENDERER) {
                gl_info.renderer = renderer.to_string_lossy().into();
            }
            if let Some(version) = get_gl_string(&gl, gl::VERSION) {
                gl_info.version = version.to_string_lossy().into();
            }

            if let Some(shaders_version) = get_gl_string(&gl, gl::SHADING_LANGUAGE_VERSION) {
                gl_info.shading_language = shaders_version.to_string_lossy().into();
            }
            self.gl_info = Some(gl_info);
        }
    }


    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
        event_loop.exit();
    }

}

fn create_gl_context(window: &Window, gl_config: &Config) -> (&'static str, NotCurrentContext) {
    let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

    // The context creation part.
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    let legacy_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
        .build(raw_window_handle);

    // Reuse the uncurrented context from a suspended() call if it exists, otherwise
    // this is the first time resumed() is called, where the context still
    // has to be created.
    let gl_display = gl_config.display();

    unsafe {
        if let Ok(c) = gl_display.create_context(gl_config, &context_attributes) {
            return ("LibGL", c);
        } else if let Ok(c) = gl_display.create_context(gl_config, &fallback_context_attributes) {
            return ("LibGLES", c);
        } else if let Ok(c) = gl_display.create_context(gl_config, &legacy_context_attributes) {
            return ("libGL", c);
        }
        panic!("Failed to create GL context");
    }
}

fn window_attributes() -> WindowAttributes {
    Window::default_attributes()
        .with_visible(false)
}

enum GlDisplayCreationState {
    /// The display was not build yet.
    Builder(DisplayBuilder),
    /// The display was already created for the application.
    Init,
}

#[derive(Debug, Clone)]
struct GLInfo {
    vendor: String,
    renderer: String,
    version: String,
    shading_language: String,
    driver: String,
}

struct App {
    template: ConfigTemplateBuilder,
    // NOTE: `AppState` carries the `Window`, thus it should be dropped after everything else.
    gl_context: Option<PossiblyCurrentContext>,
    gl_display: GlDisplayCreationState,
    exit_state: Result<(), Box<dyn Error>>,
    gl_info: Option<GLInfo>,
}


impl App {
    fn new(template: ConfigTemplateBuilder, display_builder: DisplayBuilder) -> Self {
        Self {
            template,
            gl_display: GlDisplayCreationState::Builder(display_builder),
            exit_state: Ok(()),
            gl_context: None,
            gl_info: None,
        }
    }
}


pub fn gl_config_picker(mut configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs.next().unwrap()
}


fn get_gl_string(gl: &gl::Gl, variant: gl::types::GLenum) -> Option<&'static CStr> {
    unsafe {
        let s = gl.GetString(variant);
        (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
    }
}
