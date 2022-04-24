use chrono::{ DateTime, Utc, FixedOffset, NaiveDateTime };

#[derive(Debug)]
pub struct Identity {
    name: Vec<u8>,
    email: Vec<u8>,
    at: DateTime<Utc>,
    offset: FixedOffset
}

impl Identity {
    pub fn at(&self) -> &DateTime<Utc> {
        &self.at
    }

    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub fn email(&self) -> &[u8] {
        &self.email
    }

    pub fn offset(&self) -> FixedOffset {
        self.offset
    }

    pub fn parse(input: &[u8]) -> Option<Identity> {

        #[derive(Debug)]
        enum Mode {
            FindOffset,
            FindTimestamp(usize),
            FindEmailEnd((usize, usize)),
            FindEmailStart((usize, usize, usize)),
            FindNameEnd((usize, usize, usize, usize)),
            Done((usize, usize, usize, usize, usize))
        }

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

        if let Mode::Done((name_end, email_start, email_end, time_start, time_end)) = mode {
            let name = input[0 .. name_end].to_vec();
            let email = input[email_start .. email_end].to_vec();

            let timestamp_str = std::str::from_utf8(&input[time_start + 1 .. time_end]).ok()?;

            let offset_str = std::str::from_utf8(&input[time_end + 1 ..]).ok()?;
            let offset: i32 = offset_str.parse().ok()?;

            let offset_mins = offset % 100;
            let offset_hours = offset / 100;

            let timestamp: i64 = match timestamp_str.parse() {
                Ok(xs) => xs,
                Err(_) => return None
            };

            let naive = match NaiveDateTime::from_timestamp_opt(timestamp, 0) {
                Some(xs) => xs,
                None => return None
            };

            let dt = DateTime::<Utc>::from_utc(naive, Utc);
            let tzoffset = match FixedOffset::east_opt(offset_mins * 60 + offset_hours * 60 * 60) {
                Some(xs) => xs,
                None => return None
            };

            Some(Identity {
                name,
                email,
                at: dt,
                offset: tzoffset
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Identity;

    #[test]
    fn read_identity() {
        let bytes = "Chris Dickinson <christopher.s.dickinson@gmail.com> 1545286964 -0800".as_bytes();

        let ident = Identity::parse(&bytes);
        assert_eq!(ident.is_some(), true);

        let ident = ident.unwrap();
        assert_eq!(ident.email(), b"christopher.s.dickinson@gmail.com");
    }
}
