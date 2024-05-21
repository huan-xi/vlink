
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::Router;


#[cfg(any(target_os = "macos", target_os = "darwin"))]
mod darwin;


#[cfg(any(target_os = "macos", target_os = "darwin"))]
pub use darwin::Router;
pub trait IRouter {

}