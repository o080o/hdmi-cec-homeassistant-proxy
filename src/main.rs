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
                    println!("turning on");
                    state.update_state("ON".to_string());
                    thread::sleep(Duration::from_secs(5));
                    println!("turning off");
                    state.update_state("OFF".to_string());
                    thread::sleep(Duration::from_secs(5));
                }
            });
        })
        .with_commands(hdmicec.command(move |hdmicec, payload| {
            hdmicec.set_tv(false);
            println!("command! {}", payload);
        }));

    let volup_hdmicec = hdmicec.clone();
    let mut vol_up = device
        .entity("volume_up", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            println!("volume up! {}", payload);
            hdmicec.volume_up();
        }));

    let mut vol_down = device
        .entity("volume_down", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            println!("volume down! {}", payload);
            hdmicec.volume_down();
        }));

    let mut homeassistant = HaBroker::from_config(config);
    homeassistant.add_entity(switch);
    homeassistant.add_entity(vol_up);
    homeassistant.add_entity(vol_down);
    hdmicec.listen();
    homeassistant.listen();
}
