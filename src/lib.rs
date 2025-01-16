use std::collections::HashMap;
use std::fs::File;
use std::io::{Result, Write};
use std::time::Duration;

use rand::prelude::*;
use svg::node::element::path::Data;
use svg::node::element::{Group, Line, Path, Rectangle, Text};
use alphanumeric_sort::compare_str;

const COLORS: &'static [&'static str] = &[
    "blue",
    "red",
    "green",
    "purple",
    "orange",
    "yellow",
    "palegreen",
    "pink",
    "cyan",
    "brown",
    "gray",
    "magenta",
    "olive",
    "teal",
    "navy",
    "maroon",
    "lime",
    "aqua",
    "silver",
    "fuchsia",
];

#[derive(Debug, PartialEq, Clone)]
pub enum TimeUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}

pub struct Timeline {
    start_time: u64,
    end_time: u64,
    events: HashMap<String, Vec<Event>>,
    triggers: Vec<Trigger>,
    units: TimeUnit,
    major_unit: TimeUnit,
    row_height: u64,
    width_per_unit: f64,
    row_padding: u64,
    column_padding: u64,
    width_per_major_unit: f64,
    min_elapsed_time: u64,
    time_scale: u64,
}

#[derive(Debug, Clone)]
struct Event {
    name: String,
    start_time: u64,
    end_time: u64,
    location: String,
}

struct Trigger {
    start_location: String,
    end_location: String,
    time: u64,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            start_time: u64::MAX,
            end_time: 0,
            events: HashMap::new(),
            triggers: Vec::new(),
            units: TimeUnit::Nanoseconds,
            major_unit: TimeUnit::Seconds,
            row_height: 100,
            width_per_unit: 10.0,
            row_padding: 1,
            column_padding: 0,
            width_per_major_unit: 1000.0,
            min_elapsed_time: u64::MAX,
            time_scale: 1,
        }
    }
}

impl Timeline {
    /*
    fn calculate_time_scale(&mut self) {
        let mut units = self.units.clone();
        let mut min = self.min_elapsed_time;
        self.time_scale = 1;
        if units == TimeUnit::Nanoseconds {
            if min < 1_000 {
                self.major_unit = TimeUnit::Microseconds;
                return;
            }
            min /= 1_000;
            self.time_scale *= 1_000;
            units = TimeUnit::Microseconds;
        }
        if units == TimeUnit::Microseconds {
            if min < 1_000 {
                self.major_unit = TimeUnit::Milliseconds;
                return;
            }
            min /= 1_000;
            self.time_scale *= 1_000;
            units = TimeUnit::Milliseconds;
        }
        if units == TimeUnit::Milliseconds {
            if min < 1_000 {
                self.major_unit = TimeUnit::Seconds;
                return;
            }
            self.time_scale *= 1_000;
            self.major_unit = TimeUnit::Seconds;
        }
    }
    */

    /// Add an event to the timeline
    ///
    /// This function adds an event to the timeline. Events do not need to be added in
    /// chronological order. `name` will be placed into a rectangle on the timeline, on the row
    /// indicated by `location`. The rectangle will span from `start_time` to `end_time`.
    pub fn add_event(&mut self, name: String, start_time: u64, end_time: u64, location: String) {
        let event = Event {
            name,
            start_time,
            end_time,
            location,
        };
        if event.start_time < self.start_time {
            self.start_time = event.start_time;
        }
        if event.end_time > self.end_time {
            self.end_time = event.end_time;
        }
        let elapsed = event.end_time - event.start_time;
        if elapsed < self.min_elapsed_time {
            self.min_elapsed_time = elapsed;
            //self.calculate_time_scale();
        }
        match self.events.get_mut(&event.location) {
            Some(events) => events.push(event),
            None => {
                self.events.insert(event.location.clone(), vec![event]);
            }
        }
    }

