#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod error;
mod geometry;
mod presentation;
mod window;

pub use error::{SplashError, SplashFailure};
pub use presentation::{SplashPhase, SplashReady, SplashShown};
pub use window::{Splash, SplashPlugin, SplashSystems, SplashWindow};
