pub mod lib;

use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream}, thread, sync::{mpsc, Arc, Mutex}, collections::HashMap, string::FromUtf8Error
};

use lib::Builder;


pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
        {
            let job = Box::new(f);

            self.sender.as_ref().unwrap().send(job).unwrap();
        }
    
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    job();
                }
                Err(_) => {
                    break;
                }
            }
        });

        Worker {
            _id: id,
            thread: Some(thread),
        }
    }
}

trait SliceExt {
    fn trim(&self) -> &Self;
}

impl SliceExt for [u8] {
    fn trim(&self) -> &[u8] {
        fn is_whitespace(c: &u8) -> bool {
            c == &b'\t' || c == &b' '
        }

        fn is_not_whitespace(c: &u8) -> bool {
            !is_whitespace(c)
        }

        if let Some(first) = self.iter().position(is_not_whitespace) {
            if let Some(last) = self.iter().rposition(is_not_whitespace) {
                &self[first..last + 1]
            } else {
                unreachable!();
            }
        } else {
            &[]
        }
    }
}

trait ToJson {
    fn to_json(&self) -> Result<String, FromUtf8Error>;
}

impl ToJson for HashMap<&str, &str> {
    fn to_json(&self) -> Result<String, FromUtf8Error> {
        let mut json_builder = Builder::default();
        let mut i = 0;
        json_builder.append("{");
        for (key, value) in self {
            json_builder.append(format!(
                "{:?}:{:?}{}",
                key,
                value,
                if i == self.len() - 1 { "" } else { "," }));
            i += 1;
        }
        json_builder.append("}");
        json_builder.string()
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.")
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer).into_owned();

    let request: Vec<_> = request
        .trim_matches(char::from(0))
        .split("\r\n")
        .collect();

    let request_line: Vec<_> = request[0].split(" ").collect();

    let mut headers = Vec::new();

    for i in 1..request.len() {
        if request[i].is_empty() { break; }
        headers.push(request[i]);
    }

    let request_method = request_line[0];
    let request_path = request_line[1];

    let (response_code, response_body) = match request_method {
        "HEAD" => (200, String::new()),
        "GET" => {
            let mut json_res: HashMap<&str, &str> = HashMap::new();
            json_res.insert("hello", "world");
            json_res.insert("test", "ing");
            (200, json_res.to_json().unwrap())
        },
        "POST" => (200, format!("{{\"request_method\": \"{request_method}\"}}")),
        _ => (405, format!("{{\"request_method\": \"{request_method}\"}}"))
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
        response_code,
        request_path,
        response_body.len(),
        response_body
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
