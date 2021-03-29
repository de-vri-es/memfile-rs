//! This contains thin wrappers around `memfd_create` and the associated file sealing API.
//!
//! The [`MemFile`] struct represents a file created by the `memfd_create` syscall.
//! Such files are memory backed and fully anonymous, meaning no other process can see them (well... except by looking in `/proc` on Linux).
//!
//! After creation, the file descriptors can be shared with child processes, or even sent to another process over a Unix socket.
//! The files can then be memory mapped to be used as shared memory.
//!
//! This is all quite similar to `shm_open`, except that files created by `shm_open` are not anoynmous.
//! Depending on your application, the anonymous nature of `memfd` may be a nice property.
//! Additionally, files created by `shm_open` do not support file sealing.
//!
//! # File sealing
//! You can enable file sealing for [`MemFile`] by creating them with [`CreateOptions::allow_sealing(true)`][CreateOptions::allow_sealing].
//! This allows you to use [`MemFile::add_seals`] to add seals to the file.
//! You can also get the list of seals with [`MemFile::get_seals`].
//!
//! Once a seal is added to a file, it can not be removed.
//! Each seal prevents certain types of actions on the file.
//! For example: the [`Seal::Write`] seal prevents writing to the file, both through syscalls and memory mappings,
//! and the [`Seal::Shrink`] and [`Seal::Grow`] seals prevent the file from being resized.
//!
//! This is quite interesting for Rust, as it is the only guaranteed safe way to map memory:
//! when a file is sealed with [`Seal::Write`] and [`Seal::Shrink`], the file contents can not change, and the file can not be shrinked.
//! The latter is also important, because trying to read from a memory mapping a of file that was shrinked too far will raise a `SIGBUS` signal
//! and likely crash your application.
//!
//! Another interesting option is to first create a shared, writable memory mapping for your [`MemFile`],
//! and then add the [`Seal::FutureWrite`] and [`Seal::Shrink`] seals.
//! In that case, only the existing memory mapping can be used to change the contents of the file, even after the seals have been added.
//! When sharing the file with other processes, it prevents those processes from shrinking or writing to the file,
//! while the original process can still change the file contents.
//!
//! # Example
//! ```
//! # fn main() -> std::io::Result<()> {
//! use memfile::{MemFile, CreateOptions, Seal};
//! use std::io::Write;
//!
//! let mut file = MemFile::create("foo", CreateOptions::new().allow_sealing(true))?;
//! file.write_all(b"Hello world!")?;
//! file.add_seals(Seal::Write | Seal::Shrink | Seal::Grow)?;
//! // From now on, all writes or attempts to created shared, writable memory mappings will fail.
//! # Ok(())
//! # }
//! ```

use std::fs::File;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

mod sys;
mod seal;

pub use seal::{Seal, Seals};

/// A memory backed file that can have seals applied to it.
///
/// The struct implements [`AsRawFd`], [`IntoRawFd`] and [`FromRawFd`].
/// When using [`FromRawFd::from_raw_fd`], you must ensure that the file descriptor is a valid `memfd`.
#[derive(Debug)]
pub struct MemFile {
	file: File,
}

impl MemFile {
	/// Create a new `memfd` with the given options.
	///
	/// The `name` argument is purely for debugging purposes.
	/// On Linux it shows up in `/proc`, but it serves no other purpose.
	/// In particular, multiple files can be created with the same name.
	///
	/// The close-on-exec flag is set on the created file descriptor.
	/// If you want to pass it to a child process, you should use [`libc::dup2`] or something similar *after forking*.
	/// Disabling the close-on-exec flag before forking causes a race condition with other threads.
	pub fn create(name: &str, options: &CreateOptions) -> std::io::Result<Self> {
		let file = sys::memfd_create(name, options.as_flags())?;
		Ok(Self { file })
	}

	/// Create a new `memfd` with default options.
	///
	/// Sealing is not enabled for the created file.
	///
	/// See [`Self::create`] for more information.
	pub fn create_default(name: &str) -> std::io::Result<Self> {
		Self::create(name, &CreateOptions::default())
	}

	/// Create a new `memfd` with file sealing enabled.
	///
	/// Sealing is enabled for the created file.
	/// All other options are the same as the defaults.
	///
	/// See [`Self::create`] for more information.
	pub fn create_sealable(name: &str) -> std::io::Result<Self> {
		Self::create(name, CreateOptions::new().allow_sealing(true))
	}

