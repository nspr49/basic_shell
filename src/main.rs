use std::{
    error::Error,
    io::{self, stdin, Read, Write},
    os::fd::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use command::command::cmd::{self, Command, Execute, ProcessStatus, SimpleCommand};
use nix::{
    libc::{self, wait, WNOHANG, WNOWAIT},
    sys::{
        signal::{
            self, sigaction, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow,
            Signal::{self, SIGCHLD},
        },
        wait::{self, waitpid, WaitPidFlag},
    },
    unistd::Pid,
};
use once_cell::sync::Lazy;
use signal_hook::{
    consts::{SIGINT, SIGSTOP},
    iterator::{exfiltrator::WithOrigin, Signals, SignalsInfo},
};

mod command;

static PROCESS_LIST: Lazy<Arc<Mutex<Vec<ProcessStatus>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

extern "C" fn handle_sigchild(
    sig: libc::c_int,
    info: *mut libc::siginfo_t,
    _context: *mut libc::c_void,
) {
    let list = PROCESS_LIST.lock().unwrap();
    for item in list.iter() {}
    /*
    unsafe {
        if !info.is_null() {
            let pid = (*info).si_pid(); // PID of the process that sent the signal
            let mut code: i32;
            waitpid(Pid::from_raw(pid), Some(WaitPidFlag::WNOHANG)).unwrap();
        }
    }
    */
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGSTOP);
    mask.add(Signal::SIGKILL);
    mask.add(Signal::SIGTTOU);
    mask.add(Signal::SIGINT);
    nix::sys::signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None);

    let sig_action = SigAction::new(
        SigHandler::SigAction(handle_sigchild), // Use SigAction to access siginfo
        SaFlags::SA_SIGINFO,                    // SA_SIGINFO gives access to siginfo_t
        SigSet::empty(),                        // No additional blocked signals
    );

    unsafe {
        signal::sigaction(Signal::SIGCHLD, &sig_action).expect("Failed to set SIGCHLD handler");
    }

    thread::spawn(move || {
        let mut signals: signal_hook::iterator::SignalsInfo<WithOrigin> =
            signal_hook::iterator::SignalsInfo::<WithOrigin>::new(&[nix::libc::SIGCHLD])
                .expect("Failed to create signal handler");

        for sig_info in signals.forever() {
            if sig_info.signal == nix::libc::SIGCHLD {}
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
