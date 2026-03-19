# Using `dist-render` as a crate

_Please note that the project is experimental. Shipping games/apps is not one of its current goals, and is not actively supported._

`dist-render` is not currently published on `crates.io`, and doesn't have an asset packaging system. For those reasons, using it as a crate is a bit fiddly. It's possible though.

Documentation is currently scarce, meaning that it's best to follow examples (see [`crates/bin/hello`](../crates/bin/hello)).

## VFS

The renderer has a basic virtual file system used for loading assets (models, textures, shaders). That makes it possible to work on a game/app while pointing it at the assets in a separately synced Dist Render repository.

```rust
// Point `dist-render` to standard assets and shaders in the parent directory
set_standard_vfs_mount_points("../dist-render");

// Game-specific assets in the current directory
set_vfs_mount_point("/cache", "./cache");
```

## Cargo patches

For a standalone project to compile, please copy the `[patch.crates-io]` section from the top-level [`Cargo.toml`](../Cargo.toml)

# Larger examples

* [Cornell McRay t'Racing](https://github.com/h3r2tic/cornell-mcray) -- a simple racing game
