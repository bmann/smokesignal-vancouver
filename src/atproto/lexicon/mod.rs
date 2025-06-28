mod com_atproto_repo;
mod community_lexicon_calendar_event;
mod community_lexicon_calendar_rsvp;
mod community_lexicon_location;
mod events_smokesignal_calendar_event;
mod events_smokesignal_calendar_rsvp;

pub mod com {
    pub mod atproto {
        pub mod repo {
            pub use crate::atproto::lexicon::com_atproto_repo::*;
        }
    }
}

pub mod community {
    pub mod lexicon {
        pub mod calendar {
            pub mod event {
                pub use crate::atproto::lexicon::community_lexicon_calendar_event::*;
            }
            pub mod rsvp {
                pub use crate::atproto::lexicon::community_lexicon_calendar_rsvp::*;
            }
        }
        pub mod location {
            pub use crate::atproto::lexicon::community_lexicon_location::*;
        }
    }
}

// events.smokesignal.calendar.event
pub mod events {
    pub mod smokesignal {
        pub mod calendar {
            pub mod event {
                pub use crate::atproto::lexicon::events_smokesignal_calendar_event::*;
            }
            pub mod rsvp {
                pub use crate::atproto::lexicon::events_smokesignal_calendar_rsvp::*;
            }
        }
    }
}
