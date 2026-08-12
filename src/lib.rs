#[cfg(feature = "desktop")]
pub mod chart;
#[cfg(feature = "desktop")]
pub mod chart_engine;
#[cfg(feature = "desktop")]
pub mod contracts;
#[cfg(feature = "desktop")]
pub mod control_plane;
pub mod data_plane;
pub mod ledger;
pub mod market_state;
#[cfg(feature = "desktop")]
pub mod office;
#[cfg(all(feature = "desktop", feature = "fixtures"))]
mod office_fixture;
pub mod operating;
#[cfg(feature = "desktop")]
mod operating_chart;
#[cfg(feature = "desktop")]
pub mod operating_ui;
#[cfg(feature = "desktop")]
mod operating_ui_status;
#[cfg(feature = "desktop")]
pub mod replay;
#[cfg(feature = "desktop")]
pub mod runtime;
#[cfg(all(feature = "desktop", feature = "fixtures"))]
pub mod ui;
#[cfg(feature = "desktop")]
mod ui_theme;

#[cfg(feature = "desktop")]
pub use operating_ui::OperatingApp;
#[cfg(all(feature = "desktop", feature = "fixtures"))]
pub use runtime::PrototypeRuntime;
#[cfg(all(feature = "desktop", feature = "fixtures"))]
pub use ui::NorthstarApp;
