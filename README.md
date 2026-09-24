# esp-flash-access

Minimal process-wide ESP flash ownership for bare-metal Rust.

The crate owns exactly one `esp_storage::FlashStorage` and serializes access
through an Embassy mutex. It deliberately knows nothing about NVS, ConfigSpace,
OTA state, keys, namespaces, partitions, or application health.

Consumers lock the shared capability and apply their own backend semantics:

```text
ESP FLASH
   |
esp-flash-access
   |
   +-- config-space-manager ESP/NVS backend
   +-- FiBeWI ESP backend
```

The GitHub repository retains its historical name for now; the Rust package is
`esp-flash-access`.

## License

MIT.
