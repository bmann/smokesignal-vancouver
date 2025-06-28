use crate::{
    atproto::lexicon::{
        community::lexicon::calendar::event::NSID,
        events::smokesignal::calendar::event::NSID as LegacyNSID,
    },
    http::errors::UrlError,
};

pub type QueryParam<'a> = (&'a str, &'a str);
pub type QueryParams<'a> = Vec<QueryParam<'a>>;

pub fn stringify(query: QueryParams) -> String {
    query.iter().fold(String::new(), |acc, &tuple| {
        acc + tuple.0 + "=" + tuple.1 + "&"
    })
}

pub struct URLBuilder {
    host: String,
    path: String,
    params: Vec<(String, String)>,
}

pub fn build_url(host: &str, path: &str, params: Vec<Option<(&str, &str)>>) -> String {
    let mut url_builder = URLBuilder::new(host);
    url_builder.path(path);

    for (key, value) in params.iter().filter_map(|x| *x) {
        url_builder.param(key, value);
    }

    url_builder.build()
}

impl URLBuilder {
    pub fn new(host: &str) -> URLBuilder {
        let host = if host.starts_with("https://") {
            host.to_string()
        } else {
            format!("https://{}", host)
        };

        let host = if let Some(trimmed) = host.strip_suffix('/') {
            trimmed.to_string()
        } else {
            host
        };

        URLBuilder {
            host: host.to_string(),
            params: vec![],
            path: "/".to_string(),
        }
    }

    pub fn param(&mut self, key: &str, value: &str) -> &mut Self {
        self.params
            .push((key.to_owned(), urlencoding::encode(value).to_string()));
        self
    }

    pub fn path(&mut self, path: &str) -> &mut Self {
        path.clone_into(&mut self.path);
        self
    }

    pub fn build(self) -> String {
        let mut url_params = String::new();

        if !self.params.is_empty() {
            url_params.push('?');

            let qs_args = self.params.iter().map(|(k, v)| (&**k, &**v)).collect();
            url_params.push_str(stringify(qs_args).as_str());
        }

        format!("{}{}{}", self.host, self.path, url_params)
    }
}

pub fn url_from_aturi(external_base: &str, aturi: &str) -> Result<String, UrlError> {
    let aturi = aturi.strip_prefix("at://").unwrap_or(aturi);
    let parts = aturi.split("/").collect::<Vec<_>>();
    if parts.len() == 3 && parts[1] == NSID {
        let path = format!("/{}/{}", parts[0], parts[2]);
        return Ok(build_url(external_base, &path, vec![]));
    }
    if parts.len() == 3 && parts[1] == LegacyNSID {
        let path = format!("/{}/{}", parts[0], parts[2]);
        return Ok(build_url(external_base, &path, vec![]));
    }
    Err(UrlError::UnsupportedCollection)
}

fn find_char_bytes_len(ch: &char) -> i32 {
    let mut b = [0; 4];
    ch.encode_utf8(&mut b);
    let mut clen = 0;
    for a in b.iter() {
        clen += match a {
            0 => 0,
            _ => 1,
        }
    }
    clen
}

pub fn truncate_text(text: &str, tlen: usize, suffix: Option<String>) -> String {
    if text.len() <= tlen {
        return text.to_string();
    }

    let c = text.chars().nth(tlen);
    let ret = match c {
        Some(s) => match char::is_whitespace(s) {
            true => text.split_at(tlen).0,
            false => {
                let chars: Vec<_> = text.chars().collect();
                let truncated = chars.split_at(tlen);
                let mut first_len = 0;
                for ch in truncated.0.iter() {
                    first_len += find_char_bytes_len(ch);
                }

                let mut prev_ws = first_len - 1;
                for ch in truncated.0.iter().rev() {
                    if char::is_whitespace(*ch) {
                        break;
                    }
                    prev_ws -= find_char_bytes_len(ch);
                }

                let mut next_ws = first_len + 1;
                for ch in truncated.1.iter() {
                    let mut b = [0; 4];
                    ch.encode_utf8(&mut b);
                    if char::is_whitespace(*ch) {
                        break;
                    }
                    next_ws += find_char_bytes_len(ch);
                }

                match next_ws > prev_ws && prev_ws > 0 {
                    true => text.split_at(prev_ws as usize).0,
                    false => text.split_at(next_ws as usize).1,
                }
            }
        },
        None => text,
    };

    if ret.len() < text.len() {
        if let Some(suffix) = suffix {
            return format!("{} {}", ret, suffix.clone());
        }
    }
    ret.to_string()
}
