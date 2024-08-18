use ha_entity::{Device, DeviceClass, EntityClass, SimpleCommand};
use rumqttc::{Client, QoS};
use std::{alloc::handle_alloc_error, fs, thread, time::Duration};

mod config;
mod ha_entity;
mod payloads;
mod service;

const CONFIG_FILE: &'static str = "config.toml";

fn main() {
    use config::Config;
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

    let device = Device::from_config(&config);

    let switch = device
        .entity("tv", EntityClass::Switch, DeviceClass::Switch)
        .with_state(|state| {
            thread::spawn(move || {
                while true {
                    println!("turning on");
                    state.update_state("ON".to_string());
                    thread::sleep(Duration::from_secs(5));
                    println!("turning off");
                    state.update_state("OFF".to_string());
                    thread::sleep(Duration::from_secs(5));
                }
            });
        })
        .with_commands(SimpleCommand::new(|payload| {
            println!("command! {}", payload)
        }));

    let mut vol_up = device
        .entity("volume_up", EntityClass::Button, DeviceClass::None)
        .with_commands(SimpleCommand::new(|payload| {
            println!("volume up! {}", payload)
        }));

    let mut vol_down = device
        .entity("volume_down", EntityClass::Button, DeviceClass::None)
        .with_commands(SimpleCommand::new(|payload| {
            println!("volume down! {}", payload)
        }));

    let mut homeassistant = HaBroker::from_config(config);
    homeassistant.add_entity(switch);
    homeassistant.add_entity(vol_up);
    homeassistant.add_entity(vol_down);

    homeassistant.configure();
    println!("device configured");

    println!("listening for messages...");
    homeassistant.listen();
}
