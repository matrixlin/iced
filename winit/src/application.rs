use crate::{
    column, conversion, input::mouse, renderer::Windowed, Cache, Column,
    Element, Event, Length, UserInterface,
};

pub trait Application {
    type Renderer: Windowed + column::Renderer;

    type Message;

    fn update(&mut self, message: Self::Message);

    fn view(&mut self) -> Element<Self::Message, Self::Renderer>;

    fn run(mut self)
    where
        Self: 'static + Sized,
    {
        use winit::{
            event::{self, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        };

        let event_loop = EventLoop::new();

        // TODO: Ask for window settings and configure this properly
        let window = WindowBuilder::new()
            .build(&event_loop)
            .expect("Open window");

        let size = window.inner_size().to_physical(window.hidpi_factor());;
        let (width, height) = (size.width as u16, size.height as u16);

        let mut renderer = Self::Renderer::new(&window);
        let mut target = renderer.target(width, height);

        let user_interface = UserInterface::build(
            document(&mut self, width, height),
            Cache::default(),
            &mut renderer,
        );

        let mut primitive = user_interface.draw(&mut renderer);
        let mut cache = Some(user_interface.into_cache());
        let mut events = Vec::new();

        window.request_redraw();

        event_loop.run(move |event, _, control_flow| match event {
            event::Event::MainEventsCleared => {
                // TODO: We should be able to keep a user interface alive
                // between events once we remove state references.
                //
                // This will allow us to rebuild it only when a message is
                // handled.
                let mut user_interface = UserInterface::build(
                    document(&mut self, width, height),
                    cache.take().unwrap(),
                    &mut renderer,
                );

                let messages = user_interface.update(events.drain(..));

                if messages.is_empty() {
                    primitive = user_interface.draw(&mut renderer);

                    cache = Some(user_interface.into_cache());
                } else {
                    // When there are messages, we are forced to rebuild twice
                    // for now :^)
                    let temp_cache = user_interface.into_cache();

                    for message in messages {
                        log::debug!("Updating");

                        self.update(message);
                    }

                    let user_interface = UserInterface::build(
                        document(&mut self, width, height),
                        temp_cache,
                        &mut renderer,
                    );

                    primitive = user_interface.draw(&mut renderer);

                    cache = Some(user_interface.into_cache());
                }

                window.request_redraw();
            }
            event::Event::RedrawRequested(_) => {
                renderer.draw(&mut target, &primitive);

                // TODO: Handle animations!
                // Maybe we can use `ControlFlow::WaitUntil` for this.
            }
            event::Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::CursorMoved { position, .. } => {
                    let physical_position =
                        position.to_physical(window.hidpi_factor());

                    events.push(Event::Mouse(mouse::Event::CursorMoved {
                        x: physical_position.x as f32,
                        y: physical_position.y as f32,
                    }));
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    events.push(Event::Mouse(mouse::Event::Input {
                        button: conversion::mouse_button(button),
                        state: conversion::button_state(state),
                    }));
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            _ => {
                *control_flow = ControlFlow::Wait;
            }
        })
    }
}

fn document<Application>(
    application: &mut Application,
    width: u16,
    height: u16,
) -> Element<Application::Message, Application::Renderer>
where
    Application: self::Application,
    Application::Message: 'static,
{
    Column::new()
        .width(Length::Units(width))
        .height(Length::Units(height))
        .push(application.view())
        .into()
}
