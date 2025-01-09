use std::fs::File;
use std::io::{Result, Write};
use std::collections::HashMap;

use rand::prelude::*;
use svg::node::element::path::Data;
use svg::node::element::{Group, Line, Path, Rectangle, Text};

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
    "black",
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
    "white",
];

pub enum TimeUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

pub struct Timeline {
    start_time: u64,
    end_time: u64,
    events: Vec<Event>,
    triggers: Vec<Trigger>,
    units: TimeUnit,
    row_height: u64,
    column_width: u64,
    row_padding: u64,
    column_padding: u64,
}

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
            events: Vec::new(),
            triggers: Vec::new(),
            units: TimeUnit::Nanoseconds,
            row_height: 20,
            column_width: 200,
            row_padding: 1,
            column_padding: 0,
        }
    }
}

impl Timeline {
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
        self.events.push(event);
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
        self.triggers.push(trigger);
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

    fn make_timeline_box(&self) -> Group {
        let num_secs = self.end_time - self.start_time;
        let width = num_secs * self.column_width;
        let big_tick = self.row_height / 2;
        let small_tick = self.row_height / 4;

        let mut g = Group::new();
        g = g.add(
            Line::new()
                .set("x1", 0)
                .set("y1", self.row_height)
                .set("x2", width)
                .set("y2", self.row_height)
                .set("stroke", "black")
                .set("stroke-width", 1),
        );

        for i in 0..num_secs {
            // Big tick for our start
            g = g
                .add(
                    Line::new()
                        .set("x1", i * self.column_width)
                        .set("y1", self.row_height)
                        .set("x2", i * self.column_width)
                        .set("y2", self.row_height - big_tick)
                        .set("stroke", "black")
                        .set("stroke-width", 1),
                )
                .add(
                    Text::new(format!("{}", i))
                        .set("x", i * self.column_width)
                        .set("y", self.row_height - big_tick)
                        .set("font-size", 10)
                        .set("fill", "black"),
                );

            // Small ticks for the middle parts
            for tick in 1..9 {
                let x = i * self.column_width + (self.column_width / 10) * tick;
                let line = Line::new()
                    .set("x1", x)
                    .set("y1", self.row_height)
                    .set("x2", x)
                    .set("y2", self.row_height - small_tick)
                    .set("stroke", "black")
                    .set("stroke-width", 1);
                g = g.add(line);
            }
        }
        g
    }

    // Calculate the x position of a time
    fn time_x(&self, time: u64) -> u64 {
        let padding = if time == self.start_time {
            0
        } else {
            self.column_padding
        };
        (time - self.start_time) * self.column_width + padding
    }

    // Calculate the y position of a category
    fn category_y(&self, category: &String, categories: &Vec<String>) -> u64 {
        let y = categories.iter().position(|c| c == category).unwrap() as u64;
        (y + 1) * self.row_height + self.row_padding
    }

    /// Write the SVG of the timeline to a writer
    ///
    /// This function writes the SVG of the timeline to a writer. The timeline is drawn with events
    /// on each category, with triggers connecting the events. Random colors are used for the
    /// events, and the colors are kept consistent with the same event.
    pub fn write(&self, writer: &mut dyn Write) -> Result<()> {
        let mut categories: Vec<String> = self
            .events
            .iter()
            .map(|event| event.location.clone())
            .collect::<Vec<String>>();
        categories.sort();
        let mut colormap: HashMap<String, String> = HashMap::new();

        let num_secs = self.end_time - self.start_time;
        let width = num_secs * self.column_width;
        let height = (categories.len() as u64) * self.row_height + self.row_height;

        let mut doc = svg::Document::new()
            .set("width", width)
            .set("height", height)
            .add(self.make_timeline_box());

        for event in &self.events {
            let color = colormap
                .entry(event.name.clone())
                .or_insert_with(|| {
                    let mut rng = rand::thread_rng();
                    COLORS[rng.gen_range(0..COLORS.len())].to_string()
                });
            let x = self.time_x(event.start_time);
            let y = self.category_y(&event.location, &categories);
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", self.column_width)
                .set("height", self.row_height)
                .set("fill", (*color).clone());
            let label = Text::new(event.name.clone())
                .set("x", x)
                .set("y", y + 10)
                .set("font-size", 10)
                .set("fill", "black");
            let g = Group::new().add(rect).add(label);
            doc = doc.add(g);
        }

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
        timeline.add_event("Event 1".to_string(), 1, 2, "Location 1".to_string());
        timeline.add_event("Event 2".to_string(), 3, 4, "Location 2".to_string());
        timeline.add_trigger("Location 1".to_string(), "Location 2".to_string(), 1);
        let categories = vec!["Location 1".to_string(), "Location 2".to_string()];

        assert_eq!(timeline.time_x(1), 0);
        assert_eq!(timeline.time_x(2), 200);
        assert_eq!(timeline.time_x(3), 400);
        assert_eq!(timeline.time_x(4), 600);
        assert_eq!(
            timeline.category_y(&"Location 1".to_string(), &categories),
            21
        );
        assert_eq!(
            timeline.category_y(&"Location 2".to_string(), &categories),
            41
        );
    }
}
