use chrono::{ DateTime, Utc, FixedOffset, NaiveDateTime };
use once_cell::sync::OnceCell;
use std::ops::Deref;

/// Metadata about a human taking an action in the Git database.
/// This includes information about the human's name, email, the UNIX epoch instant they took the
/// action, and the timezone offset at the time they took the action.
///
/// Most commonly, you'll see this information in a commit as the Author
/// or Committer data. See this output from `git log --pretty=raw`:
///
/// ```text
/// commit a8fefd7db9817724b9202fac41cb9f4183229920
/// tree 36a82dd9132a48f3beb8543c9ad174ec391111da
/// parent 52a9830591232fbabe56fba67200e09e53ff560e
/// author Chris Dickinson <chris@neversaw.us> 1650783883 -0700
/// committer Chris Dickinson <chris@neversaw.us> 1650784360 -0700
/// 
///    golf down the commit identity parser
///
/// ```
///
/// The author and committer data are examples of human metadata.
///
/// `HumanMetadata` is lazily parsed into `ParsedHumanMetadata`.
#[derive(Debug)]
pub struct HumanMetadata {
    data: Vec<u8>,
    parsed: OnceCell<ParsedHumanMetadata>
}

#[derive(Debug)]
pub struct ParsedHumanMetadata {
    name_end: usize,
    email_start: usize,
    email_end: usize,
    time_start: usize,
    time_end: usize,

    name: String,
    email: String,
    timezone_offset: Option<FixedOffset>,
    timestamp: Option<DateTime<Utc>>
}

impl ParsedHumanMetadata {
    fn new(input: &[u8]) -> Self {
        let mut mode = Mode::FindOffset;

        // Chris Dickinson <christopher.s.dickinson@gmail.com> 1546491006 -0800
        for (idx, &ch) in input.iter().enumerate().rev() {
            mode = match (ch, mode) {
                (b' ', Mode::FindOffset)                => Mode::FindTimestamp(idx),
                (_,    Mode::FindOffset)                => Mode::FindOffset,

                (b' ', Mode::FindTimestamp(a))          => Mode::FindEmailEnd((idx, a)),
                (_,    Mode::FindTimestamp(a))          => Mode::FindTimestamp(a),

                (b'>', Mode::FindEmailEnd((a, b)))      => Mode::FindEmailStart((idx, a, b)),
                (_,    Mode::FindEmailEnd((a, b)))      => Mode::FindEmailEnd((a, b)),

                (b'<', Mode::FindEmailStart((a, b, c))) => Mode::FindNameEnd((idx + 1, a, b, c)),
                (_,    Mode::FindEmailStart((a, b, c))) => Mode::FindEmailStart((a, b, c)),

                (b' ', Mode::FindNameEnd((a, b, c, d))) => Mode::FindNameEnd((a, b, c, d)),
                (_,    Mode::FindNameEnd((a, b, c, d))) => Mode::Done((idx, a, b, c, d)),

                (_,    mode @ Mode::Done(_)) => mode
            };

            if let Mode::Done(_) = &mode {
                break
            }
        }

        #[derive(Debug)]
        enum Mode {
            FindOffset,
            FindTimestamp(usize),
            FindEmailEnd((usize, usize)),
            FindEmailStart((usize, usize, usize)),
            FindNameEnd((usize, usize, usize, usize)),
            Done((usize, usize, usize, usize, usize))
        }

        if let Mode::Done((name_end, email_start, email_end, time_start, time_end)) = mode {
            let name = String::from_utf8_lossy(&input[0 .. name_end]).to_string();
            let email = String::from_utf8_lossy(&input[email_start .. email_end]).to_string();

            let timestamp_str = String::from_utf8_lossy(&input[time_start + 1 .. time_end]).to_string();

            let offset_str = String::from_utf8_lossy(&input[time_end + 1 ..]).to_string();
            let offset: i32 = offset_str.parse().unwrap_or(0);

            let offset_mins = offset % 100;
            let offset_hours = offset / 100;

            let timestamp = timestamp_str.parse::<i64>()
                .ok()
                .and_then(|ts| NaiveDateTime::from_timestamp_opt(ts, 0))
                .map(|naive| DateTime::<Utc>::from_utc(naive, Utc));

            let timezone_offset = FixedOffset::east_opt(offset_mins * 60 + offset_hours * 60 * 60);

            Self {
                name_end,
                email_start,
                email_end,
                time_start,
                time_end,

                name,
                email,
                timezone_offset,
                timestamp
            }
        } else {
            Self {
                name_end: 0,
                email_start: 0,
                email_end: 0,
                time_start: 0,
                time_end: 0,

                name: String::new(),
                email: String::new(),
                timezone_offset: None,
                timestamp: None
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn timezone_offset(&self) -> Option<&FixedOffset> {
        self.timezone_offset.as_ref()
    }

    pub fn timestamp(&self) -> Option<&DateTime<Utc>> {
        self.timestamp.as_ref()
    }
}

impl Deref for HumanMetadata {
    type Target = ParsedHumanMetadata;

    fn deref(&self) -> &Self::Target {
        self.parsed.get_or_init(|| {
            ParsedHumanMetadata::new(&self.data)
        })
    }
}

impl HumanMetadata {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            parsed: OnceCell::new()
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn raw_at(&self) -> &[u8] {
        let offsets = self.parsed.get_or_init(|| {
            ParsedHumanMetadata::new(&self.data)
        });

        &self.data[offsets.time_start..offsets.time_end]
    }

    pub fn raw_name(&self) -> &[u8] {
        let offsets = self.parsed.get_or_init(|| {
            ParsedHumanMetadata::new(&self.data)
        });

        &self.data[0..offsets.name_end]
    }

    pub fn raw_email(&self) -> &[u8] {
        let offsets = self.parsed.get_or_init(|| {
            ParsedHumanMetadata::new(&self.data)
        });

        &self.data[offsets.email_start..offsets.email_end]
    }

    pub fn raw_offset(&self) -> &[u8] {
        let offsets = self.parsed.get_or_init(|| {
            ParsedHumanMetadata::new(&self.data)
        });

        &self.data[offsets.time_end + 1..]
    }
}

#[cfg(test)]
mod tests {
    use super::HumanMetadata;

    #[test]
    fn read_identity() {
        let bytes = "Chris Dickinson <christopher.s.dickinson@gmail.com> 1545286964 -0800".as_bytes();

        let ident = HumanMetadata::new(bytes.into());

        assert_eq!(ident.raw_email(), b"christopher.s.dickinson@gmail.com");
    }
}
