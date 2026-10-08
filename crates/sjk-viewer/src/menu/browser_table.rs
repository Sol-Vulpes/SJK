//! The browser table's pointer tokens, shared by the classic and SJK UI
//! browsers.

/// Wheel target under the rows.
pub(crate) const TABLE_TOKEN: u16 = 16;
/// Draggable track beside the rows.
pub(crate) const SCROLLBAR_TOKEN: u16 = 14;
/// Header cells `SERVER`..`MODE` are `HEADER_TOKEN + column`.
pub(crate) const HEADER_TOKEN: u16 = 100;
/// Row `n` is `ROW_TOKEN + n`.
pub(crate) const ROW_TOKEN: u16 = 1_000;
