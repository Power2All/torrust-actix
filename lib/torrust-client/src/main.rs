#![windows_subsystem = "windows"]

slint::include_modules!();

fn main() {
    let app = AppWindow::new().expect("failed to create window");

    // Both the Close button and the native X button route through this callback.
    app.on_quit(|| {
        slint::quit_event_loop().expect("failed to quit event loop");
    });

    app.run().expect("failed to run application");
}
