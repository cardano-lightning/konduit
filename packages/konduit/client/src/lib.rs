mod adaptor;
pub use adaptor::Adaptor;

/// The cln rail, consumer side. Enabled with the executable, which is
/// where a rail is named.
#[cfg(feature = "cln")]
pub mod cln;

#[cfg(feature = "cli")]
pub mod cli;

/// Which chain backend the L1 commands talk to. Enabled with the
/// executable, since it's the clap layer that names a backend.
#[cfg(feature = "cli")]
pub mod connector;

pub mod l1;
pub mod l2;

mod prelude;
pub(crate) use prelude::core;
