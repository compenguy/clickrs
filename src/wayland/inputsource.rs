use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::EventSpec;
use anyhow::Result;
use log::{debug, info};
use uinput::event::controller::Mouse;
use uinput::event::keyboard::Key;
use uinput::event::relative::Position::{X, Y};
use uinput::event::Relative::Position;
use uinput::Event::Relative;

struct NumlockWatcher {
    keyboard_devices: Vec<evdev::Device>,
}

impl std::fmt::Debug for NumlockWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let keyboard_statuses: Vec<(String, Result<bool>)> = self
            .keyboard_devices
            .iter()
            .map(|d| {
                (
                    d.name().unwrap_or_default().to_string(),
                    d.get_led_state()
                        .map(|l| l.contains(evdev::LedType::LED_NUML))
                        .map_err(|e| e.into()),
                )
            })
            .collect();
        write!(f, "{:?}", keyboard_statuses)
    }
}

impl Default for NumlockWatcher {
    // See https://github.com/emberian/evdev/blob/main/examples/_pick_device.rs
    fn default() -> Self {
        let keyboard_devices = evdev::enumerate()
            .map(|t| t.1)
            .inspect(|d| debug!("Found input device {:?}", d.name()))
            .filter(|d| {
                d.name()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("keyboard")
            })
            .inspect(|d| debug!("Found keyboard input device {:?}", d.name()))
            .collect::<Vec<_>>();
        Self { keyboard_devices }
    }
}

impl NumlockWatcher {
    fn enabled(&self) -> bool {
        self.keyboard_devices
            .iter()
            .map(|d| {
                d.get_led_state()
                    .map(|l| l.contains(evdev::LedType::LED_NUML))
            })
            .find(|state_res| *state_res.as_ref().unwrap_or(&false))
            .unwrap_or(Ok(false))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct ModifiedEvent {
    pub event: uinput::Event,
    shift: bool,
    control: bool,
    alt: bool,
}

impl std::fmt::Display for ModifiedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut chain: bool = false;
        if self.shift {
            if chain {
                write!(f, " + ")?;
            }
            write!(f, "<SHIFT>")?;
            chain = true;
        }
        if self.control {
            if chain {
                write!(f, " + ")?;
            }
            write!(f, "<CONTROL>")?;
            chain = true;
        }
        if self.alt {
            if chain {
                write!(f, " + ")?;
            }
            write!(f, "<ALT>")?;
            chain = true;
        }

        if chain {
            write!(f, " + ")?;
        }
        write!(f, "{:?}", self.event)
    }
}

impl From<u8> for ModifiedEvent {
    fn from(button: u8) -> Self {
        let event = match button {
            // I can't remember whether the x11 code started counting from 0 or 1
            0 => Mouse::Left.into(),
            1 => Mouse::Left.into(),
            2 => Mouse::Middle.into(),
            3 => Mouse::Right.into(),
            // TODO: we should error here
            _ => Mouse::Extra.into(),
        };
        ModifiedEvent {
            event,
            shift: false,
            alt: false,
            control: false,
        }
    }
}

