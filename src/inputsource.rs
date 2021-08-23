use std::collections::HashMap;
use std::fmt;
use std::num::Wrapping;
use std::ptr;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::task::{Context, Poll};
use std::pin::Pin;

use once_cell::sync::Lazy;
use regex::Regex;
use x11::xlib;
use x11::xtest;

use crate::errors::Error;
use anyhow::Result;

// X11/extensions/XKB.h:#define    XkbUseCoreKbd           0x0100
const XKBUSECOREKBD: u32 = 0x0100;
// X11/X.h:#define None 0L
//const XNONE: std::os::raw::c_int = 0;

static MOUSE_SPEC_RE: Lazy<Mutex<Regex>> = Lazy::new(|| {
    Mutex::new(
        Regex::new(concat!(
            r"^(?P<button>[[:digit:]])",
            r":",
            r"(?P<interval>\d+)$"
        ))
        .expect("Failed while processing mouse specification regex"),
    )
});
static KEY_SPEC_RE: Lazy<Mutex<Regex>> = Lazy::new(|| {
    Mutex::new(
        Regex::new(concat!(
            r"^(?P<key>[[:print:]]+)",
            r":",
            r"(?P<interval>\d+)$"
        ))
        .expect("Failed while processing keyboard specification regex"),
    )
});

#[derive(Debug, Clone)]
pub struct XContext {
    pub display_name: Option<String>,
    display: *mut xlib::Display,
    window: Option<xlib::Window>,
    key_name_to_code: HashMap<String, u8>,
}

impl fmt::Display for XContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.display_name.clone().unwrap_or_default();
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
                display_name,
                display: xlib::XOpenDisplay(name_ptr as *const i8),
                window: None,
                key_name_to_code: HashMap::new(),
            }
        }
    }

    pub fn keycode_lookup(&mut self, key_name: &str) -> u8 {
        let display = self.display;
        let keycode = self
            .key_name_to_code
            .entry(key_name.to_owned())
            .or_insert_with(|| unsafe {
                let keysym = xlib::XStringToKeysym(key_name.as_ptr() as *const i8);
                xlib::XKeysymToKeycode(display, keysym)
            });
        log::debug!("{} -> {}", key_name, *keycode);
        *keycode
    }

    pub fn get_window(&self) -> (xlib::Window, i32) {
        let mut win: xlib::Window = xlib::PointerRoot as xlib::Window;
        let mut state: std::os::raw::c_int = 0;
        unsafe {
            xlib::XGetInputFocus(
                self.display,
                &mut win as *mut xlib::Window,
                &mut state as *mut std::os::raw::c_int,
            );
        }
        (win, state)
    }

    pub fn set_focus(&self, win: xlib::Window, state: i32) {
        unsafe {
            xlib::XSetInputFocus(self.display, win, state, xlib::CurrentTime);
        }
    }

    pub fn flip_to_saved_window(&self) -> (xlib::Window, i32) {
        let (old_win, state) = self.get_window();
        let new_win = self.window.unwrap_or(old_win);
        self.set_focus(new_win, state);
        (old_win, state)
    }

    pub fn restore_original_window(&self, saved: (xlib::Window, i32)) {
        let (win, state) = saved;
        self.set_focus(win, state);
    }

    pub fn flush_events(&self) {
        unsafe {
            xlib::XFlush(self.display);
        }
    }

    pub fn fake_button_event(&self, button: u8) {
        unsafe {
            xtest::XTestFakeButtonEvent(self.display, button as u32, xlib::True, xlib::CurrentTime);
            xtest::XTestFakeButtonEvent(
                self.display,
                button as u32,
                xlib::False,
                xlib::CurrentTime,
            );
        }
    }

    pub fn send_button_event_to_window(&self, button: u8) {
        let saved = self.flip_to_saved_window();
        self.fake_button_event(button);
        self.restore_original_window(saved);
        self.flush_events();
    }

    pub fn fake_key_event(&self, keycode: u8) {
        unsafe {
            xtest::XTestFakeKeyEvent(self.display, keycode as u32, xlib::True, xlib::CurrentTime);
            xtest::XTestFakeKeyEvent(self.display, keycode as u32, xlib::False, xlib::CurrentTime);
        }
    }

    pub fn send_key_event_to_window(&mut self, keycode: u8) {
        let saved = self.flip_to_saved_window();
        self.fake_key_event(keycode);
        self.restore_original_window(saved);
        self.flush_events();
    }

    pub fn send_key_to_window(&mut self, key_name: &str) {
        let keycode = self.keycode_lookup(key_name);
        self.send_key_event_to_window(keycode);
        let saved = self.flip_to_saved_window();
        self.fake_key_event(keycode);
        self.restore_original_window(saved);
        self.flush_events();
    }
}

#[derive(Debug, Clone)]
pub enum InputType {
    Keyboard(String),
    XKeyboard(u8),
    Mouse(u8),
}

impl InputType {
    fn to_x<F: FnMut(String) -> u8>(&mut self, mut translate_keycode: F) {
        if let InputType::Keyboard(key_name) = self {
            *self = InputType::XKeyboard(translate_keycode(key_name.to_owned()))
        }
    }
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InputType::Keyboard(ref key) => write!(f, "key {:>8}", key),
            InputType::XKeyboard(ref key) => write!(f, "key {:>8}", key),
            InputType::Mouse(ref but) => write!(f, "button {:>5}", but),
        }
    }
}

