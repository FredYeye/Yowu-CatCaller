use std::error::Error;

use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, BDAddr, WriteType};
use btleplug::platform::Manager;

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub enum BtCommands {
    SetColor([u8; 3]),
    SetMode(usize),
}

#[derive(Debug, Default)]
pub enum BtToGui {
    #[default] Init,
    AdapterConnected,
    Found(HeadsetType),
    Connected,
    Ready,
}

#[derive(Debug)]
pub struct BtInfo {
    pub name: String,
    pub is_connected: bool,
    pub address: BDAddr,
}

#[derive(Debug)]
pub enum HeadsetType {
    YowuSelkirk4,
}

impl HeadsetType {
    pub fn name(&self) -> String {
        match self {
            HeadsetType::YowuSelkirk4 => "Yowu Selkirk 4".to_string(),
        }
    }
}

pub async fn bt_stuff(rx: &mut mpsc::Receiver<BtCommands>, tx: &mpsc::Sender<BtToGui>) -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let mut adapter_list = Vec::new();

    while adapter_list.is_empty() { //find BT adapter
        adapter_list = manager.adapters().await?;
        sleep(Duration::from_millis(750)).await;
    }

    tx.send(BtToGui::AdapterConnected).await?;

    let headset;

    'outer: loop { //find headset
        for adapter in adapter_list.iter() {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .expect("Can't scan BLE adapter for connected devices...");

            sleep(Duration::from_millis(850)).await;

            for peripheral in adapter.peripherals().await? {
                let properties = peripheral.properties().await?;
                let local_name = properties.unwrap().local_name.unwrap_or(String::from("name unknown"));

                if local_name == "YOWU-SELKIRK-4" {
                    headset = peripheral;
                    tx.send(BtToGui::Found(HeadsetType::YowuSelkirk4)).await?;
                    break 'outer;
                }
            }
        }
    }

    while !headset.is_connected().await? { //connect
        if let Err(err) = headset.connect().await {
            println!("Error connecting to peripheral: {}", err);
        }

        sleep(Duration::from_millis(500)).await;
    }

    tx.send(BtToGui::Connected).await?;

    headset.discover_services().await?;

    sleep(Duration::from_millis(120)).await;

    let chara = uuid_from_u16(0x2A06);
    let chars = headset.characteristics();
    let cmd_char = chars.iter().find(|c| c.uuid == chara).expect("Unable to find characterics");

    tx.send(BtToGui::Ready).await?;

    while let Some(commands) = rx.recv().await {
        match commands {
            BtCommands::SetColor(rgb) => {
                if headset.is_connected().await? {
                    headset.write(cmd_char, &color_command(rgb), WriteType::WithoutResponse).await?;
                }
            }

            BtCommands::SetMode(mode) => {
                if headset.is_connected().await? {
                    headset.write(cmd_char, &mode_command(mode), WriteType::WithoutResponse).await?;
                }
            }
        }
    }

    Ok(())
}

fn color_command(rgb: [u8; 3]) -> [u8; 11] {
    let mut cmd = [0xFC, 0x04, 0x01, 0x06, 0x00, rgb[0], rgb[1], rgb[2], 0x00, 0x00, 0x00];
    cmd[10] = -(cmd.iter().map(|&x| x as i16).sum::<i16>()) as u8;

    cmd
}

fn mode_command(mode: usize) -> [u8; 11] {
    let modes = vec![
        [0xFC, 0x04, 0x01, 0x06, 0x01, 0x00, 0x00, 0x00, 0x08, 0x0D, 0x00], //default
        [0xFC, 0x04, 0x01, 0x06, 0x02, 0x00, 0x00, 0x00, 0x08, 0x03, 0x00], //flash
        [0xFC, 0x04, 0x01, 0x06, 0x03, 0x00, 0x00, 0x00, 0x08, 0x04, 0x00], //breathe
        [0xFC, 0x04, 0x01, 0x06, 0x04, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00], //pulse (react to sound)
        [0xFC, 0x04, 0x01, 0x06, 0x05, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00], //yowu
        [0xFC, 0x04, 0x01, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], //lights off
        [0xFC, 0x04, 0x01, 0x06, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], //lights on
        [0xFC, 0x04, 0x01, 0x06, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], //?
    ];

    let mut cmd = modes[mode];
    cmd[10] = -(cmd.iter().map(|&x| x as i16).sum::<i16>()) as u8;

    cmd
}

// [0xFC, 0x04, 0x01, 0x06, 0x04, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00]
//  header?--------------|  mode  r     g     b     ?     ?     checksum
// sending 0 = no change?


// fn new_mode(rgb: [u8; 3], mode: u8) -> [u8; 11] {
//     let base_cmd = [0xFC, 0x04, 0x01, 0x06, mode, rgb[0], rgb[1], rgb[2], 0x00, 0x00, 0x00];
// }