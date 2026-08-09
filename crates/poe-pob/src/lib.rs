use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use quick_xml::{events::Event, Reader};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::Read,
    path::Path,
};

const MAX_XML_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct PobStat {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PobEquipment {
    pub slot: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PobBuild {
    pub level: Option<u32>,
    pub class_name: String,
    pub ascendancy: String,
    pub main_skill: String,
    pub stats: Vec<PobStat>,
    pub skill_gems: Vec<String>,
    pub equipment: Vec<PobEquipment>,
}

impl PobBuild {
    pub fn stat(&self, names: &[&str]) -> Option<&str> {
        names.iter().find_map(|wanted| {
            self.stats
                .iter()
                .find(|stat| stat.name.eq_ignore_ascii_case(wanted))
                .map(|stat| stat.value.as_str())
        })
    }

    pub fn summary(&self) -> String {
        let identity = [
            self.level.map(|level| format!("Level {level}")),
            (!self.class_name.is_empty()).then(|| self.class_name.clone()),
            (!self.ascendancy.is_empty()).then(|| self.ascendancy.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
        let stats = self
            .stats
            .iter()
            .filter(|stat| is_headline_stat(&stat.name))
            .take(16)
            .map(|stat| format!("{}={}", stat.name, stat.value))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Character: {identity}\nMain skill: {}\nStats: {stats}\nSkills: {}\nEquipment: {}",
            value_or_unknown(&self.main_skill),
            self.skill_gems.join(", "),
            self.equipment
                .iter()
                .map(|item| format!("{}: {}", item.slot, item.name))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn import_path(path: &Path) -> Result<PobBuild> {
    let metadata =
        fs::metadata(path).with_context(|| format!("could not inspect {}", path.display()))?;
    if metadata.len() > MAX_XML_BYTES {
        bail!("Path of Building file is larger than 16 MiB");
    }
    let input =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    import(&input)
}

pub fn import(input: &str) -> Result<PobBuild> {
    let input = input.trim();
    if input.is_empty() {
        bail!("paste a Path of Building export code or select an XML file");
    }
    if input.len() as u64 > MAX_XML_BYTES {
        bail!("Path of Building input is larger than 16 MiB");
    }
    if input.starts_with("http://") || input.starts_with("https://") {
        bail!("web build links are not downloaded; paste the Path of Building export code instead");
    }
    let xml = if input.starts_with('<') {
        input.to_string()
    } else {
        decode_export(input)?
    };
    parse_xml(&xml)
}

fn decode_export(input: &str) -> Result<String> {
    let compact: String = input
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let compressed = general_purpose::URL_SAFE_NO_PAD
        .decode(&compact)
        .or_else(|_| general_purpose::URL_SAFE.decode(&compact))
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(&compact))
        .or_else(|_| general_purpose::STANDARD.decode(&compact))
        .context("the pasted text is not a valid Path of Building export code")?;

    decompress_limited(ZlibDecoder::new(compressed.as_slice()))
        .or_else(|_| decompress_limited(DeflateDecoder::new(compressed.as_slice())))
        .context("the export code could not be decompressed")
}

fn decompress_limited(mut decoder: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    decoder
        .by_ref()
        .take(MAX_XML_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_XML_BYTES {
        bail!("decompressed Path of Building data is larger than 16 MiB");
    }
    String::from_utf8(bytes).context("Path of Building data is not UTF-8 XML")
}

fn parse_xml(xml: &str) -> Result<PobBuild> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut build = PobBuild::default();
    let mut items = HashMap::<String, String>::new();
    let mut slots = BTreeMap::<String, String>::new();
    let mut current_item: Option<(String, String)> = None;
    let mut active_item_set = String::new();
    let mut current_item_set = String::new();
    let mut active_skill_set = String::new();
    let mut current_skill_set = String::new();
    let mut main_socket_group = 1_usize;
    let mut skill_group_index = 0_usize;
    let mut current_skill_is_main = false;
    let mut current_main_skill_index = 1_usize;
    let mut current_active_gems = Vec::<String>::new();
    let mut saw_path_of_building = false;

    loop {
        match reader
            .read_event()
            .context("invalid Path of Building XML")?
        {
            Event::Start(element) => {
                let name = element.name();
                let name = name.as_ref();
                if name == b"PathOfBuilding" {
                    saw_path_of_building = true;
                } else if name == b"Build" {
                    build.level = attribute(&element, b"level", reader.decoder())?
                        .and_then(|value| value.parse().ok());
                    build.class_name =
                        attribute(&element, b"className", reader.decoder())?.unwrap_or_default();
                    build.ascendancy = attribute(&element, b"ascendClassName", reader.decoder())?
                        .unwrap_or_default();
                    build.main_skill =
                        attribute(&element, b"mainSkill", reader.decoder())?.unwrap_or_default();
                    main_socket_group = attribute(&element, b"mainSocketGroup", reader.decoder())?
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(1);
                } else if name == b"PlayerStat" {
                    collect_stat(&mut build, &element, reader.decoder())?;
                } else if name == b"Skills" {
                    active_skill_set = attribute(&element, b"activeSkillSet", reader.decoder())?
                        .unwrap_or_default();
                } else if name == b"SkillSet" {
                    current_skill_set =
                        attribute(&element, b"id", reader.decoder())?.unwrap_or_default();
                    if skill_set_is_active(&active_skill_set, &current_skill_set) {
                        skill_group_index = 0;
                    }
                } else if name == b"Skill"
                    && skill_set_is_active(&active_skill_set, &current_skill_set)
                {
                    skill_group_index += 1;
                    current_skill_is_main = skill_group_index == main_socket_group
                        && attribute(&element, b"enabled", reader.decoder())?.as_deref()
                            != Some("false");
                    current_main_skill_index =
                        attribute(&element, b"mainActiveSkill", reader.decoder())?
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(1);
                    current_active_gems.clear();
                } else if name == b"Gem"
                    && skill_set_is_active(&active_skill_set, &current_skill_set)
                {
                    if current_skill_is_main {
                        if let Some(name) = active_gem_name(&element, reader.decoder())? {
                            if !is_support_gem(&name) {
                                current_active_gems.push(name);
                            }
                        }
                    }
                    collect_gem(&mut build, &element, reader.decoder())?;
                } else if name == b"Items" {
                    active_item_set = attribute(&element, b"activeItemSet", reader.decoder())?
                        .unwrap_or_default();
                } else if name == b"ItemSet" {
                    current_item_set =
                        attribute(&element, b"id", reader.decoder())?.unwrap_or_default();
                } else if name == b"Item" {
                    if let Some(id) = attribute(&element, b"id", reader.decoder())? {
                        current_item = Some((id, String::new()));
                    }
                } else if name == b"Slot" && item_set_is_active(&active_item_set, &current_item_set)
                {
                    collect_slot(&mut slots, &element, reader.decoder())?;
                }
            }
            Event::Empty(element) => {
                let name = element.name();
                let name = name.as_ref();
                if name == b"PlayerStat" {
                    collect_stat(&mut build, &element, reader.decoder())?;
                } else if name == b"Gem"
                    && skill_set_is_active(&active_skill_set, &current_skill_set)
                {
                    if current_skill_is_main {
                        if let Some(name) = active_gem_name(&element, reader.decoder())? {
                            if !is_support_gem(&name) {
                                current_active_gems.push(name);
                            }
                        }
                    }
                    collect_gem(&mut build, &element, reader.decoder())?;
                } else if name == b"Slot" && item_set_is_active(&active_item_set, &current_item_set)
                {
                    collect_slot(&mut slots, &element, reader.decoder())?;
                }
            }
            Event::Text(text) => {
                if let Some((_, content)) = &mut current_item {
                    content.push_str(&text.decode().context("invalid item text")?);
                }
            }
            Event::CData(text) => {
                if let Some((_, content)) = &mut current_item {
                    content.push_str(&text.decode().context("invalid item text")?);
                }
            }
            Event::End(element) => {
                let name = element.name();
                let name = name.as_ref();
                if name == b"Item" {
                    if let Some((id, content)) = current_item.take() {
                        items.insert(id, item_display_name(&content));
                    }
                } else if name == b"ItemSet" {
                    current_item_set.clear();
                } else if name == b"Skill" {
                    if current_skill_is_main && build.main_skill.is_empty() {
                        build.main_skill = current_active_gems
                            .get(current_main_skill_index.saturating_sub(1))
                            .or_else(|| current_active_gems.first())
                            .cloned()
                            .unwrap_or_default();
                    }
                    current_skill_is_main = false;
                    current_active_gems.clear();
                } else if name == b"SkillSet" {
                    current_skill_set.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if !saw_path_of_building {
        bail!("XML is not a Path of Building document");
    }
    build.skill_gems.sort();
    build.skill_gems.dedup();
    build.equipment = slots
        .into_iter()
        .filter_map(|(slot, item_id)| {
            items
                .get(&item_id)
                .filter(|name| !name.is_empty())
                .map(|name| PobEquipment {
                    slot,
                    name: name.clone(),
                })
        })
        .collect();
    Ok(build)
}

fn attribute(
    element: &quick_xml::events::BytesStart<'_>,
    name: &[u8],
    decoder: quick_xml::encoding::Decoder,
) -> Result<Option<String>> {
    for attribute in element.attributes().with_checks(false) {
        let attribute = attribute.context("invalid XML attribute")?;
        if attribute.key.as_ref() == name {
            return Ok(Some(
                attribute
                    .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, decoder)?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn collect_stat(
    build: &mut PobBuild,
    element: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<()> {
    if let (Some(name), Some(value)) = (
        attribute(element, b"stat", decoder)?,
        attribute(element, b"value", decoder)?,
    ) {
        build.stats.push(PobStat { name, value });
    }
    Ok(())
}

fn collect_gem(
    build: &mut PobBuild,
    element: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<()> {
    if let Some(name) = active_gem_name(element, decoder)? {
        build.skill_gems.push(name);
    }
    Ok(())
}

fn active_gem_name(
    element: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<Option<String>> {
    if attribute(element, b"enabled", decoder)?.as_deref() == Some("false") {
        return Ok(None);
    }
    Ok(attribute(element, b"nameSpec", decoder)?
        .or(attribute(element, b"skillId", decoder)?)
        .filter(|name| !name.is_empty()))
}

fn is_support_gem(name: &str) -> bool {
    name.ends_with(" Support") || name.starts_with("Support")
}

fn collect_slot(
    slots: &mut BTreeMap<String, String>,
    element: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<()> {
    if let (Some(name), Some(item_id)) = (
        attribute(element, b"name", decoder)?,
        attribute(element, b"itemId", decoder)?,
    ) {
        if item_id != "0" {
            slots.insert(name, item_id);
        }
    }
    Ok(())
}

fn item_set_is_active(active: &str, current: &str) -> bool {
    current.is_empty() || active.is_empty() || active == current
}

fn skill_set_is_active(active: &str, current: &str) -> bool {
    current.is_empty() || active.is_empty() || active == current
}

fn item_display_name(content: &str) -> String {
    let lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let Some(rarity) = lines.first().and_then(|line| line.strip_prefix("Rarity: ")) else {
        return lines.first().copied().unwrap_or_default().to_string();
    };
    match rarity {
        "RARE" | "UNIQUE" | "RELIC" => lines.get(1..=2).unwrap_or(&[]).join(" — "),
        _ => lines.get(1).copied().unwrap_or_default().to_string(),
    }
}

fn is_headline_stat(name: &str) -> bool {
    matches!(
        name,
        "Life"
            | "LifeUnreserved"
            | "EnergyShield"
            | "Mana"
            | "Armour"
            | "Evasion"
            | "Ward"
            | "FireResist"
            | "ColdResist"
            | "LightningResist"
            | "ChaosResist"
            | "FullDPS"
            | "CombinedDPS"
            | "TotalDPS"
            | "TotalDot"
            | "Speed"
            | "CritChance"
    )
}

fn value_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "unknown"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
  <Build level="94" className="Witch" ascendClassName="Elementalist" mainSocketGroup="1">
    <PlayerStat stat="Life" value="4123"/>
    <PlayerStat stat="FullDPS" value="1250000.5"/>
  </Build>
  <Skills><Skill mainActiveSkill="2"><Gem enabled="true" nameSpec="Arc"/><Gem enabled="true" nameSpec="Added Lightning Damage Support"/><Gem enabled="true" nameSpec="Ice Nova"/><Gem enabled="false" nameSpec="Clarity"/></Skill></Skills>
  <Items activeItemSet="1">
    <Item id="7">Rarity: UNIQUE
Storm Call
Prophecy Wand
Unique ID: abc</Item>
    <ItemSet id="1"><Slot name="Weapon 1" itemId="7"/></ItemSet>
  </Items>
</PathOfBuilding>"#;

    #[test]
    fn imports_raw_xml() {
        let build = import(SAMPLE).unwrap();
        assert_eq!(build.level, Some(94));
        assert_eq!(build.ascendancy, "Elementalist");
        assert_eq!(build.main_skill, "Ice Nova");
        assert_eq!(build.stat(&["Life"]), Some("4123"));
        assert_eq!(
            build.skill_gems,
            ["Added Lightning Damage Support", "Arc", "Ice Nova"]
        );
        assert_eq!(
            build.equipment,
            [PobEquipment {
                slot: "Weapon 1".into(),
                name: "Storm Call — Prophecy Wand".into()
            }]
        );
    }

    #[test]
    fn imports_encoded_export() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(SAMPLE.as_bytes()).unwrap();
        let code = general_purpose::URL_SAFE_NO_PAD.encode(encoder.finish().unwrap());
        assert_eq!(import(&code).unwrap().level, Some(94));
    }

    #[test]
    fn rejects_links_and_unrelated_xml() {
        assert!(import("https://pobb.in/example").is_err());
        assert!(import("<document />").is_err());
    }
}
