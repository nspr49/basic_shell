use std::{
    error::Error,
    io::{self, stdin, Read, Write},
    thread,
};

use command::command::cmd::{self, Command, Execute, ProcessStatus, SimpleCommand};
use signal_hook::{consts::SIGINT, iterator::Signals};

mod command;

fn main() -> Result<(), Box<dyn Error>> {
    let mut signals = Signals::new([SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    println!("Sigterm received");
                }
                (_) => {
                    println!("Signal");
                }
            }
        }
    });
    let mut processList: Vec<ProcessStatus> = Vec::new();
    loop {
        print!("bshell> ");
        io::stdout().flush().unwrap();
        let mut s = String::new();
        stdin().read_line(&mut s).expect("err");
        let cmd: Command = command::parse::parse(&mut s);
        cmd.execute(&mut processList)
    }
    Ok(())
}
