<p align="center">
  <img src="bulicloud.png" width="600" alt="BuliCloud Logo">
</p>

# ⛅ BuliCloud

BuliCloud is a lightweight Minecraft cloud system written in Rust.

The goal is to provide a simple and understandable way to manage Minecraft server infrastructure without unnecessary complexity.

BuliCloud focuses on a small core, clear architecture and predictable behavior.

## 💡 Vision

BuliCloud should make it easy to define, create and manage server instances while keeping the underlying system transparent.

The project is designed to stay lightweight and only introduce complexity where it is actually needed.

## ✨ What BuliCloud should feel like

The ideal BuliCloud experience is something like:
```text
$ bulicloud start

[INFO] Starting BuliCloud...
[INFO] Loading configuration...
[INFO] Loading templates...
[INFO] Preparing infrastructure...

BuliCloud is ready.

```

No giant setup wizard.
No twenty required services.
No mysterious infrastructure.
Just a cloud that starts and does its job.

## 🛠️ Project Status

BuliCloud is currently in early development phase.

The architecture and APIs are not fully implemented yet and may still change.

```text
Cloud
 ├── Groups
 │    ├── Templates
 │    │    ├── Instance
 │    │    ├── Instance
 │    │    └── Instance
 │    │
 │    └── Templates
 │
 └── Infrastructure
 ```

Templates describe what should run.
Groups describe how those templates belong together.
Instances are just the running result.
The long-term vision is to make Minecraft infrastructure feel less like manually managing server folders and more like operating a small, purpose-built platform.

## License

See `LICENSE`.
