use serde::Serialize;

use crate::config::Config;
use crate::ha_entity::Device;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HaDeviceClass {
    Switch,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevicePayload {
    name: String,
    identifiers: Vec<String>,
}

impl DevicePayload {
    pub fn from_config(config: &Config) -> Self {
        Self {
            name: config
                .device
                .device_name
                .clone()
                .unwrap_or(config.device.unique_id.clone()),
            identifiers: vec![config.device.unique_id.clone()],
        }
    }

    pub fn from_device(config: &Device) -> Self {
        Self {
            name: config.name.clone().unwrap_or(config.unique_id.clone()),
            identifiers: vec![config.unique_id.clone()],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginPayload {
    name: String,
    sw_version: String,
    support_url: String,
}

impl Default for OriginPayload {
    fn default() -> Self {
        let crate_name = env!("CARGO_PKG_NAME");
        let crate_version = env!("CARGO_PKG_VERSION");
        Self {
            name: crate_name.to_string(),
            sw_version: crate_version.to_string(),
            support_url: "https://github.com/o080o/ha-cec-proxy".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPayload {
    name: Option<String>,
    object_id: Option<String>,
    state_topic: Option<String>,
    command_topic: Option<String>,
    device_class: Option<HaDeviceClass>,
    value_template: Option<String>,
    unique_id: Option<String>,
    device: Option<DevicePayload>,
    origin: Option<OriginPayload>,
}

impl ConfigPayload {
    pub fn new(
        state_topic: Option<String>,
        command_topic: Option<String>,
        device: &Device,
        id: &str,
    ) -> Self {
        Self {
            name: None,
            state_topic,
            command_topic,
            device_class: Some(HaDeviceClass::Switch),
            unique_id: Some(format!("{}-{}", device.unique_id, id)),
            origin: Some(OriginPayload::default()),
            device: Some(DevicePayload::from_device(device)),
            object_id: None,
            value_template: None,
        }
    }
}