    /// Add a trigger to the timeline
    ///
    /// This function adds a trigger to the timeline. They are independent of the events, but the
    /// common usecase is that triggers exist where events occur. `start_location` and
    /// `end_location` are the locations of the trigger, and `time` is the time at which the
    /// trigger occurs.
    ///
    /// As an example, process A on CPU 0 wakes up process B on CPU 1. The sequence would look
    /// something like this
    ///
    /// ```
    /// # extern crate timeline_svg;
    /// # fn main() {
    /// use timeline_svg::Timeline;
    ///
    /// let mut timeline = Timeline::default();
    /// timeline.set_units(timeline_svg::TimeUnit::Milliseconds);
    /// timeline.add_event("Process A".to_string(), 0, 1, "CPU 0".to_string());
    /// timeline.add_event("Process B".to_string(), 1, 2, "CPU 1".to_string());
    /// timeline.add_trigger("CPU 0".to_string(), "CPU 1".to_string(), 1);
    /// # }
    /// ```
    pub fn add_trigger(&mut self, start_location: String, end_location: String, time: u64) {
        let trigger = Trigger {
            start_location,
            end_location,
            time,
        };
        if trigger.time < self.start_time {
            self.start_time = trigger.time;
        }
        if trigger.time > self.end_time {
            self.end_time = trigger.time;
        }

        //        self.triggers.push(trigger);
    }

    /// Save the timeline to a file
    ///
    /// This function saves the timeline to a file. The timeline is saved as an SVG file. The
    /// `filename` is created or overwritten with the SVG of the timeline.  This can return an
    /// `Result<io::Error>` if there is an issue writing the file.
    pub fn save(&self, filename: &str) -> Result<()> {
        let mut file = File::create(filename)?;
        self.write(&mut file)
    }

    /// Set the units of the timeline
    ///
    /// This function sets the units of the timeline. The default is nanoseconds. The units are
    /// used to properly label the timeline.
    pub fn set_units(&mut self, units: TimeUnit) {
        self.units = units;
    }

    fn time_to_major_unit(&self, time: Duration) -> u64 {
        match self.major_unit {
            TimeUnit::Nanoseconds => time.as_nanos() as u64,
            TimeUnit::Microseconds => time.as_micros() as u64,
            TimeUnit::Milliseconds => time.as_millis() as u64,
            TimeUnit::Seconds => time.as_secs(),
        }
    }

    fn make_timeline_box(&self) -> Group {
        let row_height = 20;
        let total_duration = Duration::from_nanos(self.end_time - self.start_time);
        let width = total_duration.as_millis() as f64 / 100.0;
        let width_per_major = 100.0;
        let big_tick = row_height / 2;
        let small_tick = row_height / 4;

        println!(
            "Making timeline box, width {}, min_elapsed_time {}",
            width, self.min_elapsed_time
        );
        let mut g = Group::new();
        g = g.add(
            Line::new()
                .set("x1", 0)
                .set("y1", row_height)
                .set("x2", width)
                .set("y2", row_height)
                .set("stroke", "black")
                .set("stroke-width", 1),
        );

        let mut cur_time = Duration::from_nanos(0);
        while cur_time < total_duration {
            // Big tick for our start
            let x = cur_time.as_secs() as f64 * width_per_major;
            g = g
                .add(
                    Line::new()
                        .set("x1", x)
                        .set("y1", row_height)
                        .set("x2", x)
                        .set("y2", row_height - big_tick)
                        .set("stroke", "black")
                        .set("stroke-width", 1),
                )
                .add(
                    Text::new(format!("{}", cur_time.as_secs()))
                        .set("x", x)
                        .set("y", row_height - big_tick)
                        .set("font-size", 10)
                        .set("fill", "black"),
                );

            // Small ticks for the middle parts
            for tick in 1..10 {
                let x = cur_time.as_secs() as f64 * width_per_major
                    + (width_per_major / 10.0) * tick as f64;
                let line = Line::new()
                    .set("x1", x)
                    .set("y1", row_height)
                    .set("x2", x)
                    .set("y2", row_height - small_tick)
                    .set("stroke", "black")
                    .set("stroke-width", 1);
                g = g.add(line);
            }
            cur_time += Duration::from_secs(1);
        }
        g
    }

