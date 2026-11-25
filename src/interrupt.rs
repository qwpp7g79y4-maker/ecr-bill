//! SIGINT (Ctrl-C) ends the run with exit code 130 and one line on standard
//! error, instead of the shell's "killed by signal". The handler runs only
//! async-signal-safe calls (`write`, `_exit`). The report is written to
//! standard output in one step after everything has been read and priced, so
//! a signal before that leaves standard output empty; once the report is being
//! written the signal is ignored, so a report is never cut in half.

#[cfg(unix)]
mod imp {
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
        fn write(fd: i32, buf: *const core::ffi::c_void, count: usize) -> isize;
        fn _exit(status: i32) -> !;
    }
    const SIGINT: i32 = 2;
    const SIG_IGN: usize = 1;
    const MESSAGE: &[u8] = b"ecr-bill: interrupted by SIGINT\n";

    extern "C" fn on_sigint(_signum: i32) {
        // SAFETY: `write` and `_exit` are async-signal-safe and the message is
        // a static byte string.
        unsafe {
            let _ = write(2, MESSAGE.as_ptr().cast(), MESSAGE.len());
            _exit(130);
        }
    }

    pub fn install() {
        // SAFETY: `signal` installs a handler that only calls
        // async-signal-safe functions.
        unsafe {
            signal(SIGINT, on_sigint as extern "C" fn(i32) as usize);
        }
    }

    pub fn ignore() {
        // SAFETY: setting the disposition to "ignore" runs no code.
        unsafe {
            signal(SIGINT, SIG_IGN);
        }
    }
}

#[cfg(not(unix))]
mod imp {
    pub fn install() {}
    pub fn ignore() {}
}

/// Exit with code 130 on SIGINT from now on.
pub fn install() {
    imp::install();
}

/// The report is complete and is being written: a SIGINT no longer cuts the run short.
pub fn ignore() {
    imp::ignore();
}
