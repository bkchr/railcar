use errors::*;
use nix::c_int;
use nix::sys::signal::{SigAction, SigHandler, SaFlags, SigSet, Signal};
use nix::sys::signal::{sigaction, kill, raise};

pub fn pass_signals(child_pid: i32) -> Result<()> {
    unsafe {
        CHILD_PID = child_pid;
        set_handler(SigHandler::Handler(child_handler))?;
    }
    Ok(())
}

// NOTE: signal handlers need to know which child to pass
// a signal to. We store the child's pid in a global variable.
// The child pid is only set once prior to setting up the
// signal handler, so it should be safe to access it from the
// signal handler.
static mut CHILD_PID: i32 = 0;


extern "C" fn child_handler(signo: c_int) {
    unsafe {
        let _ = kill(CHILD_PID, Signal::from_c_int(signo).unwrap());
    }
}

unsafe fn set_handler(handler: SigHandler) -> Result<()> {
    let a = SigAction::new(handler, SaFlags::empty(), SigSet::all());
    sigaction(Signal::SIGTERM, &a).chain_err(
        || "failed to sigaction",
    )?;
    sigaction(Signal::SIGQUIT, &a).chain_err(
        || "failed to sigaction",
    )?;
    sigaction(Signal::SIGINT, &a).chain_err(
        || "failed to sigaction",
    )?;
    sigaction(Signal::SIGHUP, &a).chain_err(
        || "failed to sigaction",
    )?;
    sigaction(Signal::SIGUSR1, &a).chain_err(
        || "failed to sigaction",
    )?;
    sigaction(Signal::SIGUSR2, &a).chain_err(
        || "failed to sigaction",
    )?;
    Ok(())
}

pub fn signal_children(signal: Signal) -> Result<()> {
    // don't signal this thread
    let mut s = SigSet::empty();
    s.add(signal);
    s.thread_block()?;
    kill(0, signal)?;
    Ok(())
}

const SIGNALS: &[(&[&str], Signal)] = &[
    ( &[ "HUP", "SIGHUP" ], Signal::SIGHUP ),
    ( &[ "INT", "SIGINT" ], Signal::SIGINT ),
    ( &[ "QUIT", "SIGQUIT" ], Signal::SIGQUIT ),
    ( &[ "ILL" , "SIGILL" ], Signal::SIGILL ),
    ( &[ "BUS" , "SIGBUS" ], Signal::SIGBUS ),
    ( &[ "ABRT", "IOT", "SIGABRT", "SIGIOT" ], Signal::SIGABRT),
    ( &[ "TRAP", "SIGTRAP" ], Signal::SIGTRAP),
    ( &[ "FPE", "SIGFPE" ], Signal::SIGFPE),
    ( &[ "KILL", "SIGKILL" ], Signal::SIGKILL),
    ( &[ "USR1", "SIGUSR1" ], Signal::SIGUSR1),
    ( &[ "SEGV", "SIGSEGV" ], Signal::SIGSEGV),
    ( &[ "USR2", "SIGUSR2" ], Signal::SIGUSR2),
    ( &[ "PIPE", "SIGPIPE" ], Signal::SIGPIPE),
    ( &[ "ALRM", "SIGALRM" ], Signal::SIGALRM),
    ( &[ "TERM", "SIGTERM" ], Signal::SIGTERM),
    #[cfg(all(any(target_os = "linux", target_os = "android", target_os = "emscripten"), not(target_arch = "mips")))]
    ( &[ "STKFLT", "SIGSTKFLT" ], Signal::SIGSTKFLT),
    ( &[ "CHLD", "SIGCHLD" ], Signal::SIGCHLD),
    ( &[ "CONT", "SIGCONT" ], Signal::SIGCONT),
    ( &[ "STOP", "SIGSTOP" ], Signal::SIGSTOP),
    ( &[ "TSTP", "SIGTSTP" ], Signal::SIGTSTP),
    ( &[ "TTIN", "SIGTTIN" ], Signal::SIGTTIN),
    ( &[ "TTOU", "SIGTTOU" ], Signal::SIGTTOU),
    ( &[ "URG", "SIGURG" ], Signal::SIGURG),
    ( &[ "XCPU", "SIGXCPU" ], Signal::SIGXCPU),
    ( &[ "XFSZ", "SIGXFSZ" ], Signal::SIGXFSZ),
    ( &[ "VTALRM", "SIGVTALRM" ], Signal::SIGVTALRM),
    ( &[ "PROF", "SIGPROF" ], Signal::SIGPROF),
    ( &[ "WINCH", "SIGWINCH" ], Signal::SIGWINCH),
    ( &[ "IO", "SIGIO" ], Signal::SIGIO),
    ( &[ "PWR", "SIGPWR" ], Signal::SIGPWR),
    ( &[ "SYS", "SIGSYS" ], Signal::SIGSYS),
];

pub fn to_signal(signal: &str) -> Result<Signal> {
    let signal_num = if let Ok(num) = signal.parse::<usize>() { num } else { SIGNALS.len() + 1 };

    for (i, &(signals, sig)) in SIGNALS.iter().enumerate() {
        if signals.contains(&signal) || i + 1 == signal_num {
            return Ok(sig);
        }
    }

    bail!("{} is not a valid signal", signal)
}


pub fn signal_process<T: Into<Option<Signal>>>(
    pid: i32,
    signal: T,
) -> Result<()> {
    kill(pid, signal)?;
    Ok(())
}

pub fn raise_for_parent(signal: Signal) -> Result<()> {
    // reset the sigaction for the signal
    if signal != Signal::SIGKILL && signal != Signal::SIGSTOP {
        let a =
            SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::all());
        unsafe {
            sigaction(signal, &a).chain_err(|| "failed to sigaction")?;
        }
    }
    // make sure the signal is unblocked
    let mut s = SigSet::empty();
    s.add(signal);
    s.thread_unblock().chain_err(|| "failed to unblock signal")?;
    // raise the signal
    raise(signal).chain_err(|| {
        format!("failed to raise signal {:?}", signal)
    })?;
    Ok(())
}

pub fn wait_for_signal() -> Result<Signal> {
    let s = SigSet::all();
    s.thread_block()?;
    let result = s.wait()?;
    s.thread_unblock()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_signal() {
        assert_eq!(to_signal("1").unwrap(), Signal::SIGHUP);
        assert_eq!(to_signal("HUP").unwrap(), Signal::SIGHUP);
        assert_eq!(to_signal("SIGHUP").unwrap(), Signal::SIGHUP);

        assert_eq!(to_signal("6").unwrap(), Signal::SIGABRT);
        assert_eq!(to_signal("ABRT").unwrap(), Signal::SIGABRT);
        assert_eq!(to_signal("IOT").unwrap(), Signal::SIGABRT);
        assert_eq!(to_signal("SIGABRT").unwrap(), Signal::SIGABRT);
    }

    #[test]
    fn test_to_signal_fail() {
        assert!(to_signal("34").is_err());     
        assert!(to_signal("SIGTESTP").is_err());     
    }

}
