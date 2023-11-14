use glutin::{event_loop::EventLoop, WindowedContext};
use imgui_winit_support::WinitPlatform;

type Window = WindowedContext<glutin::PossiblyCurrent>;

const TITLE: &str = "daw";
pub const WINDOW_W: f32 = 1024.0;
pub const WINDOW_H: f32 = 768.0;

pub fn create_window() -> (EventLoop<()>, Window) {
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new()
        .with_title(TITLE)
        .with_inner_size(glutin::dpi::LogicalSize::new(WINDOW_W, WINDOW_H));
    let window = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(window, &event_loop)
        .expect("could not create window");
    let window = unsafe {
        window
            .make_current()
            .expect("could not make window context current")
    };
    (event_loop, window)
}

pub fn glow_context(window: &Window) -> glow::Context {
    unsafe { glow::Context::from_loader_function(|s| window.get_proc_address(s).cast()) }
}

pub fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window.window(),
        imgui_winit_support::HiDpiMode::Rounded,
    );

    // fonts
    let font_size = 16.0;

    imgui_context.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../../resources/Roboto-Regular.ttf"),
            size_pixels: font_size,
            config: Some(imgui::FontConfig {
                oversample_h: 4,
                oversample_v: 4,
                ..imgui::FontConfig::default()
            }),
        },
        imgui::FontSource::DefaultFontData { config: None },
    ]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    // styles
    imgui_context.style_mut().window_padding = [0.0, 0.0];
    imgui_context.style_mut().item_spacing = [0.0, 1.0];
    imgui_context.style_mut().window_border_size = 0.0;

    (winit_platform, imgui_context)
}
