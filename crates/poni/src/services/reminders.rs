//! Fetch today's workout reminders only while the Reminders page is open.

use crate::events::{AppEvent, EventSink};
use crate::services::http::{Client, MtaGuard};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

const USER_AGENT: &str = concat!("OyenzaStudio/", env!("CARGO_PKG_VERSION"));
const DEFAULT_URL: &str = "http://localhost:8080/api/v1/reminders";
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Debug)]
pub enum ReminderEvent {
    Loading,
    Loaded(Vec<poni_core::reminders::Reminder>),
    Failed(String),
}

pub struct RemindersService {
    shared: Arc<Shared>,
}

impl RemindersService {
    pub fn start(sink: EventSink) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            cv: Condvar::new(),
        });
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("poni-reminders".into())
            .spawn(move || worker_main(worker, sink))
            .map_err(|e| {
                std::io::Error::other(format!("could not start the Reminders thread: {e}"))
            })?;
        Ok(Self { shared })
    }

    /// The page is visible, or is no longer visible.
    pub fn set_open(&self, open: bool) {
        let mut q = self.shared.lock();
        if q.open != open {
            q.open = open;
            q.generation = q.generation.wrapping_add(1);
            drop(q);
            self.shared.cv.notify_all();
        }
    }

    /// Ask again after the page's Refresh button is clicked.
    pub fn refresh(&self) {
        let mut q = self.shared.lock();
        if q.open {
            q.generation = q.generation.wrapping_add(1);
            drop(q);
            self.shared.cv.notify_all();
        }
    }
}

impl Drop for RemindersService {
    fn drop(&mut self) {
        self.shared.lock().shutdown = true;
        self.shared.cv.notify_all();
    }
}

#[derive(Default)]
struct Queue {
    open: bool,
    generation: u64,
    shutdown: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    cv: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn worker_main(shared: Arc<Shared>, sink: EventSink) {
    let mut handled = 0;
    loop {
        let generation = {
            let mut q = shared.lock();
            loop {
                if q.shutdown {
                    return;
                }
                if q.open && q.generation != handled {
                    handled = q.generation;
                    break handled;
                }
                q = shared.cv.wait(q).unwrap_or_else(|p| p.into_inner());
            }
        };
        sink.send(AppEvent::Reminders(ReminderEvent::Loading));
        let result = fetch();
        let q = shared.lock();
        if q.open && q.generation == generation {
            drop(q);
            sink.send(AppEvent::Reminders(match result {
                Ok(items) => ReminderEvent::Loaded(items),
                Err(error) => ReminderEvent::Failed(error),
            }));
        }
    }
}

fn fetch() -> Result<Vec<poni_core::reminders::Reminder>, String> {
    let _com = MtaGuard::new();
    let url = std::env::var("PONI_REMINDERS_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("reminder URL must start with http:// or https://".into());
    }
    let client = Client::new(USER_AGENT).map_err(|e| e.to_string())?;
    let response = client.get(&url, TIMEOUT).map_err(|e| e.to_string())?;
    if response.status != 200 {
        return Err(format!("server responded with HTTP {}", response.status));
    }
    poni_core::reminders::parse_response(&response.body)
}
