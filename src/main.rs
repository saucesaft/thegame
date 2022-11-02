mod state;
mod texture;
mod camera;
mod instance;
mod model;
mod resources;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use shalrath::repr::*;

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = state::State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {

        match event {
            
            Event::WindowEvent { ref event, window_id, }

            if window_id == window.id() => {

                if !state.input(event) {

                    match event {
                    
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,


                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }

                        WindowEvent::ScaleFactorChanged {new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }

                        _ => {}
                    }

                }

            }

            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();

                match state.render() {
                    Ok(_) => {},

                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),

                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                    Err(e) => eprintln!("{:?}", e),
                }
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }

        _ => {}


    }
});

}

fn main(){
    //let out = pollster::block_on(resources::load_string("cube.map"));

    //let map = out.unwrap().parse::<Map>();

    //println!("{:?}", map.unwrap());

    pollster::block_on(run());
} 