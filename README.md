[![Docs.rs](https://docs.rs/memfile/badge.svg)](https://docs.rs/crate/memfile/)
[![CI](https://github.com/de-vri-es/memfile-rs/workflows/CI/badge.svg)](https://github.com/de-vri-es/memfile-rs/actions?query=workflow%3ACI+branch%3Amain)

# memfile

This contains thin wrappers around `memfd_create` and the associated file sealing API.

The [`MemFile`] struct represents a file created by the `memfd_create` syscall.
Such files are memory backed and fully anonymous, meaning no other process can see them (well... except by looking in `/proc` on Linux).

After creation, the file descriptors can be shared with child processes, or even sent to another process over a Unix socket.
The files can then be memory mapped to be used as shared memory.

This is all quite similar to `shmopen`, except that files created by `shmopen` are not anoynmous.
Depending on your application, the anonymous nature of `memfd` may be a nice property.
Additionally, files created by `shmopen` do not support file sealing.

## File sealing
You can enable file sealing for [`MemFile`] by creating them with [`CreateOptions::allow_sealing(true)`][CreateOptions::allow_sealing].
This allows you to use [`MemFile::add_seals`] to add seals to the file.
You can also get the list of seals with [`MemFile::get_seals`].

Once a seal is added to a file, it can not be removed.
Each seal prevents certain types of actions on the file.
For example: the [`Seal::Write`] seal prevents writing to the file, both through syscalls and memory mappings,
and the [`Seal::Shrink`] and [`Seal::Grow`] seals prevent the file from being resized.

This is quite interesting for Rust, as it is the only guaranteed safe way to map memory:
when a file is sealed with [`Seal::Write`] and [`Seal::Shrink`], the file contents can not change, and the file can not be shrinked.
The latter is also important, because trying to read from a memory mapping a of file that was shrinked too far will raise a `SIGBUS` signal
and likely crash your application.

Another interesting option is to first create a shared, writable memory mapping for your [`MemFile`],
and then add the [`Seal::FutureWrite`] and [`Seal::Shrink`] seals.
In that case, only the existing memory mapping can be used to change the contents of the file, even after the seals have been added.
When sharing the file with other processes, it prevents those processes from shrinking or writing to the file,
while the original process can still change the file contents.

## Example
```rust
use memfile::{MemFile, CreateOptions, Seal};
use std::io::Write;

let mut file = MemFile::create("foo", CreateOptions::new().allow_sealing(true))?;
file.write_all(b"Hello world!")?;
file.add_seals(Seal::Write | Seal::Shrink | Seal::Grow)?;
// From now on, all writes or attempts to created shared, writable memory mappings will fail.
```

[`MemFile`]: https://docs.rs/memfile/latest/memfile/struct.MemFile.html
[CreateOptions::allow_sealing]: https://docs.rs/memfile/latest/memfile/struct.CreateOptions.html#method.allow_sealing
[`MemFile::add_seals`]: https://docs.rs/memfile/latest/memfile/struct.MemFile.html#method.add_seals
[`MemFile::get_seals`]: https://docs.rs/memfile/latest/memfile/struct.MemFile.html#method.get_seals
[`Seal::Write`]: https://docs.rs/memfile/latest/memfile/enum.Seal.html#variant.Write
[`Seal::Shrink`]: https://docs.rs/memfile/latest/memfile/enum.Seal.html#variant.Shrink
[`Seal::Grow`]: https://docs.rs/memfile/latest/memfile/enum.Seal.html#variant.Grow
[`Seal::FutureWrite`]: https://docs.rs/memfile/latest/memfile/enum.Seal.html#variant.FutureWrite
