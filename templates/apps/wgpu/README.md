# wgpu-app

This is just the [`wgpu-rs` triangle example](https://github.com/gfx-rs/wgpu-rs/blob/v0.5/examples/hello-triangle/main.rs) with a handful of small changes, which I'll outline below:

- Renamed the `main` function to `start_app` and annotated it with `#[mobile_entry_point]`, which generates all the `extern` functions we need for mobile. Note that the name of the function doesn't actually matter for this.
- Changes conditionally compiled on Android:
  - Use `android_logger` instead of `env_logger`
  - Use `Rgba8UnormSrgb` instead of `Bgra8UnormSrgb` (ideally, the supported format would be detected dynamically instead)
  - Use `std::thread::sleep` to shoddily workaround a race condition (oh dear)
  - Render directly upon `MainEventsCleared` instead of calling `request_redraw`, since winit doesn't implement that method on Android yet

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open`/`cargo apple open` to open in Android Studio and Xcode respectively).
