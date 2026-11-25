//! The only `unsafe` code of the project.
//!
//! Rust replaces a closed file descriptor 0, 1 or 2 with `/dev/null`
//! (read-write) before `main` runs. After that a really closed stdout cannot be
//! told from `1<>/dev/null`. A constructor in `.init_array` runs earlier than
//! that replacement and records whether fd 1 was already closed, or open for
//! reading only (`1<file`): a write to that fails with EBADF, which Rust also
//! counts as written.

use std::sync::atomic::{AtomicBool, Ordering};

static FD1_WAS_CLOSED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
mod probe {
    use super::{AtomicBool, Ordering, FD1_WAS_CLOSED};

    extern "C" {
        fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    }
    const F_GETFD: i32 = 1;
    const F_GETFL: i32 = 3;
    const O_ACCMODE: i32 = 3;
    const O_RDONLY: i32 = 0;

    extern "C" fn record() {
        // SAFETY: `fcntl` with `F_GETFD` and `F_GETFL` only reads flags and
        // changes nothing.
        let unusable =
            unsafe { fcntl(1, F_GETFD) == -1 || fcntl(1, F_GETFL) & O_ACCMODE == O_RDONLY };
        let flag: &AtomicBool = &FD1_WAS_CLOSED;
        flag.store(unusable, Ordering::Relaxed);
    }

    #[used]
    #[link_section = ".init_array"]
    static INIT: extern "C" fn() = record;
}

/// Was fd 1 closed, or open for reading only, when the process started?
/// Reliable on Linux only.
pub fn stdout_was_closed_at_start() -> bool {
    FD1_WAS_CLOSED.load(Ordering::Relaxed)
}
