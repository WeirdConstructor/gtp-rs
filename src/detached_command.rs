// Copyright (c) 2019 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of gtp-rs. See README.md and COPYING for details.

/*!
An abstraction for running background processes with a line based
I/O protocol on stdin/stdout.

It uses 3 threads for I/O of the child stdin/stdout/stderr and
some std::sync::mpsc channels for synchronization. It's not the
fastest as `tokio_process` would provide a non-blocking
interface to child processes (which is hopefully more
efficiently implemented).

Unfortunately at the time of this writing I only have prototype code for
tokio_process and the futures don't really make the solution easier to read and
maintain than this one. I also believe, that the bottleneck of todays GTP
engines is not the interface with the GTP controller. So this might
never gets optimized.
*/

use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::io::Write;
use std::io::BufRead;
use std::thread;

use super::ResponseParser;

#[derive(Debug, Clone)]
pub enum CapturedOutput {
    Stderr(String),
    Stdout(String),
}

pub struct DetachedCommand {
    child:          std::process::Child,
    reader:         Option<std::thread::JoinHandle<()>>,
    err_reader:     Option<std::thread::JoinHandle<()>>,
    writer:         Option<std::thread::JoinHandle<()>>,
    rd_rx:          Option<mpsc::Receiver<CapturedOutput>>,
    wr_tx:          Option<mpsc::Sender<Vec<u8>>>,
    stdout_chunks:  Vec<String>,
    stderr_chunks:  Vec<String>,
}

#[derive(Debug)]
pub enum Error {
    StartupFailed(std::io::Error),
    Disconnected,
}

impl DetachedCommand {
    pub fn start(cmd: &str, args: &[&str]) -> Result<DetachedCommand, Error> {
        let mut o = Command::new(cmd);
        o.stdout(Stdio::piped())
         .stderr(Stdio::piped())
         .stdin(Stdio::piped());

        for arg in args.iter() {
            o.arg(arg);
        }

        let o = o.spawn();

        if let Err(io_err) = o {
            return Err(Error::StartupFailed(io_err));
        }

        let mut o = o.unwrap();

        let stdin    = o.stdin.take().unwrap();
        let stdout   = o.stdout.take().unwrap();
        let stderr   = o.stderr.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (stdin_tx , stdin_rx) : (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = std::sync::mpsc::channel();

        let writer = thread::spawn(move || {
            let mut bw = std::io::BufWriter::new(stdin);
            while let Ok(bytes) = stdin_rx.recv() {
                if let Ok(s) = bw.write(&bytes) {
                    if s == 0 { break; }
                    if bw.flush().is_err() { break; }
                } else {
                    break;
                }
            };
        });

        let tx_stdout = tx.clone();
        let reader = thread::spawn(move || {
            let mut br = std::io::BufReader::new(stdout);
            loop {
                let mut line = String::from("");
                if let Ok(s) = br.read_line(&mut line) {
                    if tx_stdout.send(CapturedOutput::Stdout(line)).is_err() { break; }
                    if s == 0 { break; }
                } else {
                    break;
                }
            }
        });

        let tx_stderr = tx.clone();
        let err_reader = thread::spawn(move || {
            let mut br = std::io::BufReader::new(stderr);
            loop {
                let mut line = String::from("");
                if let Ok(s) = br.read_line(&mut line) {
                    if tx_stderr.send(CapturedOutput::Stderr(line)).is_err() { break; }
                    if s == 0 { break; }
                } else {
                    break;
                }
            }
        });

        Ok(DetachedCommand {
            child:              o,
            stderr_chunks:      Vec::new(),
            stdout_chunks:      Vec::new(),
            reader:             Some(reader),
            err_reader:         Some(err_reader),
            writer:             Some(writer),
            rd_rx:              Some(rx),
            wr_tx:              Some(stdin_tx),
        })
    }

    pub fn send_str(&mut self, s: &str) {
        let b : Vec<u8> = s.bytes().collect();
        self.send(b);
//        dc.wr_tx.as_ref().unwrap().send("foobar!\n".to_string());
    }

    #[allow(unused_must_use)]
    pub fn send(&mut self, buffer: Vec<u8>) {
        self.wr_tx.as_ref().unwrap().send(buffer);
    }

    #[allow(dead_code)]
    pub fn recv_blocking(&mut self) -> CapturedOutput {
        self.rd_rx.as_ref().unwrap().recv().unwrap()
    }

    pub fn stdout_available(&self) -> bool {
        !self.stdout_chunks.is_empty()
    }

    pub fn stderr_available(&self) -> bool {
        !self.stderr_chunks.is_empty()
    }

    pub fn recv_stdout(&mut self) -> String {
        let ret : String = self.stdout_chunks.join("");
        self.stdout_chunks.clear();
        ret
    }

    pub fn recv_stderr(&mut self) -> String {
        let ret : String = self.stderr_chunks.join("");
        self.stderr_chunks.clear();
        ret
    }

    pub fn poll(&mut self) -> Result<(), Error>  {
        if self.rd_rx.is_none() {
            return Err(Error::Disconnected);
        }

        loop {
            match self.rd_rx.as_ref().unwrap().try_recv() {
                Ok(CapturedOutput::Stdout(input)) => {
                    self.stdout_chunks.push(input);
                },
                Ok(CapturedOutput::Stderr(input)) => {
                    self.stderr_chunks.push(input);
                },
                Err(mpsc::TryRecvError::Empty) => {
                    return Ok(());
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(Error::Disconnected);
                },
            }
        }
    }

    #[allow(unused_must_use)]
    pub fn shutdown(&mut self) {
        drop(self.wr_tx.take().unwrap());
        self.child.kill();
        self.writer.take().unwrap().join();
        self.reader.take().unwrap().join();
        self.err_reader.take().unwrap().join();
    }
}

pub fn doit() {
    println!("FOO {}", std::env::current_dir().unwrap().to_str().unwrap());
    let mut dc =
        DetachedCommand::start("gnugo-3.8\\gnugo.exe", &["--mode", "gtp"])
        .expect("failed gnugo");

    let mut rp = self::ResponseParser::new();

    dc.send_str("10 list_commands\n");
    loop {
        let p = dc.poll();
        if p.is_err() {
            println!("stdout: [{}]", dc.recv_stdout());
            println!("stderr: [{}]", dc.recv_stderr());
            println!("Error in poll: {:?}", p.unwrap_err());
            break;

        }
        if dc.stderr_available() {
            println!("err: {}", dc.recv_stderr());
        }

        if dc.stdout_available() {
            rp.feed(&dc.recv_stdout());

            if let Ok(resp) = rp.get_response() {
                match resp.id_0() {
                    10 => {
                        let ents = resp.entities(|ep| { while !ep.is_eof() { ep.s(); } ep }).unwrap();
                        for cmd in ents.iter() {
                            println!("command {}", cmd.to_string());
                        }
                        dc.send_str("11 showboard\n");
                    },
                    11 => {
                        println!("board: {}", resp.text());
                        dc.send_str("12 genmove w\n");
                    },
                    12 => {
                        println!("Vertex: {:?}", resp.entities(|ep| ep.vertex()).unwrap()[0]);
                        dc.send_str("quit\n");
                    },
                    _ => {
                        println!("resp: {}", resp.text());
                        dc.send_str("quit\n");
                    },
                }
            }
        }
    }

    dc.shutdown();
}
