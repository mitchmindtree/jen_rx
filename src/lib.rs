use nannou::osc;
use std::collections::HashMap;
use std::time::Instant;

/// The most recently received state of Jen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The last time a note on was received for each instrument.
    pub note_ons: HashMap<Instrument, Instant>,
    /// The last time a playhead bang was received.
    pub playhead_bangs: HashMap<Measure, Instant>,
    /// The most recently received playhead position.
    pub playhead_positions: HashMap<Measure, f32>,
}

/// Some event emitted by Jen.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    NoteOn(Instrument),
    PlayheadBang(Measure),
    PlayheadPosition(Measure, f32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instrument {
    Snare,
    Kick,
    Ride,
    Ghost,
    Bass,
    Melodic,
    Chordal,
    Atmos,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Measure {
    Phrase,
    Segment,
    Bar,
    Minim,
    Beat,
    Quaver,
    SemiQuaver,
}

const NOTE_ON: i32 = 100;
const PLAYHEAD_BANG: i32 = 101;
const PLAYHEAD_POSITION: i32 = 102;

impl State {
    /// Construct an empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the jen state via the given events.
    pub fn update_by_events<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = Event>,
    {
        let now = Instant::now();
        for event in events {
            match event {
                Event::NoteOn(inst) => {
                    self.note_ons.insert(inst, now);
                }
                Event::PlayheadBang(meas) => {
                    self.playhead_bangs.insert(meas, now);
                }
                Event::PlayheadPosition(meas, pos) => {
                    self.playhead_positions.insert(meas, pos);
                }
            }
        }
    }

    /// Convert all the messages within the given packet into events and use them to update the
    /// state.
    pub fn update_by_osc_packet(&mut self, packet: osc::Packet) {
        for msg in packet.into_msgs() {
            self.update_by_events(osc_msg_to_events(msg));
        }
    }

    /// Returns `None` if no events have been received for that instrument.
    pub fn secs_since_note_on(&self, inst: Instrument) -> Option<f64> {
        let now = Instant::now();
        self.note_ons.get(&inst).and_then(|&then| {
            if then > now {
                return None
            } else {
                let duration = nannou::state::time::Duration::from(now.duration_since(then));
                Some(duration.secs())
            }
        })
    }

    /// Returns `None` if no events have been received for that measure.
    pub fn secs_since_measure(&self, meas: Measure) -> Option<f64> {
        let now = Instant::now();
        self.playhead_bangs.get(&meas).and_then(|&then| {
            if then > now {
                None
            } else {
                let duration = nannou::state::time::Duration::from(now.duration_since(then));
                Some(duration.secs())
            }
        })
    }

    /// The playhead position over the given measure.
    pub fn playhead_position(&self, meas: Measure) -> Option<f32> {
        self.playhead_positions.get(&meas).map(|&f| f)
    }
}

impl Instrument {
    pub const TOTAL_VARIANTS: usize = 8;

    pub fn from_i32(i: i32) -> Option<Self> {
        match i {
            0 => Some(Instrument::Snare),
            1 => Some(Instrument::Kick),
            2 => Some(Instrument::Ride),
            3 => Some(Instrument::Ghost),
            4 => Some(Instrument::Bass),
            5 => Some(Instrument::Melodic),
            6 => Some(Instrument::Chordal),
            7 => Some(Instrument::Atmos),
            _ => None,
        }
    }
}

impl Measure {
    pub const TOTAL_VARIANTS: usize = 7;

    pub fn from_i32(i: i32) -> Option<Self> {
        match i {
            0 => Some(Measure::Phrase),
            1 => Some(Measure::Segment),
            2 => Some(Measure::Bar),
            3 => Some(Measure::Minim),
            4 => Some(Measure::Beat),
            5 => Some(Measure::Quaver),
            6 => Some(Measure::SemiQuaver),
            _ => None,
        }
    }
}

/// Convert the given OSC message to a list of events.
pub fn osc_msg_to_events(msg: osc::Message) -> Vec<Event> {
    let mut events = vec![];

    // OSC address must be jen.
    if msg.addr != "/jen" {
        return events;
    }

    // A message without args indicates no events.
    let args = match msg.args {
        Some(args) => args,
        None => return events,
    };

    // Test if an `Int` message is a mode.
    fn int_is_mode(i: i32) -> bool {
        i == NOTE_ON || i == PLAYHEAD_BANG || i == PLAYHEAD_POSITION
    }

    // Decode the events.
    let mut iter = args.into_iter();
    let mut arg = iter.next();
    'modes: loop {
        match arg {
            None => break,
            Some(osc::Type::Int(NOTE_ON)) => {
                loop {
                    match iter.next() {
                        Some(osc::Type::Int(i)) if !int_is_mode(i) => {
                            let inst = Instrument::from_i32(i).expect("unexpected instrument");
                            events.push(Event::NoteOn(inst));
                        }
                        a => {
                            arg = a;
                            continue 'modes;
                        }
                    }
                }
            }
            Some(osc::Type::Int(PLAYHEAD_BANG)) => {
                loop {
                    match iter.next() {
                        Some(osc::Type::Int(i)) if !int_is_mode(i) => {
                            let measure = Measure::from_i32(i).expect("unexpected measure");
                            events.push(Event::PlayheadBang(measure));
                        }
                        a => {
                            arg = a;
                            continue 'modes;
                        }
                    }
                }
            }
            Some(osc::Type::Int(PLAYHEAD_POSITION)) => {
                for i in 0..Measure::TOTAL_VARIANTS as i32 {
                    let measure = Measure::from_i32(i).expect("unexpected measure");
                    let pos = match iter.next() {
                        Some(osc::Type::Float(pos)) => pos,
                        a => {
                            arg = a;
                            continue 'modes;
                        }
                    };
                    events.push(Event::PlayheadPosition(measure, pos));
                }
            }
            a => {
                eprintln!("unexpected arg: {:?}", a);
                break;
            },
        }

        arg = iter.next();
    }
    events
}
