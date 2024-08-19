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
    tv_state: Arc<Mutex<Option<bool>>>,
}

impl HdmiCecProcess {
    pub fn new() -> Self {
        let command = Command::new("cat");
        let process = CommandProcess::new(command);
        return Self {
            process: Mutex::new(process),
            state: Mutex::new(None),
            tv_state: Arc::new(Mutex::new(Some(false))),
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
        let state = self.state.lock().expect("could not lock state").clone();
        let tv_state = self.tv_state.clone();
        let mut process = self.process.lock().expect("could not lock process");

        process
            .with_output(move |line| {
                println!("got line from stdout: {}", line);
                let line = "parse line...";
                if line == "the status of the TV is xxxxxx" {
                    tv_state.lock().expect("could not get lock").replace(true);
                }
            })
            .expect("could not start listening process");
    }

    pub fn volume_up(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("volup 0.0.0.0\n").unwrap();
    }

    pub fn set_tv(&self, state: bool) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("volup 0.0.0.0\n").unwrap();
    }

    pub fn volume_down(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("voldown 0.0.0.0\n").unwrap();
    }

    pub fn query_tv_state(&self) {
        let mut process = self.process.lock().expect("could not lock process");
        process.send("status 0.0.0.0\n").unwrap();
    }
}
