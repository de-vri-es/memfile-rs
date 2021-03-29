# v0.2.0
* Add `CreateOptions::create()` as shorthand for `MemFile::create()`.

# v0.1.2 - 2021-02-02
* Added `MemFile::into_file()` for safe ineroperability with crates that expect an `std::fs::File`.

# v0.1.2 - 2021-02-02
* Added `create_default()` and `create_sealable()` shorthand constructors.
* Added `MemFile::metadata()` method.
* Tweaked documentation.

# v0.1.1 - 2021-02-02
* Added repository and documentation links to `Cargo.toml`.

# v0.1.0 - 2021-02-02
* Initial release.
* Implemented support for Linux and FreeBSD.
* Implemented memfd creation and I/O operations.
* Implemented file sealing support.
