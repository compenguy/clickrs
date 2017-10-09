use std::rc::Rc;
use std::sync::Mutex;
use std::string::String;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;
extern crate loggerv;
extern crate sys_info;

extern crate regex;
extern crate x11;

mod errors;
use errors::Result;

mod inputsource;
use inputsource::{InputEvent, InputEventQueue, XContext};

quick_main!(|| -> Result<()> {
    let app = app_from_crate!("")
        .setting(clap::AppSettings::ColorAuto)
        .setting(clap::AppSettings::ColoredHelp)
        .arg(clap::Arg::with_name("displayname")
            .short("x")
            .short("x11-display")
            .help("The X11 display to send the input to. Default: DISPLAY env var.")
            .value_name("NAME")
            .takes_value(true)
            .required(false))
        .arg(clap::Arg::with_name("initial_delay_ms")
            .short("d")
            .short("delay")
            .help("Delay in msecs before sending any input events.")
            .value_name("N")
            .takes_value(true)
            .required(false)
            .default_value("250"))
        .arg(clap::Arg::with_name("mousebutton_and_interval")
            .short("m")
            .short("mousebutton-and-interval")
            .help("Click mouse button X at regular intervals, with Y msecs between.")
            .value_name("X:Y")
            .takes_value(true)
            .multiple(true)
            .required(false))
        .arg(clap::Arg::with_name("keypress_and_interval")
            .short("k")
            .short("keypress-and-interval")
            .help("Press keyboard key X at regular intervals, with Y msecs between.")
            .value_name("X:Y")
            .takes_value(true)
            .multiple(true)
            .required(false))
        .arg(clap::Arg::with_name("debug")
            .short("g")
            .long("debug")
            .multiple(true)
            .hidden(true)
            .help("print debug information"));

    let matches = app.get_matches();

    // Start logging at "info" verbosity
    loggerv::init_with_verbosity(1 + matches.occurrences_of("debug")).unwrap();

    debug!("{} version {}", crate_name!(), crate_version!());
    debug!("OS:      {}",
           sys_info::os_type().unwrap_or_else(|_| "Unknown".to_owned()));
    debug!("Release: {}",
           sys_info::os_release().unwrap_or_else(|_| "Unknown".to_owned()));
    debug!("Host:    {}",
           sys_info::hostname().unwrap_or_else(|_| "Unknown".to_owned()));

    info!("Welcome to {} version {}!", crate_name!(), crate_version!());
    info!("{}", crate_description!());
    info!("Created by {}", crate_authors!());

    let display = Rc::new(Mutex::new(XContext::new(matches.value_of("displayname")
        .map(|str| str.to_owned()))));
    let mut event_queue = InputEventQueue::new(display);

    if matches.occurrences_of("mousebutton_and_interval") == 0 &&
       matches.occurrences_of("keypress_and_interval") == 0 {
        warn!("No events specified.  Nothing to do...");
        println!("{}", matches.usage());
        return Ok(());
    }

    if let Some(mevent_strs) = matches.values_of("mousebutton_and_interval") {
        for event_str in mevent_strs {
            event_queue.add_event(InputEvent::parse_mouse(event_str)?);
        }
    } else {
        info!("No mousebutton events specified.");
    };

    if let Some(kevent_strs) = matches.values_of("keypress_and_interval") {
        for event_str in kevent_strs {
            event_queue.add_event(InputEvent::parse_key(event_str)?);
        }
    } else {
        info!("No key events specified.");
    };

    debug!("All input events: {:?}", event_queue);
    let start_delay_ms: u64 = matches.value_of("initial_delay_ms").unwrap().parse().unwrap();
    event_queue.start(start_delay_ms)
});
