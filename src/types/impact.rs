use serde::{Deserialize, Serialize};
use std::fmt;

/// Impact level of an economic event on the market.
/// Forex Factory uses Low (yellow), Medium (orange), High (red).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    /// Low impact - typically minor market movement expected
    Low,
    /// Medium impact - moderate market movement possible
    Medium,
    /// High impact - significant market movement likely
    High,
}

impl Impact {
    /// Convert to star rating (1-3 stars)
    pub fn stars(self) -> u8 {
        match self {
            Impact::Low => 1,
            Impact::Medium => 2,
            Impact::High => 3,
        }
    }

    /// Create from star rating (1-3), returns None for invalid values
    pub fn from_stars(stars: u8) -> Option<Self> {
        match stars {
            1 => Some(Impact::Low),
            2 => Some(Impact::Medium),
            3 => Some(Impact::High),
            _ => None,
        }
    }

    /// Parse from Forex Factory impact class names
    /// e.g., "icon--ff-impact-yel" -> Low, "icon--ff-impact-ora" -> Medium, "icon--ff-impact-red" -> High
    pub fn from_ff_class(class: &str) -> Option<Self> {
        if class.contains("yel") || class.contains("yellow") {
            Some(Impact::Low)
        } else if class.contains("ora") || class.contains("orange") {
            Some(Impact::Medium)
        } else if class.contains("red") {
            Some(Impact::High)
        } else {
            None
        }
    }
}

impl fmt::Display for Impact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Impact::Low => write!(f, "Low"),
            Impact::Medium => write!(f, "Medium"),
            Impact::High => write!(f, "High"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stars_conversion() {
        assert_eq!(Impact::Low.stars(), 1);
        assert_eq!(Impact::Medium.stars(), 2);
        assert_eq!(Impact::High.stars(), 3);
    }

    #[test]
    fn test_from_stars() {
        assert_eq!(Impact::from_stars(1), Some(Impact::Low));
        assert_eq!(Impact::from_stars(2), Some(Impact::Medium));
        assert_eq!(Impact::from_stars(3), Some(Impact::High));
        assert_eq!(Impact::from_stars(0), None);
        assert_eq!(Impact::from_stars(4), None);
    }

    #[test]
    fn test_from_ff_class() {
        assert_eq!(
            Impact::from_ff_class("icon--ff-impact-yel"),
            Some(Impact::Low)
        );
        assert_eq!(
            Impact::from_ff_class("icon--ff-impact-ora"),
            Some(Impact::Medium)
        );
        assert_eq!(
            Impact::from_ff_class("icon--ff-impact-red"),
            Some(Impact::High)
        );
        assert_eq!(Impact::from_ff_class("icon--ff-impact-gra"), None);
    }

    #[test]
    fn test_ordering() {
        assert!(Impact::Low < Impact::Medium);
        assert!(Impact::Medium < Impact::High);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Impact::Low), "Low");
        assert_eq!(format!("{}", Impact::Medium), "Medium");
        assert_eq!(format!("{}", Impact::High), "High");
    }
}
