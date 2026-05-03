use super::*;

#[test]
fn completer_empty_shows_all() {
    let mut c = completer::Completer::new(completer::APP_COMMANDS);
    c.update("");
    assert_eq!(c.matches.len(), completer::APP_COMMANDS.len());
}

#[test]
fn completer_prefix_narrows() {
    let mut c = completer::Completer::new(completer::APP_COMMANDS);
    c.update("ex");
    assert!(c.matches.iter().all(|m| m.starts_with("ex")));
    assert!(!c.matches.is_empty());
}

#[test]
fn completer_next_cycles() {
    let mut c = completer::Completer::new(&["alpha", "beta"]);
    c.update("");
    let first = c.next().unwrap();
    assert_eq!(first, "alpha");
    let second = c.next().unwrap();
    assert_eq!(second, "beta");
    // Wraps around
    let third = c.next().unwrap();
    assert_eq!(third, "alpha");
}

#[test]
fn completer_prev_cycles() {
    let mut c = completer::Completer::new(&["alpha", "beta"]);
    c.update("");
    let first = c.prev().unwrap();
    assert_eq!(first, "beta"); // starts from end
    let second = c.prev().unwrap();
    assert_eq!(second, "alpha");
    let third = c.prev().unwrap();
    assert_eq!(third, "beta"); // wraps
}

#[test]
fn completer_reset_selection() {
    let mut c = completer::Completer::new(&["alpha", "beta"]);
    c.update("");
    c.next();
    assert!(c.selected.is_some());
    c.reset_selection();
    assert!(c.selected.is_none());
}

#[test]
fn completer_no_matches_returns_none() {
    let mut c = completer::Completer::new(&["alpha"]);
    c.update("zzz");
    assert!(c.matches.is_empty());
    assert!(c.next().is_none());
    assert!(c.prev().is_none());
}
