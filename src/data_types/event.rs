use crate::MAX_EVENTS;

/// A struct that represents a MIDI event in the audio engine.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Event {
    // 0 = note off, 1 = note on
    pub event_type: i32,
    // A MIDI note number.
    pub pitch: f32,
    // A MIDI velocity value.
    pub velocity: f32,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            event_type: 0,
            pitch: 0.0,
            velocity: 0.0,
        }
    }
}

impl Event {
    pub fn new(event_type: i32, pitch: f32, velocity: f32) -> Self {
        Self {
            event_type,
            pitch,
            velocity,
        }
    }
}

/// A set of MIDI events to send multiple events at once.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EventSlot {
    /// An array of MIDI events.
    pub events: [Event; MAX_EVENTS],
    /// The number of MIDI events in the array.
    pub count: i32,
}

impl Default for EventSlot {
    fn default() -> Self {
        Self {
            events: [Event::default(); MAX_EVENTS],
            count: 0,
        }
    }
}

impl EventSlot {
    /// Returns whether the event slot is full.
    pub fn is_full(&self) -> bool {
        self.count >= MAX_EVENTS as i32
    }

    /// Adds a MIDI event to the array of events. Returns `true` if the event was added successfully, `false` if the array is already full.
    pub fn add_event(&mut self, event: Event) -> bool {
        if self.count < MAX_EVENTS as i32 {
            self.events[self.count as usize] = event;
            self.count += 1;
            true
        } else {
            false
        }
    }
}
