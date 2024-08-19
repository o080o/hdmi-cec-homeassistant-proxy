use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use log::{debug, error, info, trace};
use rumqttc::{Client, Connection, Event, Incoming, Publish, QoS};

use crate::{config::Config, ha_entity::HaMqttEntity};

/// A way for entities to update their state, without accessing the HaBroker. Entities can easily clone and own a copy of this object.
#[cfg_attr(test, faux::create)]
#[derive(Clone)]
pub struct StateManager {
    client: Arc<Client>,
    state_topic: String,
    entity_name: String,
}

#[cfg_attr(test, faux::methods)]
impl StateManager {
    /// create a new StateManager for a given state topic, and mqtt client reference
    pub fn new(client: Arc<Client>, state_topic: String, entity_name: String) -> Self {
        Self {
            client,
            state_topic,
            entity_name,
        }
    }

    /// update the entities state via the topic in the constructor. 'state' is the entire message payload, possibly JSON formatted. For simple switches, this may just be the string "ON" or "OFF". See Homeassistant docs for more info on what to send.
    pub fn update_state(&self, state: String) {
        self.client
            .publish(&self.state_topic, QoS::AtLeastOnce, false, state)
            .with_context(|| {
                format!(
                    "entity: \"{}\" topic: \"{}\"  ",
                    self.entity_name, self.state_topic
                )
            })
            .expect("could not publish entities state message");
    }
}

/// A representation of the connection to the MQTT Broker and HomeAssistant. Many entities or devices can be added to the same broker instance.
pub struct HaBroker {
    // we will want to share Client with other threads that might be updating entity state.
    client: Arc<Client>,
    config: Config,
    connection: Option<Connection>,
    entities: HashMap<String, Box<dyn HaMqttEntity>>,
    topic_map: HashMap<String, Vec<String>>,
}

impl HaBroker {
    /// get a copy of a reference to the client. useful if you want to publish messages to MQTT directly.
    #[allow(dead_code)]
    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    /// Create a new connection from a given config object. Automatically opens a new mqtt connection.
    pub fn from_config(config: Config) -> Self {
        let mqtt_options = config.mqtt.as_mqtt_options();
        debug!("connection options: {:?}", mqtt_options);
        let (client, connection) = Client::new(mqtt_options, config.mqtt.async_capacity);
        let ha_broker = Self {
            entities: HashMap::new(),
            config,
            client: Arc::new(client),
            connection: Some(connection),
            topic_map: HashMap::new(),
        };
        ha_broker
    }

    /// Add a new entity to homeassistant, via the mqtt discovery topics.
    pub fn add_entity<T: 'static + HaMqttEntity>(&mut self, mut entity: T) {
        let id = entity.get_name();

        // TODO should this happen only after we configure??
        if let Some(state_topic) = entity.get_state_topic() {
            entity.connect_state(StateManager::new(
                self.client.clone(),
                state_topic,
                entity.get_name(),
            ));
        };

        if let Some(command_topic) = entity.get_command_topic() {
            self.add_topic_mapping(&command_topic, id.clone());
        };

        self.send_discovery_message(&entity);
        self.subscribe_to_command_topic(&entity);

        self.entities.insert(id, Box::new(entity));
    }

    fn add_topic_mapping(&mut self, topic: &str, index: String) {
        if self.topic_map.contains_key(topic) {
            self.topic_map.get_mut(topic).unwrap().push(index);
        } else {
            self.topic_map.insert(topic.to_string(), vec![index]);
        }
    }

    fn send_discovery_message<T: 'static + HaMqttEntity + ?Sized>(&self, entity: &T) {
        let discovery_payload = entity.get_config_payload();
        let discovery_message: String = match serde_json::to_string(&discovery_payload) {
            Ok(value) => value,
            Err(err) => panic! {"cound not stringify the discovery payload! error={err}"},
        };

        debug!(
            "publishing config to topic {}: {}",
            entity.get_discovery_topic(),
            discovery_message,
        );

        let config_published = self
            .client
            .publish(
                entity.get_discovery_topic(),
                QoS::ExactlyOnce,
                false, // instead of retaining these messages, we will listen for the mqtt integration's birth/will messages, as per the docs: https://www.home-assistant.io/integrations/mqtt#use-the-birth-and-will-messages-to-trigger-discovery
                discovery_message.as_str(),
            )
            .with_context(|| {
                format!(
                    "unable to publish discovery message for entity {}",
                    entity.get_name()
                )
            });

        match config_published {
            Ok(_) => {}
            Err(err) => {
                error!("{err}");
            }
        }
    }

    fn subscribe_to_command_topic<T: 'static + HaMqttEntity + ?Sized>(&self, entity: &T) {
        if let Some(command_topic) = entity.get_command_topic() {
            self.client
                .subscribe(command_topic, QoS::AtMostOnce)
                .unwrap();
        }
    }

    fn send_all_discovery_messages(&self) {
        self.entities.iter().for_each(|(_name, entity)| {
            self.send_discovery_message(entity.as_ref());
        });
    }

    fn notify_entities(&mut self, event: &Publish) {
        let u8_array = event.payload.iter().cloned().collect::<Vec<u8>>();
        let payload =
            String::from_utf8(u8_array).expect("command payload  can not be parsed as utf_8");

        let matching_entities = self.topic_map.get(&event.topic);
        match matching_entities {
            Some(entity_indices) => entity_indices.iter().for_each(|name| {
                let entity = self
                    .entities
                    .get_mut(name)
                    .expect("invalid index into entities");
                entity.on_command(&payload);
            }),
            None => {}
        }
    }

    pub fn listen(&mut self) -> () {
        // we need to take ownership of "connection", so that we can continue to borrow from self.
        let mut connection = self
            .connection
            .take()
            .expect("connection is already being used!");

        // subscribe to the homeassistant status topic to recieve birth/will messages. see https://www.home-assistant.io/integrations/mqtt#use-the-birth-and-will-messages-to-trigger-discovery
        self.client
            .subscribe(&self.config.topic.status, QoS::AtLeastOnce)
            .with_context(|| format!("{}", &self.config.topic.status))
            .expect("unable to subscribe to homeassistant's status topic");

        info!("listening for mqtt messages...");

        // Iterate to poll the eventloop for connection progress
        for notification in connection.iter() {
            trace!("Notification = {:?}", notification);
            match notification {
                Ok(Event::Incoming(Incoming::Publish(event))) => {
                    if event.topic == self.config.topic.status {
                        if event.payload == "online" {
                            debug!("mqtt integration online. resending discovery messages",);
                            self.send_all_discovery_messages();
                        } else {
                            debug!("mqtt integration status changed {:?}", event);
                        }
                    } else {
                        debug!("new event published! {:?}", event);

                        // find an entity for event.topic, and use that
                        self.notify_entities(&event);
                    }
                }
                Err(err) => {
                    error!("connection error... {err}");
                }
                _ => {}
            }
        }
    }
}
