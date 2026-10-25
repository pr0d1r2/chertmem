//! SPDX license expressions and the license chert takes from each (dev
//! T10, owner 2026-10-25): MIT whenever the expression offers it,
//! otherwise the most permissive option; every `AND` term stays.

#[cfg(test)]
mod tests;

/// The license taken from `expr`, its ids joined by ` AND `.
///
/// # Errors
/// `expr` is not an SPDX expression.
pub fn choose(expr: &str) -> Result<String, String> {
    let _ = expr;
    Ok(String::new())
}