impl From<&str> for ModifiedEvent {
    fn from(key: &str) -> Self {
        // See https://github.com/meh/rust-uinput
        let (event, shift) = match key {
            "1" => (Key::_1.into(), false),
            "2" => (Key::_2.into(), false),
            "3" => (Key::_3.into(), false),
            "4" => (Key::_4.into(), false),
            "5" => (Key::_5.into(), false),
            "6" => (Key::_6.into(), false),
            "7" => (Key::_7.into(), false),
            "8" => (Key::_8.into(), false),
            "9" => (Key::_9.into(), false),
            "0" => (Key::_0.into(), false),
            "!" => (Key::_1.into(), true),
            "@" => (Key::_2.into(), true),
            "#" => (Key::_3.into(), true),
            "$" => (Key::_4.into(), true),
            "%" => (Key::_5.into(), true),
            "^" => (Key::_6.into(), true),
            "&" => (Key::_7.into(), true),
            "*" => (Key::_8.into(), true),
            "(" => (Key::_9.into(), true),
            ")" => (Key::_0.into(), true),
            "a" => (Key::A.into(), false),
            "b" => (Key::B.into(), false),
            "c" => (Key::C.into(), false),
            "d" => (Key::D.into(), false),
            "e" => (Key::E.into(), false),
            "f" => (Key::F.into(), false),
            "g" => (Key::G.into(), false),
            "h" => (Key::H.into(), false),
            "i" => (Key::I.into(), false),
            "j" => (Key::J.into(), false),
            "k" => (Key::K.into(), false),
            "l" => (Key::L.into(), false),
            "m" => (Key::M.into(), false),
            "n" => (Key::N.into(), false),
            "o" => (Key::O.into(), false),
            "p" => (Key::P.into(), false),
            "q" => (Key::Q.into(), false),
            "r" => (Key::R.into(), false),
            "s" => (Key::S.into(), false),
            "t" => (Key::T.into(), false),
            "u" => (Key::U.into(), false),
            "v" => (Key::V.into(), false),
            "w" => (Key::W.into(), false),
            "x" => (Key::X.into(), false),
            "y" => (Key::Y.into(), false),
            "z" => (Key::Z.into(), false),
            "A" => (Key::A.into(), true),
            "B" => (Key::B.into(), true),
            "C" => (Key::C.into(), true),
            "D" => (Key::D.into(), true),
            "E" => (Key::E.into(), true),
            "F" => (Key::F.into(), true),
            "G" => (Key::G.into(), true),
            "H" => (Key::H.into(), true),
            "I" => (Key::I.into(), true),
            "J" => (Key::J.into(), true),
            "K" => (Key::K.into(), true),
            "L" => (Key::L.into(), true),
            "M" => (Key::M.into(), true),
            "N" => (Key::N.into(), true),
            "O" => (Key::O.into(), true),
            "P" => (Key::P.into(), true),
            "Q" => (Key::Q.into(), true),
            "R" => (Key::R.into(), true),
            "S" => (Key::S.into(), true),
            "T" => (Key::T.into(), true),
            "U" => (Key::U.into(), true),
            "V" => (Key::V.into(), true),
            "W" => (Key::W.into(), true),
            "X" => (Key::X.into(), true),
            "Y" => (Key::Y.into(), true),
            "Z" => (Key::Z.into(), true),
            "," => (Key::Comma.into(), false),
            "." => (Key::Dot.into(), false),
            "/" => (Key::Slash.into(), false),
            "<" => (Key::Comma.into(), true),
            ">" => (Key::Dot.into(), true),
            "?" => (Key::Slash.into(), true),
            " " => (Key::Space.into(), false),
            // TODO: cover more characters: -_=+[{]}\|;:'"`~
            // TODO: we should error here
            _ => (Key::Reserved.into(), false),
        };

        ModifiedEvent {
            event,
            shift,
            alt: false,
            control: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputEvent {
    pub event: ModifiedEvent,
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
        let remaining = Duration::from_millis(0);
        match eventspec {
            EventSpec::MouseEvent(button, interval) => InputEvent {
                event: ModifiedEvent::from(button),
                interval,
                remaining,
            },
            EventSpec::KeyboardEvent(key, interval) => InputEvent {
                event: ModifiedEvent::from(key.as_str()),
                interval,
                remaining,
            },
        }
    }
}

pub struct InputEventQueue {
    numlock_state: NumlockWatcher,
    uinput_device: uinput::Device,
    events: VecDeque<InputEvent>,
    last_active: Instant,
}

impl std::fmt::Debug for InputEventQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Numlock: {:?}, events: {:?}, last_active: {:?}",
            &self.numlock_state, &self.events, &self.last_active
        )
    }
}

fn duration_as_f32(duration: Duration) -> f32 {
    (duration.as_secs() as f32) + ((duration.subsec_nanos() as f32) / 1000000000.0)
}

impl InputEventQueue {
    pub fn new() -> Result<Self> {
        // See https://github.com/meh/rust-uinput
        let device = uinput::default()?
            .name("clickrs")?
            .event(uinput::event::Keyboard::All)?
            .event(uinput::event::Controller::All)?
            .event(Relative(Position(X)))?
            .event(Relative(Position(Y)))?
            .create()?;

        Ok(InputEventQueue {
            numlock_state: NumlockWatcher::default(),
            uinput_device: device,
            events: VecDeque::new(),
            last_active: Instant::now(),
        })
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
            self.last_active = Instant::now();
        } else {
            // we're in catch-up time
            // fast-forward the internal clock by however much time was remaining on this event
            self.last_active += event.remaining;
        }
        self.do_event(&event)?;
        self.add_event(event);
        Ok(())
    }

    pub fn paused(&self) -> bool {
        debug!("Querying numlock state");
        !self.numlock_state.enabled()
    }

    pub fn start(&mut self, start_delay: Duration) -> Result<()> {
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
            self.last_active = Instant::now();
        }
    }

    fn do_event(&mut self, event: &InputEvent) -> Result<()> {
        info!(
            "{} (next in {:2.3}s)",
            event.event,
            duration_as_f32(event.interval)
        );
        if event.event.shift {
            self.uinput_device.press(&Key::LeftShift)?;
        }
        if event.event.alt {
            self.uinput_device.press(&Key::LeftAlt)?;
        }
        if event.event.control {
            self.uinput_device.press(&Key::LeftControl)?;
        }

        self.uinput_device.synchronize()?;
        self.uinput_device.send(event.event.event, 1)?;
        self.uinput_device.synchronize()?;
        self.uinput_device.send(event.event.event, 0)?;
        self.uinput_device.synchronize()?;

        if event.event.control {
            self.uinput_device.release(&Key::LeftControl)?;
        }
        if event.event.alt {
            self.uinput_device.release(&Key::LeftAlt)?;
        }
        if event.event.shift {
            self.uinput_device.release(&Key::LeftShift)?;
        }
        Ok(())
    }
}
