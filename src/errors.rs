error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Encoding(::std::str::Utf8Error);
        BadMode(::std::num::ParseIntError);
    }
}
