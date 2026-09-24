# esp-config-space-backend

ESP/NVS backend adapter for config-space-manager.

It connects the generic ConfigSpace API to the single-owner
esp-storage-manager StorageManager without introducing application
configuration schemas into either crate.

Responsibilities:

- map one ConfigSpace name to one opaque NVS blob;
- persist ConfigSpace generations;
- provide complete-value replacement through ESP NVS blobs;
- conservatively reserve NVS entry capacity for claims;
- serialize access through the shared StorageManager.

Application keys, Wi-Fi/TLS schemas, lifecycle state and OTA state remain
outside this crate.
