use anyhow::{Context, Result};
use clap::{crate_authors, crate_description, crate_name, crate_version, value_parser, ArgAction};
use flexi_logger::Logger;
use log::{debug, info, warn};

mod errors;
mod eventspec;
#[cfg(feature = "uinput")]
mod uinput;
#[cfg(feature = "x11")]
mod x11;

use crate::eventspec::EventSpec;

// Start logging this crate at "warn" verbosity
const BASE_VERBOSITY: u8 = 2;

fn main() -> Result<()> {
    let mut app = clap::command!("")
        .arg(
            clap::Arg::new("displayname")
                .short('x')
                .long("x11-display")
                .help("The X11 display to send the input to. Default: DISPLAY env var.")
                .value_name("NAME")
                .required(false),
        )
        .arg(
            clap::Arg::new("initial_delay_ms")
                .short('d')
                .long("delay")
                .help("Delay in msecs before sending any input events.")
                .value_name("N")
                .required(false)
                .value_parser(value_parser!(u64))
                .default_value("250"),
        )
        .arg(
            clap::Arg::new("mousebutton_and_interval")
                .short('m')
                .long("mousebutton-and-interval")
                .help("Click mouse button X at regular intervals, with Y msecs between.")
                .value_name("X:Y")
                .action(ArgAction::Append)
                .required(false),
        )
        .arg(
            clap::Arg::new("keypress_and_interval")
                .short('k')
                .long("keypress-and-interval")
                .help("Press keyboard key X at regular intervals, with Y msecs between.")
                .value_name("X:Y")
                .action(ArgAction::Append)
                .required(false),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("show informational output, repeat for increasing verbosity"),
        );

    let matches = app.get_matches_mut();

    let crate_log_level = match BASE_VERBOSITY + matches.get_count("verbose") {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    // At high verbosity, also log errors from other crates
    let general_log_level = match crate_log_level {
        log::LevelFilter::Trace | log::LevelFilter::Debug => log::LevelFilter::Error,
        _ => log::LevelFilter::Off,
    };
    let spec = format!(
        "{}, {} = {}",
        general_log_level,
        clap::crate_name!(),
        crate_log_level
    );
    Logger::try_with_str(spec)?
        .start()
        .context("Failed to start FlexiLogger logging backend")?;

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

    if !matches.contains_id("mousebutton_and_interval")
        && !matches.contains_id("keypress_and_interval")
    {
        warn!("No events specified.  Nothing to do...");
        println!("{}", app.render_usage());
        return Ok(());
    }

    let mut eventspecs: Vec<EventSpec> = Vec::with_capacity(2);
    let mouse_events = matches
        .get_many::<String>("mousebutton_and_interval")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .map(EventSpec::parse_mouse)
        .collect::<Result<Vec<EventSpec>>>()?;
    if mouse_events.is_empty() {
        warn!("No mousebutton events specified.");
    } else {
        eventspecs.extend(mouse_events);
    }

    let keyboard_events = matches
        .get_many::<String>("keypress_and_interval")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .map(EventSpec::parse_key)
        .collect::<Result<Vec<EventSpec>>>()?;
    if keyboard_events.is_empty() {
        warn!("No key events specified.");
    } else {
        eventspecs.extend(keyboard_events);
    }

    let start_delay_ms: u64 = *matches
        .get_one::<u64>("initial_delay_ms")
        .expect("Programming Error: Default was specified for this flag, so there should always be a value present");

    #[cfg(feature = "x11")]
    x11::process_events(
        matches.value_of("displayname").map(|str| str.to_owned()),
        eventspecs,
        std::time::Duration::from_millis(start_delay_ms),
    )?;

    #[cfg(feature = "uinput")]
    uinput::process_events(eventspecs, std::time::Duration::from_millis(start_delay_ms))?;

    Ok(())
}
