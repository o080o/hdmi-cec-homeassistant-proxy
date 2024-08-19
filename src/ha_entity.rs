use rumqttc::{Client, LastWill, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::string::ToString;
use std::thread;
use std::{fs, time::Duration};
use strum_macros::Display;

use crate::config::{Config, DeviceConfig};
use crate::payloads::ConfigPayload;
use crate::service::StateManager;

pub trait HaMqttEntity {
    fn get_config_payload(&self) -> ConfigPayload;
    fn get_discovery_topic(&self) -> String;
    fn get_state_topic(&self) -> Option<String>;
    fn get_command_topic(&self) -> Option<String>;
    fn get_device(&self) -> Device;
    fn get_name(&self) -> String;
    fn on_command(&mut self, payload: &str);
    fn connect_state(&mut self, state: StateManager);
}

#[derive(strum_macros::Display, Eq, PartialEq)]
pub enum EntityClass {
    #[strum(to_string = "switch")]
    Switch,
    #[strum(to_string = "button")]
    Button,
    #[strum(to_string = "sensor")]
    Sensor,
    #[strum(to_string = "binary_sensor")]
    BinarySensor,
}

#[derive(strum_macros::Display, PartialEq, Eq)]
pub enum DeviceClass {
    #[strum(to_string = "switch")]
    Switch,
    #[strum(to_string = "motion")]
    Motion,
    #[strum(to_string = "none")]
    None,
}

pub trait Commandable {
    fn on_command(&mut self, payload: &str);
}

pub struct SimpleCommand {
    on_command: Box<dyn Fn(&str) -> ()>,
}
impl SimpleCommand {
    pub fn new<T: 'static + Fn(&str) -> ()>(on_command: T) -> Self {
        Self {
            on_command: Box::new(on_command),
        }
    }
}
impl Commandable for SimpleCommand {
    fn on_command(&mut self, payload: &str) {
        (self.on_command)(payload);
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    pub unique_id: String,
    pub name: Option<String>,
    pub object_id: Option<String>,
    pub topic_prefix: String,
}

impl Device {
    pub fn from_config(config: &Config) -> Self {
        Self {
            name: config.device.device_name.clone(),
            unique_id: config.device.unique_id.clone(),
            object_id: config.device.object_id.clone(),
            topic_prefix: config.topic.prefix.clone(),
        }
    }

    pub fn entity(&self, id: &str, entity_class: EntityClass, device_class: DeviceClass) -> Entity {
        Entity {
            name: id.to_string(),
            topic_prefix: Entity::topic_prefix(self, id, &entity_class),
            entity_class,
            device_class,
            device: self.clone(),
            stateful: None,
            commands: None,
        }
    }
}

pub struct Entity {
    pub name: String,
    pub topic_prefix: String,
    pub entity_class: EntityClass,
    pub device_class: DeviceClass,
    pub device: Device,
    commands: Option<Box<dyn Commandable>>,
    stateful: Option<Box<dyn Fn(StateManager) -> ()>>,
}

impl Entity {
    pub fn topic_prefix(device: &Device, name: &str, entity_class: &EntityClass) -> String {
        let prefix = &device.topic_prefix;
        let class_str = entity_class.to_string();
        let object_id = device.object_id.as_ref().unwrap_or(&device.unique_id);
        return format!("{prefix}/{class_str}/{object_id}_{name}");
    }

    pub fn with_state<F: 'static + Fn(StateManager) -> ()>(mut self, func: F) -> Self {
        self.stateful = Some(Box::new(func));
        return self;
    }

    pub fn with_commands<T: 'static + Commandable>(mut self, commands: T) -> Self {
        self.commands = Some(Box::new(commands));
        return self;
    }
}

impl HaMqttEntity for Entity {
    fn get_config_payload(&self) -> ConfigPayload {
        return ConfigPayload::new(
            self.get_state_topic(),
            self.get_command_topic(),
            &self.device,
            &self.device_class,
            &self.name,
        );
    }

    fn get_device(&self) -> Device {
        return self.device.clone();
    }

    fn get_name(&self) -> String {
        return self.name.clone();
    }

    fn get_discovery_topic(&self) -> String {
        let prefix = &self.topic_prefix;
        return format!("{prefix}/config");
    }

    fn get_state_topic(&self) -> Option<String> {
        if self.stateful.is_some() {
            let prefix = &self.topic_prefix;
            return Some(format!("{prefix}/state"));
        } else {
            return None;
        }
    }

    fn get_command_topic(&self) -> Option<String> {
        if self.commands.is_some() {
            let prefix = &self.topic_prefix;
            return Some(format!("{prefix}/set"));
        } else {
            return None;
        }
    }

    fn on_command(&mut self, payload: &str) {
        if let Some(command) = self.commands.as_mut() {
            command.on_command(payload);
        }
    }

    fn connect_state(&mut self, state: StateManager) {
        if let Some(state_listener) = &self.stateful {
            (state_listener)(state)
        }
    }
}
