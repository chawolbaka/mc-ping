use std::fmt;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct Total {
    pub transmitted: usize,
    pub received: usize,
    pub time: Duration,
    pub rtts: Vec<Duration>,
}

impl Total {
    fn loss_pct(&self) -> f64 {
        if self.transmitted == 0 {
            return 0.0;
        }
        let lost = self.transmitted.saturating_sub(self.received);
        lost as f64 * 100.0 / self.transmitted as f64
    }
}

impl fmt::Display for Total {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let transmitted = self.transmitted;
        let received = self.received;
        let loss_pct = self.loss_pct();

        write!(
            f,
            "{transmitted} packets transmitted, {received} received, {loss_pct:.1}% packet loss, time {}ms",
            self.time.as_millis()
        )?;

        if !self.rtts.is_empty() {
            let to_ms = |d: &Duration| d.as_secs_f64() * 1000.0;
            let min = self
                .rtts
                .iter()
                .map(to_ms)
                .fold(f64::INFINITY, |a, b| a.min(b));
            let max = self
                .rtts
                .iter()
                .map(to_ms)
                .fold(0.0_f64, |a, b| a.max(b));
            let sum_ms: f64 = self.rtts.iter().map(to_ms).sum();
            let avg = sum_ms / self.rtts.len() as f64;

            let mdev = (self
                .rtts
                .iter()
                .map(|d| {
                    let diff = to_ms(d) - avg;
                    diff * diff
                })
                .sum::<f64>()
                / self.rtts.len() as f64)
                .sqrt();

            write!(
                f,
                "\nrtt min/avg/max/mdev = {min:.3}/{avg:.3}/{max:.3}/{mdev:.3}ms"
            )?;
        }

        Ok(())
    }
}
