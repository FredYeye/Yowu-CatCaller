use tokio::sync::mpsc;
use egui::Context;
use crate::bt::{BtCommands, BtToGui, CmdData};

#[derive(Default)]
pub struct UiState {
    pub bt_state: BtToGui,
    pub headset_type: String,
    pub headset_color: [u8; 3],
    pub headset_bpm: u8,
    pub headset_duration: u8,
}

pub fn create_ui(ctx: &mut Context, tx: &mpsc::Sender<BtCommands>, ui_state: &mut UiState) {
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
