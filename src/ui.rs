use tokio::sync::mpsc;
use egui::{Context, Color32, TextStyle, FontId};
use crate::bt::{BtCommands, BtToGui, CmdData};

#[derive(Default)]
pub struct UiState {
    pub bt_state: BtToGui,
    pub headset_type: String,
    pub headset_color: [u8; 3],
    pub headset_settings: [u8; 2],
}

pub fn create_ui(ctx: &mut Context, tx: &mpsc::Sender<BtCommands>, ui_state: &mut UiState) {
    let mut central_frame = egui::containers::Frame::default();
    central_frame.inner_margin = egui::style::Margin { left: 15.0, right: 15.0, top: 15.0, bottom: 15.0 };
    central_frame.fill = Color32::from_rgb(0xB4, 0xE4, 0xFF);

    egui::CentralPanel::default()
    .frame(central_frame)
    .show(ctx, |ui| {
        match ui_state.bt_state {
            BtToGui::Ready => {
                ui.colored_label(Color32::from_rgb(21, 40, 51), &ui_state.headset_type);

                ui.add_space(18.0);

                ui.colored_label(Color32::from_rgb(21, 40, 51), "Color:");
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

                ui.colored_label(Color32::from_rgb(21, 40, 51), "Mode:");

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

                ui.colored_label(Color32::from_rgb(21, 40, 51), "Settings:");

                let settings = [
                    "Brightness", "Speed", //todo: figure out ranges
                    "BPM", "Duration", //if miku is detected. need to change ranges as well?
                ];

                for x in 0 .. 2 {
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut ui_state.headset_settings[x], 0 ..= 63)
                            .text(settings[x])
                            .text_color(Color32::from_rgb(21, 40, 51)));
    
                        if ui.button("apply").clicked() {
                            let mut data = CmdData::default();
                            data.settings[x] = ui_state.headset_settings[x];
    
                            match tx.try_send(BtCommands::SetMode(data)) {
                                Ok(_) => (),
                                Err(_) => println!("queue full!"),
                            };
                        }
                    });
                }
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

                    ui.colored_label(Color32::from_rgb(21, 40, 51), status);
                    ui.spinner();
                });
            }
        };
    });
}

pub fn set_egui_visuals(ctx: &mut Context) {
    use egui::FontFamily::Proportional;

    ctx.set_pixels_per_point(2.0);

    let mut visuals = egui::Visuals::default();
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(255, 206, 254); //button
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(255, 153, 253);  //button, hover
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(76, 0, 51);  //button text
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(51, 0, 34);   //button text, hover
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(201, 244, 170);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(15.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Monospace, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Small, FontId::new(15.0, Proportional)),
    ].into();

    ctx.set_style(style);
}
