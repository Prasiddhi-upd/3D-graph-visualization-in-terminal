use winit::{
    event::{Event, WindowEvent, MouseButton},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use pollster::block_on;

mod app;
mod graph;
mod camera;
mod gpu;
mod input;

use app::App;
use input::InputState;
fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("3D Graph Visualizer")
        .build(&event_loop)
        .unwrap();

    let mut app = block_on(App::new(&window)).unwrap_or_else(|e| {
        eprintln!("Failed to create App: {:?}", e);
        std::process::exit(1);
    });

    let mut input_state = InputState::default();
    
    // Create a reference to the window that we can move into the closure
    let window_ref = &window;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(new_size) => {
                    app.resize(new_size.width, new_size.height);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    input_state.on_keyboard(&event, app.get_camera_mut());
                }
                WindowEvent::CursorMoved { position, .. } => {
                    input_state.on_cursor_move(position, app.get_camera_mut());
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    input_state.on_mouse(state);
                }
                WindowEvent::RedrawRequested => {
                    if let Err(e) = app.render() {
                        eprintln!("Render error: {}", e);
                        // Handle resize on surface lost/outdated
                        app.resize(app.gpu_state.config.width, app.gpu_state.config.height);
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window_ref.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}