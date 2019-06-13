// Copyright (c) 2019 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of gtp-rs. See README.md and COPYING for details.

/*!
This module provides the abstraction of a GTP engine controller.

See also [`Engine`](struct.Engine.html) for more information.
*/

const WAIT_POLL_DIV : u32 = 4;

/// This represents the controller of an GTP Engine.
///
/// You establish a connection like this:
/// ```
/// use std::time::Duration;
/// use gtp::Command;
/// use gtp::controller::Engine;
///
/// let mut ctrl = Engine::new("/usr/bin/gnugo", &["--mode", "gtp"]);
/// assert!(ctrl.start().is_ok());
///
/// ctrl.send(Command::cmd("name", |e| e));
/// let resp = ctrl.wait_response(Duration::from_millis(500)).unwrap();
/// let ev = resp.entities(|ep| ep.s().s()).unwrap();
/// assert_eq!(ev[0].to_string(), "GNU");
/// assert_eq!(ev[1].to_string(), "Go");
/// assert_eq!(resp.text(), "GNU Go");
/// ```
pub struct Engine {
    cur_id:     u32,
    cmd:        String,
    rp:         super::ResponseParser,
    args:       Vec<String>,
    handle:     Option<super::detached_command::DetachedCommand>,
    stderr:     String,
}

/// Error as returned by this module.
#[derive(Debug)]
pub enum Error {
    /// This error is forwarded from the `detached_command`
    /// module, it's about running the engine process.
    ProcessError(super::detached_command::Error),
    /// This is an error when parsing responses from the engine.
    /// It might indicate either a bug in this crate or the
    /// Engine.
    ProtocolError(super::ResponseError),
    /// Returned when no engine has been `start()`ed.
    NoHandle,
    /// Returned when no responses were received from the engine yet.
    /// It means you have to call methods like `poll_response()` or `wait_response()`
    /// again.
    PollAgain,
}

impl Engine {
    /// Creates a new Engine instance with the path
    /// to the engine binary and the arguments to pass to
    /// the engine.
    pub fn new(cmd: &str, args: &[&str]) -> Engine {
        Engine {
            cmd:    cmd.to_string(),
            rp:     super::ResponseParser::new(),
            cur_id: 0,
            args:   args.iter().map(|s| s.to_string()).collect(),
            handle: None,
            stderr: String::from(""),
        }
    }

    /// Starts the engine in the background.
    pub fn start(&mut self) -> Result<(), Error> {
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

    /// Sends a command to the engine. Returns the
    /// ID of the command.
    pub fn send(&mut self, mut cmd: super::Command) -> u32 {
        if self.handle.is_none() { return 0; }

        self.cur_id += 1;
        cmd.set_id(self.cur_id);
        let cmd_buf = cmd.to_bytes();
        self.handle.as_mut().unwrap().send(cmd_buf);
        self.cur_id
    }

    /// Returns the currently captured stderr output of the engine.
    #[allow(dead_code)]
    pub fn stderr(&self) -> String { self.stderr.clone() }

    /// Clears the up to now returned output of the engine.
    #[allow(dead_code)]
    pub fn clear_stderr(&mut self) { self.stderr = String::from(""); }

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

#[allow(unused_imports)]
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
