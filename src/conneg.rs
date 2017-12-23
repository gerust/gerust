use std;
use mime;
use http;
use regex::Regex;

use std::cmp::Ordering;

#[derive(Debug, PartialEq)]
enum Error {
    ParseError,
    NotProvided
}

impl From<std::string::ParseError> for Error {
    fn from(_e: std::string::ParseError) -> Error {
        Error::ParseError
    }
}

impl From<mime::FromStrError> for Error {
    fn from(_e: mime::FromStrError) -> Error {
        Error::ParseError
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(_e: std::num::ParseFloatError) -> Error {
        Error::ParseError
    }
}

fn choose_mediatype<'a>(provided: &'a [mime::Mime], header: &http::header::HeaderValue) -> Result<&'a mime::Mime, Error> {
    lazy_static! {
        static ref CONNEG: Regex = Regex::new(r"^\s*([^;]+)(?:;\s*q=(\S*))?\s*$").unwrap();
    }

    let header = header.to_str().unwrap();

    let res = header.split(",")
        .map(|s| CONNEG.captures(s))
        .map(|c| {
            match c {
                Some(c) => {
                    if let Some(mime_string) = c.get(1) {
                        let m: mime::Mime = mime_string.as_str().parse()?;

                        let q = if let Some(q_string) = c.get(2) {
                            q_string.as_str().parse()?
                        } else {
                            1.0
                        };

                        Ok((m, q))
                    } else {
                        Err(Error::ParseError)
                    }
                }
                None => Err(Error::ParseError)
            }

        });

    let mut found = None;
    let mut found_quality = 0.0;
    let mut partial = 0;

    for parsed in res {
        let (mime_type, quality) = parsed?;

        let (provided_mime_type, current_partial) = if let Some((t, partial)) = mime_type_provided(&mime_type, provided) {
            (t, partial)
        } else {
            continue;
        };

        let (next, next_quality) = match found {
            Some(t) if quality > found_quality => {
                partial = current_partial;
                (provided_mime_type, quality)
            },
            Some(t) if quality == found_quality => {
                if partial > current_partial {
                    partial = current_partial;
                    (provided_mime_type, quality)
                } else {
                    continue;
                }
            },
            Some(t) => {
                continue;
            },
            None => {
                (provided_mime_type, quality)
            }
        };
        found = Some(next);
        found_quality = quality;
    }


    if let Some(f) = found {
        Ok(f)
    } else {
        Err(Error::ParseError)
    }
}

fn mime_type_provided<'a>(mime_type: &mime::Mime, provided: &'a [mime::Mime]) -> Option<(&'a mime::Mime, u8)> {
    for provided in provided {
        if mime_type.type_() == provided.type_() {
            if mime_type.subtype() == provided.subtype() {
                return Some((provided, 0))
            }
            if mime_type.subtype() == mime::STAR {
                return Some((provided, 1))
            }
        } else {
            if *mime_type == mime::STAR_STAR {
                return Some((provided, 2))
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use http;
    use mime;
    use super::choose_mediatype;

    #[test]
    fn accept_type_parsing() {
        let header = http::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8");
        let xml: mime::Mime = "application/xml".parse().unwrap();

        let provided = &[mime::TEXT_HTML];
        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&mime::TEXT_HTML));

        let provided = &[xml.clone()];

        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&xml.clone()));

        let provided = &[mime::TEXT_HTML, xml.clone()];

        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&mime::TEXT_HTML));

        let provided = &[mime::TEXT_PLAIN, mime::IMAGE_PNG];

        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&mime::TEXT_PLAIN));
    }

    #[test]
    fn accept_headers_priority_rules() {
        let header = http::header::HeaderValue::from_static("text/html,text/*,*/*");
        let provided = &[mime::TEXT_PLAIN];

        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&mime::TEXT_PLAIN));

        let provided = &[mime::TEXT_PLAIN, mime::APPLICATION_JSON];

        let res = choose_mediatype(provided, &header);

        assert_eq!(res, Ok(&mime::TEXT_PLAIN));

    }
}