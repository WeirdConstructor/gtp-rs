// Copyright (c) 2019 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of gtp-rs. See README.md and COPYING for details.

const WAIT_POLL_DIV : u32 = 4;

pub struct Engine {
    cur_id:     u32,
    cmd:        String,
    rp:         super::ResponseParser,
    args:       Vec<String>,
    handle:     Option<super::detached_command::DetachedCommand>,
    stderr:     String,
}

#[derive(Debug)]
pub enum Error {
    ProcessError(super::detached_command::Error),
    ProtocolError(super::ResponseError),
    NoHandle,
    PollAgain,
}

impl Engine {
    fn new(cmd: &str, args: &[&str]) -> Engine {
        Engine {
            cmd:    cmd.to_string(),
            rp:     super::ResponseParser::new(),
            cur_id: 0,
            args:   args.iter().map(|s| s.to_string()).collect(),
            handle: None,
            stderr: String::from(""),
        }
    }

    fn start(&mut self) -> Result<(), Error> {
        if self.handle.is_some() {
            self.handle.as_mut().unwrap().shutdown();
            self.handle = None;
        }

        let sl : Vec<&str> = self.args.iter().map(|s| &s[..]).collect();

        match super::detached_command::DetachedCommand::start(&self.cmd, &sl[..]) {
            Ok(hdl) => {
                self.handle = Some(hdl);
                return Ok(());
            },
            Err(e) => {
                return Err(Error::ProcessError(e));
            }
        }
    }

    fn send(&mut self, mut cmd: super::Command) -> u32 {
        if self.handle.is_none() { return 0; }

        self.cur_id += 1;
        cmd.set_id(self.cur_id);
        let cmd_buf = cmd.to_bytes();
        self.handle.as_mut().unwrap().send(cmd_buf);
        self.cur_id
    }

    #[allow(dead_code)]
    fn clear_stderr(&mut self) { self.stderr = String::from(""); }
    #[allow(dead_code)]
    fn stderr(&self) -> String { self.stderr.clone() }

    /// This method waits for a maximum amount of time for a response
    /// from the GTP engine.
    ///
    /// If no response was received in the given time `Error::PollAgain`
    /// is returned.
    pub fn wait_response(&mut self, timeout: std::time::Duration) -> Result<super::Response, Error> {
        let interval = timeout.checked_div(WAIT_POLL_DIV).unwrap();
        let instant = std::time::Instant::now();

        loop {
            match self.poll_response() {
                Ok(resp)              => return Ok(resp),
                Err(Error::PollAgain) => (),
                Err(e)                => return Err(e),
            }

            if instant.elapsed() > timeout {
                return Err(Error::PollAgain);
            }

            std::thread::sleep(interval);
        }
    }

    /// This method polls once for a response from the GTP engine.
    ///
    /// If no response was found `Error::PollAgain` is returned.
    pub fn poll_response(&mut self) -> Result<super::Response, Error> {
        if self.handle.is_none() { return Err(Error::NoHandle); }

        let hdl = self.handle.as_mut().unwrap();

        let p = hdl.poll();
        if p.is_err() {
            return Err(Error::ProcessError(p.unwrap_err()));
        }

        if hdl.stderr_available() {
            self.stderr += &hdl.recv_stderr();
            println!("err: {}", self.stderr);
        }

        if hdl.stdout_available() {
            self.rp.feed(&hdl.recv_stdout());

            if let Ok(resp) = self.rp.get_response() {
                return Ok(resp);
            }
        }

        return Err(Error::PollAgain);
    }
}

use super::Command;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_gnugo_version() {
        let mut ctrl = Engine::new("/usr/bin/gnugo", &["--mode", "gtp"]);

        assert!(ctrl.start().is_ok());

        ctrl.send(Command::cmd("name", |e| e));
        let resp = ctrl.wait_response(std::time::Duration::from_millis(500)).unwrap();
        let ev = resp.entities(|ep| ep.s().s()).unwrap();
        assert_eq!(ev[0].to_string(), "GNU");
        assert_eq!(ev[1].to_string(), "Go");
        assert_eq!(resp.text(), "GNU Go");
    }

}
