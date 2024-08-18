use rumqttc::{LastWill, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

#[derive(Debug, Clone, Deserialize)]
pub struct MqttCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl Into<QoS> for MqttQos {
    fn into(self) -> QoS {
        match self {
            Self::AtMostOnce => QoS::AtMostOnce,
            Self::AtLeastOnce => QoS::AtLeastOnce,
            Self::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// configuration for the MQTT client. see the rumqttc docs for most of these options.
    pub mqtt: MqttConfig,
    pub topic: TopicConfig,
    pub device: DeviceConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TopicConfig {
    /// the prefix for the discovery topic. This is "homeassistant" by default.
    #[serde(default = "default_discovery_topic_prefix")]
    pub prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    /// The unique ID for this entity. By default, it is "hdmi-cec-proxy", but will need to be changed if you are running multiple instances on the same homeassistant server.
    #[serde(default = "default_unique_id")]
    pub unique_id: String,

    /// The object_id to use in the topic names. defaults to the unique id if not present.
    pub object_id: Option<String>,

    /// The device name to use. By default, it defaults to the unique_id.
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttLastWill {
    pub topic: String,
    pub message: String,
    pub qos: MqttQos,
    pub retain: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_device_id")]
    pub deviceid: String,
    #[serde(default = "default_keep_alive")]
    pub keep_alive: f64,
    /// the size of the bounded async channel the client is started with
    #[serde(default = "default_async_capacity")]
    pub async_capacity: usize,
    pub max_packet_size: Option<usize>,
    pub clean_session: Option<bool>,
    pub credentials: Option<MqttCredentials>,
    pub request_channel_capacity: Option<usize>,
    pub pending_throttle: Option<f64>,
    pub inflight: Option<u16>,
    pub manual_acks: Option<bool>,
    pub last_will: Option<MqttLastWill>,
}

impl MqttConfig {
    pub fn as_mqtt_options(&self) -> MqttOptions {
        let mut mqtt_options = MqttOptions::new(&self.deviceid, &self.host, self.port);
        mqtt_options.set_keep_alive(Duration::from_secs_f64(self.keep_alive));

        self.max_packet_size.as_ref().map(|value| {
            mqtt_options.set_max_packet_size(*value, *value);
        });
        self.clean_session.as_ref().map(|value| {
            mqtt_options.set_clean_session(*value);
        });
        self.credentials.as_ref().map(|credentials| {
            mqtt_options.set_credentials(&credentials.username, &credentials.password);
        });
        self.request_channel_capacity.as_ref().map(|capacity| {
            mqtt_options.set_request_channel_capacity(*capacity);
        });
        self.pending_throttle.as_ref().map(|throttle| {
            mqtt_options.set_pending_throttle(Duration::from_secs_f64(*throttle));
        });
        self.inflight.as_ref().map(|value| {
            mqtt_options.set_inflight(*value);
        });
        self.manual_acks.as_ref().map(|value| {
            mqtt_options.set_manual_acks(*value);
        });

        // TODO we will want to make a last will message, even if not configured.
        self.last_will.as_ref().map(|last_will| {
            mqtt_options.set_last_will(LastWill {
                topic: last_will.topic.clone(),
                message: last_will.message.clone().into(),
                qos: last_will.qos.clone().into(),
                retain: last_will.retain,
            });
        });

        return mqtt_options;
    }
}

fn default_device_id() -> String {
    return "hdmi-cec-proxy".to_string();
}

fn default_keep_alive() -> f64 {
    return 5.0;
}

fn default_async_capacity() -> usize {
    return 10;
}

fn default_discovery_topic_prefix() -> String {
    return "homeassistant".to_string();
}

fn default_unique_id() -> String {
    return "hdmi-device".to_string();
}
