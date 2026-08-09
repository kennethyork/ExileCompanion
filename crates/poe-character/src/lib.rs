use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemBonuses {
    pub life: i32,
    pub mana: i32,
    pub energy_shield: i32,
    pub armour: i32,
    pub evasion: i32,
    pub fire_resistance: i32,
    pub cold_resistance: i32,
    pub lightning_resistance: i32,
    pub chaos_resistance: i32,
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
}

impl ItemBonuses {
    pub fn add(&mut self, other: &Self) {
        self.life += other.life;
        self.mana += other.mana;
        self.energy_shield += other.energy_shield;
        self.armour += other.armour;
        self.evasion += other.evasion;
        self.fire_resistance += other.fire_resistance;
        self.cold_resistance += other.cold_resistance;
        self.lightning_resistance += other.lightning_resistance;
        self.chaos_resistance += other.chaos_resistance;
        self.strength += other.strength;
        self.dexterity += other.dexterity;
        self.intelligence += other.intelligence;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedItem {
    pub slot: String,
    pub item_class: String,
    pub rarity: String,
    pub name: String,
    pub base_type: String,
    pub bonuses: ItemBonuses,
    pub raw_text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedGem {
    pub group: String,
    pub name: String,
    pub level: u32,
    pub quality: i32,
    pub tags: Vec<String>,
    pub raw_text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFreshness {
    pub identity_at: Option<i64>,
    pub equipment_at: Option<i64>,
    pub gems_at: Option<i64>,
    pub sheet_at: Option<i64>,
    pub passives_at: Option<i64>,
    pub sheet_confidence: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineCharacter {
    #[serde(default)]
    pub profile_id: String,
    pub name: String,
    pub class_name: String,
    pub ascendancy: String,
    pub league: String,
    pub level: u32,
    pub passive_tree_url: String,
    pub sheet_stats: BTreeMap<String, String>,
    pub items: Vec<CapturedItem>,
    #[serde(default)]
    pub gems: Vec<CapturedGem>,
    #[serde(default)]
    pub freshness: CaptureFreshness,
    #[serde(default)]
    pub ollama_review: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetectedCharacterIdentity {
    pub name: Option<String>,
    pub class_name: Option<String>,
    pub ascendancy: Option<String>,
    pub league: Option<String>,
    pub level: Option<u32>,
}

impl DetectedCharacterIdentity {
    pub fn has_values(&self) -> bool {
        self.name.is_some()
            || self.class_name.is_some()
            || self.ascendancy.is_some()
            || self.league.is_some()
            || self.level.is_some()
    }
}

impl OfflineCharacter {
    pub fn equipment_bonuses(&self) -> ItemBonuses {
        let mut total = ItemBonuses::default();
        for item in &self.items {
            total.add(&item.bonuses);
        }
        total
    }

    pub fn summary(&self) -> String {
        let identity = format!(
            "{} | Level {} {} {} | League {}",
            value_or_unknown(&self.name),
            self.level,
            value_or_unknown(&self.class_name),
            value_or_unknown(&self.ascendancy),
            value_or_unknown(&self.league)
        );
        let stats = self
            .sheet_stats
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let items = self
            .items
            .iter()
            .map(|item| format!("{}: {}", item.slot, item.name))
            .collect::<Vec<_>>()
            .join(", ");
        let gems = self
            .gems
            .iter()
            .map(|gem| {
                format!(
                    "{}: {} (level {}, quality {:+}%)",
                    gem.group, gem.name, gem.level, gem.quality
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{identity}\nCharacter sheet: {stats}\nEquipment: {items}\nCaptured gems: {gems}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveTreeInfo {
    pub version: u32,
    pub class_id: u8,
    pub ascendancy_id: u8,
    pub bloodline_id: u8,
    pub allocated_nodes: usize,
    pub extended_nodes: usize,
    pub masteries: usize,
    pub allocated_node_ids: Vec<u16>,
    pub extended_node_ids: Vec<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalBuildAssessment {
    pub captured_life: Option<i32>,
    pub captured_energy_shield: Option<i32>,
    pub raw_life_es_pool: Option<i32>,
    pub resistance_gaps: Vec<String>,
    pub warnings: Vec<String>,
    pub gem_groups: usize,
    pub coverage: Vec<(String, bool)>,
}

pub fn parse_item_text(slot: &str, input: &str) -> Result<CapturedItem> {
    let text = input.trim();
    if text.is_empty() {
        bail!("copy an item in Path of Exile and paste its text here");
    }
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let item_class = prefixed_value(&lines, "Item Class:").unwrap_or_default();
    let rarity_index = lines
        .iter()
        .position(|line| line.starts_with("Rarity:"))
        .context("this does not look like Path of Exile item text (missing Rarity)")?;
    let rarity = lines[rarity_index]
        .trim_start_matches("Rarity:")
        .trim()
        .to_string();
    let identity = lines[rarity_index + 1..]
        .iter()
        .filter(|line| !line.is_empty() && **line != "--------")
        .take(2)
        .copied()
        .collect::<Vec<_>>();
    let (name, base_type) = match rarity.to_ascii_lowercase().as_str() {
        "rare" | "unique" | "relic" => (
            identity
                .first()
                .copied()
                .unwrap_or("Unknown item")
                .to_string(),
            identity.get(1).copied().unwrap_or_default().to_string(),
        ),
        _ => {
            let base = identity
                .first()
                .copied()
                .unwrap_or("Unknown item")
                .to_string();
            (base.clone(), base)
        }
    };

    let mut bonuses = ItemBonuses {
        life: sum_stat(text, r"(?mi)^\+([0-9,]+) to maximum Life(?:\s|$)")?,
        mana: sum_stat(text, r"(?mi)^\+([0-9,]+) to maximum Mana(?:\s|$)")?,
        energy_shield: sum_stat(text, r"(?mi)^\+([0-9,]+) to maximum Energy Shield(?:\s|$)")?
            + property_value(text, "Energy Shield"),
        armour: property_value(text, "Armour"),
        evasion: property_value(text, "Evasion Rating"),
        fire_resistance: sum_stat(text, r"(?mi)^\+?(-?[0-9,]+)% to Fire Resistance")?,
        cold_resistance: sum_stat(text, r"(?mi)^\+?(-?[0-9,]+)% to Cold Resistance")?,
        lightning_resistance: sum_stat(text, r"(?mi)^\+?(-?[0-9,]+)% to Lightning Resistance")?,
        chaos_resistance: sum_stat(text, r"(?mi)^\+?(-?[0-9,]+)% to Chaos Resistance")?,
        strength: sum_stat(text, r"(?mi)^\+([0-9,]+) to Strength(?:\s|$)")?,
        dexterity: sum_stat(text, r"(?mi)^\+([0-9,]+) to Dexterity(?:\s|$)")?,
        intelligence: sum_stat(text, r"(?mi)^\+([0-9,]+) to Intelligence(?:\s|$)")?,
    };
    let all_res = sum_stat(text, r"(?mi)^\+?(-?[0-9,]+)% to all Elemental Resistances")?;
    bonuses.fire_resistance += all_res;
    bonuses.cold_resistance += all_res;
    bonuses.lightning_resistance += all_res;

    Ok(CapturedItem {
        slot: slot.trim().to_string(),
        item_class,
        rarity,
        name,
        base_type,
        bonuses,
        raw_text: text.to_string(),
    })
}

pub fn parse_gem_text(group: &str, input: &str) -> Result<CapturedGem> {
    let text = input.trim();
    if text.is_empty() {
        bail!("copy a skill or support gem in Path of Exile first");
    }
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let rarity_index = lines
        .iter()
        .position(|line| line.starts_with("Rarity:"))
        .context("this does not look like copied Path of Exile gem text")?;
    let name = lines
        .get(rarity_index + 1)
        .filter(|line| !line.is_empty() && **line != "--------")
        .context("gem name is missing")?
        .to_string();
    let level = prefixed_value(&lines, "Level:")
        .and_then(|value| value.split_whitespace().next()?.parse().ok())
        .unwrap_or(1);
    let quality = prefixed_value(&lines, "Quality:")
        .and_then(|value| parse_number(&value))
        .unwrap_or_default();
    let tags = lines
        .iter()
        .position(|line| *line == "--------")
        .and_then(|separator| lines.get(separator + 1))
        .filter(|line| line.contains(','))
        .map_or_else(Vec::new, |line| {
            line.split(',').map(|tag| tag.trim().to_string()).collect()
        });
    Ok(CapturedGem {
        group: group.trim().to_string(),
        name,
        level,
        quality,
        tags,
        raw_text: text.to_string(),
    })
}

pub fn assess_character(character: &OfflineCharacter) -> LocalBuildAssessment {
    let captured_life = sheet_number(character, "Life");
    let captured_energy_shield = sheet_number(character, "Energy Shield");
    let raw_life_es_pool = match (captured_life, captured_energy_shield) {
        (Some(life), Some(es)) => Some(life + es),
        (Some(life), None) => Some(life),
        (None, Some(es)) => Some(es),
        (None, None) => None,
    };
    let mut resistance_gaps = Vec::new();
    for name in ["Fire Resistance", "Cold Resistance", "Lightning Resistance"] {
        match sheet_number(character, name) {
            Some(value) if value < 75 => {
                resistance_gaps.push(format!("{name}: {}% below 75%", 75 - value))
            }
            None => resistance_gaps.push(format!("{name}: not captured")),
            Some(_) => {}
        }
    }
    let mut warnings = Vec::new();
    if character.items.len() < 8 {
        warnings.push(format!(
            "Only {} equipment slots are captured",
            character.items.len()
        ));
    }
    if character.gems.is_empty() {
        warnings.push("No skill gems are captured".into());
    }
    if character.passive_tree_url.is_empty() {
        warnings.push("Passive tree is missing".into());
    }
    if let Some(chaos) = sheet_number(character, "Chaos Resistance") {
        if chaos < 0 {
            warnings.push(format!("Chaos Resistance is negative ({chaos}%)"));
        }
    } else {
        warnings.push("Chaos Resistance is not captured".into());
    }
    let gem_groups = character
        .gems
        .iter()
        .map(|gem| gem.group.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let gem_names = character
        .gems
        .iter()
        .map(|gem| gem.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let gem_tags = character
        .gems
        .iter()
        .flat_map(|gem| gem.tags.iter().map(|tag| tag.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let has_name = |needles: &[&str]| {
        gem_names
            .iter()
            .any(|name| needles.iter().any(|needle| name.contains(needle)))
    };
    let coverage = vec![
        (
            "Movement skill".into(),
            gem_tags.iter().any(|tag| tag == "movement"),
        ),
        (
            "Guard skill".into(),
            has_name(&[
                "molten shell",
                "steelskin",
                "immortal call",
                "arcane cloak",
                "frost shield",
            ]),
        ),
        (
            "Aura or reservation skill".into(),
            gem_tags
                .iter()
                .any(|tag| tag == "aura" || tag == "reservation"),
        ),
        (
            "Curse or mark".into(),
            gem_tags
                .iter()
                .any(|tag| tag == "curse" || tag == "hex" || tag == "mark"),
        ),
    ];
    LocalBuildAssessment {
        captured_life,
        captured_energy_shield,
        raw_life_es_pool,
        resistance_gaps,
        warnings,
        gem_groups,
        coverage,
    }
}

pub fn parse_character_sheet_text(input: &str) -> Result<BTreeMap<String, String>> {
    let fields = [
        ("Life", r"Life"),
        ("Mana", r"Mana"),
        ("Energy Shield", r"Energy\s+Shield"),
        ("Armour", r"Armou?r"),
        ("Evasion", r"Evasion(?:\s+Rating)?"),
        ("Fire Resistance", r"Fire\s+Resistance"),
        ("Cold Resistance", r"Cold\s+Resistance"),
        ("Lightning Resistance", r"Lightning\s+Resistance"),
        ("Chaos Resistance", r"Chaos\s+Resistance"),
        ("Damage per Second", r"Damage\s+per\s+Second"),
        ("Attack Speed", r"Attacks?\s+per\s+Second"),
        ("Cast Speed", r"Casts?\s+per\s+Second"),
        ("Critical Strike Chance", r"Critical\s+Strike\s+Chance"),
        ("Movement Speed", r"Movement\s+Speed"),
    ];
    let mut stats = BTreeMap::new();
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        for (label, pattern) in fields {
            if let Some(value) = value_near_label(line, pattern)? {
                stats.entry(label.to_string()).or_insert(value);
            }
        }
    }
    if stats.is_empty() {
        bail!("no supported character-sheet values were recognized");
    }
    Ok(stats)
}

pub fn parse_character_identity_text(input: &str) -> Result<DetectedCharacterIdentity> {
    let mut identity = DetectedCharacterIdentity {
        name: labelled_text(
            input,
            r"(?:Character\s+)?Name",
            r"[A-Za-z][A-Za-z0-9_'\-]{1,39}",
        )?,
        class_name: labelled_text(input, r"Class", r"[A-Za-z][A-Za-z ]{2,30}")?,
        ascendancy: labelled_text(input, r"Ascendancy", r"[A-Za-z][A-Za-z ]{2,30}")?,
        league: labelled_text(input, r"League", r"[A-Za-z0-9][A-Za-z0-9 '\-]{1,40}")?,
        ..Default::default()
    };

    let level_regex = Regex::new(r"(?im)^\s*Level\s*[:\-]?\s*([0-9]{1,3})\b")?;
    identity.level = level_regex
        .captures(input)
        .and_then(|capture| capture[1].parse::<u32>().ok())
        .filter(|level| (1..=100).contains(level));

    let compact = Regex::new(
        r"(?im)^\s*([A-Za-z][A-Za-z0-9_'\-]{1,39})\s+(?:[-·|]\s*)?Level\s+([0-9]{1,3})\b\s*(.*)$",
    )?;
    if let Some(capture) = compact.captures(input) {
        identity
            .name
            .get_or_insert_with(|| capture[1].trim().to_string());
        if identity.level.is_none() {
            identity.level = capture[2]
                .parse::<u32>()
                .ok()
                .filter(|level| (1..=100).contains(level));
        }
        apply_known_class(&mut identity, capture[3].trim());
    }

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        apply_known_class(&mut identity, line);
    }

    Ok(identity)
}

pub fn inspect_passive_tree_url(input: &str) -> Result<PassiveTreeInfo> {
    let parsed = url::Url::parse(input.trim()).context("invalid passive-tree URL")?;
    if parsed.scheme() != "https"
        || !matches!(
            parsed.host_str(),
            Some("pathofexile.com" | "www.pathofexile.com")
        )
        || (!parsed.path().contains("passive-skill-tree")
            && !parsed.path().contains("fullscreen-passive-skill-tree"))
    {
        bail!("use an official Path of Exile passive skill tree URL");
    }
    let code = parsed
        .path_segments()
        .and_then(|mut segments| segments.rfind(|part| !part.is_empty()))
        .context("passive-tree URL has no encoded tree")?;
    let bytes = general_purpose::URL_SAFE
        .decode(code)
        .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(code))
        .context("passive-tree data is not valid base64url")?;
    if bytes.len() < 7 {
        bail!("passive-tree data is incomplete");
    }
    let version = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
    let class_id = bytes[4];
    let ascendancy_data = bytes[5];
    let ascendancy_id = ascendancy_data & 0x03;
    let bloodline_id = ascendancy_data >> 2;
    let mut cursor = 6;
    let (allocated_node_ids, extended_node_ids, masteries) = match version {
        6 => {
            let nodes = read_counted_u16_values(&bytes, &mut cursor)?;
            let extended = read_counted_u16_values(&bytes, &mut cursor)?;
            let mastery_count = read_u8(&bytes, &mut cursor)? as usize;
            let required = mastery_count.saturating_mul(4);
            if bytes.len().saturating_sub(cursor) < required {
                bail!("passive-tree mastery data is incomplete");
            }
            (nodes, extended, mastery_count)
        }
        5 => (
            read_counted_u16_values(&bytes, &mut cursor)?,
            read_counted_u16_values(&bytes, &mut cursor)?,
            0,
        ),
        4 => {
            cursor = 7;
            let mut nodes = Vec::new();
            while bytes.len().saturating_sub(cursor) >= 2 {
                nodes.push(u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]));
                cursor += 2;
            }
            (nodes, Vec::new(), 0)
        }
        _ => bail!("unsupported passive-tree format version {version}"),
    };
    Ok(PassiveTreeInfo {
        version,
        class_id,
        ascendancy_id,
        bloodline_id,
        allocated_nodes: allocated_node_ids.len(),
        extended_nodes: extended_node_ids.len(),
        masteries,
        allocated_node_ids,
        extended_node_ids,
    })
}

fn prefixed_value(lines: &[&str], prefix: &str) -> Option<String> {
    lines.iter().find_map(|line| {
        line.strip_prefix(prefix)
            .map(|value| value.trim().to_string())
    })
}

fn labelled_text(input: &str, label: &str, value: &str) -> Result<Option<String>> {
    let regex = Regex::new(&format!(r"(?im)^\s*{label}\s*[:\-]\s*({value})\s*$"))?;
    Ok(regex
        .captures(input)
        .map(|capture| capture[1].trim().to_string()))
}

fn apply_known_class(identity: &mut DetectedCharacterIdentity, text: &str) {
    const CLASSES: &[&str] = &[
        "Marauder", "Ranger", "Witch", "Duelist", "Templar", "Shadow", "Scion",
    ];
    const ASCENDANCIES: &[(&str, &str)] = &[
        ("Juggernaut", "Marauder"),
        ("Berserker", "Marauder"),
        ("Chieftain", "Marauder"),
        ("Deadeye", "Ranger"),
        ("Raider", "Ranger"),
        ("Pathfinder", "Ranger"),
        ("Necromancer", "Witch"),
        ("Elementalist", "Witch"),
        ("Occultist", "Witch"),
        ("Slayer", "Duelist"),
        ("Gladiator", "Duelist"),
        ("Champion", "Duelist"),
        ("Inquisitor", "Templar"),
        ("Hierophant", "Templar"),
        ("Guardian", "Templar"),
        ("Assassin", "Shadow"),
        ("Saboteur", "Shadow"),
        ("Trickster", "Shadow"),
        ("Ascendant", "Scion"),
    ];
    let words = text.split(|character: char| !character.is_ascii_alphabetic());
    for word in words {
        if let Some(class_name) = CLASSES
            .iter()
            .find(|class_name| class_name.eq_ignore_ascii_case(word))
        {
            identity
                .class_name
                .get_or_insert_with(|| (*class_name).to_string());
        }
        if let Some((ascendancy, class_name)) = ASCENDANCIES
            .iter()
            .find(|(ascendancy, _)| ascendancy.eq_ignore_ascii_case(word))
        {
            identity
                .ascendancy
                .get_or_insert_with(|| (*ascendancy).to_string());
            identity
                .class_name
                .get_or_insert_with(|| (*class_name).to_string());
        }
    }
}

fn sum_stat(input: &str, pattern: &str) -> Result<i32> {
    let regex = Regex::new(pattern)?;
    Ok(regex
        .captures_iter(input)
        .filter_map(|capture| parse_number(&capture[1]))
        .sum())
}

fn property_value(input: &str, name: &str) -> i32 {
    let prefix = format!("{name}:");
    input
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .and_then(|value| value.split_whitespace().next())
                .and_then(parse_number)
        })
        .unwrap_or_default()
}

fn value_near_label(line: &str, label_pattern: &str) -> Result<Option<String>> {
    let number = r"(-?[0-9][0-9,.]*%?)";
    let after = Regex::new(&format!(r"(?i){label_pattern}\s*[:\-]?\s*{number}"))?;
    if let Some(capture) = after.captures(line) {
        return Ok(Some(capture[1].to_string()));
    }
    let before = Regex::new(&format!(r"(?i){number}\s*{label_pattern}"))?;
    Ok(before.captures(line).map(|capture| capture[1].to_string()))
}

fn parse_number(value: &str) -> Option<i32> {
    value
        .trim()
        .trim_end_matches('%')
        .replace(',', "")
        .parse()
        .ok()
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8> {
    let value = *bytes
        .get(*cursor)
        .context("passive-tree data is incomplete")?;
    *cursor += 1;
    Ok(value)
}

fn read_counted_u16_values(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u16>> {
    let count = read_u8(bytes, cursor)? as usize;
    let required = count.saturating_mul(2);
    if bytes.len().saturating_sub(*cursor) < required {
        bail!("passive-tree node data is incomplete");
    }
    let values = bytes[*cursor..*cursor + required]
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();
    *cursor += required;
    Ok(values)
}

fn sheet_number(character: &OfflineCharacter, name: &str) -> Option<i32> {
    character
        .sheet_stats
        .get(name)
        .and_then(|value| parse_number(value))
}

fn value_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "unknown"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_copied_item_and_sums_resistances() {
        let item = parse_item_text(
            "Helmet",
            "Item Class: Helmets\nRarity: Rare\nDoom Crown\nHubris Circlet\n--------\nEnergy Shield: 210 (augmented)\n--------\n+72 to maximum Life\n+35% to Fire Resistance\n+12% to all Elemental Resistances\n+24 to Intelligence",
        )
        .unwrap();
        assert_eq!(item.name, "Doom Crown");
        assert_eq!(item.base_type, "Hubris Circlet");
        assert_eq!(item.bonuses.life, 72);
        assert_eq!(item.bonuses.energy_shield, 210);
        assert_eq!(item.bonuses.fire_resistance, 47);
        assert_eq!(item.bonuses.cold_resistance, 12);
    }

    #[test]
    fn parses_common_ocr_layouts() {
        let stats = parse_character_sheet_text(
            "Life: 4,123\n75% Fire Resistance\nDamage per Second 1,245.6\nEnergy Shield - 820",
        )
        .unwrap();
        assert_eq!(stats["Life"], "4,123");
        assert_eq!(stats["Fire Resistance"], "75%");
        assert_eq!(stats["Energy Shield"], "820");
    }

    #[test]
    fn parses_copied_gem_and_assesses_character() {
        let gem = parse_gem_text(
            "Main skill",
            "Item Class: Skill Gems\nRarity: Gem\nFireball\n--------\nSpell, Projectile, Fire\nLevel: 20\nQuality: +20%",
        )
        .unwrap();
        assert_eq!(gem.name, "Fireball");
        assert_eq!(gem.level, 20);
        assert_eq!(gem.quality, 20);
        let mut character = OfflineCharacter::default();
        character.gems.push(gem);
        character.sheet_stats.insert("Life".into(), "4,000".into());
        character
            .sheet_stats
            .insert("Fire Resistance".into(), "60%".into());
        let assessment = assess_character(&character);
        assert_eq!(assessment.raw_life_es_pool, Some(4000));
        assert!(assessment
            .resistance_gaps
            .iter()
            .any(|gap| gap.contains("15%")));
    }

    #[test]
    fn parses_labelled_character_identity() {
        let identity = parse_character_identity_text(
            "Character Name: BoneCollector\nLevel: 94\nAscendancy: Necromancer\nLeague: Settlers",
        )
        .unwrap();
        assert_eq!(identity.name.as_deref(), Some("BoneCollector"));
        assert_eq!(identity.level, Some(94));
        assert_eq!(identity.class_name.as_deref(), Some("Witch"));
        assert_eq!(identity.ascendancy.as_deref(), Some("Necromancer"));
        assert_eq!(identity.league.as_deref(), Some("Settlers"));
    }

    #[test]
    fn parses_compact_character_identity() {
        let identity = parse_character_identity_text("MapRunner - Level 88 Pathfinder").unwrap();
        assert_eq!(identity.name.as_deref(), Some("MapRunner"));
        assert_eq!(identity.level, Some(88));
        assert_eq!(identity.class_name.as_deref(), Some("Ranger"));
        assert_eq!(identity.ascendancy.as_deref(), Some("Pathfinder"));
    }

    #[test]
    fn inspects_official_version_six_tree() {
        let bytes = [
            0, 0, 0, 6, 3, 2, 2, 0x12, 0x34, 0x56, 0x78, 1, 0x9a, 0xbc, 1, 0, 1, 0, 2,
        ];
        let code = general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let info = inspect_passive_tree_url(&format!(
            "https://www.pathofexile.com/passive-skill-tree/3.28.0/{code}"
        ))
        .unwrap();
        assert_eq!(info.version, 6);
        assert_eq!(info.allocated_nodes, 2);
        assert_eq!(info.extended_nodes, 1);
        assert_eq!(info.masteries, 1);
        assert_eq!(info.allocated_node_ids, vec![0x1234, 0x5678]);
    }

    #[test]
    fn rejects_non_official_tree_urls() {
        assert!(inspect_passive_tree_url("https://example.com/passive-skill-tree/abc").is_err());
    }
}
