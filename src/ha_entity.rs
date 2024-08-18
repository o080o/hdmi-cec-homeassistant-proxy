use rumqttc::{Client, LastWill, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::string::ToString;
use std::thread;
use std::{fs, time::Duration};
use strum_macros::Display;

use crate::config::{Config, DeviceConfig};
use crate::payloads::ConfigPayload;
use crate::service::{State, StateManager};

pub trait HaMqttEntity {
    fn get_config_payload(&self) -> ConfigPayload;
    fn get_discovery_topic(&self) -> String;
    fn get_state_topic(&self) -> Option<String>;
    fn get_command_topic(&self) -> Option<String>;
    fn get_id(&self) -> usize;
    fn get_device(&self) -> Device;
    fn get_name(&self) -> String;
    fn on_command(&mut self, payload: &str);
    fn connect_state(&mut self, state: StateManager);
}

#[derive(strum_macros::Display)]
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

pub trait Commandable {
    fn on_command(&mut self, payload: &str);
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

    pub fn entity(&self, id: &str, entity_class: EntityClass) -> Entity {
        Entity {
            id: 0, //TODO remove id's or implement for realsies.
            name: id.to_string(),
            topic_prefix: Entity::topic_prefix(self, id, &entity_class),
            entity_class,
            stateful: false,
            device: self.clone(),
            state: None,
            commands: None,
        }
    }
}

pub struct Entity {
    pub id: usize,
    pub name: String,
    pub topic_prefix: String,
    pub entity_class: EntityClass,
    pub device: Device,
    state: Option<StateManager>,
    commands: Option<Box<dyn Commandable>>,
    stateful: bool,
}

impl Entity {
    pub fn topic_prefix(device: &Device, name: &str, entity_class: &EntityClass) -> String {
        let prefix = &device.topic_prefix;
        let class_str = entity_class.to_string();
        let object_id = device.object_id.as_ref().unwrap_or(&device.unique_id);
        return format!("{prefix}/{class_str}/{object_id}-{name}");
    }
}

impl HaMqttEntity for Entity {
    fn get_config_payload(&self) -> ConfigPayload {
        return ConfigPayload::new(
            self.get_state_topic(),
            self.get_command_topic(),
            &self.device,
            &self.name,
        );
    }

    fn get_id(&self) -> usize {
        return self.id;
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
        if self.stateful {
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
        self.state = Some(state);
    }
}
