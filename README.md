# HDMI-CEC Homeassistant Proxy

This projet allows controlling HDMI-CEC devices, like you TV, from homeassistant. Unlike the builtin "HDMI-CEC" integration, this doesn't require the device running homeassistant to be connected to the TV. Any device can be connected to the TV, such as a raspberry pi, and run this software on that device to provide homeassistant with the required entities.

# Requirements

1. Homeassistant setup
2. MQTT Broker setup, and configured in Homeassistant (can use the built-in mosquitto broker add-on)
3. A device capable of Hdmi-Cec control. See the Homeassistant docs on this, most devices do not support it.
4. Docker installed on the device connected to the TV, or cargo + cec-utils installed if building from source.

# Installation

## Using Docker

using a docker-compose file is usually the simplest way to get the service running, and starting automatically. here is an example compose file:

1. Create a config file at 'config.toml'. See config.toml.example
2. Create a docker-compose.yaml file in the same directory
3. docker compose up -d

example docker-compose.yaml file:


## From Source

1. Create a config file at 'config.toml' in the project root. See config.toml.example
2. cargo run
