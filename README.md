# esp-storage-manager

Single-owner flash and cached NVS coordination for bare-metal Rust on ESP MCUs.

The crate exists for applications where several subsystems need persistent NVS
storage **and** one subsystem also needs raw flash access (for example an OTA
backend). It owns the one `esp_storage::FlashStorage` instance, keeps one
long-lived `esp_nvs::Nvs` view, and serializes both forms of access through a
single mutable manager.

It deliberately does not define application namespaces or keys.

## Why

Constructing independent flash/NVS objects per subsystem creates two problems:

- the physical flash must have a single owner;
- multiple long-lived `Nvs` instances over the same partition can hold stale
  in-memory views of each other’s writes.

`StorageManager` centralizes that ownership while still allowing bounded raw
access to non-NVS flash ranges.

## Supported chip features

- `esp32c3`
- `esp32s3`

No chip feature is enabled by default.

## Example

```rust
use esp_storage_manager::{Key, NvsPartition, StorageManager};

let mut storage = StorageManager::new(
    peripherals.FLASH,
    NvsPartition::new(0x9000, 0x6000),
);

const NS: Key = Key::from_str("app");
const KEY: Key = Key::from_str("counter");
storage.set_u32(&NS, &KEY, 42)?;
```

Raw access is closure-scoped so the flash reference cannot escape:

```rust
storage.with_raw_flash(|flash| {
    // Read/write a partition that does not overlap the cached NVS partition.
});
```

## Status

Extracted from `embewi-agent-esp`. The API is pre-stable and will be validated
first against ESP32-C3 hardware, then ESP32-S3.

## License

MIT.
