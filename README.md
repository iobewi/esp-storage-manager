# esp-storage-manager

Shared hardware storage backend for bare-metal ESP Rust firmware.

The crate currently retains the package name `esp-flash-access` for compatibility,
but its responsibility is broader than flash ownership alone: it centralizes the
ESP-specific storage primitives shared by higher-level backends.

```text
ESP hardware storage
        |
esp-storage-manager
├── flash ownership + serialization
├── NVS platform adapter
└── partition-table / raw erase helpers
        |
        +-----------------------------+
        |                             |
config-space-manager             FiBeWI
ESP/NVS adapter                  ESP firmware adapter
```

This crate owns **hardware mechanics only**. It deliberately knows nothing about:

- ConfigSpace namespaces, quotas, generations or record framing;
- FiBeWI transactions, staging, EWBT policy or rollback decisions;
- HTTP, TLS, provisioning or application health.

Higher-level repositories keep those semantics and consume this crate as their
common ESP hardware layer.

## Chip features

- `esp32c3`
- `esp32s3`

No chip feature is enabled by default.

## License

MIT.
