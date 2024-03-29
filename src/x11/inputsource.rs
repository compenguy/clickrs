use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use log::{debug, info};
use x11::{xlib, xtest};

use crate::eventspec::EventSpec;
use anyhow::Result;

// X11/extensions/XKB.h:#define    XkbUseCoreKbd           0x0100
const XKBUSECOREKBD: u32 = 0x0100;
// X11/X.h:#define None 0L
//const XNONE: std::os::raw::c_int = 0;

#[derive(Debug, Clone)]
pub struct XContext {
    pub display_name: Option<String>,
    display: *mut xlib::Display,
    window: Option<xlib::Window>,
    key_name_to_code: HashMap<String, u8>,
}

impl std::fmt::Display for XContext {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = self.display_name.clone().unwrap_or_default();
        write!(f, "XDisplay({})", name)
    }
}

impl XContext {
    pub fn new(display_name: Option<String>) -> Self {
        let name_ptr = match display_name {
            Some(ref name_str) => name_str.as_ptr(),
            None => std::ptr::null(),
        };
        unsafe {
            let display = xlib::XOpenDisplay(name_ptr as *const i8);
            if display.is_null() {
                panic!("Failed to open specified display '{:?}'", display_name);
            }
            XContext {
                display_name,
                display,
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
                let c_key_name =
                    std::ffi::CString::new(key_name).expect("Invalid string key representation");
                let keysym = xlib::XStringToKeysym(c_key_name.as_ptr());
                xlib::XKeysymToKeycode(display, keysym)
            });
        debug!("{} -> {}", key_name, *keycode);
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
    /*
    pub fn get_root(&self) -> xlib::Window {
        unsafe {
            xlib::XDefaultRootWindow(self.display)
        }
    }

    pub fn key_xevent(&mut self, key_name: &str) {
        let keycode = self.keycode_lookup(key_name);
        let window = self.window.unwrap_or(xlib::PointerWindow as xlib::Window);
        let mut event = xlib::XEvent {
            key: xlib::XKeyEvent {
                type_: xlib::KeyPress,
                serial: 0,
                send_event: xlib::True,
                display: self.display,
                window,
                root: self.get_root(),
                subwindow: XNONE as xlib::Window,
                time: xlib::CurrentTime,
                x: 1,
                y: 1,
                x_root: 1,
                y_root: 1,
                state: XNONE as std::os::raw::c_uint,
                keycode: keycode as u32,
                same_screen: xlib::True,
            }
        };
        unsafe { xlib::XSendEvent(event.key.display, event.key.window, xlib::True, xlib::KeyPressMask, &mut event as *mut xlib::XEvent) };
        event.type_ = xlib::KeyRelease;
        unsafe {
            xlib::XSendEvent(event.key.display, event.key.window, xlib::True, xlib::KeyReleaseMask, &mut event as *mut xlib::XEvent);
            xlib::XFlush(self.display);
        }
    }

    pub fn mouse_xevent(&self, button: u8) {
        let window = self.window.unwrap_or(xlib::PointerWindow as xlib::Window);
        let mut event = xlib::XEvent {
            button: xlib::XButtonEvent {
                type_: xlib::ButtonPress,
                serial: 0,
                send_event: xlib::True,
                display: self.display,
                window,
                root: self.get_root(),
                subwindow: XNONE as xlib::Window,
                time: xlib::CurrentTime,
                x: 1,
                y: 1,
                x_root: 1,
                y_root: 1,
                state: XNONE as std::os::raw::c_uint,
                button: button as u32,
                same_screen: xlib::True,
            }
        };
        unsafe { xlib::XSendEvent(event.button.display, event.button.window, xlib::True, xlib::ButtonPressMask, &mut event as *mut xlib::XEvent) };
        event.type_ = xlib::ButtonRelease;
        unsafe {
            xlib::XSendEvent(event.button.display, event.button.window, xlib::True, xlib::ButtonReleaseMask, &mut event as *mut xlib::XEvent);
            xlib::XFlush(self.display);
        }
    }
    */
}

#[derive(Debug, Clone)]
pub enum InputType {
    Keyboard(String),
    XKeyboard(u8),
    Mouse(u8),
}

impl InputType {
    fn as_x<F: FnMut(String) -> u8>(&mut self, mut translate_keycode: F) {
        if let InputType::Keyboard(key_name) = self {
            *self = InputType::XKeyboard(translate_keycode(key_name.to_owned()))
        }
    }
}

impl std::fmt::Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

#[derive(Debug, Clone)]
pub struct InputEvent {
    pub event: InputType,
    pub interval: Duration,
    pub remaining: Duration,
}

impl std::fmt::Display for InputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} every {:?}", self.event, self.interval)?;
        if self.remaining > Duration::from_millis(0) {
            write!(f, " ({:?} remaining)", self.remaining)?;
        }
        Ok(())
    }
}

