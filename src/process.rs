use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread::{self},
};

use anyhow::Context;
use log::debug;

pub struct CommandProcess {
    input: ChildStdin,
    output: Option<ChildStdout>,
    // we need a place to keep the child process reference. not sure what happens if we drop it.
    #[allow(dead_code)]
    child: Child,
}

impl CommandProcess {
    pub fn new(command: &mut Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .with_context(|| format!("{:?}", command.get_program()))
            .expect("could not open process");

        return Self {
            input: child.stdin.take().unwrap(),
            output: child.stdout.take(),
            child: child,
        };
    }

    pub fn send(&mut self, input: &str) -> Result<usize, std::io::Error> {
        debug!("sending to process: {}", input);
        return self.input.write(input.as_bytes());
    }

    pub fn with_output<F: 'static + FnMut(String) -> () + Send>(
        &mut self,
        func: F,
    ) -> Result<(), &str> {
        if self.output.is_some() {
            debug!("spawning reader thread...");
            let reader = BufReader::new(self.output.take().unwrap());
            thread::spawn(|| {
                reader.lines().filter_map(|line| line.ok()).for_each(func);
            });
            return Ok(());
        } else {
            return Err("Can not read from output twice! output is already taken!");
        }
    }
}

#[test]
fn read_output() {
    let mut process = CommandProcess::new(Command::new("echo").arg("Hello World!"));

    process.with_output(|line| {
        assert_eq!(line, "Hello World!\n");
    });
}

#[test]
fn send_input_and_read_output() {
    use std::cell::Cell;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // make a new process using cat: we will send some input, and expect the same
    // content as output to be read.
    let mut process = CommandProcess::new(&mut Command::new("cat"));

    let lines_read_cell = Arc::new(Mutex::new(Cell::new(0 as usize)));
    let lines_read_clone = lines_read_cell.clone();

    // setup the listener, with our expectations.
    // we will count the number of lines read, so
    // that we know we got some input.
    let mut lines_read = 0;
    process.with_output(move |line| {
        assert_eq!(line, "Hello World!");
        lines_read = lines_read + 1;

        let lines = lines_read_clone.lock().expect("could not take lock");
        lines.replace(lines_read);
    });

    // send a known input.
    process
        .send("Hello World!\n")
        .expect("could not send message");

    // wait for a little bit, for the reader thread to process
    // the input, and gather the output from the process.
    thread::sleep(Duration::from_millis(500));

    // and finally, read the number of lines, and ensure that we
    // read the correct number of lines.
    let lines_read_usize: usize = lines_read_cell.lock().expect("could not take lock").take();
    assert_eq!(lines_read_usize, 1);
}
