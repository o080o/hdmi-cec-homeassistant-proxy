use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::{Config, DeviceConfig};
use crate::ha_entity::SimpleCommand;
use crate::payloads::ConfigPayload;
use crate::process::CommandProcess;
use crate::service::StateManager;

pub trait ClonableHdmiCecProcess {
    fn command<F: 'static + Fn(&HdmiCecProcess, &str) -> ()>(&self, func: F) -> SimpleCommand;
}

impl ClonableHdmiCecProcess for Arc<HdmiCecProcess> {
    fn command<F: 'static + Fn(&HdmiCecProcess, &str) -> ()>(&self, func: F) -> SimpleCommand {
        let hdmicec = self.clone();
        return SimpleCommand::new(move |payload| {
            return func(&hdmicec, payload);
        });
    }
}

pub struct HdmiCecProcess {
    process: Mutex<CommandProcess>,
    state: Mutex<Option<StateManager>>,
    tv_state: Arc<Mutex<Option<String>>>,
}

impl HdmiCecProcess {
    pub fn new() -> Self {
        let command = Command::new("cat");
        let process = CommandProcess::new(command);
        return Self {
            process: Mutex::new(process),
            state: Mutex::new(None),
            tv_state: Arc::new(Mutex::new(None)),
        };
    }

    pub fn attach_statemanager(&self, statemanager: StateManager) {
        self.state
            .lock()
            .expect("could not get lock")
            .replace(statemanager);
    }

    pub fn listen(&self) {
        println!("starting to listen for the hdmicec process...");
        let state = self
            .state
            .lock()
            .expect("could not lock state")
            .clone()
            .expect("no state manager present while listening.");
        let tv_state = self.tv_state.clone();
        let mut process = self.process.lock().expect("could not lock process");

        process
            .with_output(move |line| {
                println!("got line from stdout: {}", line);
                if line.starts_with("power status:") {
                    let state_string = &line[12..];
                    let mqtt_state = match state_string {
                        "on" => "ON",
                        "off" => "OFF",
                        _ => "UNKNOWN",
                    };
                    println!("parsed power status: {mqtt_state} from section {state_string} of line {line}");

                    tv_state
                        .lock()
                        .expect("could not get lock")
                        .replace(mqtt_state.to_string());
                    state.update_state(mqtt_state.to_string());
                }
            })
            .expect("could not start listening process");
    }

    pub fn volume_up(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("volup\n").unwrap();
    }

    pub fn set_tv(&self, state: bool) {
        let mut process = self.process.lock().expect("could not lock process");
        if state {
            process.send("on 0.0.0.0\n").unwrap();
        } else {
            process.send("standby 0.0.0.0\n").unwrap();
        }
    }

    pub fn volume_down(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("voldown\n").unwrap();
    }

    pub fn query_tv_state(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("pow 0.0.0.0\n").unwrap();
    }
}
