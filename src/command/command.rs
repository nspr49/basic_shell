pub mod cmd {
    use home::home_dir;
    use std::{
        env,
        ops::{Deref, DerefMut},
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

                (_) => {
                    let mut proc = std::process::Command::new(&self.command)
                        .args(&self.args)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .spawn();
                    if let Ok(mut process) = proc {
                        if let Ok(mut res) = process.wait() {
                            if !res.success() {
                                println!("{}", res);
                            }
                        }
                    } else {
                        if let Err(mut process) = proc {
                            println!("{}", process);
                        }
                    }
                }
            }
        }
    }

    impl Execute for CommandList {
        fn execute(&self, processList: &mut Vec<ProcessStatus>) {
            match self.kind {
                CommandListType::OR => {
                    for cmd in self.commands.iter() {
                        let mut proc = std::process::Command::new(&cmd.command)
                            .args(&cmd.args)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .spawn();
                        if let Ok(mut cmd) = proc {
                            let res = cmd.wait().unwrap();
                            if res.success() {
                                break;
                            }
                        }
                    }
                }
                CommandListType::AND => {
                    for cmd in self.commands.iter() {
                        let mut proc = std::process::Command::new(&cmd.command)
                            .args(&cmd.args)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .spawn();
                        if let Ok(mut cmd) = proc {
                            let res = cmd.wait().unwrap();
                            if !res.success() {
                                break;
                            }
                        }
                    }
                }
                CommandListType::PIPE => {
                    let mut cmds: Vec<Child> = Vec::new();
                    for (i, cmd) in self.commands.iter().enumerate() {
                        if i == 0 {
                            let mut child = std::process::Command::new(&cmd.command)
                                .args(&cmd.args)
                                .stdin(Stdio::inherit())
                                .stdout(Stdio::piped())
                                .spawn();
                            if let Ok(proc) = child {
                                cmds.push(proc)
                            } else {
                                cmds.iter_mut().for_each(move |p| p.kill().unwrap());
                                println!("Failed at {}", cmd.command);
                                break;
                            }
                        } else if i == self.commands.len() - 1 {
                            let mut child = std::process::Command::new(&cmd.command)
                                .args(&cmd.args)
                                .stdin(Stdio::from(
                                    cmds[i - 1]
                                        .stdout
                                        .take()
                                        .expect("Who took my filedescriptor???"),
                                ))
                                .stdout(Stdio::inherit())
                                .spawn();
                            if let Ok(process) = child {
                                cmds.push(process);
                            } else {
                                println!("failed to execute {}", cmd.command);
                                cmds.iter_mut().for_each(|p| p.kill().unwrap());
                                break;
                            }
                        } else {
                            let mut child = std::process::Command::new(&cmd.command)
                                .args(&cmd.args)
                                .stdin(Stdio::from(cmds[i - 1].stdout.take().unwrap()))
                                .stdout(Stdio::piped())
                                .spawn();
                            if let Ok(process) = child {
                                cmds.push(process);
                            } else {
                                cmds.iter_mut().for_each(|p| p.kill().unwrap());
                                println!(
                                    "Failed to execute 
                                    {}",
                                    cmd.command
                                )
                            }
                        }
                    }
                    for cmd in cmds.iter_mut() {
                        cmd.wait().unwrap();
                    }
                }
            }
        }
    }
}
