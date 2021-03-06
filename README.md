# cargo-mobile

*The answer to "how do I use Rust on iOS/Android?"*

**This project hasn't been announced yet. Please don't make any posts about it on reddit/etc!**

cargo-mobile takes care of generating Xcode and Android Studio project files, building and running on device, generating project boilerplate, and a few other things!

## Status

Everything here works and is already used internally! However, this hasn't seen a lot of external use yet, so there could still be some rough edges.

**Building for iOS is broken on Rust versions later than 1.45.2!**

You'll need to stay on 1.45.2 for now. Upstream PR: [rust-lang/rust#77716](https://github.com/rust-lang/rust/pull/77716)

```bash
rustup install stable-2020-08-03
rustup default stable-2020-08-03
```

Don't worry about the `'+cyclone' is not a recognized feature for this target (ignoring feature)` messages. They're harmless, and have since been fixed upstream.

## Installation

The build will probably take a bit, so feel free to go get a snack or something.

```bash
cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

cargo-mobile is currently only supported on macOS. Adding Linux support would likely only take a small PR, but Windows support is potentially a small nightmare. (Note that only macOS can support iOS development, so other platforms could only be used for Android development!)

You'll need to have Xcode and the Android SDK/NDK installed. Some of this will ideally be automated in the future, or at least we'll provide a helpful guide and diagnostics.

Whenever you want to update:

```bash
cargo mobile update
```

Note that until cargo-mobile's official release, breaking changes may be common.

## Usage

To start a new project, all you need to do is make a directory with a cute name, `cd` into it, and then run this command:

```bash
cargo mobile init
```

After some straightforward prompts, you'll be asked to select a template pack. Template packs are used to generate project boilerplate, i.e. using the `bevy` template pack gives you a minimal [Bevy](https://bevyengine.org/) project that runs out-of-the-box on desktop and mobile.

| name      | info                                                                                                                              |
| --------- | --------------------------------------------------------------------------------------------------------------------------------- |
| bevy      | Minimal Bevy project derived from [sprite](https://github.com/bevyengine/bevy/blob/master/examples/2d/sprite.rs) example          |
| bevy-demo | Bevy [breakout](https://github.com/bevyengine/bevy/blob/master/examples/game/breakout.rs) demo                                    |
| wgpu      | Minimal wgpu project derived from [hello-triangle](https://github.com/gfx-rs/wgpu-rs/tree/master/examples/hello-triangle) example |
| winit     | Minimal winit project derived from [window](https://github.com/rust-windowing/winit/tree/master/examples/window) exmaple          |

**Template pack contribution is encouraged**; we'd love to have very nice template packs for Bevy, Amethyst, and whatever else people find helpful! We'll write up a guide for template pack creation soon, but in the mean time, the existing ones are a great reference point. Any template pack placed into `~./cargo-mobile/templates/apps/` will appear as an option in `cargo mobile init`.

Once you've generated your project, you can run `cargo run` as usual to run your app on desktop. However, now you can also do `cargo apple run` and `cargo android run` to run on connected iOS and Android devices respectively!

If you prefer to work in the usual IDEs, you can use `cargo apple open` and `cargo android open` to open your project in Xcode and Android Studio respectively.

For more commands, run `cargo mobile`, `cargo apple`, or `cargo android` to see help information.

A more comprehensive guide will come soon!
