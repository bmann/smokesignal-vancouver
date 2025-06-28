use crate::atproto::lexicon::{
    community::lexicon::calendar::event::EventLocation, community::lexicon::location::Address,
};

/// Checks whether location is editable based on the event's locations
pub fn check_location_edit_status(locations: &[EventLocation]) -> LocationEditStatus {
    if locations.is_empty() {
        // Return editable status with a default empty address when no locations exist
        return LocationEditStatus::Editable(Address::Current {
            country: "".to_string(), // Default empty country
            postal_code: None,
            region: None,
            locality: None,
            street: None,
            name: None,
        });
    }

    if locations.len() > 1 {
        return LocationEditStatus::MultipleLocations;
    }

    // We have exactly one location
    match &locations[0] {
        EventLocation::Address(address @ Address::Current { .. }) => {
            LocationEditStatus::Editable(address.clone())
        }
        _ => LocationEditStatus::UnsupportedLocationType,
    }
}

/// Represents the different states of location editability for an event
#[derive(Debug, Clone)]
pub enum LocationEditStatus {
    /// Single address location that can be edited
    Editable(Address),

    /// Multiple locations present, cannot be edited through web interface
    MultipleLocations,

    /// Unsupported location type, cannot be edited through web interface
    UnsupportedLocationType,

    /// No locations present, cannot be edited through web interface
    NoLocations,
}

impl LocationEditStatus {
    /// Returns whether the location is editable
    pub fn is_editable(&self) -> bool {
        matches!(self, Self::Editable(_))
    }

    /// Returns a human-readable reason why location isn't editable
    pub fn edit_reason(&self) -> Option<&'static str> {
        match self {
            Self::Editable(_) => None,
            Self::MultipleLocations => Some("Event has multiple locations"),
            Self::UnsupportedLocationType => Some("Event has an unsupported location type"),
            Self::NoLocations => Some("Event has no locations"),
        }
    }
}
