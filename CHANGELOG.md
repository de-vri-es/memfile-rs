# v0.2.0 - 2021-04-01
* Add `CreateOptions::create()` as shorthand for `MemFile::create()`.
* Change `CreateOptions::huge_tlb()` to accept any `Into<Option<HugeTlb>>`.
* Take and return `CreateOptions` by value.
* Add functions to create a `MemFile` that don't allocate a `CString` for the name.

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
