use std::{
    error::Error,
    io::{self, stdin, Write},
    sync::{Arc, Mutex},
    thread,
};

use command::command::cmd::{self, Command, ProcessStatus, SimpleCommand};
use nix::{
    libc::{WIFEXITED, WNOHANG},
    sys::{
        signal::{
            SigSet, SigmaskHow,
            Signal::{self},
        },
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};
use once_cell::sync::Lazy;
use signal_hook::{iterator::exfiltrator::WithOrigin, low_level::siginfo::Process};

mod command;

static PROCESS_LIST: Lazy<Arc<Mutex<Vec<ProcessStatus>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

fn main() -> Result<(), Box<dyn Error>> {
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGSTOP);
    mask.add(Signal::SIGKILL);
    mask.add(Signal::SIGTTOU);
    mask.add(Signal::SIGINT);
    nix::sys::signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None);

    thread::spawn(move || {
        let mut signals: signal_hook::iterator::SignalsInfo<WithOrigin> =
            signal_hook::iterator::SignalsInfo::<WithOrigin>::new(&[nix::libc::SIGCHLD])
                .expect("Failed to create signal handler");

        for sig_info in signals.forever() {
            if sig_info.signal == nix::libc::SIGCHLD {
                let process = sig_info.process.unwrap();
                let mut process_list = PROCESS_LIST.lock().unwrap();
                for (index, i) in process_list.iter().enumerate() {
                    if i.pid.as_raw() == sig_info.process.unwrap().pid {
                        match waitpid(Pid::from_raw(process.pid), Some(WaitPidFlag::WNOHANG)) {
                            Ok(WaitStatus::Exited(_, _)) => {
                                process_list.remove(index);
                            }
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        break;
                    }
                }
            }
        }
    });

    loop {
        print!("bshell> ");
        io::stdout().flush().unwrap();
        let mut s = String::new();
        stdin().read_line(&mut s).expect("err");
        let cmd: Command = command::parse::parse(&mut s);
        cmd.execute()
    }
}
