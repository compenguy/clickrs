use anyhow::Result;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};
use log::{debug, info, warn};

mod errors;
mod eventspec;
mod x11;

use crate::eventspec::EventSpec;

const BASE_VERBOSITY: u64 = 0;

fn main() -> Result<()> {
    let mut app = app_from_crate!("")
        .arg(
            clap::Arg::new("displayname")
                .short('x')
                .long("x11-display")
                .help("The X11 display to send the input to. Default: DISPLAY env var.")
                .value_name("NAME")
                .takes_value(true)
                .required(false),
        )
        .arg(
            clap::Arg::new("initial_delay_ms")
                .short('d')
                .long("delay")
                .help("Delay in msecs before sending any input events.")
                .value_name("N")
                .takes_value(true)
                .required(false)
                .default_value("250"),
        )
        .arg(
            clap::Arg::new("mousebutton_and_interval")
                .short('m')
                .long("mousebutton-and-interval")
                .help("Click mouse button X at regular intervals, with Y msecs between.")
                .value_name("X:Y")
                .takes_value(true)
                .multiple_occurrences(true)
                .required(false),
        )
        .arg(
            clap::Arg::new("keypress_and_interval")
                .short('k')
                .long("keypress-and-interval")
                .help("Press keyboard key X at regular intervals, with Y msecs between.")
                .value_name("X:Y")
                .takes_value(true)
                .multiple_occurrences(true)
                .required(false),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("show informational output, repeat for increasing verbosity"),
        );

    let matches = app.get_matches_mut();

    // Start logging at "warn" verbosity
    loggerv::init_with_verbosity(BASE_VERBOSITY + matches.occurrences_of("verbose")).unwrap();

    debug!("{} version {}", crate_name!(), crate_version!());
    debug!(
        "OS:      {}",
        sys_info::os_type().unwrap_or_else(|_| "Unknown".to_owned())
    );
    debug!(
        "Release: {}",
        sys_info::os_release().unwrap_or_else(|_| "Unknown".to_owned())
    );
    debug!(
        "Host:    {}",
        sys_info::hostname().unwrap_or_else(|_| "Unknown".to_owned())
    );

    info!("Welcome to {} version {}!", crate_name!(), crate_version!());
    info!("{}", crate_description!());
    info!("Created by {}", crate_authors!());

    if matches.occurrences_of("mousebutton_and_interval") == 0
        && matches.occurrences_of("keypress_and_interval") == 0
    {
        warn!("No events specified.  Nothing to do...");
        println!("{}", app.render_usage());
        return Ok(());
    }

    let mut eventspecs: Vec<EventSpec> = Vec::with_capacity(2);
    if let Some(mevent_strs) = matches.values_of("mousebutton_and_interval") {
        eventspecs.extend(
            mevent_strs
                .into_iter()
                .map(EventSpec::parse_mouse)
                .collect::<Result<Vec<EventSpec>>>()?,
        );
    } else {
        warn!("No mousebutton events specified.");
    };

    if let Some(kevent_strs) = matches.values_of("keypress_and_interval") {
        eventspecs.extend(
            kevent_strs
                .into_iter()
                .map(EventSpec::parse_key)
                .collect::<Result<Vec<EventSpec>>>()?,
        );
    } else {
        warn!("No key events specified.");
    };

    let start_delay_ms: u64 = matches
        .value_of("initial_delay_ms")
        .unwrap()
        .parse()
        .unwrap();

    #[cfg(feature = "x11")]
    x11::process_events(
        matches.value_of("displayname").map(|str| str.to_owned()),
        eventspecs,
        std::time::Duration::from_millis(start_delay_ms),
    )
}