	/// Try to create a new [`MemFile`] Createinstance that shares the same underlying file handle as the existing [`MemFile`] instance.
	///
	/// Reads, writes, and seeks will affect both [`MemFile`] instances simultaneously.
	pub fn try_clone(&self) -> std::io::Result<Self> {
		let file = self.file.try_clone()?;
		Ok(Self { file })
	}

	/// Wrap an already-open file as [`MemFile`].
	///
	/// This function returns an error if the file was not created by `memfd_create`.
	///
	/// If the function succeeds, the passed in file object is consumed and the returned [`MemFile`] takes ownership of the file descriptor.
	/// If the function fails, the original file object is included in the returned error.
	pub fn from_file<T: AsRawFd + IntoRawFd>(file: T) -> Result<Self, FromFdError<T>> {
		match sys::memfd_get_seals(file.as_raw_fd()) {
			Ok(_) => {
				let file = unsafe { File::from_raw_fd(file.into_raw_fd()) };
				Ok(Self { file })
			},
			Err(error) => Err(FromFdError { error, file }),
		}
	}

	/// Convert this [`MemFile`] into an [`std::fs::File`].
	///
	/// This may be useful for interoperability with other crates.
	pub fn into_file(self) -> std::fs::File {
		self.file
	}

	/// Query metadata about the underlying file.
	///
	/// Note that not all information in the metadata is not very meaningfull for a `memfd`.
	/// The file type is particularly useless since it is always the same.
	/// Some information, like the file size, may be useful.
	pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
		self.file.metadata()
	}

	/// Truncate or extend the underlying file, updating the size of this file to become size.
	///
	/// If the size is less than the current file's size, then the file will be shrunk.
	/// If it is greater than the current file's size, then the file will be extended to size and have all of the intermediate data filled in with 0s.
	/// The file's cursor isn't changed.
	/// In particular, if the cursor was at the end and the file is shrunk using this operation, the cursor will now be past the end.
	pub fn set_len(&self, size: u64) -> std::io::Result<()> {
		self.file.set_len(size)
	}

	/// Get the active seals of the file.
	pub fn get_seals(&self) -> std::io::Result<Seals> {
		let seals = sys::memfd_get_seals(self.as_raw_fd())?;
		Ok(Seals::from_bits_truncate(seals as u32))
	}

	/// Add a single seal to the file.
	///
	/// If you want to add multiple seals, you should prefer [`Self::add_seals`] to reduce the number of syscalls.
	///
	/// This function will fail if the file was not created with sealing support,
	/// if the file has already been sealed with [`Seal::Seal`],
	/// or if you try to add [`Seal::Write`] while a shared, writable memory mapping exists for the file.
	///
	/// Adding a seal that is already active is a no-op.
	pub fn add_seal(&self, seal: Seal) -> std::io::Result<()> {
		self.add_seals(seal.into())
	}

	/// Add multiple seals to the file.
	///
	/// This function will fail if the file was not created with sealing support,
	/// if the file has already been sealed with [`Seal::Seal`],
	/// or if you try to add [`Seal::Write`] while a shared, writable memory mapping exists for the file.
	///
	/// Adding seals that are already active is a no-op.
	pub fn add_seals(&self, seals: Seals) -> std::io::Result<()> {
		sys::memfd_add_seals(self.as_raw_fd(), seals.bits() as std::os::raw::c_int)
	}
}

impl FromRawFd for MemFile {
	unsafe fn from_raw_fd(fd: RawFd) -> Self {
		let file = File::from_raw_fd(fd);
		Self { file }
	}
}

impl AsRawFd for MemFile {
	fn as_raw_fd(&self) -> RawFd {
		self.file.as_raw_fd()
	}
}

impl IntoRawFd for MemFile {
	fn into_raw_fd(self) -> RawFd {
		self.file.into_raw_fd()
	}
}

impl std::os::unix::fs::FileExt for MemFile {
	fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
		self.file.read_at(buf, offset)
	}

	fn write_at(&self, buf: &[u8], offset: u64) -> std::io::Result<usize> {
		self.file.write_at(buf, offset)
	}
}

impl std::io::Write for MemFile {
	fn flush(&mut self) -> std::io::Result<()> {
		self.file.flush()
	}

	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.file.write(buf)
	}
}

impl std::io::Read for MemFile {
	fn read(&mut self, buf: &mut[u8]) -> std::io::Result<usize> {
		self.file.read(buf)
	}
}

impl std::io::Seek for MemFile {
	fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
		self.file.seek(pos)
	}
}

impl From<MemFile> for std::process::Stdio {
	fn from(other: MemFile) -> Self {
		other.file.into()
	}
}

