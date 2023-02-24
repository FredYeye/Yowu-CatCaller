const TIME: u64 = 25000;

mod graphics;
mod bt;
mod ui;

use tokio::sync::mpsc;
use ui::{UiState, set_egui_visuals};
use winit::event_loop::{EventLoop, ControlFlow};
use bt::{bt_stuff, BtToGui};

#[tokio::main]
async fn main() {
    let el = EventLoop::new();
    let mut graphics_state = graphics::Graphics::setup(&el, (400, 400));
    set_egui_visuals(&mut graphics_state.egui_state.ctx);

    let mut ui_state = UiState::default();

    let (tx, mut rx) = mpsc::channel(4);
    let (tx2, mut rx2) = mpsc::channel(4);

    tokio::spawn(async move {
        match bt_stuff(&mut rx, &tx2).await {
            Ok(_) => (),
            Err(e) => println!("error! {e}"),
        };
    });

    let mut last_time = std::time::Instant::now();
    let mut frame_time = std::time::Duration::new(0, 0);

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(3));

        let current_time = std::time::Instant::now();
        frame_time += current_time - last_time;
        last_time = current_time;

        graphics::event_handling(event, control_flow, &mut graphics_state);

        while frame_time >= std::time::Duration::from_micros(TIME) {
            frame_time -= std::time::Duration::from_micros(TIME);

            if let Ok(bt_recv) = rx2.try_recv() {
                ui_state.bt_state = bt_recv;
                if let BtToGui::Found(asd) = &ui_state.bt_state {
                    ui_state.headset_type = asd.name();
                }
            }

            graphics_state.egui_state.ctx.begin_frame(graphics_state.egui_state.raw_input.take());
            ui::create_ui(&mut graphics_state.egui_state.ctx, &tx, &mut ui_state);
            graphics_state.paint();
        }
    });
}
