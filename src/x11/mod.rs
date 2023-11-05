use anyhow::Result;
use log::debug;

mod inputsource;
use crate::eventspec::EventSpec;
use crate::x11::inputsource::{InputEvent, InputEventQueue, XContext};

pub(crate) fn process_events(
    displayname: Option<String>,
    eventspecs: Vec<EventSpec>,
    start_delay: std::time::Duration,
) -> Result<()> {
    let display = std::rc::Rc::new(std::sync::Mutex::new(XContext::new(displayname)));
    let mut event_queue = InputEventQueue::new(display);
    for inputevent in eventspecs.into_iter().map(InputEvent::from) {
        event_queue.add_event(inputevent);
    }

    debug!("All input events: {:?}", event_queue);
    event_queue.start(start_delay)
}
