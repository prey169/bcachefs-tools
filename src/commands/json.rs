use bch_bindgen::bcachefs::{bch_ioctl_query_counters, bch_persistent_counters_stable};
use clap::Parser;
use libc::{self, c_void, ioctl};
use nix::{
    fcntl::{open, OFlag},
    ioctl_write_ptr,
    sys::stat::Mode,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    os::fd::{AsRawFd, RawFd},
    path::PathBuf,
    time::SystemTime,
    {env, io},
};
use strum::{EnumCount, IntoEnumIterator};
use uuid::Uuid;

const BCH_IOCTL_QUERY_UUID: libc::Ioctl = 0x8010bc01; // _IOR(0xbc, 1, [u8; 16])

ioctl_write_ptr!(bch_query_counters, 0xbc, 21, bch_ioctl_query_counters);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Where the filesystem should be mounted. If not set, then the filesystem
    /// checked will be the current working dir's filesystem
    mountpoint: Option<PathBuf>,
}

fn read_counters(fd: RawFd) -> nix::Result<Box<bch_ioctl_query_counters>> {
    let size = size_of::<bch_ioctl_query_counters>()
        + (bch_persistent_counters_stable::COUNT) * size_of::<u64>();

    let mut bch_counters: Box<bch_ioctl_query_counters> = unsafe {
        let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
        let ptr = std::alloc::alloc_zeroed(layout);
        Box::from_raw(ptr as *mut bch_ioctl_query_counters)
    };

    bch_counters.nr = bch_persistent_counters_stable::COUNT as u16;

    unsafe {
        bch_query_counters(fd, &*bch_counters)?;
    }

    Ok(bch_counters)
}

fn parse_counters(counters: Box<bch_ioctl_query_counters>) -> HashMap<std::string::String, u64> {
    let mut counter_values = HashMap::new();

    unsafe {
        let counters_slice = std::slice::from_raw_parts(counters.d.as_ptr(), counters.nr as usize);

        for stable in bch_persistent_counters_stable::iter() {
            let debug_str = format!("{:?}", stable);
            let short = debug_str
                .trim_start_matches("BCH_COUNTER_STABLE_")
                .to_string();
            let index = stable as usize;
            counter_values.insert(short, counters_slice[index]);
        }
    }

    counter_values
}

fn query_bcachefs_uuid(raw_fd: RawFd) -> Result<String, i32> {
    let mut uuid_bytes: [u8; 16] = [0; 16];

    let ret = unsafe {
        ioctl(
            raw_fd,
            BCH_IOCTL_QUERY_UUID,
            uuid_bytes.as_mut_ptr() as *mut c_void,
        )
    };

    if ret >= 0 {
        Ok(Uuid::from_bytes(uuid_bytes).to_string())
    } else {
        Err(ret)
    }
}

#[derive(Serialize, Debug, Default)]
struct FsJsonOutput {
    time: Option<u128>,
    uuid: Option<String>,
    counters: Option<HashMap<String, u64>>,
}

pub fn json(mut argv: Vec<String>) -> std::process::ExitCode {
    // The CLI parser here expects the device at position 1.
    if argv.len() > 1 {
        argv.remove(0);
    }
    let cli = Cli::parse_from(argv);
    let binding: PathBuf = if let Some(p) = cli.mountpoint {
        p.into()
    } else {
        env::current_dir().unwrap()
    };

    let mountpoint = binding.to_str().unwrap();

    let fd = match open(mountpoint, OFlag::O_RDONLY, Mode::empty()) {
        Ok(fd) => fd,
        Err(err) => {
            eprintln!("ERROR: Failed to open mountpoint {mountpoint}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let raw_fd = fd.as_raw_fd();
    let mut output = FsJsonOutput::default();
    output.time = Some(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
    );

    match query_bcachefs_uuid(raw_fd) {
        Ok(uuid_str) => {
            output.uuid = Some(uuid_str);
        }
        Err(_) => {
            eprintln!("ERROR: Failed to get UUID: {}", io::Error::last_os_error());
            eprintln!("ERROR: {mountpoint} is most likely not apart of a bcachefs filesystem");
            drop(fd);
            return std::process::ExitCode::FAILURE;
        }
    }

    let counters = match read_counters(raw_fd) {
        Ok(counters) => counters,
        Err(err) => {
            eprintln!("ERROR: Failed to read counters: {err}");
            eprintln!("ERROR: {mountpoint} is most likely not apart of a bcachefs filesystem");
            drop(fd);
            return std::process::ExitCode::FAILURE;
        }
    };

    output.counters = Some(parse_counters(counters));

    let json_str = serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string());
    println!("{}", json_str);
    drop(fd);
    std::process::ExitCode::SUCCESS
}
