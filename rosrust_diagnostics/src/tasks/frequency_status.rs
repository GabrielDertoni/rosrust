use crate::{Level, Status, Task};
use rosrust::Time;
use std::collections::VecDeque;
use std::sync::Mutex;

/// The structure for building a frequency status task.
///
/// Use `FrequencyStatus::builder()` to create an instance of this structure.
pub struct FrequencyStatusBuilder<'a> {
    min_frequency: f64,
    max_frequency: f64,
    tolerance: f64,
    window_size: usize,
    name: &'a str,
    allow_no_events: bool,
}

impl<'a> FrequencyStatusBuilder<'a> {
    #[inline]
    fn new() -> Self {
        Self {
            min_frequency: 0.0,
            max_frequency: std::f64::INFINITY,
            tolerance: 0.1,
            window_size: 5,
            name: "Frequency Status",
            allow_no_events: false,
        }
    }

    /// Sets the minimum frequency that is expected.
    ///
    /// Defaults to zero.
    #[inline]
    pub fn min_frequency(&mut self, value: f64) -> &mut Self {
        self.min_frequency = value;
        self
    }

    /// Sets the maximum frequency that is expected.
    ///
    /// Defaults to infinity.
    #[inline]
    pub fn max_frequency(&mut self, value: f64) -> &mut Self {
        self.max_frequency = value;
        self
    }

    /// Sets the tolerance to how far out of bounds a frequency is allowed to go.
    ///
    /// Defaults to `0.1`.
    ///
    /// It is provided as a fraction of the frequency limits.
    ///
    /// So, setting tolerance to `0.2` will accept frequencies 20% smaller than the
    /// minimum frequency, and 20% larger than the maximum frequency.
    #[inline]
    pub fn tolerance(&mut self, value: f64) -> &mut Self {
        self.tolerance = value;
        self
    }

    /// Sets the number of ticks we're averaging while estimating the frequency.
    ///
    /// Defaults to `5` ticks.
    ///
    /// A smaller window size could cause false positives, while a too large window
    /// size could cause false negatives.
    #[inline]
    pub fn window_size(&mut self, value: usize) -> &mut Self {
        self.window_size = value;
        self
    }

    /// Sets the name of the task.
    ///
    /// Defaults to "Frequency Status".
    #[inline]
    pub fn name(&mut self, name: &'a str) -> &mut Self {
        self.name = name;
        self
    }

    /// Sets whether or not a lack of events within the window is an error.
    ///
    /// While frequencies outside of the desired range cause warnings,
    /// getting no events triggered at all causes an error condition.
    ///
    /// Setting this flag allows you to ignore that error condition, and just emmit warnings
    /// when the rates aren't within the frequency window.
    ///
    /// A special case where this is very significant is when the minimum frequency is zero.
    /// Without this flag set, actually having zero events would cause an error condition.
    ///
    /// Defaults to `false`.
    #[inline]
    pub fn allow_no_events(&mut self, allow_no_events: bool) -> &mut Self {
        self.allow_no_events = allow_no_events;
        self
    }

    /// Builds the frequency status with the provided parameters.
    #[inline]
    pub fn build(&self) -> FrequencyStatus {
        FrequencyStatus::new(
            self.min_frequency,
            self.max_frequency,
            self.tolerance,
            self.window_size,
            self.name.into(),
            self.allow_no_events,
        )
    }
}

/// Diagnostic task that monitors the frequency of an event.
///
/// This diagnostic task monitors the frequency of calls to its tick method,
/// and creates corresponding diagnostics. It will report a warning if the
/// frequency is outside acceptable bounds, and report an error if there have
/// been no events in the latest window.
pub struct FrequencyStatus {
    min_frequency: f64,
    max_frequency: f64,
    min_tolerated_frequency: f64,
    max_tolerated_frequency: f64,
    name: String,
    allow_no_events: bool,
    tracker: Mutex<Tracker>,
}

struct Tracker {
    count: usize,
    history: VecDeque<HistoryEntry>,
    window_size: usize,
}

impl Tracker {
    #[inline]
    fn new(window_size: usize) -> Tracker {
        let mut tracker = Tracker {
            count: 0,
            history: VecDeque::with_capacity(window_size),
            window_size,
        };

        tracker.clear();

        tracker
    }

