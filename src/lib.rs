use std::collections::HashMap;
use std::fs::File;
use std::io::{Result, Write};
use std::time::Duration;

use alphanumeric_sort::compare_str;
use rand::prelude::*;
use roundable::{Roundable, Tie, SECOND};
use svg::node::element::path::Data;
use svg::node::element::{Group, Line, Path, Rectangle, Style, Text, Title};

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
    Minutes,
}

trait TimeUnitTrait {
    fn as_time_unit(&self, unit: &TimeUnit) -> u64;
    fn from_time_unit(unit: &TimeUnit, value: u64) -> Self;
}

impl TimeUnitTrait for Duration {
    fn as_time_unit(&self, unit: &TimeUnit) -> u64 {
        match unit {
            TimeUnit::Nanoseconds => self.as_nanos() as u64,
            TimeUnit::Microseconds => self.as_micros() as u64,
            TimeUnit::Milliseconds => self.as_millis() as u64,
            TimeUnit::Seconds => self.as_secs() as u64,
            TimeUnit::Minutes => self.as_secs() as u64 / 60,
        }
    }

    fn from_time_unit(unit: &TimeUnit, value: u64) -> Self {
        match unit {
            TimeUnit::Nanoseconds => Duration::from_nanos(value),
            TimeUnit::Microseconds => Duration::from_micros(value),
            TimeUnit::Milliseconds => Duration::from_millis(value),
            TimeUnit::Seconds => Duration::from_secs(value),
            TimeUnit::Minutes => Duration::from_secs(value * 60),
        }
    }
}

