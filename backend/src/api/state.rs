use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use serde::Serialize;

use crate::{
    aprs::Status,
    position::{calculate_distance, Position},
    time::get_current_timestamp,
};

use super::routes::aircraft::StatusDto;

const MAX_AGE_DIFF: u64 = 60 * 5; /* 5 minutes */

/// Our shared application state for the API
#[derive(Clone)]
pub struct App {
    /// Reference to all currently stored states
    states: Arc<Mutex<HashMap<String, Status>>>,
    /// Timestamp of last APRS line received
    last_aprs_update: Arc<AtomicU64>,
}

/// DTO for status overview
#[derive(Serialize)]
pub struct Overview {
    /// Number of currently stored states
    pub count: usize,
    /// Timestamp of last status update, if states is not empty
    pub last_status_update: Option<u64>,
    /// Timestamp of last APRS update received
    pub last_aprs_update: Option<u64>,
}

impl App {
    /// Creates a new `App`
    ///
    /// # Examples
    ///
    /// ```
    /// use api::App;
    ///
    /// let app = App::create();
    /// ```
    pub fn create() -> App {
        App {
            states: Arc::new(Mutex::new(HashMap::new())),
            last_aprs_update: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns the states in the `App` that match given filters as dtos.
    ///
    /// # Arguments
    /// * `position` - The position that should be searched for
    /// * `range` - Range around given `position` that should be searched for.
    ///
    /// # Returns
    ///
    /// Returns dtos of the states within `range` around given `position`, sorted in ascending
    /// oder by distance to `position`.
    ///
    /// # Examples
    ///
    /// * test `state::get_filtered_states_checks_age`
    /// * test `state::get_filtered_states_checks_range`
    /// * test `state::get_filtered_states_orders_correctly`
    pub fn get_filtered_status_dtos(&self, position: &Position, range: f32) -> Vec<StatusDto> {
        let mut states = self.states.lock().expect("Mutex was poisoned");

        App::remove_outdated_states(&mut states);

        let mut status_dtos = states
            .values()
            .map(|status| (status, calculate_distance(position, &status.position)))
            .filter(|&(_, distance)| distance <= range)
            .map(|(status, distance)| StatusDto::from(status, distance))
            .collect::<Vec<StatusDto>>();

        status_dtos.sort_unstable_by(|status_dto_1, status_dto_2| {
            status_dto_1
                .distance
                .partial_cmp(&status_dto_2.distance)
                .unwrap()
        });

        status_dtos
    }

    /// Stores / updates a new status in the `App`
    ///
    /// # Arguments
    ///
    /// * `status` - The status to store / update
    ///
    /// # Examples
    ///
    /// * test `state::get_filtered_states_checks_age`
    /// * test `state::get_filtered_states_checks_range`
    pub fn push_status(&self, new_status: Status) {
        let mut states = self.states.lock().expect("Mutex was poisoned");

        App::remove_outdated_states(&mut states);

        states.insert(new_status.aircraft.id.clone(), new_status);
    }

    /// Updates timestamp of latest APRS update in the `App`
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Timestamp of latest APRS update
    ///
    /// # Examples
    ///
    /// * test `state::get_overview_works`
    pub fn push_last_aprs_update_timestamp(&self, timestamp: u64) {
        self.last_aprs_update.store(timestamp, Ordering::Relaxed);
    }

    /// Returns an overview of the currently stored states
    ///
    /// # Examples
    ///
    /// * test `state::get_overview_works`
    pub fn get_overview(&self) -> Overview {
        /* Shortcut: As AtomicU64 seems to be the best choice for a shared timestamp value,
         * we can't use an `Option` directly in the `App`. But as the timestamp is initialized
         * with 0, we can just convert the 0 to `None`. */
        let last_aprs_update = match self.last_aprs_update.load(Ordering::Relaxed) {
            0 => None,
            v => Some(v),
        };

        let mut states = self.states.lock().expect("Mutex was poisoned");
        App::remove_outdated_states(&mut states);

        Overview {
            count: states.len(),
            last_status_update: states.values().map(|s| s.time_stamp).max(),
            last_aprs_update,
        }
    }

    /// Removes outdated states (by max age)
    ///
    /// # Arguments
    ///
    /// * `states` - `MutexGuard` of states map
    fn remove_outdated_states(states: &mut MutexGuard<HashMap<String, Status>>) {
        let current_timestamp = get_current_timestamp();

        let outdated_keys = states
            .values()
            .filter(|e| current_timestamp - e.time_stamp > MAX_AGE_DIFF)
            .map(|e| e.aircraft.id.clone())
            .collect::<Vec<String>>();

        for key in outdated_keys {
            states.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ogn::Aircraft;

    use super::*;

    #[test]
    fn get_filtered_status_dtos_checks_age() {
        let sut = App::create();
        let current_timestamp = get_current_timestamp();
        let outdated_timestamp = current_timestamp - MAX_AGE_DIFF - 1;

        let position = Position {
            longitude: 48.858222,
            latitude: 2.2945,
        };

        sut.push_status(create_status(
            String::from("AB1234"),
            position.clone(),
            current_timestamp,
        ));
        sut.push_status(create_status(
            String::from("CD5678"),
            position.clone(),
            outdated_timestamp,
        ));

        let result = sut.get_filtered_status_dtos(&position, 1.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].aircraft.id, "AB1234");
    }

    #[test]
    fn get_filtered_status_dtos_checks_range() {
        let sut = App::create();
        let current_timestamp = get_current_timestamp();

        let position = Position {
            latitude: 48.858222,
            longitude: 2.2945,
        };

        sut.push_status(create_status(
            String::from("AB1234"),
            position.clone(),
            current_timestamp,
        ));

        sut.push_status(create_status(
            String::from("CD5678"),
            Position {
                /* see position.rs -> 3.16 km */
                latitude: 48.86055,
                longitude: 2.3376,
            },
            current_timestamp,
        ));

        sut.push_status(create_status(
            String::from("EF9012"),
            Position {
                longitude: 48.84,
                latitude: 2.2,
            },
            current_timestamp,
        ));

        let result = sut.get_filtered_status_dtos(&position, 4.0);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|s| s.aircraft.id == "AB1234"));
        assert!(result.iter().any(|s| s.aircraft.id == "CD5678"));
    }

    #[test]
    fn get_filtered_status_dtos_orders_correctly() {
        let sut = App::create();
        let current_timestamp = get_current_timestamp();

        let position = Position {
            latitude: 48.858222,
            longitude: 2.2945,
        };

        sut.push_status(create_status(
            String::from("CD5678"),
            Position {
                latitude: position.latitude + 0.0001,
                longitude: position.longitude + 0.0001,
            },
            current_timestamp,
        ));

        sut.push_status(create_status(
            String::from("AB1234"),
            Position {
                latitude: position.latitude,
                longitude: position.longitude,
            },
            current_timestamp,
        ));

        sut.push_status(create_status(
            String::from("EF9012"),
            Position {
                latitude: position.latitude + 0.0002,
                longitude: position.longitude + 0.0002,
            },
            current_timestamp,
        ));

        let result = sut.get_filtered_status_dtos(&position, 4.0);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].aircraft.id, "AB1234");
        assert_eq!(result[1].aircraft.id, "CD5678");
        assert_eq!(result[2].aircraft.id, "EF9012");
    }

    #[test]
    fn get_overview_works() {
        let sut = App::create();

        let result_empty = sut.get_overview();

        let current_timestamp = get_current_timestamp();

        let position = Position {
            latitude: 48.858222,
            longitude: 2.2945,
        };

        sut.push_status(create_status(
            String::from("AB1234"),
            position.clone(),
            current_timestamp - 50,
        ));

        sut.push_status(create_status(
            String::from("CD5678"),
            position.clone(),
            current_timestamp,
        ));

        sut.push_last_aprs_update_timestamp(current_timestamp);

        let result_filled = sut.get_overview();

        assert_eq!(result_empty.count, 0);
        assert_eq!(result_empty.last_status_update, None);
        assert_eq!(result_empty.last_aprs_update, None);

        assert_eq!(result_filled.count, 2);
        assert_eq!(result_filled.last_status_update, Some(current_timestamp));
        assert_eq!(result_filled.last_aprs_update, Some(current_timestamp));
    }

    fn create_status(aircraft_id: String, position: Position, time_stamp: u64) -> Status {
        Status {
            aircraft: Aircraft {
                id: aircraft_id,
                call_sign: None,
                registration: None,
                model: None,
                visible: true,
            },
            position,
            speed: None,
            vertical_speed: None,
            altitude: None,
            turn_rate: None,
            course: None,
            time_stamp,
        }
    }
}