    fn clear(&mut self) {
        self.count = 0;

        self.history.clear();
        let history_entry = HistoryEntry::new(0);

        self.history
            .extend((0..self.window_size).map(|_| history_entry.clone()));
    }
}

impl FrequencyStatus {
    /// Creates a builder for a new frequency status task.
    #[inline]
    pub fn builder<'a>() -> FrequencyStatusBuilder<'a> {
        FrequencyStatusBuilder::new()
    }

    /// Creates a new frequency status based on the provided parameters.
    ///
    /// Look at the `FrequencyStatusBuilder` for more information about the parameters and
    /// reasonable defaults.
    #[inline]
    pub fn new(
        min_frequency: f64,
        max_frequency: f64,
        tolerance: f64,
        window_size: usize,
        name: String,
        allow_no_events: bool,
    ) -> Self {
        Self {
            min_frequency,
            max_frequency,
            min_tolerated_frequency: min_frequency * (1.0 - tolerance),
            max_tolerated_frequency: max_frequency * (1.0 + tolerance),
            name,
            allow_no_events,
            tracker: Mutex::new(Tracker::new(window_size)),
        }
    }

    /// Signals that an event has occurred.
    #[inline]
    pub fn tick(&self) {
        self.tracker.lock().expect(FAILED_TO_LOCK).count += 1;
    }

    /// Resets the statistics.
    ///
    /// This is good to do right before the looped routine that is being observed, to prevent
    /// the delay between task creation and the first call to give a too large initial reading.
    #[inline]
    pub fn clear(&self) {
        self.tracker.lock().expect(FAILED_TO_LOCK).clear();
    }

    fn frequency_to_summary(&self, frequency: f64) -> (Level, &str) {
        match frequency {
            v if v == 0.0 && !self.allow_no_events => (Level::Error, "No events recorded."),
            v if v < self.min_tolerated_frequency => (Level::Warn, "Frequency too low."),
            v if v > self.max_tolerated_frequency => (Level::Warn, "Frequency too high."),
            _ => (Level::Ok, "Desired frequency met"),
        }
    }

    #[allow(clippy::float_cmp)]
    fn add_frequency_info(&self, status: &mut Status) {
        if self.max_frequency == self.min_frequency {
            status.add("Target frequency (Hz)", self.min_frequency)
        }
        if self.min_frequency > 0.0 {
            status.add(
                "Minimum acceptable frequency (Hz)",
                self.min_tolerated_frequency,
            )
        }
        if self.max_frequency != std::f64::INFINITY {
            status.add(
                "Maximum acceptable frequency (Hz)",
                self.max_tolerated_frequency,
            )
        }
    }
}

impl Task for FrequencyStatus {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, status: &mut Status) {
        let mut tracker = match self.tracker.lock() {
            Ok(value) => value,
            Err(_err) => {
                status.set_summary(
                    Level::Error,
                    "Failed to acquire Mutex lock inside frequency check. This can only be caused by a thread unexpectedly crashing inside the node.",
                );
                return;
            }
        };
        let history_end = HistoryEntry::new(tracker.count);

        let end_count = history_end.count;
        let end_time = history_end.time;

        let history_start = match tracker.history.pop_front() {
            Some(value) => value,
            None => {
                status.set_summary(
                    Level::Error,
                    "History in frequency status tracker is unexpectedly missing elements.",
                );
                return;
            }
        };
        tracker.history.push_back(history_end);

        drop(tracker);

        let events = end_count - history_start.count;
        let window = (end_time - history_start.time).seconds();
        let frequency = events as f64 / window;

        let (level, message) = self.frequency_to_summary(frequency);
        status.set_summary(level, message);

        status.add("Events in window", events);
        status.add("Events since startup", end_count);
        status.add("Duration of window (s)", window);
        status.add("Actual frequency (Hz)", frequency);

        self.add_frequency_info(status)
    }
}

#[derive(Clone)]
struct HistoryEntry {
    count: usize,
    time: Time,
}

impl HistoryEntry {
    fn new(count: usize) -> HistoryEntry {
        HistoryEntry {
            count,
            time: rosrust::now(),
        }
    }
}

static FAILED_TO_LOCK: &str = "Failed to acquire lock";