    fn scale_time(&self, time: Duration) -> f64 {
        let start_time = Duration::from_nanos(self.start_time);
        let duration = time - start_time;
        if duration < Duration::from_millis(1) {
            return 0.0;
        }

        duration.as_millis() as f64 * self.width_per_unit
    }

    // Calculate the x position of a time
    fn time_x(&self, time: u64) -> f64 {
        let padding = if time == self.start_time {
            0
        } else {
            self.column_padding
        };
        let time = time - self.start_time;
        ((time as f64 / self.min_elapsed_time as f64) * self.width_per_unit) + padding as f64
    }

    // Calculate the width of a time duration
    fn time_width(&self, start_time: u64, end_time: u64) -> f64 {
        (end_time - start_time) as f64 / self.min_elapsed_time as f64 * self.width_per_unit
    }

    // Calculate the y position of a category
    fn category_y(&self, category: &String, categories: &Vec<String>) -> u64 {
        let y = categories.iter().position(|c| c == category).unwrap() as u64;
        y * self.row_height + self.row_padding + 20
    }

    /// Write the SVG of the timeline to a writer
    ///
    /// This function writes the SVG of the timeline to a writer. The timeline is drawn with events
    /// on each category, with triggers connecting the events. Random colors are used for the
    /// events, and the colors are kept consistent with the same event.
    pub fn write(&self, writer: &mut dyn Write) -> Result<()> {
        println!("Sorting categories events {}", self.events.len());
        let mut colormap: HashMap<String, String> = HashMap::new();
        let mut categories: Vec<String> = self.events.clone().into_keys().collect();
        categories.sort_by(|a, b| compare_str(a, b));
        println!("eh??");
        let width = (self.end_time - self.start_time) as f64 / self.min_elapsed_time as f64
            * self.width_per_unit;
        let height = (categories.len() as u64) * self.row_height + 10;

        let mut doc = svg::Document::new()
            .set("width", width)
            .set("height", height)
            .add(self.make_timeline_box());

        let mut max_len = 0;
        for category in categories.iter() {
            doc = doc.add(
                Text::new(category.clone())
                    .set("x", 0)
                    .set("y", self.category_y(category, &categories) + 10)
                    .set("font-size", 10)
                    .set("fill", "black"),
            );
            if category.len() > max_len {
                max_len = category.len();
            }
        }

        println!("Adding events {}", self.events.len());
        let mut first = true;
        for category in &categories {
            let events = self.events.get(category).unwrap();
            if events.len() == 0 {
                continue;
            }
            println!("Events for category {} {}", category, events.len());
            let mut events = events.clone();
            events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
            let mut busy_time = Duration::new(0, 0);
            let mut cur_x = max_len * 10;
            let mut cutoff = Duration::from_nanos(self.start_time) + Duration::from_millis(10);
            for event in events {
                let mut start_time = Duration::from_nanos(event.start_time);
                let end_time = Duration::from_nanos(event.end_time);
                while start_time >= cutoff {
                    if busy_time > Duration::from_nanos(0) {
                        println!("Busy time is {:?}", busy_time.as_millis());
                        let y1 = self.category_y(&category, &categories) + self.row_height;
                        let y2 = y1 - (busy_time.as_millis() * 10) as u64;
                        let line = Line::new()
                            .set("x1", cur_x)
                            .set("y1", y1)
                            .set("x2", cur_x)
                            .set("y2", y2)
                            .set("stroke", "black")
                            .set("stroke-width", 1);
                        doc = doc.add(line);
                    }
                    busy_time = Duration::from_nanos(0);
                    cutoff += Duration::from_millis(10);
                    cur_x += 1;
                }

                while end_time >= cutoff {
                    if start_time < cutoff {
                        busy_time += cutoff - start_time;
                    }
                    if busy_time > Duration::from_nanos(0) {
                        println!("Busy time is {:?}", busy_time.as_millis());
                        let y1 = self.category_y(&category, &categories) + self.row_height;
                        let y2 = y1 - (busy_time.as_millis() * 10) as u64;
                        let line = Line::new()
                            .set("x1", cur_x)
                            .set("y1", y1)
                            .set("x2", cur_x)
                            .set("y2", y2)
                            .set("stroke", "black")
                            .set("stroke-width", 1);
                        doc = doc.add(line);
                    }
                    start_time = cutoff;
                    busy_time = Duration::from_nanos(0);
                    while start_time >= cutoff {
                        cutoff += Duration::from_millis(10);
                        cur_x += 1;
                    }
                }
                busy_time += end_time - start_time;

                if first {
                    println!("Event {:?}, startime {}", event, self.start_time);
                    first = false;
                }
            }
            println!("cur_x is {}", cur_x);
        }
        println!("Adding triggers {}", self.triggers.len());
        for trigger in &self.triggers {
            let x = self.time_x(trigger.time);
            let start_y = self.category_y(&trigger.start_location, &categories);
            let end_y = self.category_y(&trigger.end_location, &categories);
            let data = Data::new().move_to((x, start_y)).line_to((x, end_y));
            let path = Path::new()
                .set("d", data)
                .set("stroke", "black")
                .set("stroke-width", 1)
                .set("fill", "none");
            doc = doc.add(path);
        }
        println!("Writing to file");

        writer.write_all(doc.to_string().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_event() {
        let mut timeline = Timeline::default();
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        assert_eq!(timeline.start_time, 1);
        assert_eq!(timeline.end_time, 2);
        assert_eq!(timeline.events.len(), 1);

        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
        assert_eq!(timeline.start_time, 1);
        assert_eq!(timeline.end_time, 4);
        assert_eq!(timeline.events.len(), 2);
    }

    #[test]
    fn test_add_trigger() {
        let mut timeline = Timeline::default();
        timeline.add_trigger("Location 1".to_string(), "Location 2".to_string(), 1);
        assert_eq!(timeline.start_time, 1);
        assert_eq!(timeline.end_time, 1);
        assert_eq!(timeline.triggers.len(), 1);
    }

    #[test]
    fn test_save() {
        let mut timeline = Timeline::default();
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
        timeline.add_trigger("Location 1".to_string(), "Location 2".to_string(), 1);
        timeline.save("timeline.svg").unwrap();
    }

    #[test]
    fn test_offsets() {
        let mut timeline = Timeline::default();
        timeline.set_units(TimeUnit::Seconds);
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
        timeline.add_trigger("Location 1".to_string(), "Location 2".to_string(), 1);
        let categories = vec!["Location 1".to_string(), "Location 2".to_string()];

        assert_eq!(timeline.time_x(1), 0.0);
        assert_eq!(timeline.time_x(2), 200.0);
        assert_eq!(timeline.time_x(3), 400.0);
        assert_eq!(timeline.time_x(4), 600.0);
        assert_eq!(
            timeline.category_y(&"Location 1".to_string(), &categories),
            21
        );
        assert_eq!(
            timeline.category_y(&"Location 2".to_string(), &categories),
            41
        );
    }

    #[test]
    fn test_widths() {
        let mut timeline = Timeline::default();
        timeline.set_units(TimeUnit::Milliseconds);
        timeline.add_event("Event 1".to_string(), 0, 100, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 100, 200, "Location 2".to_string());

        assert_eq!(timeline.time_width(0, 100), 20.0);
        assert_eq!(timeline.time_width(100, 200), 20.0);
    }
}