pub struct Timeline {
    start_time: u64,
    end_time: u64,
    events: HashMap<String, Vec<Event>>,
    units: TimeUnit,
    major_unit: TimeUnit,
    row_height: u64,
    width_per_unit: f64,
    row_padding: u64,
    column_padding: u64,
    width_per_major_unit: f64,
    min_elapsed_time: u64,
    time_scale: u64,
    timeline_width: u64,
    colormap: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct Event {
    name: String,
    start_time: u64,
    end_time: u64,
    location: String,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            start_time: u64::MAX,
            end_time: 0,
            events: HashMap::new(),
            units: TimeUnit::Nanoseconds,
            major_unit: TimeUnit::Seconds,
            row_height: 100,
            width_per_unit: 10.0,
            row_padding: 1,
            column_padding: 0,
            width_per_major_unit: 1000.0,
            min_elapsed_time: u64::MAX,
            time_scale: 1,
            timeline_width: 1000,
            colormap: HashMap::new(),
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
        if !self.colormap.contains_key(&event.name) {
            let color = COLORS[rand::thread_rng().gen_range(0..COLORS.len())];
            self.colormap.insert(event.name.clone(), color.to_string());
        }
        match self.events.get_mut(&event.location) {
            Some(events) => events.push(event),
            None => {
                self.events.insert(event.location.clone(), vec![event]);
            }
        }
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

    fn make_busy_line(
        &self,
        processes: &HashMap<String, Duration>,
        max_duration: Duration,
        x: u64,
        mut y: u64,
    ) -> Group {
        let mut g = Group::new();

        // (x,y) is the top part of the line, we want to draw the line from the bottom to the top,
        // so we need to adjust y to be the bottom of the range for the category.
        y += self.row_height;
        for (process, busy_time) in processes {
            let percentage = ((*busy_time * 100).as_nanos() / max_duration.as_nanos()) as u64;
            if percentage == 0 {
                continue;
            }
            let y2 = y - percentage;
            let color = match self.colormap.get(process) {
                Some(color) => color.clone(),
                None => "black".to_string(),
            };
            let mut line = Line::new()
                .set("x1", x as f64 + 0.5)
                .set("y1", y)
                .set("x2", x as f64 + 0.5)
                .set("y2", y2)
                .set("stroke", color);
            line = line.add(Title::new(process.clone()));
            g = g.add(line);
            y = y2;
        }
        g
    }

    fn make_busy_rect(&self, process: &String, x: u64, y: u64, width: u64) -> Group {
        let color = match self.colormap.get(process) {
            Some(color) => color.clone(),
            None => "black".to_string(),
        };
        let mut rect = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", width)
            .set("height", self.row_height)
            .set("fill", color);
        rect = rect.add(Title::new(process.clone()));
        Group::new().add(rect)
    }

    fn make_timeline_box(&self, unit: &TimeUnit, start_x: u64) -> Group {
        let row_height = 20;
        let total_duration = Duration::from_nanos(self.end_time - self.start_time);
        let time_per_major = total_duration / 10;
        let width_per_major = self.timeline_width as f64 / 10.0;
        let width_per_minor = width_per_major / 10.0;
        let mut rounded_duration = total_duration.round_to(Duration::from_millis(100), Tie::Up);
        let big_tick = row_height / 2;
        let small_tick = row_height / 4;

        if total_duration > rounded_duration {
            rounded_duration += Duration::from_millis(100);
        }

        let width = rounded_duration.as_millis() as f64 / 10.0 + start_x as f64;
        println!(
            "The total duration rounded up is {:?}",
            total_duration.round_to(Duration::from_millis(100), Tie::Up)
        );
        println!(
            "Making timeline box, width {}, min_elapsed_time {}",
            width, self.min_elapsed_time
        );
        let mut g = Group::new();
        g = g.add(
            Line::new()
                .set("x1", start_x)
                .set("y1", row_height)
                .set("x2", start_x + self.timeline_width)
                .set("y2", row_height)
                .set("stroke", "black"),
        );

        let mut cur_time = Duration::from_nanos(0);
        for i in 0..10 {
            // Big tick for our start
            let x = i as f64 * width_per_major + start_x as f64;
            g = g
                .add(
                    Line::new()
                        .set("x1", x)
                        .set("y1", row_height)
                        .set("x2", x)
                        .set("y2", row_height - big_tick)
                        .set("stroke", "black"),
                )
                .add(
                    Text::new(format!("{}", cur_time.as_time_unit(unit)))
                        .set("x", x)
                        .set("y", row_height - big_tick)
                        .set("fill", "black"),
                );

            // If we're at the end of the timeline, we don't need to draw the small ticks
            if i == 10 {
                break;
            }

            // Small ticks for the middle parts
            for tick in 1..10 {
                let x = tick as f64 * width_per_minor + i as f64 * width_per_major + start_x as f64;
                let line = Line::new()
                    .set("x1", x)
                    .set("y1", row_height)
                    .set("x2", x)
                    .set("y2", row_height - small_tick)
                    .set("stroke", "black");
                g = g.add(line);
            }
            cur_time += time_per_major;
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
        if !categories.contains(category) {
            println!(
                "Category {} not found in categories {:?}",
                category, categories
            );
        }
        let y = categories.iter().position(|c| c == category).unwrap() as u64;
        y * self.row_height + self.row_padding + 20
    }

    /// Write the SVG of the timeline to a writer
    ///
    /// This function writes the SVG of the timeline to a writer. The timeline is drawn with events
    /// on each category. Random colors are used for the events, and the colors are kept consistent
    /// with the same event.
    pub fn write(&self, writer: &mut dyn Write) -> Result<()> {
        println!("Sorting categories events {}", self.events.len());
        let mut categories: Vec<String> = self.events.clone().into_keys().collect();
        categories.sort_by(|a, b| compare_str(a, b));
        println!("eh??");
        let total_time = Duration::from_nanos(self.end_time - self.start_time);
        let height = (categories.len() as u64) * self.row_height + 10;
        let mut unit = TimeUnit::Nanoseconds;

        while total_time.as_time_unit(&unit) > self.timeline_width {
            if unit == TimeUnit::Minutes {
                break;
            }
            unit = match unit {
                TimeUnit::Nanoseconds => TimeUnit::Microseconds,
                TimeUnit::Microseconds => TimeUnit::Milliseconds,
                TimeUnit::Milliseconds => TimeUnit::Seconds,
                TimeUnit::Seconds => TimeUnit::Minutes,
                TimeUnit::Minutes => TimeUnit::Minutes,
            };
        }

        let mut doc = svg::Document::new().set("height", height);

        doc = doc.add(Style::new("text { font-size: 10px; }"));
        let mut max_len: u64 = 0;
        for category in categories.iter() {
            doc = doc.add(
                Text::new(category.clone())
                    .set("x", 0)
                    .set("y", self.category_y(category, &categories) + 10)
                    .set("fill", "black"),
            );
            if category.len() as u64 > max_len {
                max_len = category.len() as u64;
            }
        }

        max_len *= 10;
        let width = total_time.round_to(SECOND, Tie::Up).as_millis() as f64 / 10.0 + max_len as f64;
        doc = doc.set("width", self.timeline_width + max_len);
        doc = doc.add(self.make_timeline_box(&unit, max_len));

        println!("Adding events {}", self.events.len());
        let duration_per_line = (Duration::from_time_unit(&unit, 1) / 100) / 10;
        for category in &categories {
            let events = self.events.get(category).unwrap();
            if events.len() == 0 {
                continue;
            }
            let y = self.category_y(&category, &categories);
            let mut events = events.clone();
            events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
            let mut cur_x = max_len as u64;
            let mut cutoff = Duration::from_nanos(self.start_time) + duration_per_line;
            let mut processes: HashMap<String, Duration> = HashMap::new();
            let mut busy_time = Duration::from_nanos(0);

            for event in events {
                let mut start_time = Duration::from_nanos(event.start_time);
                let end_time = Duration::from_nanos(event.end_time);

                // First skip any time that isn't covered by the event
                while start_time >= cutoff {
                    // We have some entries for the previous time series, so we need to draw them
                    if !processes.is_empty() {
                        doc = doc.add(self.make_busy_line(&processes, duration_per_line, cur_x, y));
                    }
                    processes.clear();
                    cutoff += duration_per_line;
                    cur_x += 1;
                }

                // At this point we know the start_time is less than or equal to the cutoff, see if
                // we can consume the entire event.
                while end_time > cutoff {
                    // If we're offset from the last cutoff then we need to update processes and
                    // make the line for the previous cutoff.
                    if start_time > (cutoff - duration_per_line) {
                        match processes.get_mut(&event.name) {
                            Some(time) => {
                                *time += cutoff - start_time;
                            }
                            None => {
                                processes.insert(event.name.clone(), cutoff - start_time);
                            }
                        };
                        busy_time += cutoff - start_time;
                        doc = doc.add(self.make_busy_line(&processes, duration_per_line, cur_x, y));
                        start_time = cutoff;
                        cur_x += 1;
                        processes.clear();
                        cutoff += duration_per_line;
                        busy_time = Duration::from_nanos(0);
                        continue;
                    }

                    // At this point our start == previous cutoff, and our end_time is higher than
                    // the current cutoff.  We want to see if we span multiple timeseries, so we
                    // can save some instructions and add a rectangle rather than doing individual
                    // lines.
                    let num_lines =
                        ((end_time - start_time).as_nanos() / duration_per_line.as_nanos()) as u64;
                    if num_lines > 1 {
                        doc = doc.add(self.make_busy_rect(&event.name, cur_x, y, num_lines));
                        let duration = duration_per_line * num_lines as u32;
                        start_time += duration;
                        cutoff += duration;
                        cur_x += num_lines;
                        continue;
                    }

                    // Ok we are larger than 1 time series, but smaller than 2, so we can just add
                    // one line, and continue on.
                    processes.insert(event.name.clone(), cutoff - start_time);
                    busy_time += cutoff - start_time;
                    doc = doc.add(self.make_busy_line(&processes, duration_per_line, cur_x, y));
                    start_time = cutoff;
                    processes.clear();
                    cutoff += duration_per_line;
                    cur_x += 1;
                }

                // If we're here we might have consumed the entire event already, so check to see
                // if start_time == end_time, if so we move to the next event, otherwise we know
                // this event only covers a part of the current time series.
                if start_time == end_time {
                    continue;
                }

                match processes.get_mut(&event.name) {
                    Some(time) => {
                        *time += end_time - start_time;
                    }
                    None => {
                        processes.insert(event.name.clone(), end_time - start_time);
                    }
                };
                busy_time += end_time - start_time;
                if busy_time > Duration::from_millis(100) {
                    println!("Busy time is greater than 100ms 3 {:?}", busy_time);
                }
            }

            // We may have a line that needs to be drawn for the last time series
            if !processes.is_empty() {
                doc = doc.add(self.make_busy_line(&processes, duration_per_line, cur_x, y));
            }
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
    fn test_save() {
        let mut timeline = Timeline::default();
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
        timeline.save("timeline.svg").unwrap();
    }

    #[test]
    fn test_offsets() {
        let mut timeline = Timeline::default();
        timeline.set_units(TimeUnit::Seconds);
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
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
