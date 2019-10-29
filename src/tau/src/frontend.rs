use glib::SyncSender;
use xrl::{Client, Frontend, FrontendBuilder, MeasureWidth, ViewId, XiNotification};

/// Wrapper enum to use one rx/tx pair for all `XiNotifications` and requests
#[derive(Debug)]
pub enum XiEvent {
    Notification(XiNotification),
    MeasureWidth(MeasureWidth),
    NewView(Result<(ViewId, Option<String>), String>),
}

/// Wrapper enum to use one rx/tx pair for all `XiRequest` results we send back to Xi.
#[derive(Debug)]
pub enum XiRequest {
    MeasureWidth(Vec<Vec<f32>>),
}

/// Struct that is passed into `xrl::spawn` to start it. See `GxiFrontend` for more
pub struct TauFrontendBuilder {
    pub event_tx: SyncSender<XiEvent>,
    pub request_rx: crossbeam_channel::Receiver<XiRequest>,
    pub request_tx: crossbeam_channel::Sender<XiRequest>,
}

/// This struct is only really there to satisfy the `xrl::Frontend` Trait. It holds `event_tx`,
/// which sends `XiNotifications` and the `request_rx`/`request_tx` pair, which sends
/// `XiRequests`to our main thread where `MainWin` deals with them.
pub struct TauFrontend {
    pub event_tx: SyncSender<XiEvent>,
    pub request_rx: crossbeam_channel::Receiver<XiRequest>,
    pub request_tx: crossbeam_channel::Sender<XiRequest>,
}

impl Frontend for TauFrontend {
    type NotificationResult = Result<(), ()>;
    type MeasureWidthResult = Result<Vec<Vec<f32>>, ()>;

    /// Send `XiNotification`s to the thread GTK is running on
    fn handle_notification(&mut self, ev: XiNotification) -> Self::NotificationResult {
        // Send all `XiNotifications` to the MainWin
        self.event_tx.send(XiEvent::Notification(ev)).unwrap();

        Ok(())
    }

    fn handle_measure_width(&mut self, request: MeasureWidth) -> Self::MeasureWidthResult {
        self.event_tx.send(XiEvent::MeasureWidth(request)).unwrap();

        if let Ok(res) = self.request_rx.recv() {
            match res {
                XiRequest::MeasureWidth(width) => Ok(width),
            }
        } else {
            // xrl doesn't do anything meaningful with errors yet and we can't really fail at this stage.
            Err(())
        }
    }
}

impl FrontendBuilder for TauFrontendBuilder {
    type Frontend = TauFrontend;

    fn build(self, _client: Client) -> Self::Frontend {
        TauFrontend {
            event_tx: self.event_tx,
            request_rx: self.request_rx,
            request_tx: self.request_tx,
        }
    }
}
