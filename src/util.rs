use std::time::Duration;

pub trait HumanDuration {
    fn human_duration(&self) -> String;
}

impl HumanDuration for Duration {
    fn human_duration(&self) -> String {
        let secs_total = self.as_secs();
        let mins = secs_total / 60;
        let secs = secs_total % 60;

        format!("{} minute(s) {:02} second(s)", mins, secs)
    }
}
