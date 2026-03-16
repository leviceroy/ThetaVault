/// Shared UI-state enums & form types used by both main.rs and ui.rs.

// ─────────────────────────────────────────────────────────────────────────────
// Visual row model (trade table with month/year group headers)
// ─────────────────────────────────────────────────────────────────────────────

/// One "row" in the trade table — year header, month header, or a trade.
/// `table_state.selected()` indexes into the visual_rows vec.
#[derive(Clone, Debug)]
pub enum VisualRowKind {
    /// Top-level collapsible year group header
    YearHeader { year: i32 },
    /// Collapsible month group header (nested under YearHeader)
    MonthHeader { year: i32, month: u32 },
    /// A trade; value is the index into `AppState::trades`
    Trade(usize),
    /// Chain View: top-level ticker group header
    TickerHeader { ticker: String, open_count: usize, closed_count: usize, net_pnl: f64 },
    /// Chain View: roll chain summary row (one per root trade)
    ChainHeader { root_id: i32, ticker: String, strategy: String, roll_count: i32, net_credit: f64, chain_pnl: f64, is_open: bool, entry_date: chrono::DateTime<chrono::Utc> },
}

// ─────────────────────────────────────────────────────────────────────────────
// Filter / Sort
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterStatus {
    All,
    Open,
    Closed,
    Expired,
}

impl FilterStatus {
    pub fn next(self) -> Self {
        match self {
            FilterStatus::All     => FilterStatus::Open,
            FilterStatus::Open    => FilterStatus::Closed,
            FilterStatus::Closed  => FilterStatus::Expired,
            FilterStatus::Expired => FilterStatus::All,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FilterStatus::All     => "All",
            FilterStatus::Open    => "Open",
            FilterStatus::Closed  => "Closed",
            FilterStatus::Expired => "Expired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKey {
    Date,
    Ticker,
    Pnl,
    Roc,
    Dte,
    Credit,
    PctMax,
}

impl SortKey {
    pub fn next(self) -> Self {
        match self {
            SortKey::Date   => SortKey::Ticker,
            SortKey::Ticker => SortKey::Pnl,
            SortKey::Pnl    => SortKey::Roc,
            SortKey::Roc    => SortKey::Dte,
            SortKey::Dte    => SortKey::Credit,
            SortKey::Credit => SortKey::PctMax,
            SortKey::PctMax => SortKey::Date,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Date   => "Date",
            SortKey::Ticker => "Ticker",
            SortKey::Pnl    => "P&L",
            SortKey::Roc    => "ROC%",
            SortKey::Dte    => "DTE",
            SortKey::Credit => "Credit",
            SortKey::PctMax => "%Max",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// App mode
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    FilterInput,    // live-typing ticker search
    EditTrade,
    CloseTrade,
    ConfirmDelete,
    EditPlaybook,
    EditThesis,     // in-place editing of the selected playbook's thesis text
    AdminSettings,  // editing admin/risk management settings
    AnalyzeTrade,   // payoff-at-expiration chart (activated with 'a')
    DatePicker,     // calendar overlay over EditTrade / CloseTrade
}

// ─────────────────────────────────────────────────────────────────────────────
// Edit form field types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    Bool,
    Select(Vec<String>),  // value is the selected index as a string
    Button(String),       // label string, e.g. "[+ Add Leg]"; activated with Enter
    Multiline,            // long text rendered word-wrapped across multiple lines
    Date,                 // stored as "YYYY-MM-DD"; Enter opens calendar popup
}

#[derive(Debug, Clone)]
pub struct EditField {
    pub label:          String,
    pub value:          String,  // always stored as string, parsed on save
    pub kind:           FieldKind,
    pub section_header: Option<String>,  // if set, a header line is drawn before the field
}

impl EditField {
    pub fn text(label: &str, value: &str) -> Self {
        Self { label: label.to_string(), value: value.to_string(), kind: FieldKind::Text, section_header: None }
    }
    pub fn number(label: &str, value: &str) -> Self {
        Self { label: label.to_string(), value: value.to_string(), kind: FieldKind::Number, section_header: None }
    }
    pub fn bool_field(label: &str, value: bool) -> Self {
        Self {
            label: label.to_string(),
            value: if value { "true" } else { "false" }.to_string(),
            kind: FieldKind::Bool,
            section_header: None,
        }
    }
    pub fn select(label: &str, value: &str, options: Vec<String>) -> Self {
        Self { label: label.to_string(), value: value.to_string(), kind: FieldKind::Select(options), section_header: None }
    }
    pub fn multiline(label: &str, value: &str) -> Self {
        Self { label: label.to_string(), value: value.to_string(), kind: FieldKind::Multiline, section_header: None }
    }
    pub fn date(label: &str, value: &str) -> Self {
        Self { label: label.to_string(), value: value.to_string(), kind: FieldKind::Date, section_header: None }
    }
    pub fn with_section(mut self, header: &str) -> Self {
        self.section_header = Some(header.to_string());
        self
    }
}
