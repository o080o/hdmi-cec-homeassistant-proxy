use serde::Serialize;

use crate::ha_entity::{Device, DeviceClass};

#[derive(Debug, Clone, Serialize)]
pub struct DevicePayload {
    name: String,
    identifiers: Vec<String>,
}

impl DevicePayload {
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

    #[serde(skip_serializing_if = "Option::is_none")]
    object_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    state_topic: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    command_topic: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<String>, //TODO limit to all available device classes via enum!

    #[serde(skip_serializing_if = "Option::is_none")]
    value_template: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    unique_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<DevicePayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<OriginPayload>,
}

impl ConfigPayload {
    pub fn new(
        state_topic: Option<String>,
        command_topic: Option<String>,
        device: &Device,
        device_class: &DeviceClass,
        id: &str,
    ) -> Self {
        let device_class = if *device_class == DeviceClass::None {
            None
        } else {
            Some(device_class.to_string())
        };

        Self {
            name: Some(id.to_string()),
            state_topic,
            command_topic,
            device_class,
            unique_id: Some(format!("{}_{}", device.unique_id, id)),
            origin: Some(OriginPayload::default()),
            device: Some(DevicePayload::from_device(device)),
            object_id: None,
            value_template: None,
        }
    }
}
