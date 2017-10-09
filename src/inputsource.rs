use std::fmt;
use std::time;
use std::thread;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::ptr;

use regex::Regex;
use x11::xlib;

use errors::{Result, ErrorKind};

#[derive(Debug)]
#[derive(Clone)]
pub struct XContext {
    pub display_name: Option<String>,
    pub display: *mut xlib::Display,
}

impl fmt::Display for XContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.display_name.clone().unwrap_or_else(|| "".to_owned());
        write!(f, "XDisplay({})", name)
    }
}

impl XContext {
    pub fn new(display_name: Option<String>) -> Self {

        let name_ptr = match display_name {
            Some(ref name_str) => name_str.as_ptr(),
            None => ptr::null(),
        };
        unsafe {
            XContext {
                display_name: display_name,
                display: xlib::XOpenDisplay(name_ptr as *const i8),
            }
        }
    }
    unsafe fn xdisplay(&mut self) -> *mut xlib::Display {
        self.display as *mut xlib::Display
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub enum InputType {
    Keyboard(String),
    Mouse(u8),
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InputType::Keyboard(ref key) => write!(f, "key {:>8}", key),
            InputType::Mouse(ref but) => write!(f, "button {:>5}", but),
        }
    }
}

fn duration_as_f32(duration: time::Duration) -> f32 {
    (duration.as_secs() as f32) + ((duration.subsec_nanos() as f32) / 1000000000.0)
}

#[derive(Debug)]
#[derive(Clone)]
pub struct InputEvent {
    pub event: InputType,
    pub interval: time::Duration,
    pub remaining: time::Duration,
}

impl fmt::Display for InputEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} every {:?}", self.event, self.interval)?;
        if self.remaining > time::Duration::from_millis(0) {
            write!(f, " ({:?} remaining)", self.remaining)?;
        }
        Ok(())
    }
}

impl InputEvent {
    pub fn parse_mouse(arg: &str) -> Result<Self> {
        debug!("Parsing mouse str option {}.", arg);
        lazy_static! {
            static ref MOUSE_SPEC_RE: Regex = Regex::new(concat!(r"^(?P<button>[[:digit:]])",
                                                         r":",
                                                         r"(?P<interval>\d+)$")).unwrap();
        }

        if let Some(caps) = MOUSE_SPEC_RE.captures(arg) {
            let button = caps.name("button").unwrap().as_str().parse()?;
            let interval = caps.name("interval").unwrap().as_str().parse()?;
            Ok(InputEvent {
                event: InputType::Mouse(button),
                interval: time::Duration::from_millis(interval),
                remaining: time::Duration::from_millis(0),
            })
        } else {
            Err(ErrorKind::InvalidMouseEventSpec(arg.to_owned()).into())
        }
    }

    pub fn parse_key(arg: &str) -> Result<Self> {
        debug!("Parsing keyboard str option {}.", arg);
        lazy_static! {
            static ref KEY_SPEC_RE: Regex = Regex::new(concat!(r"^(?P<key>[[:print:]]+)",
                                                       r":",
                                                       r"(?P<interval>\d+)$")).unwrap();
        }

        if let Some(caps) = KEY_SPEC_RE.captures(arg) {
            let key = caps.name("key").unwrap().as_str().to_owned();
            let interval = caps.name("interval").unwrap().as_str().parse()?;
            Ok(InputEvent {
                event: InputType::Keyboard(key),
                interval: time::Duration::from_millis(interval),
                remaining: time::Duration::from_millis(0),
            })
        } else {
            Err(ErrorKind::InvalidKeyboardEventSpec(arg.to_owned()).into())
        }
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct InputEventQueue {
    events: VecDeque<InputEvent>,
    display: Arc<Mutex<XContext>>,
    last_active: time::Instant,
}

impl InputEventQueue {
    pub fn new(display: Arc<Mutex<XContext>>) -> Self {
        InputEventQueue {
            events: VecDeque::new(),
            display: display,
            last_active: time::Instant::now(),
        }
    }

    fn find_insertion_point(&self, event: &mut InputEvent) -> usize {
        event.remaining = event.interval;
        debug!("Looking for insertion point for event with {}s left",
               duration_as_f32(event.remaining));
        for (i, v_event) in self.events.iter().enumerate() {
            debug!("	{} <=> {}",
                   duration_as_f32(event.remaining),
                   duration_as_f32(v_event.remaining));
            if event.remaining < v_event.remaining {
                debug!("	Found insertion point!");
                return i;
            }
            event.remaining -= v_event.remaining;
            debug!("	time remaining after event in queue: {}",
                   duration_as_f32(event.remaining));
        }
        debug!("	at end of queue!");
        self.events.len()
    }

    pub fn add_event(&mut self, mut event: InputEvent) {
        let insert_index = self.find_insertion_point(&mut event);
        if let Some(ref mut next_event) = self.events.get_mut(insert_index) {
            debug!("current time delta for next event: {}",
                   duration_as_f32(next_event.remaining));
            debug!("decrementing time delta for next event by {}",
                   duration_as_f32(event.remaining));
            next_event.remaining -= event.remaining;
            debug!("new time delta for next event: {}",
                   duration_as_f32(next_event.remaining));
        }
        self.events.insert(insert_index, event);
    }

    pub fn run_next(&mut self) -> Result<()> {
        if self.events.is_empty() {
            // Sleep here in case run_next is being called in a tight loop
            // this way we yield time to the OS
            debug!("Nothing to do...");
            thread::sleep(time::Duration::from_millis(100));
            return Ok(());
        }

        let event = self.events.pop_front().unwrap();
        debug!("wall time passed since last check: {:?}",
               self.last_active.elapsed());
        debug!("event time remaining: {:?}", event.remaining);
        if event.remaining > self.last_active.elapsed() {
            // sleep for however much time is left until the next event is ready
            // minus however much time has passed since the last event ran
            thread::sleep(event.remaining - self.last_active.elapsed());
            self.last_active = time::Instant::now();
        } else {
            // we're in catch-up time
            // fast-forward the internal clock by however much time was remaining on this event
            self.last_active += event.remaining;
        }
        self.do_event(&event)?;
        self.add_event(event);
        Ok(())
    }

    fn do_event(&self, event: &InputEvent) -> Result<()> {
        // TODO: actually do the event
        let secs_fl: f32 = event.interval.as_secs() as f32;
        let subsecs_fl: f32 = (event.interval.subsec_nanos() as f32) / 1000000000.0;
        info!("{} (next in {:2.3}s)", event.event, secs_fl + subsecs_fl);
        Ok(())
    }
}
