//! A parser for the Prometheus text exposition format.
//!
//! Implements the classic text format (not OpenMetrics):
//! `<metric_name>[{<label_name>="<label_value>",...}] <value> [<timestamp>]`.

use std::collections::BTreeMap;
use std::fmt;

/// A single sample line: name, labels, value and optional millisecond timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSample {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
    pub timestamp_ms: Option<i64>,
}

/// A parse failure; the message is `line {n}: {detail}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses a Prometheus text exposition document into samples.
///
/// Blank lines and `#` comment lines (including `# HELP` and `# TYPE`) are
/// skipped. A trailing `\r` on a line is tolerated, as is surrounding
/// whitespace on sample lines.
///
/// # Errors
///
/// Returns the first error encountered, prefixed with its 1-based line number.
pub fn parse_exposition(input: &str) -> Result<Vec<MetricSample>, ParseError> {
    let mut samples = Vec::new();
    for (index, raw_line) in input.split('\n').enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let sample = parse_sample(line)
            .map_err(|message| ParseError(format!("line {line_number}: {message}")))?;
        samples.push(sample);
    }
    Ok(samples)
}

fn parse_sample(line: &str) -> Result<MetricSample, String> {
    let mut cursor = LineCursor::new(line);
    cursor.skip_ws();

    match cursor.peek() {
        Some(byte) if is_name_start(byte) => {}
        _ => return Err("invalid metric name".to_string()),
    }
    let name = cursor.take_while(is_name_continue).to_string();

    // Whitespace after the name may precede the label set, not the value.
    let mut separated = cursor.skip_ws();
    let mut labels = BTreeMap::new();
    if cursor.peek() == Some(b'{') {
        cursor.bump();
        parse_label_list(&mut cursor, &mut labels)?;
        separated = false;
    }

    if cursor.peek().is_none() {
        return Err("missing value".to_string());
    }
    if !separated && !cursor.skip_ws() {
        return Err("expected whitespace before value".to_string());
    }

    let raw_value = cursor.take_while(is_field_byte);
    let value = raw_value
        .parse::<f64>()
        .map_err(|_| format!("invalid value '{raw_value}'"))?;

    let mut timestamp_ms = None;
    if cursor.skip_ws() && cursor.peek().is_some() {
        let raw_timestamp = cursor.take_while(is_field_byte);
        let timestamp = raw_timestamp
            .parse::<i64>()
            .map_err(|_| format!("invalid timestamp '{raw_timestamp}'"))?;
        cursor.skip_ws();
        if cursor.peek().is_some() {
            return Err("trailing characters after timestamp".to_string());
        }
        timestamp_ms = Some(timestamp);
    }

    Ok(MetricSample {
        name,
        labels,
        value,
        timestamp_ms,
    })
}

fn parse_label_list(
    cursor: &mut LineCursor<'_>,
    labels: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    cursor.skip_ws();
    if cursor.peek() == Some(b'}') {
        cursor.bump();
        return Ok(());
    }

    loop {
        cursor.skip_ws();
        match cursor.peek() {
            Some(byte) if is_label_start(byte) => {}
            _ => return Err("invalid label name".to_string()),
        }
        let label_name = cursor.take_while(is_label_continue).to_string();

        cursor.skip_ws();
        if cursor.peek() != Some(b'=') {
            return Err(format!("expected '=' after label name '{label_name}'"));
        }
        cursor.bump();

        cursor.skip_ws();
        if cursor.peek() != Some(b'"') {
            return Err(format!("expected a quoted value for label '{label_name}'"));
        }
        let label_value = parse_label_value(cursor)?;
        labels.insert(label_name, label_value);

        cursor.skip_ws();
        match cursor.peek() {
            Some(b',') => cursor.bump(),
            Some(b'}') => {
                cursor.bump();
                return Ok(());
            }
            _ => return Err("expected ',' or '}' in label list".to_string()),
        }
    }
}

fn parse_label_value(cursor: &mut LineCursor<'_>) -> Result<String, String> {
    cursor.bump(); // opening quote
    let mut value = String::new();
    loop {
        let Some(character) = cursor.next_char() else {
            return Err("unterminated label value".to_string());
        };
        match character {
            '"' => return Ok(value),
            '\\' => {
                let Some(escaped) = cursor.next_char() else {
                    return Err("unterminated label value".to_string());
                };
                match escaped {
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    'n' => value.push('\n'),
                    other => return Err(format!("unknown escape sequence '\\{other}'")),
                }
            }
            other => value.push(other),
        }
    }
}

fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b':'
}

fn is_name_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':'
}

fn is_label_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_label_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_field_byte(byte: u8) -> bool {
    byte != b' ' && byte != b'\t'
}

struct LineCursor<'a> {
    line: &'a str,
    pos: usize,
}

impl<'a> LineCursor<'a> {
    fn new(line: &'a str) -> Self {
        Self { line, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.line[self.pos..]
    }

    fn peek(&self) -> Option<u8> {
        self.line.as_bytes().get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn next_char(&mut self) -> Option<char> {
        let character = self.rest().chars().next()?;
        self.pos += character.len_utf8();
        Some(character)
    }

    /// Skips spaces and tabs, reporting whether any were consumed.
    fn skip_ws(&mut self) -> bool {
        let start = self.pos;
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.bump();
        }
        self.pos > start
    }

    fn take_while(&mut self, predicate: impl Fn(u8) -> bool) -> &'a str {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if !predicate(byte) {
                break;
            }
            self.bump();
        }
        &self.line[start..self.pos]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    fn expect_error(input: &str) -> String {
        parse_exposition(input)
            .expect_err("input should not parse")
            .to_string()
    }

    #[test]
    fn parses_samples_with_and_without_labels() {
        let input = "http_requests_total 1027\ntemperature_celsius{room=\"office\"} 21.5\n";
        let samples = parse_exposition(input).unwrap();
        assert_eq!(
            samples,
            vec![
                MetricSample {
                    name: "http_requests_total".to_string(),
                    labels: BTreeMap::new(),
                    value: 1027.0,
                    timestamp_ms: None,
                },
                MetricSample {
                    name: "temperature_celsius".to_string(),
                    labels: labels(&[("room", "office")]),
                    value: 21.5,
                    timestamp_ms: None,
                },
            ]
        );
    }

    #[test]
    fn preserves_multiple_labels_in_sorted_order() {
        let input = "http_requests_total{method=\"post\",code=\"200\",handler=\"/api/v1\"} 42\n";
        let samples = parse_exposition(input).unwrap();
        let sample = &samples[0];

        assert_eq!(sample.labels.len(), 3);
        assert_eq!(sample.labels.get("code").map(String::as_str), Some("200"));
        assert_eq!(
            sample.labels.get("handler").map(String::as_str),
            Some("/api/v1")
        );
        assert_eq!(
            sample.labels.get("method").map(String::as_str),
            Some("post")
        );

        let keys: Vec<&str> = sample.labels.keys().map(String::as_str).collect();
        assert_eq!(keys, ["code", "handler", "method"]);
        assert_eq!(sample.value, 42.0);
    }

    #[test]
    fn unescapes_label_values() {
        let input = r#"escapes{backslash="a\\b",quote="say \"hi\"",newline="line1\nline2"} 1"#;
        let samples = parse_exposition(input).unwrap();
        let sample = &samples[0];

        assert_eq!(
            sample.labels.get("backslash").map(String::as_str),
            Some("a\\b")
        );
        assert_eq!(
            sample.labels.get("quote").map(String::as_str),
            Some("say \"hi\"")
        );
        assert_eq!(
            sample.labels.get("newline").map(String::as_str),
            Some("line1\nline2")
        );
    }

    #[test]
    fn parses_optional_timestamp() {
        let input = "requests_total 7.5 1700000000123\nrequests_total 1 -5000\nrequests_total 3\n";
        let samples = parse_exposition(input).unwrap();

        assert_eq!(samples[0].timestamp_ms, Some(1_700_000_000_123));
        assert_eq!(samples[1].timestamp_ms, Some(-5000));
        assert_eq!(samples[2].timestamp_ms, None);
    }

    #[test]
    fn parses_special_float_values() {
        let input = "temperature{kind=\"pos\"} +Inf\ntemperature{kind=\"neg\"} -Inf\n\
                     temperature{kind=\"bare\"} Inf\ntemperature{kind=\"nan\"} NaN\n";
        let samples = parse_exposition(input).unwrap();

        assert_eq!(samples.len(), 4);
        assert!(samples[0].value.is_infinite() && samples[0].value.is_sign_positive());
        assert!(samples[1].value.is_infinite() && samples[1].value.is_sign_negative());
        assert!(samples[2].value.is_infinite() && samples[2].value.is_sign_positive());
        assert!(samples[3].value.is_nan());
    }

    #[test]
    fn ignores_comments_help_type_and_crlf() {
        let input = "\
# HELP http_requests_total Total number of HTTP requests.
# TYPE http_requests_total counter
http_requests_total{code=\"200\"} 17\r
# a plain comment
\r
# TYPE http_requests_total untyped
http_requests_total{code=\"500\"} 2\r
";
        let samples = parse_exposition(input).unwrap();

        assert_eq!(samples.len(), 2);
        assert_eq!(
            samples[0].labels.get("code").map(String::as_str),
            Some("200")
        );
        assert_eq!(samples[0].value, 17.0);
        assert_eq!(samples[1].value, 2.0);
    }

    #[test]
    fn rejects_invalid_metric_name() {
        let message = expect_error("valid_metric 1\n9lives 2\n");
        assert!(message.starts_with("line 2: "), "{message}");
    }

    #[test]
    fn rejects_missing_value() {
        let message = expect_error("valid_metric 1\nlonely_metric\n");
        assert!(message.starts_with("line 2: "), "{message}");
    }

    #[test]
    fn rejects_unterminated_label_value() {
        let message = expect_error("valid_metric 1\nbroken{path=\"/tmp} 2\n");
        assert!(message.starts_with("line 2: "), "{message}");
    }

    #[test]
    fn rejects_unknown_escape_sequence() {
        let message = expect_error("valid_metric 1\nbroken{path=\"a\\tb\"} 2\n");
        assert!(message.starts_with("line 2: "), "{message}");
        assert!(message.contains("escape"), "{message}");
    }

    #[test]
    fn rejects_trailing_garbage_after_timestamp() {
        let message = expect_error("valid_metric 1 1700000000000 extra\n");
        assert!(message.starts_with("line 1: "), "{message}");
    }

    const NODE_EXPORTER: &str = "\
# HELP node_cpu_seconds_total Seconds the CPUs spent in each mode.
# TYPE node_cpu_seconds_total counter
node_cpu_seconds_total{cpu=\"0\",mode=\"idle\"} 12345.67
node_cpu_seconds_total{cpu=\"0\",mode=\"user\"} 234.5
node_cpu_seconds_total{cpu=\"1\",mode=\"idle\"} 9876.5
# HELP node_memory_MemTotal_bytes Memory information field MemTotal_bytes.
# TYPE node_memory_MemTotal_bytes gauge
node_memory_MemTotal_bytes 1.6777216e+10
# HELP node_filesystem_avail_bytes Filesystem space available to non-root users in bytes.
# TYPE node_filesystem_avail_bytes gauge
node_filesystem_avail_bytes{device=\"/dev/sda1\",fstype=\"ext4\",mountpoint=\"/\"} 1.5e10
node_filesystem_avail_bytes{device=\"/dev/sdb1\",fstype=\"xfs\",mountpoint=\"/data\"} 2e9
# TYPE node_load1 gauge
node_load1 0.42
# TYPE node_boot_time_seconds gauge
node_boot_time_seconds 1.7e9 1699999999000
";

    #[test]
    fn parses_node_exporter_fixture() {
        let samples = parse_exposition(NODE_EXPORTER).unwrap();
        assert_eq!(samples.len(), 8);

        let names: Vec<&str> = samples.iter().map(|sample| sample.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "node_cpu_seconds_total",
                "node_cpu_seconds_total",
                "node_cpu_seconds_total",
                "node_memory_MemTotal_bytes",
                "node_filesystem_avail_bytes",
                "node_filesystem_avail_bytes",
                "node_load1",
                "node_boot_time_seconds",
            ]
        );

        assert_eq!(samples[0].labels, labels(&[("cpu", "0"), ("mode", "idle")]));
        assert_eq!(samples[0].value, 12345.67);
        assert_eq!(samples[2].labels, labels(&[("cpu", "1"), ("mode", "idle")]));
        assert_eq!(samples[3].value, 16_777_216_000.0);

        let root_filesystem = &samples[4];
        assert_eq!(
            root_filesystem.labels,
            labels(&[
                ("device", "/dev/sda1"),
                ("fstype", "ext4"),
                ("mountpoint", "/"),
            ])
        );
        assert_eq!(root_filesystem.value, 15_000_000_000.0);
        assert_eq!(root_filesystem.timestamp_ms, None);

        assert_eq!(samples[5].value, 2_000_000_000.0);
        assert_eq!(samples[6].value, 0.42);
        assert_eq!(samples[7].value, 1_700_000_000.0);
        assert_eq!(samples[7].timestamp_ms, Some(1_699_999_999_000));
    }
}
