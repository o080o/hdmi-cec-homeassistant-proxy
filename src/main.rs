use ha_entity::{Device, DeviceClass, EntityClass};
use log::{debug, info};
use std::{env, fs, sync::Arc, thread, time::Duration};

mod config;
mod ha_entity;
mod hdmicec_entity;
mod payloads;
mod process;
mod service;

const CONFIG_FILE: &'static str = "config.toml";

fn main() {
    use env_logger::Env;
    use hdmicec_entity::{ClonableHdmiCecProcess, HdmiCecProcess};
    use service::HaBroker;

    // default to sending info or above messages.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting up...");

    // load in the config file
    let args: Vec<String> = env::args().collect();
    let config_file_path: &str = args.get(1).map(|s| s.as_str()).unwrap_or(CONFIG_FILE);
    info!("Reading config file at {config_file_path}");
    let config_file_contents: String = match fs::read_to_string(config_file_path) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading config file \"{config_file_path}\" contents: {err}");
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
    let hdmicec = Arc::new(HdmiCecProcess::new());
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
            thread::spawn(move || loop {
                debug!("querying TV...");
                switch_hdmicec.query_tv_state();
                thread::sleep(Duration::from_secs(10));
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
    let vol_up = device
        .entity("volumeup", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, _payload| {
            info!("Volume Up");
            hdmicec.volume_up();
        }));

    // Setup a simple button for turning the volume down
    let vol_down = device
        .entity("volumedown", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, _payload| {
            info!("Volume Down");
            hdmicec.volume_down();
        }));

    // Setup a simple button for muting
    let mute = device
        .entity("mute", EntityClass::Button, DeviceClass::None)
        .with_commands(hdmicec.command(|hdmicec, _payload| {
            info!("Mute");
            hdmicec.mute();
        }));

    // Setup a simple button for 4 sources. It's unclear to me if CEC even
    //supports more than 4 input sources. TODO maybe use a Select entity? that
    // will require reading the state though, unless we want to use optimistic mode.
    let sources = (1..5).map(|i| {
        return device
            .entity(
                &format!("Source{}", i),
                EntityClass::Button,
                DeviceClass::None,
            )
            .with_commands(hdmicec.command(move |hdmicec, _payload| {
                info!("Source {}", i);
                hdmicec.set_active_source(i as usize);
            }));
    });

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
    sources.for_each(|source| {
        homeassistant.add_entity(source);
    });
    hdmicec.listen();
    homeassistant.listen();
}
