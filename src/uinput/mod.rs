use anyhow::Result;
use log::debug;

mod inputsource;
use crate::eventspec::EventSpec;
use crate::uinput::inputsource::{InputEvent, InputEventQueue};

pub(crate) fn process_events(
    eventspecs: Vec<EventSpec>,
    start_delay: std::time::Duration,
) -> Result<()> {
    let mut event_queue = InputEventQueue::new()?;
    for inputevent in eventspecs.into_iter().map(InputEvent::from) {
        event_queue.add_event(inputevent);
    }

    debug!("All input events: {:?}", event_queue);
    event_queue.start(start_delay)
}
