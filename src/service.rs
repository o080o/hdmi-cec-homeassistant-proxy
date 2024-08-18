use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use rumqttc::{Client, Connection, Event, Incoming, Publish, QoS};

use crate::{config::Config, ha_entity::HaMqttEntity};

pub struct StateManager {
    client: Arc<Client>,
    state_topic: String,
}

pub enum State {
    Stateful(StateManager),
    Stateless,
}

impl StateManager {
    pub fn new(client: Arc<Client>, state_topic: String) -> Self {
        Self {
            client,
            state_topic,
        }
    }

    pub fn update_state(&self, state: String) {
        self.client
            .publish(&self.state_topic, QoS::AtLeastOnce, true, state);
    }
}

pub struct HaBroker {
    // we will want to share Client with other threads that might be updating entity state.
    client: Arc<Client>,
    config: Config,
    connection: Option<Connection>,
    entities: HashMap<String, Box<dyn HaMqttEntity>>,
    topic_map: HashMap<String, Vec<String>>,
}

impl HaBroker {
    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    pub fn from_config(config: Config) -> Self {
        let mqtt_options = config.mqtt.as_mqtt_options();
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

    pub fn add_entity<T: 'static + HaMqttEntity>(&mut self, mut entity: T) {
        let id = entity.get_name();

        // TODO should this happen only after we configure??
        if let Some(state_topic) = entity.get_state_topic() {
            entity.connect_state(StateManager::new(self.client.clone(), state_topic));
        };

        if let Some(command_topic) = entity.get_command_topic() {
            self.add_topic_mapping(&command_topic, id.clone());
        };

        self.entities.insert(id, Box::new(entity));
    }

    fn add_topic_mapping(&mut self, topic: &str, index: String) {
        if self.topic_map.contains_key(topic) {
            self.topic_map.get_mut(topic).unwrap().push(index);
        } else {
            self.topic_map.insert(topic.to_string(), vec![index]);
        }
    }

    pub fn configure(&mut self) {
        self.entities.iter().for_each(|(_name, entity)| {
            let discovery_payload = entity.get_config_payload();
            let discovery_message: String = match serde_json::to_string(&discovery_payload) {
                Ok(value) => value,
                Err(err) => panic! {"cound not stringify the discovery payload! error={err}"},
            };

            let config_published = self
                .client
                .publish(
                    entity.get_discovery_topic(),
                    QoS::ExactlyOnce,
                    false, // we want to retain the config message on the broker so HA will still see this device if it restarts. TODO set to false during testing to make life easier.
                    discovery_message.as_str(),
                )
                .with_context(|| {
                    format!(
                        "unable to publish discovery message for entity {}",
                        entity.get_id()
                    )
                });

            match config_published {
                Ok(_) => {
                    if let Some(command_topic) = entity.get_command_topic() {
                        self.client
                            .subscribe(command_topic, QoS::AtMostOnce)
                            .unwrap();
                    }
                }
                Err(err) => {
                    print!("{err}");
                }
            }
        });
    }

    fn notify_entities(&mut self, event: &Publish) {
        let u8_array = event.payload.iter().cloned().collect::<Vec<u8>>();
        let payload = String::from_utf8(u8_array).expect("payload is can not be parsed as utf_8");

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
        // Iterate to poll the eventloop for connection progress
        // we need to take ownership of "connection", so that we can continue to borrow from self.
        let mut connection = self
            .connection
            .take()
            .expect("connection is already being used!");

        for notification in connection.iter() {
            println!("Notification = {:?}", notification);
            match notification {
                Ok(Event::Incoming(Incoming::Publish(event))) => {
                    println!("new event published! {:?}", event);
                    // find an entity for event.topic, and use that
                    self.notify_entities(&event);
                }
                Err(err) => {
                    println!("connection error... {err}");
                }
                _ => {
                    println!("event that I don't care about...");
                }
            }
        }
    }
}
