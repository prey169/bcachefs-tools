use super::utils::handle_mountpoint;

use anyhow::{Context, Result};
use bch_bindgen::bcachefs::{bch_ioctl_query_counters, bch_persistent_counters_stable};
use libc::{self, c_void, ioctl};
use nix::{
    errno::Errno,
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
};
use strum::{EnumCount, IntoEnumIterator};
use uuid::Uuid;

const BCH_IOCTL_QUERY_UUID: libc::Ioctl = 0x8010bc01; // _IOR(0xbc, 1, [u8; 16])

ioctl_write_ptr!(bch_query_counters, 0xbc, 21, bch_ioctl_query_counters);

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

fn query_bcachefs_uuid(raw_fd: RawFd) -> Result<String> {
    let mut uuid_bytes: [u8; 16] = [0; 16];

    let ret = unsafe {
        ioctl(
            raw_fd,
            BCH_IOCTL_QUERY_UUID,
            uuid_bytes.as_mut_ptr() as *mut c_void,
        )
    };

    if ret < 0 {
        let err = Errno::last();

        return Err(anyhow::Error::new(err)
            .context(format!("ioctl(BCH_IOCTL_QUERY_UUID) failed on fd {raw_fd}"))
            .context("Is this really a bcachefs filesystem?"));
    }
    Ok(Uuid::from_bytes(uuid_bytes).to_string())
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct FsJsonOutput {
    pub time: Option<u128>,
    pub uuid: Option<String>,
    pub counters: HashMap<String, u64>,
}

pub fn json(mountpoint: &Option<PathBuf>) -> Result<FsJsonOutput> {
    let mountpoint = handle_mountpoint(mountpoint).context("Invalid or missing mountpoint")?;

    let fd = open(&mountpoint, OFlag::O_RDONLY, Mode::empty())
        .with_context(|| format!("Failed to open mountpoint {}", mountpoint.display()))?;

    let raw_fd = fd.as_raw_fd();

    let mut output = FsJsonOutput::default();

    output.time = Some(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
    );

    let uuid_str = query_bcachefs_uuid(raw_fd)
        .context("Failed to query bcachefs UUID — is this really a bcachefs filesystem?")?;

    output.uuid = Some(uuid_str);

    let counters = read_counters(raw_fd).context("Failed to read bcachefs counters")?;

    output.counters = parse_counters(counters);

    Ok(output)
}
