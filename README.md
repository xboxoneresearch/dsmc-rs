# dsmc-rs

Rust wrapper around `dsmcdll.dll`, for communicating via FACET port.

## dsmcflash

CLI to read/write flash and get "expected 1SMCBL digest".

- Read flash: `dsmcflash read --file dump.bin`
- Write flash: `dsmcflash write --file dump.bin`
- Get expected 1SMCBL digest: `dsmcflash digest`

## Build

Windows

```
cargo build --target x86_64-pc-windows-msvc --release --all
```

Unix

```
cross build --target x86_64-pc-windows-gnu --release --all
```

## Credits

Thx original author for your initial work on this!
