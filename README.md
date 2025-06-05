[![CI](https://github.com/xboxoneresearch/dsmc-rs/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/xboxoneresearch/dsmc-rs/actions/workflows/build.yml)
![GitHub Downloads (all assets, latest release)](https://img.shields.io/github/downloads/xboxoneresearch/dsmc-rs/latest/total)
[![GitHub latest Tag](https://img.shields.io/github/v/tag/xboxoneresearch/dsmc-rs)](https://github.com/xboxoneresearch/dsmc-rs/releases/latest)

# dsmc-rs

Rust wrapper around `dsmcdll.dll`, for communicating via FACET port.

Tested version of `dsmcdll.dll`:

- ✅ 10.0.14393.1040 `(rs1_xbox_rel_1608.160816-1851)`
- ❌ 10.0.19041.4350 `(WinBuild.160101.0800)`

## dsmcflash

CLI to read/write flash and get "expected 1SMCBL digest".

Download [latest release](https://github.com/xboxoneresearch/dsmc-rs/releases/latest)

- Read flash: `dsmcflash read --file dump.bin`
- Write flash: `dsmcflash write --file dump.bin`
- Get expected 1SMCBL digest: `dsmcflash digest`

## Build

Windows

```
cargo build --target x86_64-pc-windows-msvc --release --all
```

Unix

(Utilizing [cross-rs](https://github.com/cross-rs/cross) to cross-compile)

```
cross build --target x86_64-pc-windows-gnu --release --all
```

## Credits

Thx original author for your initial work on this!
