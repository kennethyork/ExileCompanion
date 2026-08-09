use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    AreaEntered,
    LevelUp,
    Death,
    TradeWhisper,
    Chat,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEvent {
    pub occurred_at: DateTime<Local>,
    pub kind: EventKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRequest {
    pub buyer: String,
    pub item: String,
    pub price: String,
    pub league: String,
    pub location: String,
    pub raw_message: String,
}

impl GameEvent {
    pub fn new(kind: EventKind, message: impl Into<String>) -> Self {
        Self {
            occurred_at: Local::now(),
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SessionStats {
    pub areas: u32,
    pub levels: u32,
    pub deaths: u32,
    pub trade_whispers: u32,
}

impl SessionStats {
    pub fn record(&mut self, event: &GameEvent) {
        match event.kind {
            EventKind::AreaEntered => self.areas += 1,
            EventKind::LevelUp => self.levels += 1,
            EventKind::Death => self.deaths += 1,
            EventKind::TradeWhisper => self.trade_whispers += 1,
            EventKind::Chat | EventKind::System => {}
        }
    }
}

pub fn parse_client_line(line: &str) -> Option<GameEvent> {
    let message = line.split_once("] ").map_or(line, |(_, tail)| tail).trim();
    let message = message.strip_prefix(": ").unwrap_or(message);
    let (kind, useful) = if let Some(area) = message.strip_prefix("You have entered ") {
        (EventKind::AreaEntered, area.trim_end_matches('.'))
    } else if message.contains("has died") || message.contains("You have died") {
        (EventKind::Death, message)
    } else if message.contains("is now level") || message.contains("You have reached level") {
        (EventKind::LevelUp, message)
    } else if message.contains("Hi, I would like to buy your")
        || message.contains("Hi, I'd like to buy your")
    {
        (EventKind::TradeWhisper, message)
    } else if message.contains("@From ")
        || message.contains("@To ")
        || message.starts_with('#')
        || message.starts_with('%')
        || message.starts_with('&')
        || message.starts_with('$')
    {
        (EventKind::Chat, message)
    } else if message.contains("Connected to ") || message.contains("Closing game gracefully") {
        (EventKind::System, message)
    } else {
        return None;
    };
    Some(GameEvent::new(kind, useful))
}

pub fn parse_trade_request(message: &str) -> Option<TradeRequest> {
    let buyer = message
        .strip_prefix("@From ")?
        .split_once(':')?
        .0
        .trim()
        .to_string();
    let request = message.split("buy your ").nth(1)?;
    let (item, listing) = request.split_once(" listed for ")?;
    let (price, remainder) = listing.split_once(" in ").unwrap_or((listing, ""));
    let (league, details) = remainder
        .split_once(" (stash tab ")
        .unwrap_or((remainder.trim_end_matches('.'), ""));
    let location = if details.is_empty() {
        String::new()
    } else {
        details.trim_end_matches([')', '.']).to_string()
    };
    Some(TradeRequest {
        buyer,
        item: item.trim().to_string(),
        price: price.trim().to_string(),
        league: league.trim().to_string(),
        location,
        raw_message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_area() {
        let event = parse_client_line(
            "2026/08/08 12:00:00 123 [INFO Client 1] You have entered The Coast.",
        )
        .unwrap();
        assert_eq!(event.kind, EventKind::AreaEntered);
        assert_eq!(event.message, "The Coast");
    }

    #[test]
    fn ignores_noise() {
        assert!(parse_client_line("Generating level 3 area").is_none());
    }

    #[test]
    fn parses_real_client_area_format_with_colon() {
        let event = parse_client_line(
            "2026/08/08 23:31:06 37366161 cffb065b [INFO Client 332] : You have entered The Twilight Strand.",
        )
        .unwrap();
        assert_eq!(event.kind, EventKind::AreaEntered);
        assert_eq!(event.message, "The Twilight Strand");
    }

    #[test]
    fn parses_global_chat() {
        let event =
            parse_client_line("2026/08/08 23:31:33 1 abcd [INFO Client 332] #Player: hello")
                .unwrap();
        assert_eq!(event.kind, EventKind::Chat);
    }

    #[test]
    fn parses_trade_request_details() {
        let message = "@From BuyerOne: Hi, I would like to buy your Doom Crown listed for 10 chaos in Mirage (stash tab \"Sell\"; position: left 3, top 4)";
        let trade = parse_trade_request(message).unwrap();
        assert_eq!(trade.buyer, "BuyerOne");
        assert_eq!(trade.item, "Doom Crown");
        assert_eq!(trade.price, "10 chaos");
        assert_eq!(trade.league, "Mirage");
        assert!(trade.location.contains("Sell"));
    }
}
