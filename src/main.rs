use ha_entity::{Device, DeviceClass, EntityClass, SimpleCommand};
use log::{debug, info};
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
    use env_logger::Env;
    use process_entity::{ClonableHdmiCecProcess, HdmiCecProcess};
    use service::HaBroker;

    // default to sending info or above messages.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    env_logger::init();

    info!("Starting up...");

    // load in the config file
    let config_file_contents: String = match fs::read_to_string(CONFIG_FILE) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading config file contents: {err}");
        }
    };

    let config: config::Config = match toml::from_str(&config_file_contents) {
        Ok(config) => config,
        Err(err) => {
            panic!("Error parsing config file: {err}");
        }
    };

    //start up the cec-client process. We will share this in a few different
    // threads, so we'll wrap it in a Arc so we can clone it.
    let mut hdmicec = Arc::new(HdmiCecProcess::new());
    let switch_hdmicec = hdmicec.clone(); // clone so we can move into a closure later.

    // Every entity should be part of a "Device" for homeassistant.
    let device = Device::from_config(&config);

    // Setup a "switch" device for the TV's power state.
    let switch = device
        .entity("tv", EntityClass::Switch, DeviceClass::Switch)
        .with_state(move |state| {
            switch_hdmicec.attach_statemanager(state.clone());

            // make another clone for the next closure...
            let switch_hdmicec = switch_hdmicec.clone();
            thread::spawn(move || {
                while true {
                    debug!("querying TV...");
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
            info!("Switching TV {}", if status { "on" } else { "off" });
            hdmicec.set_tv(status);
        }));

    // Setup a simple button for turning the volume up
    let mut vol_up = device
        .entity("volume_up", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            info!("Volume Up");
            hdmicec.volume_up();
        }));

    // Setup a simple button for turning the volume down
    let mut vol_down = device
        .entity("volume_down", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            info!("Volume Down");
            hdmicec.volume_down();
        }));

    // Setup a simple button for muting
    let mut mute = device
        .entity("mute", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, payload| {
            info!("Mute");
            hdmicec.mute();
        }));

    // start up the mqtt client, and attach all our entities.
    // then, start listening for mqtt messages, and output from
    // cec-client.
    // (note that homeassistant.listen() does not spawn a new thread
    // it never returns, and needs to be last.)
    let mut homeassistant = HaBroker::from_config(config);
    homeassistant.add_entity(switch);
    homeassistant.add_entity(vol_up);
    homeassistant.add_entity(vol_down);
    homeassistant.add_entity(mute);
    hdmicec.listen();
    homeassistant.listen();
}
