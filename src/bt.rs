use std::error::Error;

use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, BDAddr, WriteType};
use btleplug::platform::Manager;

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub enum BtCommands {
    SetMode(CmdData),
}

#[derive(Default)]
pub struct CmdData {
    pub mode: u8,
    pub rgb: [u8; 3],
    pub bpm: u8,
    pub duration: u8,
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
            BtCommands::SetMode(data) => {
                if headset.is_connected().await? {
                    headset.write(cmd_char, &command(data), WriteType::WithoutResponse).await?;
                }
            }
        }
    }

    Ok(())
}

fn command(d: CmdData) -> [u8; 11] {
    // 0xFC, 0x04, 0x01, 0x06 = header / command
    let mut cmd = [0xFC, 0x04, 0x01, 0x06, d.mode, d.rgb[0], d.rgb[1], d.rgb[2], d.bpm, d.duration, 0x00];
    cmd[10] = cmd.iter().fold(0, |a, x| a.wrapping_sub(*x));

    cmd
}

//audio profile
// FC 05 02 02 92 xx cc
// xx = audio profile 0-3
// cc = checksum
