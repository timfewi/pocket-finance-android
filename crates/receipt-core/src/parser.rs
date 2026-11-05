use crate::{
    invalid, validate_cents, validate_date, AmountCandidate, ReceiptExtraction, Result,
    MAX_OCR_BYTES, MAX_SAFE_CENTS,
};
use chrono::NaiveDate;
use regex::Regex;
use std::{collections::BTreeSet, sync::LazyLock};

static AMOUNT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?\d[\d.,]*").expect("constant amount regex"));
static DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\d{2}\.\d{2}\.\d{4}|\d{4}-\d{2}-\d{2})\b").expect("constant date regex")
});
static FOREIGN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^A-Z])(USD|CHF|GBP|CAD|AUD|JPY|PLN|CZK|HUF|SEK|DKK|NOK)(?:[^A-Z]|$)|([$£¥])")
        .expect("constant currency regex")
});
static EURO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^A-Z])EUR(?:[^A-Z]|$)|€").expect("constant EUR regex"));

/// Convert a printed major-unit amount to EUR cents using checked integer math.
/// Accepts either decimal comma or decimal point, with exactly two fraction digits.
/// Grouped thousands must use the other separator and groups of three digits.
/// A bare integer is a whole-euro amount; a lone 1.234 is rejected as ambiguous.
pub fn parse_eur_amount(value: &str) -> Result<i64> {
    let value = value.trim();
    if value.is_empty() || value.len() > 64 {
        return Err(invalid("Invalid receipt amount"));
    }
    let (negative, digits) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };
    let separator = digits.rfind(['.', ',']);
    let (whole, fraction) = match separator {
        Some(index) => {
            let fraction = &digits[index + 1..];
            if fraction.len() != 2 || !fraction.bytes().all(|c| c.is_ascii_digit()) {
                return Err(invalid("Amount needs exactly two decimal digits; ambiguous separators require manual entry"));
            }
            let whole = &digits[..index];
            let decimal = digits.as_bytes()[index];
            if whole.as_bytes().contains(&decimal) {
                return Err(invalid("Invalid thousands grouping"));
            }
            (whole, fraction)
        }
        None => (digits, "00"),
    };
    let groups: Vec<&str> = whole.split(['.', ',']).collect();
    if groups.is_empty()
        || groups
            .iter()
            .any(|group| group.is_empty() || !group.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err(invalid("Invalid receipt amount"));
    }
    if groups.len() > 1
        && (groups[0].len() > 3 || groups.iter().skip(1).any(|group| group.len() != 3))
    {
        return Err(invalid("Invalid thousands grouping"));
    }
    let mut cents = 0_i64;
    for digit in groups
        .iter()
        .flat_map(|group| group.bytes())
        .chain(fraction.bytes())
    {
        cents = cents
            .checked_mul(10)
            .and_then(|number| number.checked_add(i64::from(digit - b'0')))
            .ok_or_else(|| invalid("Receipt amount exceeds supported integer range"))?;
    }
    if cents > MAX_SAFE_CENTS {
        return Err(invalid("Receipt amount exceeds supported integer range"));
    }
    Ok(if negative { -cents } else { cents })
}
fn words(line: &str) -> Vec<String> {
    line.to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
fn is_excluded_total(line: &str) -> bool {
    let normalized = line.to_lowercase();
    let word_list = words(line);
    word_list.iter().any(|word| {
        matches!(
            word.as_str(),
            "zwischensumme"
                | "teilsumme"
                | "subtotal"
                | "netto"
                | "net"
                | "rabatt"
                | "discount"
                | "savings"
                | "ersparnis"
                | "bar"
                | "cash"
                | "gegeben"
                | "erhalten"
                | "tender"
                | "tendered"
                | "wechselgeld"
                | "rückgeld"
                | "rueckgeld"
                | "retourgeld"
                | "change"
        )
    }) || normalized.contains("sub-total")
        || (word_list
            .iter()
            .any(|word| matches!(word.as_str(), "ust" | "mwst" | "steuer" | "vat" | "tax"))
            && !word_list
                .iter()
                .any(|word| matches!(word.as_str(), "inkl" | "inklusive" | "incl" | "including")))
}
fn nearby_amount_line(line: &str) -> bool {
    if is_excluded_total(line) || is_total(line) {
        return false;
    }
    let without_currency = line.to_uppercase().replace("EUR", "").replace('€', "");
    !without_currency.is_empty()
        && without_currency
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || ".,+-".contains(c))
}
fn is_total(line: &str) -> bool {
    if is_excluded_total(line) {
        return false;
    }
    let normalized = line.to_lowercase();
    words(line).iter().any(|word| {
        matches!(
            word.as_str(),
            "gesamtbetrag"
                | "endbetrag"
                | "zahlbetrag"
                | "rechnungsbetrag"
                | "bruttobetrag"
                | "gesamtsumme"
                | "summe"
                | "gesamt"
                | "total"
                | "betrag"
        )
    }) || normalized.contains("zu zahlen")
}
fn receipt_date(value: &str) -> Option<String> {
    let candidate = if value.contains('.') {
        NaiveDate::parse_from_str(value, "%d.%m.%Y")
            .ok()?
            .format("%Y-%m-%d")
            .to_string()
    } else {
        value.to_owned()
    };
    validate_date(&candidate).ok()?;
    Some(candidate)
}
fn merchant_from_heading(lines: &[&str]) -> Option<String> {
    lines.iter().take(8).find_map(|line| {
        let lower = line.to_lowercase();
        let letters = line.chars().filter(|c| c.is_alphabetic()).count();
        let generic = [
            "rechnung",
            "receipt",
            "kassenbon",
            "beleg",
            "bon",
            "thank you",
            "vielen dank",
        ];
        let address_markers = [
            "straße", "strasse", "gasse", "platz", "tel:", "tel.", "www.", "http", "uid:", "uid ",
        ];
        if letters < 3
            || line.chars().count() > 160
            || generic.iter().any(|word| lower == *word)
            || address_markers.iter().any(|word| lower.contains(word))
            || is_total(line)
            || is_excluded_total(line)
            || DATE.is_match(line)
            || AMOUNT
                .find_iter(line)
                .any(|m| m.as_str().contains(['.', ',']))
        {
            None
        } else {
            Some((*line).to_owned())
        }
    })
}
/// Deterministic, conservative receipt extraction. OCR is untrusted data, never
/// instructions. Totals must have a strong printed label; no line-item addition,
/// inferred tax, or model arithmetic is used. Every result requires user review.
pub fn parse_receipt(ocr_text: &str) -> Result<ReceiptExtraction> {
    if ocr_text.len() > MAX_OCR_BYTES {
        return Err(invalid("Receipt text is too long"));
    }
    let lines: Vec<&str> = ocr_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut warnings = vec![];
    let upper = ocr_text.to_uppercase();
    let foreign = FOREIGN.captures(&upper);
    let has_eur = EURO.is_match(&upper);
    let currency = if let Some(found) = foreign {
        warnings.push(
            "A non-EUR or conflicting currency was detected. Do not convert it automatically."
                .into(),
        );
        match found
            .get(1)
            .or_else(|| found.get(2))
            .expect("currency regex capture")
            .as_str()
        {
            "$" => "USD",
            "£" => "GBP",
            "¥" => "JPY",
            other => other,
        }
        .to_owned()
    } else {
        if !has_eur {
            warnings.push(
                "Currency was not printed clearly. Confirm that the receipt is in EUR.".into(),
            );
        }
        "EUR".to_owned()
    };
    let merchant = merchant_from_heading(&lines);
    if merchant.is_some() {
        warnings
            .push("Merchant was inferred from the receipt heading. Check it before saving.".into());
    } else {
        warnings.push("Merchant is missing. Enter it manually.".into());
    }
    let mut dates = BTreeSet::new();
    let mut invalid_dates = false;
    for found in DATE.find_iter(ocr_text) {
        match receipt_date(found.as_str()) {
            Some(date) => {
                dates.insert(date);
            }
            None => invalid_dates = true,
        }
    }
    let date = if dates.len() == 1 {
        dates.iter().next().cloned()
    } else {
        None
    };
    if invalid_dates {
        warnings.push("A printed date is invalid. Check the date manually.".into());
    }
    if dates.len() > 1 {
        warnings.push("Several dates were found. Select the transaction date manually.".into());
    } else if date.is_none() {
        warnings.push("Date is missing. Enter it manually.".into());
    }
    let mut amount_candidates = vec![];
    let mut invalid_total = false;
    for (line_index, line) in lines.iter().enumerate() {
        if !is_total(line) {
            continue;
        }
        let amount_line = if !AMOUNT.is_match(line) {
            lines
                .get(line_index + 1)
                .filter(|next| nearby_amount_line(next))
                .copied()
                .unwrap_or(line)
        } else {
            line
        };
        let candidate_label = if amount_line == *line {
            (*line).to_owned()
        } else {
            format!("{line} {amount_line}")
        };
        let tokens: Vec<_> = AMOUNT.find_iter(amount_line).map(|m| m.as_str()).collect();
        let decimal_tokens: Vec<_> = tokens
            .iter()
            .copied()
            .filter(|token| token.contains(['.', ',']))
            .collect();
        let selected = if decimal_tokens.is_empty() {
            &tokens
        } else {
            &decimal_tokens
        };
        if selected.is_empty() {
            invalid_total = true;
        }
        for token in selected {
            match parse_eur_amount(token) {
                Ok(cents) if validate_cents(cents).is_ok() => {
                    if amount_candidates.len() >= 20 {
                        return Err(invalid(
                            "Too many total candidates; shorten the OCR text or enter manually",
                        ));
                    }
                    amount_candidates.push(AmountCandidate {
                        id: format!("total-{}-{}", line_index, amount_candidates.len()),
                        label: candidate_label.chars().take(180).collect(),
                        amount_cents: cents,
                        line_index,
                    });
                }
                _ => invalid_total = true,
            }
        }
    }
    let values: BTreeSet<i64> = amount_candidates
        .iter()
        .map(|candidate| candidate.amount_cents)
        .collect();
    let total_cents = if values.len() == 1 && !invalid_total && currency == "EUR" {
        values.iter().next().copied()
    } else {
        None
    };
    if invalid_total {
        warnings.push("A labelled total could not be read reliably. Select a clear candidate or enter it manually.".into());
    }
    if values.len() > 1 {
        warnings.push(
            "Several different totals were found. Choose the correct total or enter it manually."
                .into(),
        );
    }
    if amount_candidates.is_empty() {
        warnings.push("Receipt total is missing. Enter the amount manually; cash given and change are not the total.".into());
    }
    Ok(ReceiptExtraction {
        merchant,
        date,
        total_cents,
        currency,
        amount_candidates,
        warnings,
    })
}
