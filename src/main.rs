use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::str;

use iou;

#[derive(Debug)]
struct IORequest {
    fd: RawFd,
    f: std::fs::File,
    fsize: u64,
    chunks: usize,
    bufs: Vec<[u8; 512]>,
}

impl IORequest {
    fn new(f: std::fs::File) -> Self {
        let fsize = f.metadata().unwrap().len();
        let fd = f.as_raw_fd();
        let chunks = ((fsize as f64) / 512.0).ceil() as usize;
        let mut bufs: Vec<[u8; 512]> = Vec::with_capacity(chunks);

        for _chunk in 0..chunks {
            bufs.push([0; 512]);
        }

        IORequest {
           fsize,
           fd, 
           chunks,
           bufs,
           f
        }
    }
}

fn raw_fds(args: Vec<String>) -> Vec<IORequest> {
    args[1..]
        .iter()
        .map(|f| {
           std::fs::File::open(f).expect(format!("Error opening {}", f).as_str())
        })
        .map(|file| {
            IORequest::new(file)
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cat [file1] [file2] [file3]...");
        return;
    }
    let mut io_reqs = raw_fds(args);

    let mut ring = iou::IoUring::new(io_reqs.len() as u32).expect("New IoUring");
    let (mut sq, mut cq, _reg) = ring.queues();

    for (i, io_req) in io_reqs.iter_mut().enumerate() {
        let mut sqe = sq.prepare_sqe().expect("Preparing SQE");
        let mut io_bufs: Vec<io::IoSliceMut> = Vec::with_capacity(io_req.chunks);
        for buf in &mut io_req.bufs {
            io_bufs.push(io::IoSliceMut::new(buf));
        }
        unsafe {
            sqe.prep_read_vectored(io_req.fd, &mut io_bufs, 0);
            sqe.set_user_data(i as u64);
        }
        sq.submit().expect("Submit queue submit failed");

        let _cqe = cq.wait_for_cqe().unwrap();
        for chunk in &io_req.bufs {
            print!("{}", str::from_utf8(chunk).ok().unwrap());
        }
        println!();
    }
}
