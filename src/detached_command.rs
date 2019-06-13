use std::process::Command;
//use std::process::Child;
use std::process::Stdio;
use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
//use std::io::BufWriter;
//use std::io::BufReader;
use std::io::Write;
use std::io::Read;
use std::io::BufRead;
use std::thread;

pub struct DetachedCommand {
    child:  std::process::Child,
    reader: Option<std::thread::JoinHandle<()>>,
    writer: Option<std::thread::JoinHandle<()>>,
    rd_rx:  Option<mpsc::Receiver<String>>,
    wr_tx:  Option<mpsc::Sender<Vec<u8>>>,
    on_stdout: Option<Box<Fn(&str)>>,
    on_stderr: Option<Box<Fn(&str)>>,
}

#[derive(Debug)]
enum Error {
    StartupFailed(std::io::Error),
    Disconnected,
}

impl DetachedCommand {
    fn start(cmd: &str, args: &Vec<&str>) -> Result<DetachedCommand, Error> {
        let mut o = Command::new(cmd);
        o.stdout(Stdio::piped())
         .stdin(Stdio::piped());

        for arg in args.iter() {
            o.arg(arg);
        }

        let mut o = o.spawn();

        if let Err(io_err) = o {
            return Err(Error::StartupFailed(io_err));
        }

        let mut o = o.unwrap();

        let stdin    = o.stdin.take().unwrap();
        let stdout   = o.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (stdin_tx , stdin_rx) : (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = std::sync::mpsc::channel();

        let writer = thread::spawn(move || {
            let mut bw = std::io::BufWriter::new(stdin);
            let mut i = 0;
            loop {
                match stdin_rx.recv() {
                    Ok(bytes) => {
                        if let Ok(s) = bw.write(&bytes) {
                            if s == 0 { break; }
                            if let Err(_) = bw.flush() { break; }
                        } else {
                            break;
                        }
                    },
                    Err(_) => break,
                }
            };
        });

        let reader = thread::spawn(move || {
            let mut br = std::io::BufReader::new(stdout);
            let mut line_cnt = 0;
            loop {
                let mut line = String::from("");
                if let Ok(s) = br.read_line(&mut line) {
                    if let Err(_) = tx.send(line) { break; }
                    if s == 0 { break; }
                } else {
                    break;
                }
            }
        });

        Ok(DetachedCommand {
            child:      o,
            on_stdout:  None,
            on_stderr:  None,
            reader:     Some(reader),
            writer:     Some(writer),
            rd_rx:      Some(rx),
            wr_tx:      Some(stdin_tx),
        })
    }

    fn send_str(&mut self, s: &str) {
        let b : Vec<u8> = s.bytes().collect();
        self.send(b);
//        dc.wr_tx.as_ref().unwrap().send("foobar!\n".to_string());
    }

    fn send(&mut self, buffer: Vec<u8>) {
        self.wr_tx.as_ref().unwrap().send(buffer);
    }

    fn recv_blocking(&mut self) -> String {
        self.rd_rx.as_ref().unwrap().recv().unwrap()
    }

    fn set_on_stdout<F>(&mut self, f: F)
        where F: Fn(&str) + 'static {
        self.on_stdout = Some(Box::new(f));
    }

    fn set_on_stderr<F>(&mut self, f: F)
        where F: Fn(&str) + 'static {
        self.on_stderr = Some(Box::new(f));
    }

    fn poll(&mut self) -> Result<(), Error>  {
        if self.rd_rx.is_none() {
            return Err(Error::Disconnected);
        }

        loop {
            match self.rd_rx.as_ref().unwrap().try_recv() {
                Ok(input) => {
                    if self.on_stdout.is_some() {
                        (self.on_stdout.as_ref().unwrap())(&input);
                    }
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

    fn shutdown(&mut self) {
        drop(self.wr_tx.take().unwrap());
        self.child.kill();
        self.writer.take().unwrap().join();
        self.reader.take().unwrap().join();
    }
}

pub fn doit() {
    println!("FOO {}", std::env::current_dir().unwrap().to_str().unwrap());
    let mut dc =
        DetachedCommand::start("gnugo-3.8\\gnugo.exe", &vec!["--mode", "gtp"])
        .expect("failed gnugo");

    let mut rp = Rc::new(RefCell::new(gtp::ResponseParser::new()));
    dc.set_on_stdout(move |input| {
        rp.borrow_mut().feed(input);

        match rp.borrow_mut().get_response() {
            Ok(resp) => {
                println!("INPUT: [{:?}]", resp);
            },
            Err(gtp::ResponseError::IncompleteResponse) => (),
            Err(err) => {
                println!("Resp error: {:?}", err);
            }
        }
    });
    dc.send_str("list_commands\n");
    loop {
        let p = dc.poll();
        if p.is_err() {
            println!("Error in poll: {:?}", p.unwrap_err());
            break;
        }
    }

    dc.shutdown();
}
