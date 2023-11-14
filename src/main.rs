mod audio_engine;
mod user_interface;

//use user_interface::UserInterface;

fn main() {
    let io = audio_engine::io_manager::IOManager::new();
    let ui = user_interface::ui::UserInterface::new();
    ui.run(io);
}
