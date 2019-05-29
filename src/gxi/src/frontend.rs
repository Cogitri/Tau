use futures::future;
use glib::SyncSender;
use xrl::{Client, Frontend, FrontendBuilder, ServerResult, XiEvent};

/// Struct that is passed into `xrl::spawn` to
pub struct GxiFrontendBuilder {
    pub event_tx: SyncSender<XiEvent>,
}

/// This struct is only really there to satisfy the `xrl::Frontend` Trait. It holds the `event_tx`
/// `Sender`, which sends `XiEvents` to our main thread where GTK will work on them.
pub struct GxiFrontend {
    pub event_tx: SyncSender<XiEvent>,
}

impl Frontend for GxiFrontend {
    /// Send `XiEvent`s to the thread GTK is running on
    fn handle_event(&mut self, ev: XiEvent) -> ServerResult<()> {
        self.event_tx.send(ev).unwrap();

        Box::new(future::ok(()))
    }
}

impl FrontendBuilder<GxiFrontend> for GxiFrontendBuilder {
    fn build(self, _client: Client) -> GxiFrontend {
        GxiFrontend {
            event_tx: self.event_tx,
        }
    }
}