/// Options for creating a [`MemFile`].
///
/// Support for options depend on platform and OS details.
/// Refer to your kernel documentation of `memfd_create` for details.
#[derive(Copy, Clone, Debug, Default)]
pub struct CreateOptions {
	allow_sealing: bool,
	huge_table: Option<HugeTlb>,
}

impl CreateOptions {
	/// Get the default creation options for a [`MemFile`].
	///
	/// Initially, file sealing is not enabled no no huge TLB page size is configured.
	///
	/// Note that the close-on-exec flag will always be set on the created file descriptor.
	/// If you want to pass it to a child process, you should use [`libc::dup2`] or something similar *after forking*.
	/// Disabling the close-on-exec flag before forking causes a race condition with other threads.
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a new [`MemFile`]` with the given options.
	///
	/// This is a shorthand for [`MemFile::create`].
	/// See that function for more details.
	pub fn create(&self, name: &str) -> std::io::Result<MemFile> {
		MemFile::create(name, self)
	}

	/// Allow sealing operations on the created [`MemFile`].
	pub fn allow_sealing(&mut self, value: bool) -> &mut Self {
		self.allow_sealing = value;
		self
	}

	/// Create the file in a `hugetlbfs` filesystem using huge pages for the translation look-aside buffer.
	///
	/// Support for this feature and specific sizes depend on the CPU and kernel configuration.
	/// See also: <https://www.kernel.org/doc/html/latest/admin-guide/mm/hugetlbpage.html>
	pub fn huge_tlb(&mut self, value: impl Into<Option<HugeTlb>>) -> &mut Self {
		self.huge_table = value.into();
		self
	}

	/// Get the options as raw flags for `libc::memfd_create`.
	fn as_flags(&self) -> std::os::raw::c_int {
		let mut flags = sys::flags::MFD_CLOEXEC;
		if self.allow_sealing {
			flags |= sys::flags::MFD_ALLOW_SEALING;
		}
		#[cfg(target_os = "linux")]
		if let Some(size) = self.huge_table {
			flags |= sys::flags::MFD_HUGETLB | size as u32 as std::os::raw::c_int;
		}
		flags
	}
}

/// Page size for the translation look-aside buffer.
///
/// Support for specific sizes depends on the CPU and kernel configuration.
/// See also: <https://www.kernel.org/doc/html/latest/admin-guide/mm/hugetlbpage.html>
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(u32)]
#[non_exhaustive]
pub enum HugeTlb {
	Huge64KB = sys::flags::MFD_HUGE_64KB as u32,
	Huge512KB = sys::flags::MFD_HUGE_512KB as u32,
	Huge1MB = sys::flags::MFD_HUGE_1MB as u32,
	Huge2MB = sys::flags::MFD_HUGE_2MB as u32,
	Huge8MB = sys::flags::MFD_HUGE_8MB as u32,
	Huge16MB = sys::flags::MFD_HUGE_16MB as u32,
	Huge32MB = sys::flags::MFD_HUGE_32MB as u32,
	Huge256MB = sys::flags::MFD_HUGE_256MB as u32,
	Huge512MB = sys::flags::MFD_HUGE_512MB as u32,
	Huge1GB = sys::flags::MFD_HUGE_1GB as u32,
	Huge2GB = sys::flags::MFD_HUGE_2GB as u32,
	Huge16GB = sys::flags::MFD_HUGE_16GB as u32,
}

/// Error returned when the file passed to [`MemFile::from_file`] is not a `memfd`.
///
/// This struct contains the [`std::io::Error`] that occurred and the original value passed to `from_file`.
/// It is also directly convertible to [`std::io::Error`], so you can pass it up using the `?` operator
/// from a function that returns an [`std::io::Result`].
pub struct FromFdError<T> {
	error: std::io::Error,
	file: T,
}

impl<T> FromFdError<T> {
	/// Get a reference to the I/O error.
	pub fn error(&self) -> &std::io::Error {
		&self.error
	}

	/// Get a reference to the original file object.
	pub fn file(&self) -> &T {
		&self.file
	}

	/// Consume the struct and return the I/O error and the original file object as tuple.
	pub fn into_parts(self) -> (std::io::Error, T) {
		(self.error, self.file)
	}

	/// Consume the struct and return the I/O error.
	pub fn into_error(self) -> std::io::Error {
		self.error
	}

	/// Consume the struct and return the original file object.
	pub fn into_file(self) -> T {
		self.file
	}
}

impl<T> From<FromFdError<T>> for std::io::Error {
	fn from(other: FromFdError<T>) -> Self {
		other.into_error()
	}
}
