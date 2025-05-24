use anyhow::{Context, ensure};
use itertools::Itertools;
use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::sync::LazyLock;

pub fn merge_contacts(inputs: Vec<PathBuf>, output: PathBuf) -> anyhow::Result<()> {
    let mut cards = Vec::new();

    for input in inputs {
        let data = fs::read_to_string(&input)
            .with_context(|| format!("failed to read {}", input.display()))?;
        let parsed =
            parse_vcf(&data).with_context(|| format!("failed to parse {}", input.display()))?;
        cards.extend(parsed);
    }
    tracing::info!("Parsed {} cards", cards.len());

    // Take unique cards with a phone number and remove CATEGORIES
    let cards: BTreeSet<_> = cards
        .into_iter()
        .filter(|card| card.items.iter().any(|item| item.property == "TEL"))
        .map(|mut card| {
            card.items.retain(|item| item.property != "CATEGORIES");
            card
        })
        .collect();
    tracing::info!("Detected {} distinct cards with telephone", cards.len());

    // Detect possible duplicates by phone number
    let possible_duplicates = cards
        .iter()
        .flat_map(|card| card.items.iter().map(move |item| (card, item)))
        .filter(|(_, item)| item.property == "TEL")
        .map(|(card, item)| {
            let tel_suffix: String = item
                .value
                .chars()
                .rev()
                .filter(|c| c.is_ascii_digit())
                .take(8)
                .collect();
            (tel_suffix, card)
        })
        .fold(HashMap::new(), |mut map, (tel_suffix, card)| {
            map.entry(tel_suffix)
                .or_insert_with(BTreeSet::new)
                .insert(card);
            map
        })
        .into_iter()
        .filter(|(_, cards)| cards.len() > 1)
        .collect_vec();

    for (tel_suffix, cards) in possible_duplicates {
        println!(
            "- Possible duplicate for phone number ending in {}:",
            tel_suffix
        );
        for card in cards {
            println!("{}", card);
        }
        println!();
    }

    let contents = cards.iter().format("\n").to_string();
    fs::write(&output, contents)?;

    Ok(())
}

#[derive(Debug, Default, Ord, PartialOrd, PartialEq, Eq)]
struct VCard {
    items: Vec<VCardItem>,
}

#[derive(Debug, Ord, PartialOrd, PartialEq, Eq)]
struct VCardItem {
    group: Option<String>,
    property: String,
    attribute: Option<String>,
    value: String,
}

fn parse_vcf(data: &str) -> anyhow::Result<Vec<VCard>> {
    let physical_lines = data.lines().collect_vec();
    let mut lines = Vec::new();
    for physical_line in physical_lines {
        if let Some(rest) = physical_line.strip_prefix(' ') {
            // Line continuation, as described in
            // https://datatracker.ietf.org/doc/html/rfc2425#section-5.8.1
            *lines.last_mut().context("missing previous line")? += rest;
        } else {
            lines.push(physical_line.to_string());
        }
    }

    let mut cards = Vec::new();

    enum State {
        BeforeBegin,
        BeforeVersion,
        BeforeItem(VCard),
    }

    let mut state = State::BeforeBegin;
    for line in lines {
        match state {
            State::BeforeBegin => {
                ensure!(line == "BEGIN:VCARD");
                state = State::BeforeVersion;
            }
            State::BeforeVersion => {
                ensure!(line == "VERSION:3.0");
                state = State::BeforeItem(VCard::default());
            }
            State::BeforeItem(mut card) => {
                if line == "END:VCARD" {
                    state = State::BeforeBegin;
                    cards.push(card);
                } else {
                    // A line like "item1.EMAIL;TYPE=INTERNET:bugabuga@hotmail.com"
                    // - group: item1 (optional)
                    // - property: EMAIL (required)
                    // - attribute: TYPE=INTERNET (optional)
                    // - value: bugabuga@hotmail.com (required)

                    static LINE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
                        Regex::new(r"^(?:([a-z0-9]+)\.)?([A-Za-z-]+)(?:;([A-Z=;]+))?:(.*)$")
                            .unwrap()
                    });

                    let captures = LINE_REGEX
                        .captures(&line)
                        .with_context(|| format!("invalid line: {}", line))?;

                    let item = VCardItem {
                        group: captures.get(1).map(|c| c.as_str().to_string()),
                        property: captures[2].to_string(),
                        attribute: captures.get(3).map(|c| c.as_str().to_string()),
                        value: captures[4].to_string(),
                    };

                    card.items.push(item);
                    state = State::BeforeItem(card);
                }
            }
        }
    }

    Ok(cards)
}

impl Display for VCardItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(group) = &self.group {
            write!(f, "{}.", group)?;
        }
        write!(f, "{}", self.property)?;
        if let Some(attribute) = &self.attribute {
            write!(f, ";{}", attribute)?;
        }
        write!(f, ":{}", self.value)
    }
}

impl Display for VCard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BEGIN:VCARD")?;
        writeln!(f, "VERSION:3.0")?;
        for item in &self.items {
            writeln!(f, "{}", item)?;
        }
        write!(f, "END:VCARD")
    }
}
