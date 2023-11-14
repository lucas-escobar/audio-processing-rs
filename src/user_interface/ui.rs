use crate::user_interface::setup;
use glow::HasContext;
use std::time::Instant;

type PhysicalSize = glutin::dpi::PhysicalSize<u32>;

pub struct UserInterface {
    event_loop: glutin::event_loop::EventLoop<()>,
    window: glutin::WindowedContext<glutin::PossiblyCurrent>,
    platform: imgui_winit_support::WinitPlatform,
    ig_context: imgui::Context,
    ig_renderer: imgui_glow_renderer::AutoRenderer,
}

impl UserInterface {
    pub fn new() -> Self {
        let (event_loop, window) = setup::create_window();
        let (winit_platform, mut imgui_context) = setup::imgui_init(&window);
        let gl = setup::glow_context(&window);
        let ig_renderer =
            imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context).unwrap();

        UserInterface {
            event_loop,
            window,
            platform: winit_platform,
            ig_context: imgui_context,
            ig_renderer,
        }
    }

    /// Run main event loop
    pub fn run(self, mut io_manager: crate::audio_engine::io_manager::IOManager) {
        let UserInterface {
            event_loop,
            window,
            mut platform,
            mut ig_context,
            mut ig_renderer,
        } = self;

        let mut last_frame = Instant::now();
        //let mut main_window_size = PhysicalSize::new(WINDOW_W as u32, WINDOW_H as u32);

        event_loop.run(move |event, _, control_flow| {
            match event {
                glutin::event::Event::NewEvents(_) => {
                    let now = Instant::now();
                    ig_context
                        .io_mut()
                        .update_delta_time(now.duration_since(last_frame));
                    last_frame = now;
                }
                // glutin::event::Event::WindowEvent {
                //     event: glutin::event::WindowEvent::Resized(size),
                //     ..
                // } => {
                //     // Update the main window size when the system window is resized
                //     main_window_size = size;
                // }
                glutin::event::Event::MainEventsCleared => {
                    platform
                        .prepare_frame(ig_context.io_mut(), window.window())
                        .unwrap();
                    window.window().request_redraw();
                }
                glutin::event::Event::RedrawRequested(_) => {
                    // The renderer assumes you'll be clearing the buffer yourself
                    unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                    let ui = ig_context.frame();

                    build_ui(ui, &mut io_manager);

                    platform.prepare_render(ui, window.window());
                    let draw_data = ig_context.render();

                    // This is the only extra render step to add
                    ig_renderer
                        .render(draw_data)
                        .expect("error rendering imgui");

                    window.swap_buffers().unwrap();
                }
                glutin::event::Event::WindowEvent {
                    event: glutin::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }
                event => {
                    platform.handle_event(ig_context.io_mut(), window.window(), &event);
                }
            }
        });
    }
}

/// Configure all components and subwindows in imgui instance
fn build_ui(ui: &mut imgui::Ui, io_manager: &mut crate::audio_engine::io_manager::IOManager) {
    //let size = main_window_size.to_logical::<f32>(1.0);

    ui.window("main")
        .size([1000.0, 800.0], imgui::Condition::FirstUseEver)
        .flags(
            imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_COLLAPSE
                | imgui::WindowFlags::NO_TITLE_BAR,
        )
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .build(|| {
            let column_count = 2;
            ui.columns(column_count, "main_columns", true);

            ui.set_column_offset(0, 0.0);
            ui.set_current_column_width(setup::WINDOW_W * (1.0 / 5.0));
            let settings_window_size = [setup::WINDOW_W * (1.0 / 5.0), setup::WINDOW_H];
            ui.child_window("settings")
                .size(settings_window_size)
                .build(|| {
                    build_settings_menu(&ui, io_manager);
                });

            ui.next_column();
            ui.set_current_column_width(setup::WINDOW_W * (4.0 / 5.0));
            let app_window_size = [setup::WINDOW_W * (4.0 / 5.0), setup::WINDOW_H];
            ui.child_window("app").size(app_window_size).build(|| {
                build_app_window(&ui, io_manager);
            });
        });
}

// TODO: uses &&mut ref, this is confusing, refactor
fn build_settings_menu(
    ui: &&mut imgui::Ui,
    io_manager: &mut crate::audio_engine::io_manager::IOManager,
) {
    if imgui::CollapsingHeader::new("Devices").build(ui) {
        let output_devices = io_manager.get_output_devices_names();

        let mut current_out_device_index = io_manager.get_current_out_device_index();

        if ui.combo(
            "Output Device",
            &mut current_out_device_index,
            &output_devices,
            |item| std::borrow::Cow::Borrowed(item),
        ) {
            io_manager.enable_output_device(current_out_device_index);
        };

        let input_devices = io_manager.get_input_device_names();

        let mut current_in_device_index = io_manager.get_current_in_device_index();

        if ui.combo(
            "Input Device",
            &mut current_in_device_index,
            &input_devices,
            |item| std::borrow::Cow::Borrowed(item),
        ) {
            io_manager.enable_input_device(current_in_device_index);
        };

        let sample_rates = [
            5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400,
            192000,
        ];
        let sample_rates = sample_rates.map(|rate| rate.to_string());
        let mut sample_rate_index: usize = 6; // 44100

        if ui.combo(
            "Sample Rate",
            &mut sample_rate_index,
            &sample_rates,
            |item| std::borrow::Cow::Borrowed(item.as_str()),
        ) {
            io_manager.set_sample_rate(sample_rates[sample_rate_index].parse::<u32>().unwrap());
        };
    }

    if imgui::CollapsingHeader::new("DSP").build(ui) {
        ui.text("test");
    }
    if imgui::CollapsingHeader::new("App Style").build(ui) {
        ui.text("test");
    }
    if imgui::CollapsingHeader::new("Recording").build(ui) {
        ui.text("test");
    }
}

fn build_app_window(
    ui: &&mut imgui::Ui,
    io_manager: &mut crate::audio_engine::io_manager::IOManager,
) {
    if ui.button("start_output") {
        io_manager.play_output();
    }
    if ui.button("stop_output") {
        io_manager.pause_output();
    }
    if ui.button("start_input") {
        io_manager.play_input();
    }
    if ui.button("stop_input") {
        io_manager.pause_input();
    }
}