fn duration_as_f32(duration: Duration) -> f32 {
    (duration.as_secs() as f32) + ((duration.subsec_nanos() as f32) / 1000000000.0)
}

#[derive(Debug)]
pub struct InputEventSource {
    event: InputType,
    timer: tokio::time::Interval,
    interval: Duration,
}

impl std::stream::Stream for InputEventSource {
    type Item = InputType;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if std::pin::Pin::new(&mut self.timer).poll_next(cx).is_pending() {
            return Poll::Pending;
        }
        let interval = self.interval;
        let _ = std::mem::replace(&mut self.timer, tokio::time::interval(interval));
        Poll::Ready(Some(self.event.clone()))
    }
}

impl InputEventSource {
    pub fn from_mouse_spec(arg: &str) -> Result<Self> {
        log::debug!("Parsing mouse str option {}.", arg);

        if let Some(caps) = MOUSE_SPEC_RE
            .lock()
            .expect("Mouse spec regex lock busy")
            .captures(arg)
        {
            let button = caps
                .name("button")
                .ok_or_else(|| Error::InvalidMouseEventSpec(arg.to_owned()))?
                .as_str()
                .parse()?;
            let interval = Duration::from_millis(caps
                .name("interval")
                .ok_or_else(|| Error::InvalidMouseEventSpec(arg.to_owned()))?
                .as_str()
                .parse()?);
            Ok(InputEventSource {
                event: InputType::Mouse(button),
                timer: tokio::time::interval(interval),
                interval,
            })
        } else {
            Err(Error::InvalidMouseEventSpec(arg.to_owned()).into())
        }
    }

    pub fn from_key_spec(arg: &str) -> Result<Self> {
        log::debug!("Parsing keyboard str option {}.", arg);

        if let Some(caps) = KEY_SPEC_RE
            .lock()
            .expect("Keyboard spec regex lock busy")
            .captures(arg)
        {
            let key = caps
                .name("key")
                .ok_or_else(|| Error::InvalidKeyboardEventSpec(arg.to_owned()))?
                .as_str()
                .to_owned();
            let interval = Duration::from_millis(caps
                .name("interval")
                .ok_or_else(|| Error::InvalidKeyboardEventSpec(arg.to_owned()))?
                .as_str()
                .parse()?);
            Ok(InputEventSource {
                event: InputType::Keyboard(key),
                timer: tokio::time::interval(interval),
                interval,
            })
        } else {
            Err(Error::InvalidKeyboardEventSpec(arg.to_owned()).into())
        }
    }

    pub fn reset(self: Pin<&mut Self>) {
        let _ = std::mem::replace(&mut self.timer, tokio::time::interval(self.interval.clone()));
    }
}

#[derive(Debug)]
pub struct InputStreamReader {
    events: Vec<Pin<InputEventSource>>,
    xctx: Rc<Mutex<XContext>>,
}

impl InputStreamReader {
    pub fn new(xctx: Rc<Mutex<XContext>>) -> Self {
        Self {
            events: Vec::new(),
            xctx,
        }
    }

    pub fn add_event(&mut self, event: InputEventSource) {
        self.events.push(Pin::new(event));
    }

    pub fn run_next(&mut self) -> Result<()> {
        for event_stream in self.events {
            if let Poll::Ready(e) = event_stream.poll_next() {
                self.do_event_fake(&e)?;
            }
        }
        Ok(())
    }

    pub fn paused(&self) -> bool {
        log::debug!("Querying numlock state");
        let mut indicators: u32 = 0;
        let xctx = self.xctx.lock().expect("X Context lock busy.");
        unsafe {
            xlib::XkbGetIndicatorState(xctx.display, XKBUSECOREKBD, &mut indicators as *mut u32);
        }
        // Checking numlock state
        (indicators & 0x02) != 0x02
    }

    pub fn start(&mut self, start_delay_ms: u64) -> Result<()> {
        thread::sleep(Duration::from_millis(start_delay_ms));
        for event in self.events {
            event.reset();
        }
        let pause_poll = Duration::from_millis(500);
        let mut noise_ctl = Wrapping(0_u64);
        loop {
            while !self.paused() {
                self.run_next()?;
            }
            if noise_ctl.0 % 10 == 0 {
                log::info!("Paused...");
            }
            noise_ctl += Wrapping(1_u64);
            thread::sleep(pause_poll);
        }
    }

    fn do_event_fake(&self, event: &InputEventSource) -> Result<()> {
        log::info!(
            "{} (next in {:2.3}s)",
            event.event,
            duration_as_f32(event.interval)
        );
        let mut xctx = self.xctx.lock().expect("X Context lock busy.");
        match event.event {
            InputType::Mouse(ref button) => xctx.send_button_event_to_window(*button),
            InputType::Keyboard(ref key) => xctx.send_key_to_window(key),
            InputType::XKeyboard(ref key) => xctx.send_key_event_to_window(*key),
        }
        Ok(())
    }
}
