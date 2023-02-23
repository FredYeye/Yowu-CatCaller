const TIME: u64 = 25000;

mod graphics;
mod bt;

use tokio::sync::mpsc;
use winit::event_loop::{EventLoop, ControlFlow};
use egui::Context;
use bt::{bt_stuff, BtCommands, BtToGui, CmdData};

#[derive(Default)]
struct UiState {
    bt_state: BtToGui,
    headset_color: [u8; 3],
    headset_type: String,
    headset_bpm: u8,
    headset_duration: u8,
}

#[tokio::main]
async fn main() {
    let el = EventLoop::new();
    let mut graphics_state = graphics::Graphics::setup(&el, (450, 300));

    graphics_state.egui_state.ctx.set_pixels_per_point(2.0);

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
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(2));

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
            create_ui(&mut graphics_state.egui_state.ctx, &tx, &mut ui_state);
            graphics_state.paint();
        }
    });
}

fn create_ui(ctx: &mut Context, tx: &mpsc::Sender<BtCommands>, ui_state: &mut UiState) {
    egui::CentralPanel::default()
    .show(ctx, |ui| {
        match ui_state.bt_state {
            BtToGui::Ready => {
                ui.label(&ui_state.headset_type);

                ui.add_space(18.0);

                ui.label("Color:");
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut ui_state.headset_color);

                    if ui.button("Apply").clicked() {
                        let mut data = CmdData::default();
                        data.rgb = ui_state.headset_color;

                        match tx.try_send(BtCommands::SetMode(data)) {
                            Ok(_) => (),
                            Err(_) => println!("queue full!"),
                        };
                    }
                });

                ui.add_space(18.0);

                let modes = [
                    "Default",   "Flash",
                    "Breath",    "Rhythm",
                    "Yowu",      "Lights off",
                    "Lights on", "?",
                ];

                let chunk_size = 2;

                ui.label("Mode:");

                for (idx, mode_chunk) in modes.chunks(chunk_size).enumerate() {
                    ui.horizontal(|ui| {
                        for (idx2, &mode) in mode_chunk.iter().enumerate() {
                            let button = ui.add_sized([90.0, 22.0], egui::Button::new(mode));
                            if button.clicked() {
                                let mut data = CmdData::default();
                                data.mode = (idx * chunk_size + idx2 + 1) as u8;

                                match tx.try_send(BtCommands::SetMode(data)) {
                                    Ok(_) => (),
                                    Err(_) => println!("queue full!"),
                                };
                            };
                        }
                    });
                }

                ui.add_space(18.0);

                ui.label("Settings:");

                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut ui_state.headset_bpm, 0..=255).text("bpm"));
                    if ui.button("apply").clicked() {
                        let mut data = CmdData::default();
                        data.bpm = ui_state.headset_bpm;

                        match tx.try_send(BtCommands::SetMode(data)) {
                            Ok(_) => (),
                            Err(_) => println!("queue full!"),
                        };
                    }
                });

                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut ui_state.headset_duration, 0..=255).text("duration"));
                    if ui.button("apply").clicked() {
                        let mut data = CmdData::default();
                        data.duration = ui_state.headset_duration;

                        match tx.try_send(BtCommands::SetMode(data)) {
                            Ok(_) => (),
                            Err(_) => println!("queue full!"),
                        };
                    }
                });
            }

            _ => {
                ui.horizontal(|ui| {
                    let status = match &ui_state.bt_state {
                        BtToGui::Init => "Searching for BT adapter...",
                        BtToGui::AdapterConnected => "Adapter connected. Searching for headset...",
                        BtToGui::Found(_) => "Headset found. Connecting to headset...",
                        BtToGui::Connected => "Connected. Discovering services...",
                        BtToGui::Ready => unreachable!(),
                    };

                    ui.label(status);
                    ui.spinner();
                });
            }
        };
    });
}
