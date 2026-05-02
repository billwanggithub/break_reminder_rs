use bitflags::bitflags;
use chrono::Datelike;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DayOfWeekMask: u8 {
        const SUN = 1 << 0;
        const MON = 1 << 1;
        const TUE = 1 << 2;
        const WED = 1 << 3;
        const THU = 1 << 4;
        const FRI = 1 << 5;
        const SAT = 1 << 6;

        const WEEKDAYS = Self::MON.bits() | Self::TUE.bits() | Self::WED.bits() | Self::THU.bits() | Self::FRI.bits();
        const ALL = Self::SUN.bits() | Self::MON.bits() | Self::TUE.bits() | Self::WED.bits()
                  | Self::THU.bits() | Self::FRI.bits() | Self::SAT.bits();
    }
}

impl DayOfWeekMask {
    /// True if the mask contains the given calendar weekday.
    pub fn matches(&self, weekday: chrono::Weekday) -> bool {
        let bit = match weekday {
            chrono::Weekday::Sun => Self::SUN,
            chrono::Weekday::Mon => Self::MON,
            chrono::Weekday::Tue => Self::TUE,
            chrono::Weekday::Wed => Self::WED,
            chrono::Weekday::Thu => Self::THU,
            chrono::Weekday::Fri => Self::FRI,
            chrono::Weekday::Sat => Self::SAT,
        };
        self.contains(bit)
    }

    fn to_wpf_string(self) -> String {
        if self.is_empty() {
            return "None".to_string();
        }
        if self == Self::ALL {
            return "All".to_string();
        }
        if self == Self::WEEKDAYS {
            return "Weekdays".to_string();
        }
        // Calendar order: Sun, Mon, Tue, Wed, Thu, Fri, Sat.
        let parts: Vec<&str> = [
            (Self::SUN, "Sun"),
            (Self::MON, "Mon"),
            (Self::TUE, "Tue"),
            (Self::WED, "Wed"),
            (Self::THU, "Thu"),
            (Self::FRI, "Fri"),
            (Self::SAT, "Sat"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| if self.contains(bit) { Some(name) } else { None })
        .collect();
        parts.join(", ")
    }

    fn from_wpf_string(s: &str) -> Self {
        let mut mask = Self::empty();
        // Tolerant: accept comma OR pipe separator (WPF C# enum.ToString uses "|").
        for token in s.split(|c: char| c == ',' || c == '|') {
            let t = token.trim();
            match t {
                "" => {}
                "None" => {} // explicit None: leave mask empty
                "All" => return Self::ALL,
                "Weekdays" => mask |= Self::WEEKDAYS,
                "Sun" => mask |= Self::SUN,
                "Mon" => mask |= Self::MON,
                "Tue" => mask |= Self::TUE,
                "Wed" => mask |= Self::WED,
                "Thu" => mask |= Self::THU,
                "Fri" => mask |= Self::FRI,
                "Sat" => mask |= Self::SAT,
                other => eprintln!("[break_reminder_rs] unknown day token '{other}', skipping"),
            }
        }
        mask
    }
}

impl Serialize for DayOfWeekMask {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_wpf_string())
    }
}

impl<'de> Deserialize<'de> for DayOfWeekMask {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Ok(Self::from_wpf_string(&s))
    }
}

use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone)]
pub struct ScheduledReminder {
    pub enabled: bool,
    pub time: NaiveTime,
    pub days: DayOfWeekMask,
    pub message: String,
    pub last_fired_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize)]
struct ReminderJson {
    #[serde(rename = "Enabled", default)]
    enabled: bool,
    #[serde(rename = "Time", default)]
    time: String,
    #[serde(rename = "Days", default)]
    days: DayOfWeekMask,
    #[serde(rename = "Message", default)]
    message: String,
    #[serde(rename = "LastFiredDate", default, skip_serializing_if = "Option::is_none")]
    last_fired_date: Option<String>,
}

impl Serialize for ScheduledReminder {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let json = ReminderJson {
            enabled: self.enabled,
            time: self.time.format("%H:%M").to_string(),
            days: self.days,
            message: self.message.clone(),
            last_fired_date: self.last_fired_date.map(|d| d.format("%Y-%m-%d").to_string()),
        };
        json.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for ScheduledReminder {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let json = ReminderJson::deserialize(de)?;
        let time = NaiveTime::parse_from_str(&json.time, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let last_fired_date = json
            .last_fired_date
            .as_deref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        Ok(Self {
            enabled: json.enabled,
            time,
            days: json.days,
            message: json.message,
            last_fired_date,
        })
    }
}

pub fn seed_defaults() -> Vec<ScheduledReminder> {
    vec![
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(11, 55, 0).unwrap(),
            days: DayOfWeekMask::WEEKDAYS,
            message: "吃飯囉！".to_string(),
            last_fired_date: None,
        },
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            days: DayOfWeekMask::WEEKDAYS,
            message: "下班時間到！".to_string(),
            last_fired_date: None,
        },
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            days: DayOfWeekMask::ALL,
            message: "該睡覺了！".to_string(),
            last_fired_date: None,
        },
    ]
}
