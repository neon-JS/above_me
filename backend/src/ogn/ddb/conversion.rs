use crate::ogn::Aircraft;

const VALUE_YES: &str = "Y";
const FIELD_SEPARATOR: char = ',';
const IDENTIFIER_COMMENT: char = '#';
const FIELD_ENCLOSURE: char = '\'';
const EMPTY: &str = "";
const TYPE_UNKNOWN: &str = "Unknown";

const INDEX_ID: usize = 1;
const INDEX_TYPE: usize = 2;
const INDEX_REGISTRATION: usize = 3;
const INDEX_CALL_SIGN: usize = 4;
const INDEX_TRACKED: usize = 5;
const INDEX_IDENTIFIED: usize = 6;

/// Tries converting a line of DDB into an `Aircraft` representation
///
/// # Arguments
///
/// * `line` - The line that should be converted
///
/// # Examples
///
/// ```
/// let aircraft = convert("'O','AB1234','ASK-21','D-6507','G1','Y','Y'").unwrap();
/// assert_eq!(aircraft.registration, "D-6507");
/// ```
pub fn convert(line: &str) -> Option<Aircraft> {
    if line.starts_with(IDENTIFIER_COMMENT) {
        return None;
    }

    let line = line.replace(FIELD_ENCLOSURE, EMPTY);

    let fields = line
        .split(FIELD_SEPARATOR)
        .map(str::trim)
        .collect::<Vec<&str>>();

    if fields.len() < 7 {
        return None;
    }

    let model = if fields[INDEX_TYPE] == TYPE_UNKNOWN {
        None
    } else {
        get_as_option(fields[INDEX_TYPE])
    };

    Some(Aircraft {
        id: get_as_option(fields[INDEX_ID])?,
        call_sign: get_as_option(fields[INDEX_CALL_SIGN]),
        registration: get_as_option(fields[INDEX_REGISTRATION]),
        model,
        visible: fields[INDEX_IDENTIFIED] == VALUE_YES && fields[INDEX_TRACKED] == VALUE_YES,
    })
}

/// Returns `Some(String)`, if `value` is not empty.
/// Otherwise returns `None`
///
/// # Arguments
///
/// * `value` - Value that may be wrapped
///
/// # Examples
/// ```
/// assert!(get_as_option("").is_none());
/// assert!(get_as_option("Value").is_some_and(|v|v == "Value"));
/// ```
fn get_as_option(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(String::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_correctly() {
        let line = "'O','AB1234','ASK-21','D-6507','G1','Y','Y'";

        let result = convert(line);
        assert!(result.is_some());

        let aircraft = result.unwrap();
        assert_eq!(aircraft.id, "AB1234");
        assert!(aircraft.call_sign.is_some_and(|v| v == "G1"));
        assert!(aircraft.registration.is_some_and(|v| v == "D-6507"));
        assert!(aircraft.model.is_some_and(|v| v == "ASK-21"));
        assert!(aircraft.visible);
    }

    #[test]
    fn handles_empty_values_correctly() {
        let line = "'O','AB1234','Unknown','','G1','Y','Y'";

        let result = convert(line);
        assert!(result.is_some());

        let aircraft = result.unwrap();
        assert_eq!(aircraft.id, "AB1234");
        assert!(aircraft.call_sign.is_some_and(|v| v == "G1"));
        assert!(aircraft.registration.is_none());
        assert!(aircraft.model.is_none());
        assert!(aircraft.visible);
    }

    #[test]
    fn sets_visible_correctly() {
        assert!(convert("'O','AB1234','','','','Y','Y'").is_some_and(|a| a.visible));
        assert!(convert("'O','AB1234','','','','Y','N'").is_some_and(|a| !a.visible));
        assert!(convert("'O','AB1234','','','','N','Y'").is_some_and(|a| !a.visible));
        assert!(convert("'O','AB1234','','','','N','N'").is_some_and(|a| !a.visible));
    }
}
