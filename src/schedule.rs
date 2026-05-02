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

// ScheduledReminder, seed_defaults, and check_due_reminders come in later tasks.

// Suppress dead-code warnings until later tasks consume these.
#[allow(dead_code)]
pub(crate) fn _ensure_compile() {
    let _ = chrono::Local::now().date_naive().weekday();
}
