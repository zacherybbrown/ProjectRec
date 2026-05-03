# Project Rec

Project Rec is a social room transport experience built in Rust. It is designed as a standalone native executable, without a game engine, and does not require administrator privileges to run.

## Features

- Host self-hosted rooms from the local computer
- Join rooms over TCP
- Call public and private sky trains for room transitions
- Load asset metadata from `assets/manifest.json`
- Enforce room creation only when PCVR mode is enabled
- Add cubes and edit the 3D world from the GUI editor mode

## Getting started

Build the project:

```bash
cargo build --release
```

Run the default GUI:

```bash
cargo run
```

Run a room host (PC mode or PCVR mode):

```bash
cargo run -- host --room-name "Lounge" --pc
```

Run the GUI explicitly:

```bash
cargo run -- gui
```

Join a room:

```bash
cargo run -- join --address 127.0.0.1:4000 --name "Player"
```

List available rooms:

```bash
cargo run -- list
```

Get info:

```bash
cargo run -- info
```
