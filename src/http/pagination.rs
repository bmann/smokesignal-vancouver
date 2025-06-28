use crate::http::utils::stringify;
use serde::{Deserialize, Serialize};

pub const PAGE_DEFAULT: i64 = 1;
pub const PAGE_MIN: i64 = 1;
pub const PAGE_MAX: i64 = 100;
pub const PAGE_SIZE_DEFAULT: i64 = 10;
pub const PAGE_SIZE_MIN: i64 = 5;
pub const PAGE_SIZE_MAX: i64 = 100;

pub const LIMITED_PAGE_DEFAULT: i64 = 1;
pub const LIMITED_PAGE_MIN: i64 = 1;
pub const LIMITED_PAGE_MAX: i64 = 5;
pub const LIMITED_PAGE_SIZE_DEFAULT: i64 = 5;
pub const LIMITED_PAGE_SIZE_MIN: i64 = 5;
pub const LIMITED_PAGE_SIZE_MAX: i64 = 5;

#[derive(Deserialize, Default)]
pub struct Pagination {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct PaginationView {
    pub previous: Option<i64>,
    pub previous_url: Option<String>,
    pub next: Option<i64>,
    pub next_url: Option<String>,
}

impl Pagination {
    pub fn admin_clamped(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).clamp(1, 25000);
        let page_size = self.page_size.unwrap_or(1).clamp(20, 100);
        (page, page_size)
    }

    pub fn clamped(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(PAGE_DEFAULT).clamp(PAGE_MIN, PAGE_MAX);
        let page_size = self
            .page_size
            .unwrap_or(PAGE_SIZE_DEFAULT)
            .clamp(PAGE_SIZE_MIN, PAGE_SIZE_MAX);
        (page, page_size)
    }
}

impl PaginationView {
    pub fn new(page_size: i64, total: i64, page: i64, params: Vec<(&str, &str)>) -> Self {
        let (previous, previous_url) = {
            if page > 1 {
                let page_value = (page - 1).to_string();
                let mut page_args: Vec<(&str, &str)> = vec![("page", &page_value)];
                page_args.extend(params.clone());
                (Some(page - 1), Some(stringify(page_args)))
            } else {
                (None, None)
            }
        };

        let (next, next_url) = {
            if total > page_size {
                let page_value = (page + 1).to_string();
                let mut page_args: Vec<(&str, &str)> = vec![("page", &page_value)];
                page_args.extend(params);
                (Some(page + 1), Some(stringify(page_args)))
            } else {
                (None, None)
            }
        };

        Self {
            previous,
            previous_url,
            next,
            next_url,
        }
    }
}
