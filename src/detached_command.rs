use std::process::Command;
use std::process::Child;
use std::process::Stdio;
use std::sync::mpsc;
use std::io::BufWriter;
use std::io::BufReader;
use std::io::Write;
use std::io::Read;
use std::io::BufRead;
use std::thread;

#[derive(Debug)]
pub struct DetachedCommand {
    child:  std::process::Child,
    reader: Option<std::thread::JoinHandle<()>>,
    writer: Option<std::thread::JoinHandle<()>>,
    rd_rx:  Option<mpsc::Receiver<String>>,
    wr_tx:  Option<mpsc::Sender<Vec<u8>>>,
}

impl DetachedCommand {
    fn start(cmd: &str) -> DetachedCommand {
        let mut o =
            Command::new("cat")
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();


        let stdin = o.stdin.take().unwrap();
        let stdout = o.stdout.take().unwrap();
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

        DetachedCommand {
            child: o,
            reader: Some(reader),
            writer: Some(writer),
            rd_rx: Some(rx),
            wr_tx: Some(stdin_tx),
        }
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

    fn shutdown(&mut self) {
        drop(self.wr_tx.take().unwrap());
        self.child.kill();
        self.writer.take().unwrap().join();
        self.reader.take().unwrap().join();
    }
}

pub fn doit() {
    println!("FOO");
    let mut dc = DetachedCommand::start("cat");
    dc.send_str("Foobar!!\n");
    let r = dc.recv_blocking();
    println!("RECV: {}", r);
    dc.shutdown();
}
