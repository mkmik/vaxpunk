//! VMS time: a 64-bit count of 100 ns units since 17-Nov-1858 00:00.
//! Stored as UTC here; VMS itself kept local time.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 1-Jan-1970 in VMS time.
pub const UNIX_EPOCH_VMS: u64 = 0x007c_9567_4beb_4000;

pub fn now() -> u64 {
    from_system(SystemTime::now())
}

pub fn from_system(t: SystemTime) -> u64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => UNIX_EPOCH_VMS + d.as_secs() * 10_000_000 + d.subsec_nanos() as u64 / 100,
        Err(e) => UNIX_EPOCH_VMS.saturating_sub(e.duration().as_nanos() as u64 / 100),
    }
}

pub fn to_system(t: u64) -> SystemTime {
    if t >= UNIX_EPOCH_VMS {
        UNIX_EPOCH + Duration::from_nanos((t - UNIX_EPOCH_VMS).saturating_mul(100))
    } else {
        UNIX_EPOCH - Duration::from_nanos((UNIX_EPOCH_VMS - t).saturating_mul(100))
    }
}

/// "25-SEP-2026 15:30:07.12", as DIRECTORY/FULL prints dates. Zero is
/// "<none specified>".
pub fn format(t: u64) -> String {
    if t == 0 {
        return "<none specified>".into();
    }
    let secs = t / 10_000_000;
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Days since 17-Nov-1858 to a civil date (proleptic Gregorian), after
    // Howard Hinnant's days_from_civil inverse, shifted to his 0000-03-01 era.
    let z = days as i64 + 678_941 - 60;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + (month <= 2) as i64;
    const M: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
    format!(
        "{:2}-{}-{} {:02}:{:02}:{:02}.{:02}",
        day,
        M[(month - 1) as usize],
        year,
        rem / 3600,
        rem / 60 % 60,
        rem % 60,
        t / 100_000 % 100
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(format(0), "<none specified>");
        assert_eq!(format(1), "17-NOV-1858 00:00:00.00");
        assert_eq!(format(UNIX_EPOCH_VMS), " 1-JAN-1970 00:00:00.00");
        // 2000-02-29 12:34:56.78 UTC is Unix 951827696.78.
        assert_eq!(format(UNIX_EPOCH_VMS + 9_518_276_967_800_000), "29-FEB-2000 12:34:56.78");
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_000_000);
        assert_eq!(to_system(from_system(t)), t);
    }
}
