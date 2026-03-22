// use tracing_subscriber::{
//     EnvFilter,
//     fmt::{self, layer},
//     layer::SubscriberExt,
//     util::SubscriberInitExt,
// };

// pub fn init_logs() {
//     tracing_subscriber::registry()
//         .with(fmt::layer().with_ansi(true))
//         // .with(EnvFilter::from_default_env())
//         .init();
// }


// tracing dont give coloured logs , so not using tracing and logs

use tracing_subscriber::{fmt, EnvFilter};


pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_ansi(true) // enable colors
        .init();
}
