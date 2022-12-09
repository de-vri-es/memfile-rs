use std::fs::File;
use std::os::raw::c_int;
use std::ffi::CStr;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod raw {
	use std::os::raw::{c_char, c_int};
	extern "C" {
		pub fn memfd_create(name: *const c_char, flags: c_int) -> c_int;
	}
}

#[cfg(target_os = "android")]
mod raw {
	use std::os::raw::{c_char, c_int};
	pub unsafe fn memfd_create(name: *const c_char, flags: c_int) -> c_int {
		libc::syscall(libc::SYS_memfd_create, name, flags) as c_int
	}
}

pub fn memfd_create(name: &str, flags: c_int) -> std::io::Result<File> {
	let name = std::ffi::CString::new(name)?;
	memfd_create_cstr(&name, flags)
}

pub fn memfd_create_cstr(name: &CStr, flags: c_int) -> std::io::Result<File> {
	let fd = unsafe { raw::memfd_create(name.as_ptr(), flags) };
	if fd < 0 {
		Err(std::io::Error::last_os_error())
	} else {
		Ok(unsafe { File::from_raw_fd(fd) })
	}
}

pub fn memfd_get_seals(fd: RawFd) -> std::io::Result<c_int> {
	match unsafe { libc::fcntl(fd, libc::F_GET_SEALS) } {
		-1 => Err(std::io::Error::last_os_error()),
		seals => Ok(seals),
	}
}

pub fn memfd_add_seals(fd: RawFd, seals: c_int) -> std::io::Result<()> {
	if unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) } == 0 {
		Ok(())
	} else {
		Err(std::io::Error::last_os_error())
	}
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "macos"))]
pub mod flags {
	// Linux values taken from:
	// https://github.com/torvalds/linux/blob/1048ba83fb1c00cd24172e23e8263972f6b5d9ac/include/uapi/linux/memfd.h
	// https://github.com/torvalds/linux/blob/1048ba83fb1c00cd24172e23e8263972f6b5d9ac/include/uapi/asm-generic/hugetlb_encode.h
	//
	// FreeBSD values taken from:
	// https://github.com/freebsd/freebsd-src/blob/de1aa3dab23c06fec962a14da3e7b4755c5880cf/sys/sys/mman.h#L210-L228

	use std::os::raw::c_int;
	pub const MFD_CLOEXEC: c_int = 0x01;
	pub const MFD_ALLOW_SEALING: c_int = 0x02;
	pub const MFD_HUGETLB: c_int = 0x04;

	const MFD_HUGE_SHIFT: c_int = 26;
	pub const MFD_HUGE_64KB: c_int = 16 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_512KB: c_int = 19 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_1MB: c_int = 20 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_2MB: c_int = 21 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_8MB: c_int = 23 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_16MB: c_int = 24 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_32MB: c_int = 25 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_256MB: c_int = 28 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_512MB: c_int = 29 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_1GB: c_int = 30 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_2GB: c_int = 31 << MFD_HUGE_SHIFT;
	pub const MFD_HUGE_16GB: c_int = 34 << MFD_HUGE_SHIFT;
}
