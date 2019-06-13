// Copyright (c) 2019 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of gtp-rs. See README.md and COPYING for details.

mod detached_command;
use detached_command::*;

#[derive(Debug)]
pub struct Engine {
    cur_id:     u32,
    cmd:        String,
    rp:         gtp::ResponseParser,
    args:       Vec<String>,
    handle:     Option<detached_command::DetachedCommand>,
    stderr:     String,
}

#[derive(Debug)]
pub enum Error {
    ProcessError(DetachedCommand),
    ProtocolError(detached_command::Error),
}

impl Engine {
    fn new(cmd: &str, args: &[&str]) -> Engine {
        Engine {
            cmd,
            rp:     gtp::ResponseParser::new(),
            cur_id: 0,
            args:   args.iter().collect(),
            handle: None,
        }
    }

    fn start(&mut self) -> Result<(), Error> {
        if self.handle.is_some() {
            self.handle.shutdown();
            self.handle = None;
        }

        match DetachedCommand::start(self.cmd, self.args) {
            Ok(hdl) => {
                self.handle = Some(hdl);
            },
            Err(e) => {
                return Err(Error::ProcessError(e));
            }
        }
    }

    fn send(&self, cmd: gtp::Command) -> u32 {
        if self.handle.is_none() { return 0; }

        self.cur_id += 1;
        cmd.set_id(self.cur_id);
        let cmd_buf = cmd.to_bytes();
        self.handle.as_ref().unwrap().send(cmd_buf);
        self.cur_id
    }

    fn clear_stderr(&mut self) { self.stderr = String::from(""); }
    fn stderr(&self) -> String { self.stderr }

    fn poll_response(&mut self) -> Result<gtp::Response, Error> {
        let p = dc.poll();
        if p.is_err() {
            return Err(Error::ProcessError);
            println!("stdout: [{}]", dc.recv_stdout());
            println!("stderr: [{}]", dc.recv_stderr());
            println!("Error in poll: {:?}", p.unwrap_err());
            break;

        }

        if dc.stderr_available() {
            self.stderr += &dc.recv_stderr();
            println!("err: {}", self.stderr);
        }

        if dc.stdout_available() {
            self.rp.feed(&dc.recv_stdout());

            if let Ok(resp) = self.rp.get_response() {
                return Ok(resp);
            }
        }
    }
}

