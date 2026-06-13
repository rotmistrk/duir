//! Bridge: config — `set color.X "fg bg"`, `set kiro.cmd "..."`, etc.
//! Variables set via `set` in Tcl are readable by the app after init.

// Config is read from Tcl variables after init.tcl loads.
// No special bridge needed — the interpreter's variables are the config store.
// The app reads them via interp.get_var("color.chrome.status_bar") etc.
//
// Example init.tcl:
//   set color.chrome.status_bar "white #303030"
//   set kiro.cmd "kiro-cli chat --resume"
//   set lifecycle.archive_after_hours 24
