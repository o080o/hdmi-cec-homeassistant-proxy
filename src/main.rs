use ha_entity::{Device, DeviceClass, EntityClass, SimpleCommand};
use rumqttc::{Client, QoS};
use std::{alloc::handle_alloc_error, fs, sync::Arc, thread, time::Duration};

mod config;
mod ha_entity;
mod payloads;
mod process;
mod process_entity;
mod service;

const CONFIG_FILE: &'static str = "config.toml";

fn main() {
    use config::Config;
    use process_entity::{ClonableHdmiCecProcess, HdmiCecProcess};
    use service::HaBroker;

    println!("Hello, world!");

    let config_file: String = match fs::read_to_string(CONFIG_FILE) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading config file contents: {err}");
        }
    };

    let config: config::Config = match toml::from_str(&config_file) {
        Ok(config) => config,
        Err(err) => {
            panic!("Error parsing config file: {err}");
        }
    };

    let mut hdmicec = Arc::new(HdmiCecProcess::new());

    let device = Device::from_config(&config);

    let switch_hdmicec = hdmicec.clone();

    let switch = device
        .entity("tv", EntityClass::Switch, DeviceClass::Switch)
        .with_state(move |state| {
            switch_hdmicec.attach_statemanager(state.clone());

            // make another clone for the next closure...
            let switch_hdmicec = switch_hdmicec.clone();
            thread::spawn(move || {
                while true {
                    println!("querying TV...");
                    switch_hdmicec.query_tv_state();
                    thread::sleep(Duration::from_secs(10));
                }
            });
        })
        .with_commands(hdmicec.command(move |hdmicec, payload| {
            let status = match payload {
                "ON" => true,
                _ => false,
            };
            println!("switch {status} ({payload})");
            hdmicec.set_tv(status);
        }));

    let mut vol_up = device
        .entity("volume_up", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            println!("volume up");
            hdmicec.volume_up();
        }));

    let mut vol_down = device
        .entity("volume_down", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            println!("volume down");
            hdmicec.volume_down();
        }));

    let mut mute = device
        .entity("mute", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            println!("mute");
            hdmicec.mute();
        }));

    let mut homeassistant = HaBroker::from_config(config);
    homeassistant.add_entity(switch);
    homeassistant.add_entity(vol_up);
    homeassistant.add_entity(vol_down);
    homeassistant.add_entity(mute);
    hdmicec.listen();
    homeassistant.listen();
}
