use serde::Serialize;

use crate::{
    context::Context, font::FontCollection, prelude::FixFnResult, status::CheckFnResult, Registry,
    Status, StatusCode, Testable,
};

pub type CheckId = String;
type CheckOneSignature = dyn Fn(&Testable, &Context) -> CheckFnResult;
type CheckAllSignature = dyn Fn(&FontCollection, &Context) -> CheckFnResult;

#[derive(Clone)]
pub struct CheckFlags {
    pub experimental: bool,
}

impl CheckFlags {
    // We can't use Default trait here because we want to use
    // it in const context.
    pub const fn default() -> Self {
        Self {
            experimental: false,
        }
    }
}

#[derive(Clone)]
pub struct Check<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub rationale: &'a str,
    pub proposal: &'a str,
    pub check_one: Option<&'a CheckOneSignature>,
    pub check_all: Option<&'a CheckAllSignature>,
    pub hotfix: Option<&'a dyn Fn(&Testable) -> FixFnResult>,
    pub fix_source: Option<&'a dyn Fn(&Testable) -> FixFnResult>,
    pub applies_to: &'a str,
    pub flags: CheckFlags,
}

// Are we? Really? I don't know. Let's find out...
unsafe impl Sync for Check<'_> {}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub check_id: CheckId,
    pub check_name: String,
    pub check_rationale: String,
    pub filename: Option<String>,
    pub section: String,
    pub subresults: Vec<Status>,
}

impl CheckResult {
    fn new(check: &Check, filename: Option<&str>, section: &str, subresults: Vec<Status>) -> Self {
        Self {
            check_id: check.id.to_string(),
            check_name: check.title.to_string(),
            check_rationale: check.rationale.to_string(),
            filename: filename.map(|x| x.to_string()),
            section: section.to_string(),
            subresults,
        }
    }

    pub fn worst_status(&self) -> StatusCode {
        self.subresults
            .iter()
            .map(|x| x.severity)
            .max()
            .unwrap_or(StatusCode::Pass)
    }

    pub fn is_error(&self) -> bool {
        self.worst_status() == StatusCode::Error
    }
}

impl<'a> Check<'a> {
    pub fn applies(&self, f: &'a Testable, registry: &Registry) -> bool {
        registry
            .filetypes
            .get(self.applies_to)
            .map_or(false, |ft| ft.applies(f))
    }

    fn status_to_result(
        &'a self,
        subresults: Vec<Status>,
        file: Option<&'a Testable>,
        section: &str,
    ) -> CheckResult {
        CheckResult::new(
            &self,
            file.map(|f| f.filename.as_ref()),
            section,
            subresults,
        )
    }

    pub fn run_one(
        &'a self,
        f: &'a Testable,
        context: &Context,
        section: &str,
    ) -> Option<CheckResult> {
        self.check_one.map(|check_one| {
            let subresults = match check_one(f, context) {
                Ok(results) => results.collect::<Vec<_>>(),
                Err(e) => vec![Status::error(&format!("Error: {}", e))],
            };
            self.status_to_result(subresults, Some(f), section)
        })
    }

    pub fn run_all(
        &'a self,
        f: &'a FontCollection,
        context: &Context,
        section: &str,
    ) -> Option<CheckResult> {
        self.check_all.map(|check_all| {
            let subresults = match check_all(f, context) {
                Ok(results) => results.collect::<Vec<_>>(),
                Err(e) => vec![Status::error(&format!("Error: {}", e))],
            };
            self.status_to_result(subresults, None, section)
        })
    }
}

pub fn return_result(problems: Vec<Status>) -> CheckFnResult {
    if problems.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Box::new(problems.into_iter()))
    }
}
