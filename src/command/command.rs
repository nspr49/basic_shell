pub mod cmd {
    use home::home_dir;
    use nix::{
        libc::{
            dup, dup2, execvpe, exit, getpgid, getpid, kill, killpg, STDIN_FILENO, STDOUT_FILENO,
        },
        sys::{
            signal::{sigprocmask, SigSet, SigmaskHow, Signal},
            wait::{self, wait, waitpid, WaitStatus},
        },
        unistd::{
            close, execv, execvp, fork, gettid, pipe, setpgid, tcgetpgrp, tcsetpgrp, ForkResult,
            Pid,
        },
    };
    use std::{
        env,
        ffi::CString,
        fs::OpenOptions,
        io,
        ops::{Deref, DerefMut},
        os::fd::{AsRawFd, IntoRawFd, OwnedFd},
        path::Path,
        process::{Child, Stdio},
    };
    pub enum Command {
        SimpleCommand(SimpleCommand),
        CommandList(CommandList),
        NoCommand,
    }

    pub struct NoCommand {}

    pub struct SimpleCommand {
        pub command: String,
        pub args: Vec<String>,
        pub background: bool,
    }

    pub struct CommandList {
        pub commands: Vec<SimpleCommand>,
        pub kind: CommandListType,
    }

    #[derive(PartialEq, Eq)]
    pub enum CommandListType {
        AND,
        OR,
        PIPE,
    }

    pub trait Execute {
        fn execute(&self, processList: &mut Vec<ProcessStatus>);
    }

    pub struct ProcessStatus {
        name: String,
        pid: u32,
        status: i32,
    }

    impl Command {
        pub fn execute(&self, processList: &mut Vec<ProcessStatus>) {
            match &self {
                &Self::CommandList(cmd) => {
                    cmd.execute(processList);
                }
                &Self::SimpleCommand(cmd) => {
                    cmd.execute(processList);
                }
                &Self::NoCommand => {}
            }
        }
    }

    impl Execute for SimpleCommand {
        fn execute(&self, processList: &mut Vec<ProcessStatus>) {
            match self.command.as_str() {
                "cd" => match self.args.len() {
                    0 => {
                        let res = std::env::set_current_dir(home_dir().unwrap());
                        if res.is_err() {
                            println!("Couln't change directory");
                        }
                    }
                    1 => {
                        let res = std::env::set_current_dir(Path::new(&self.args.first().unwrap()));
                        if res.is_err() {
                            println!("Couln't change directory");
                        }
                    }
                    arg => println!("cd only takes one argument but {} were provided ", arg),
                },
                "exit" => std::process::exit(0),

                _ => {
                    let tty = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open("/dev/tty")
                        .expect("err");
                    let fg_pid = tcgetpgrp(&tty).unwrap();
                    execute_simple(&tty, fg_pid, self, self.background);
                }
            }
        }
    }

    fn execute_simple(
        tty: &std::fs::File,
        fg_pid: Pid,
        cmd: &SimpleCommand,
        background: bool,
    ) -> i32 {
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let mut mask = SigSet::empty();
                mask.add(Signal::SIGSTOP);
                mask.add(Signal::SIGKILL);
                mask.add(Signal::SIGTTOU);
                mask.add(Signal::SIGINT);
                nix::sys::signal::sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&mask), None).unwrap();
                let cstring = CString::new(cmd.command.as_str()).unwrap();
                let mut args: Vec<CString> = Vec::new();
                for arg in cmd.args.iter() {
                    args.push(CString::new(arg.as_str()).unwrap());
                }
                execvp(&cstring, &args).unwrap();

                unsafe {
                    exit(1);
                }
            }
            Ok(ForkResult::Parent { child }) => {
                nix::unistd::setpgid(child, child).unwrap();
                if !background {
                    tcsetpgrp(&tty, child).unwrap();
                    let status: WaitStatus = waitpid(child, None).unwrap();
                    tcsetpgrp(&tty, fg_pid).unwrap();
                    return match status {
                        WaitStatus::Exited(_, code) => code,
                        _ => 1,
                    };
                }
                return 0;
            }
            Err(_) => {
                println!("child");
                return 1;
            }
        }
    }

    impl Execute for CommandList {
        fn execute(&self, processList: &mut Vec<ProcessStatus>) {
            let tty = OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .expect("err");
            let fg_pid = tcgetpgrp(&tty).unwrap();
            match self.kind {
                CommandListType::OR => {
                    for cmd in self.commands.iter() {
                        if execute_simple(&tty, fg_pid, cmd, false) == 0 {
                            break;
                        }
                    }
                }
                CommandListType::AND => {
                    for cmd in self.commands.iter() {
                        if execute_simple(&tty, fg_pid, cmd, false) != 0 {
                            break;
                        }
                    }
                }
                CommandListType::PIPE => {
                    let mut cmds: Vec<Pid> = Vec::new();
                    let mut fds: Vec<(OwnedFd, OwnedFd)> = Vec::new();
                    for _ in self.commands.iter() {
                        fds.push(pipe().unwrap());
                    }
                    for (i, command) in self.commands.iter().enumerate() {
                        match unsafe { fork() } {
                            Ok(ForkResult::Child) => {
                                let mut mask = SigSet::empty();
                                mask.add(Signal::SIGSTOP);
                                mask.add(Signal::SIGKILL);
                                mask.add(Signal::SIGTTOU);
                                mask.add(Signal::SIGINT);
                                nix::sys::signal::sigprocmask(
                                    SigmaskHow::SIG_UNBLOCK,
                                    Some(&mask),
                                    None,
                                )
                                .unwrap();
                                let cstring = CString::new(command.command.as_str()).unwrap();
                                let mut args: Vec<CString> = Vec::new();
                                for arg in command.args.iter() {
                                    args.push(CString::new(arg.as_str()).unwrap());
                                }
                                if i == 0 {
                                    unsafe {
                                        dup2(fds.first().unwrap().1.as_raw_fd(), STDOUT_FILENO);
                                    }
                                } else if i == &self.commands.len() - 1 {
                                    unsafe {
                                        dup2(fds.get(i - 1).unwrap().0.as_raw_fd(), STDIN_FILENO);
                                    }
                                } else {
                                    unsafe {
                                        dup2(fds.get_mut(i).unwrap().1.as_raw_fd(), STDOUT_FILENO);
                                        dup2(
                                            fds.get_mut(i - 1).unwrap().0.as_raw_fd(),
                                            STDIN_FILENO,
                                        );
                                    }
                                }
                                drop(fds);
                                execvp(&cstring, &args).unwrap();
                            }
                            Ok(ForkResult::Parent { child }) => {
                                cmds.push(child);
                                setpgid(child, *cmds.first().unwrap())
                                    .expect("Error settng process group");
                            }
                            Err(_) => {}
                        }
                    }
                    // Owned fds are closed upon fds being freed, so closing them manually
                    // causes a double close hence we drop fds here, which closes them
                    drop(fds);

                    tcsetpgrp(&tty, *cmds.first().unwrap());
                    for pid in cmds.iter() {
                        waitpid(*pid, None).unwrap();
                    }
                    tcsetpgrp(&tty, fg_pid).unwrap();
                }
            }
        }
    }
}
