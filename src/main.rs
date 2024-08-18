use ha_entity::{Device, EntityClass};
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
    let switch = device.entity("tv-state", EntityClass::Switch);
    //.with_state(CecTvState::new(...))
    //.with_state(|state|{ magic_stateful_component.attach_state(state) })
    //.with_state(polling_state(5, CecState::new(...)))
    //.with_commands(CecTvCommands)

    //let mut vol_up = Entity::new(EntityClass:Switch, "vol-up").with_commands(CecVolumeCommand::new(Volume::up));
    //let mut vol_up = Entity::new(EntityClass:Switch, "vol-down").with_commands(CecVolumeCommand::new(Volume::down));

    // example for the future state:
    // let mut switch = HaSwitch::new(&config, "lockscreen");
    // switch.get_state = Box::new(|| return process_exists(swayidle));
    // switch.set_state = Box::new(|state| { process_spawn_lockscreen or process_killall("swayidle")});

    //let topics = Topics::new(&config);
    //let switch = PollingEntity::new(switch, 5);

    let mut homeassistant = HaBroker::from_config(config);
    homeassistant.add_entity(switch);
    //homeassistant.add_entity(vol_up);
    //homeassistant.add_entity(vol_down);
    //homeassistant.poll_entity(switch, 5.seconds());

    homeassistant.configure();
    println!("device configured");

    println!("polling switch state");
    // thread::spawn(|| {
    //     // do complicated async thing, and every now and then get notified of something relevent!
    //     entity.update(new_state);
    //     homeassistant.update_state("tv-state", get_cec_state());

    //     thread::sleep(Duration::from_secs(3));
    // });

    println!("listening for messages...");
    homeassistant.listen();
}
