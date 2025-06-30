use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use winit::event_loop::EventLoop;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

use glutin::config::{Config, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;

use glutin_winit::{DisplayBuilder, GlWindow};

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

#[derive(Debug, Clone)]
struct GLInfo {
    driver: String,
    vendor: String,
    renderer: String,
    version: String,
    shading_language: String,
}

impl Display for GLInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} Vendor: {}\n\
             Renderer: {}\n\
             Version: {}\n\
             Shading Language: {}",
            self.driver, self.vendor, self.renderer, self.version, self.shading_language
        ))
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help") {
        println!("Usage: glinfo [-f filename]");
        return Ok(());
    }

    let event_loop =
        EventLoop::new().map_err(|err| format!("Failed to create event loop: {}", err))?;
    let template = ConfigTemplateBuilder::new();

    let display_builder = DisplayBuilder::new()
        .with_window_attributes(Some(Window::default_attributes().with_visible(false)));

    let output = match get_gl_info(template, display_builder, &event_loop) {
        Ok(gl_info) => format!("{}", gl_info),
        Err(err) => format!("ERROR: {}", err),
    };

    println!("{}", output);

    if args.len() == 3 && args[1] == "-f" {
        let filename = &args[2];
        std::fs::write(filename, output.as_bytes())?;
    }

    Ok(())
}

fn get_gl_info(
    template: ConfigTemplateBuilder,
    display_builder: DisplayBuilder,
    event_loop: &EventLoop<()>,
) -> Result<GLInfo, String> {
    // We just created the event loop, so initialize the display, pick the config, and
    // create the context.
    let (window, gl_config) =
        match display_builder
            .clone()
            .build(event_loop, template.clone(), |mut configs| {
                configs.next().expect("Could not get any configs")
            }) {
            Ok((window, gl_config)) => (window.unwrap(), gl_config),
            Err(err) => {
                return Err(err.to_string());
            }
        };

    // Create gl context.
    let (driver, context) = create_gl_context(&window, &gl_config)?;
    let gl_context = Some(context.treat_as_possibly_current());

    let attrs = window
        .build_surface_attributes(Default::default())
        .map_err(|err| format!("Failed to build surface attributes: {err}"))?;
    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .map_err(|err| format!("Failed to create a window surface: {err}"))?
    };

    // The context needs to be current for the Renderer to set up shaders and
    // buffers. It also performs function loading, which needs a current context on
    // WGL.
    let gl_context = gl_context.as_ref().ok_or("Failed to get a GL context")?;
    gl_context.make_current(&gl_surface).unwrap();

    let gl_display = gl_config.display();

    let gl = gl::Gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    let mut gl_info = GLInfo {
        driver: driver.to_string(),
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
    Ok(gl_info)
}

fn create_gl_context(
    window: &Window,
    gl_config: &Config,
) -> Result<(&'static str, NotCurrentContext), String> {
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
            return Ok(("LibGL", c));
        } else if let Ok(c) = gl_display.create_context(gl_config, &fallback_context_attributes) {
            return Ok(("LibGLES", c));
        } else if let Ok(c) = gl_display.create_context(gl_config, &legacy_context_attributes) {
            return Ok(("libGL", c));
        }
        Err("Failed to create GL context".into())
    }
}

fn get_gl_string(gl: &gl::Gl, variant: gl::types::GLenum) -> Option<&'static CStr> {
    unsafe {
        let s = gl.GetString(variant);
        (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
    }
}