impl From<EventSpec> for InputEvent {
    fn from(eventspec: EventSpec) -> Self {
        let remaining = std::Duration::from_millis(0);
        match eventspec {
            EventSpec::MouseEvent(button, interval) => InputEvent {
                event: InputType::Mouse(button),
                interval,
                remaining,
            },
            EventSpec::KeyboardEvent(key, interval) => InputEvent {
                event: InputType::Keyboard(key),
                interval,
                remaining,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputEventQueue {
    events: VecDeque<InputEvent>,
    xctx: Rc<Mutex<XContext>>,
    last_active: time::Instant,
}

impl InputEventQueue {
    pub fn new(xctx: Rc<Mutex<XContext>>) -> Self {
        InputEventQueue {
            events: VecDeque::new(),
            xctx,
            last_active: time::Instant::now(),
        }
    }

    fn find_insertion_point(&self, event: &mut InputEvent) -> usize {
        event.remaining = event.interval;
        debug!(
            "Looking for insertion point for event with {}s left",
            duration_as_f32(event.remaining)
        );
        for (i, v_event) in self.events.iter().enumerate() {
            debug!(
                "	{} <=> {}",
                duration_as_f32(event.remaining),
                duration_as_f32(v_event.remaining)
            );
            if event.remaining < v_event.remaining {
                debug!("	Found insertion point!");
                return i;
            }
            event.remaining -= v_event.remaining;
            debug!(
                "	time remaining after event in queue: {}",
                duration_as_f32(event.remaining)
            );
        }
        debug!("	at end of queue!");
        self.events.len()
    }

    pub fn add_event(&mut self, mut event: InputEvent) {
        let insert_index = self.find_insertion_point(&mut event);
        // Convert keyboard key name to keycode before inserting
        let mut xctx = self.xctx.lock().expect("X Context lock busy.");
        event.event.as_x(|name| xctx.keycode_lookup(name.as_str()));
        if let Some(ref mut next_event) = self.events.get_mut(insert_index) {
            debug!(
                "current time delta for next event: {}",
                duration_as_f32(next_event.remaining)
            );
            debug!(
                "decrementing time delta for next event by {}",
                duration_as_f32(event.remaining)
            );
            next_event.remaining -= event.remaining;
            debug!(
                "new time delta for next event: {}",
                duration_as_f32(next_event.remaining)
            );
        }
        self.events.insert(insert_index, event);
    }

    pub fn run_next(&mut self) -> Result<()> {
        let event = match self.events.pop_front() {
            None => {
                // Sleep here in case run_next is being called in a tight loop
                // this way we yield time to the OS
                debug!("Nothing to do...");
                std::thread::sleep(Duration::from_millis(100));
                return Ok(());
            }
            Some(e) => e,
        };
        debug!(
            "wall time passed since last check: {:?}",
            self.last_active.elapsed()
        );
        debug!("event time remaining: {:?}", event.remaining);
        if event.remaining > self.last_active.elapsed() {
            // sleep for however much time is left until the next event is ready
            // minus however much time has passed since the last event ran
            std::thread::sleep(event.remaining - self.last_active.elapsed());
            self.last_active = time::Instant::now();
        } else {
            // we're in catch-up time
            // fast-forward the internal clock by however much time was remaining on this event
            self.last_active += event.remaining;
        }
        //self.do_event(&event)?;
        self.do_event_fake(&event)?;
        self.add_event(event);
        Ok(())
    }

    pub fn paused(&self) -> bool {
        debug!("Querying numlock state");
        let mut indicators: u32 = 0;
        let xctx = self.xctx.lock().expect("X Context lock busy.");
        unsafe {
            xlib::XkbGetIndicatorState(xctx.display, XKBUSECOREKBD, &mut indicators as *mut u32);
        }
        // Checking numlock state
        (indicators & 0x02) != 0x02
    }

    pub fn start(&mut self, start_delay: std::Duration) -> Result<()> {
        std::thread::sleep(start_delay);
        let pause_poll = Duration::from_millis(500);
        let mut noise_ctl = std::num::Wrapping(0_u64);
        loop {
            while !self.paused() {
                self.run_next()?;
            }
            if noise_ctl.0 % 10 == 0 {
                info!("Paused...");
            }
            noise_ctl += std::num::Wrapping(1_u64);
            std::thread::sleep(pause_poll);
            self.last_active = time::Instant::now();
        }
    }

    fn do_event_fake(&self, event: &InputEvent) -> Result<()> {
        info!(
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

    /*
    fn do_event(&self, event: &InputEvent) -> Result<()> {
        info!(
            "{} (next in {:2.3}s)",
            event.event,
            duration_as_f32(event.interval)
        );
        let mut xctx = self.xctx.lock().expect("X Context lock busy.");
        match event.event {
            InputType::Mouse(ref button) => xctx.mouse_xevent(*button),
            InputType::Keyboard(ref key) => xctx.key_xevent(key),
        }
        Ok(())
    }
    */
}
