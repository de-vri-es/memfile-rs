use assert2::{assert, let_assert};
use memfile::{MemFile, Seal, Seals};
use std::io::{Read, Write, Seek};
use std::os::fd::OwnedFd;
use std::os::unix::io::AsRawFd;

#[test]
fn create_write_seek_read() {
	let_assert!(Ok(mut file) = MemFile::create_default("foo"));
	assert!(let Ok(()) = file.write_all(b"Hello world!"));

	let mut buffer = [0u8; 12];
	assert!(let Ok(0) = file.seek(std::io::SeekFrom::Start(0)));
	assert!(let Ok(()) = file.read_exact(&mut buffer));
	assert!(&buffer == b"Hello world!");
}

#[track_caller]
fn dup_stdout() -> OwnedFd {
	use std::os::fd::FromRawFd;
	use std::mem::ManuallyDrop;

	let stdout = ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(1) });
	let_assert!(Ok(file) = stdout.try_clone());
	file.into()
}

#[test]
fn from_fd() {
	// We should be able to wrap a MemFile as MemFile again.
	let_assert!(Ok(original) = MemFile::create_default("foo"));
	let original_fd = original.as_raw_fd();
	let_assert!(Ok(moved) = MemFile::from_fd(original.into_fd()));
	assert!(moved.as_raw_fd() == original_fd);

	// We should not be able to wrap stdout as MemFile.
	let dupped_stdout = dup_stdout();
	let dupped_fd = dupped_stdout.as_raw_fd();
	let_assert!(Err(error) = MemFile::from_fd(dupped_stdout));
	assert!(error.error().kind() == std::io::ErrorKind::InvalidInput);
	assert!(error.fd().as_raw_fd() == dupped_fd);
}

#[test]
fn try_clone() {
	let_assert!(Ok(original) = MemFile::create_default("foo"));
	let_assert!(Ok(dupped) = original.try_clone());

	// Dupped file descriptors should not have the same value.
	assert!(original.as_raw_fd() != dupped.as_raw_fd());
}

#[test]
fn set_len_stat() {
	let_assert!(Ok(file) = MemFile::create_default("foo"));
	assert!(let Ok(()) = file.set_len(12));
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 12);
}

#[test]
fn seal_seal() {
	// Create the file and resize it to 12 bytes.
	let_assert!(Ok(file) = MemFile::create_sealable("foo"));
	assert!(let Ok(()) = file.add_seal(Seal::Seal));

	let_assert!(Err(error) = file.add_seal(Seal::Grow));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
}

#[test]
fn seal_shrink() {
	// Create the file and resize it to 12 bytes.
	let_assert!(Ok(file) = MemFile::create_sealable("foo"));
	assert!(let Ok(()) = file.set_len(12));
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 12);

	// Seal it and try to shrink it.
	assert!(let Ok(()) = file.add_seal(Seal::Shrink));
	let_assert!(Err(error) = file.set_len(6));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 12);

	// Try to grow it.
	let_assert!(Ok(()) = file.set_len(18));
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 18);
}

#[test]
fn seal_grow() {
	// Create the file and resize it to 12 bytes.
	let_assert!(Ok(file) = MemFile::create_sealable("foo"));
	assert!(let Ok(()) = file.set_len(12));
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 12);

	// Seal it and try to grow it.
	assert!(let Ok(()) = file.add_seal(Seal::Grow));
	let_assert!(Err(error) = file.set_len(18));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 12);

	// Try to shrink it.
	let_assert!(Ok(()) = file.set_len(6));
	let_assert!(Ok(stat) = file.metadata());
	assert!(stat.len() == 6);
}

#[test]
fn seal_write() {
	let_assert!(Ok(mut file) = MemFile::create_sealable("foo"));
	assert!(let Ok(()) = file.add_seal(Seal::Write));

	let_assert!(Err(error) = file.write_all(b"Hello world!"));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
}

#[test]
#[cfg(target_os = "linux")]
fn seal_future_write() {
	// TODO: to properly test this, we need to create a shared writable memory mapping, and validate that it remains usable,
	// and that no new shared writable mappings can be made.
	let_assert!(Ok(mut file) = MemFile::create_sealable("foo"));
	assert!(let Ok(()) = file.add_seal(Seal::FutureWrite));

	let_assert!(Err(error) = file.write_all(b"Hello world!"));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
}

#[test]
fn clones_share_metadata_and_seals() {
	let_assert!(Ok(original) = MemFile::create_sealable("foo"));
	let_assert!(Ok(mut dupped) = original.try_clone());

	let_assert!(Ok(()) = original.set_len(12));
	let_assert!(Ok(stat) = dupped.metadata());
	assert!(stat.len() == 12);

	let_assert!(Ok(()) = original.add_seals(Seals::all()));
	let_assert!(Err(error) = dupped.write_all(b"Hello world!"));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
}

#[test]
fn sealing_must_be_enabled() {
	// Create MemFile without enabling sealing.
	let_assert!(Ok(original) = MemFile::create_default("foo"));

	// Now try to add a seal, which should fail.
	let_assert!(Err(error) = original.add_seals(Seals::all()));
	assert!(error.kind() == std::io::ErrorKind::PermissionDenied);
}
