//! The open port's control lines: DTR and RTS, and breaks.

use dioxus::prelude::*;

use super::{NotificationKind, Restart, SerialContext, SerialResult};

/// How long a break holds the line low: as POSIX `tcsendbreak` at least.
const BREAK_MS: u32 = 250;

impl SerialContext {
    /// Sets DTR and RTS on the open port, each if given.
    pub fn set_signals(&self, dtr: Option<bool>, rts: Option<bool>) {
        let context = *self;
        self.spawn(async move {
            if let Err(error) = context.write_signals(dtr, rts).await {
                context.report(NotificationKind::Error, "Failed to set the signals", &error);
            }
        });
    }

    /// Restarts the board on the open port.
    pub fn restart(&self, restart: Restart) {
        let context = *self;
        self.spawn(async move {
            for &(dtr, rts, wait_ms) in restart.steps() {
                if let Err(error) = context.write_signals(dtr, rts).await {
                    context.report(NotificationKind::Error, "Failed to restart", &error);
                    return;
                }
                if wait_ms > 0 {
                    let wait = context.time.read().after_ms(wait_ms);
                    wait.await;
                }
            }
        });
    }

    /// Sends a break on the open port, if not sending one already.
    pub fn send_break(&self) {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return;
        };
        if *self.breaking.peek() {
            return;
        }
        let mut breaking = self.breaking;
        breaking.set(true);
        let context = *self;
        self.spawn(async move {
            let sent = async {
                port.handle.set_break(true).await?;
                let wait = context.time.read().after_ms(BREAK_MS);
                wait.await;
                port.handle.set_break(false).await
            };
            match sent.await {
                Ok(()) => context.record_event("Break sent"),
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to send a break", &error)
                }
            }
            breaking.set(false);
        });
    }

    async fn write_signals(&self, dtr: Option<bool>, rts: Option<bool>) -> SerialResult<()> {
        let port = (*self.open.peek())
            .and_then(|id| self.port(id))
            .ok_or("No port is open")?;
        port.handle.set_signals(dtr, rts).await?;
        let mut signals = self.signals;
        let mut signals = signals.write();
        signals.dtr = dtr.unwrap_or(signals.dtr);
        signals.rts = rts.unwrap_or(signals.rts);
        Ok(())
    }
}
