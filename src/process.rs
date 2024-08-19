use std::{
    io::{BufRead, BufReader, Stdin, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread::{self, ThreadId},
};

pub struct CommandProcess {
    input: ChildStdin,
    output: Option<ChildStdout>,
    child: Child,
}

impl CommandProcess {
    pub fn new(mut command: Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("could not open process");

        return Self {
            input: child.stdin.take().unwrap(),
            output: child.stdout.take(),
            child: child,
        };
    }

    pub fn send(&mut self, input: &str) -> Result<usize, std::io::Error> {
        return self.input.write(input.as_bytes());
    }

    pub fn with_output<F: 'static + Fn(String) -> () + Send>(
        &mut self,
        func: F,
    ) -> Result<(), &str> {
        if self.output.is_some() {
            println!("spawning reader thread...");
            let reader = BufReader::new(self.output.take().unwrap());
            let thread = thread::spawn(|| {
                reader.lines().filter_map(|line| line.ok()).for_each(func);
            });
            return Ok(());
        } else {
            return Err("Can not read from output twice! output is already taken!");
        }
    }
}
