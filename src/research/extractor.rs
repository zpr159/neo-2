use super::api::{
    Citation, ExtractedDate, ExtractedEntity, ExtractedEvent, ExtractedFact, ExtractedInformation,
    ExtractedLocation, ExtractedRelationship, FetchedContent,
};
use super::config::ExtractorConfig;
use super::error::ResearchResult;

/// Extracts structured information from fetched content.
pub struct InformationExtractor {
    config: ExtractorConfig,
}

impl InformationExtractor {
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract all configured information types from content.
    pub fn extract(&self, content: &FetchedContent) -> ResearchResult<ExtractedInformation> {
        let text = &content.text_content;

        Ok(ExtractedInformation {
            entities: if self.config.extract_entities {
                self.extract_entities(text)
            } else {
                Vec::new()
            },
            relationships: if self.config.extract_relationships {
                self.extract_relationships(text)
            } else {
                Vec::new()
            },
            events: if self.config.extract_events {
                self.extract_events(text)
            } else {
                Vec::new()
            },
            dates: if self.config.extract_dates {
                self.extract_dates(text)
            } else {
                Vec::new()
            },
            locations: if self.config.extract_locations {
                self.extract_locations(text)
            } else {
                Vec::new()
            },
            citations: if self.config.extract_citations {
                self.extract_citations(content)
            } else {
                Vec::new()
            },
            facts: if self.config.extract_facts {
                self.extract_facts(text, &content.url)
            } else {
                Vec::new()
            },
        })
    }

    /// Extract from multiple fetched contents and merge.
    pub fn extract_many(
        &self,
        contents: &[FetchedContent],
    ) -> ResearchResult<ExtractedInformation> {
        let mut merged = ExtractedInformation {
            entities: Vec::new(),
            relationships: Vec::new(),
            events: Vec::new(),
            dates: Vec::new(),
            locations: Vec::new(),
            citations: Vec::new(),
            facts: Vec::new(),
        };

        for content in contents {
            let info = self.extract(content)?;
            merged.entities.extend(info.entities);
            merged.relationships.extend(info.relationships);
            merged.events.extend(info.events);
            merged.dates.extend(info.dates);
            merged.locations.extend(info.locations);
            merged.citations.extend(info.citations);
            merged.facts.extend(info.facts);
        }

        Ok(merged)
    }

    fn extract_entities(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_entity = String::new();
        let mut entity_type = String::new();

        for window in words.windows(3) {
            let w0_cap = window[0]
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let w1_cap = window[1]
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            if w0_cap && w1_cap {
                if !current_entity.is_empty() {
                    if let Some(entity) = finalize_entity(
                        &current_entity,
                        &entity_type,
                        text,
                        self.config.max_entity_length,
                    ) {
                        entities.push(entity);
                    }
                }
                current_entity = format!("{} {}", window[0], window[1]);
                entity_type = "NamedEntity".to_string();
            } else if !current_entity.is_empty() && w0_cap {
                current_entity.push(' ');
                current_entity.push_str(window[0]);
            } else if !current_entity.is_empty() {
                if let Some(entity) = finalize_entity(
                    &current_entity,
                    &entity_type,
                    text,
                    self.config.max_entity_length,
                ) {
                    entities.push(entity);
                }
                current_entity.clear();
                entity_type.clear();
            }
        }

        if !current_entity.is_empty() {
            if let Some(entity) = finalize_entity(
                &current_entity,
                &entity_type,
                text,
                self.config.max_entity_length,
            ) {
                entities.push(entity);
            }
        }

        entities.sort_by(|a, b| b.mentions.cmp(&a.mentions));
        entities.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
        entities.truncate(50);

        entities
    }

    fn extract_relationships(&self, text: &str) -> Vec<ExtractedRelationship> {
        let mut relationships = Vec::new();
        let relationship_markers = [
            "is a", "is the", "is an", "was a", "was the", "was an",
            "has a", "has the", "had a", "had the",
            "belongs to", "part of", "member of",
            "works at", "works for", "founded by", "created by",
            "located in", "based in", "headquartered in",
            "produces", "manufactures", "developed", "invented",
            "leads", "manages", "directs", "owns",
            "causes", "results in", "leads to",
            "occurred in", "happened during", "took place in",
        ];

        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();

        for sentence in &sentences {
            let sentence_lower = sentence.to_lowercase();
            for marker in &relationship_markers {
                if let Some(pos) = sentence_lower.find(marker) {
                    let before = sentence[..pos].trim();
                    let after = sentence[pos + marker.len()..].trim();

                    if !before.is_empty() && !after.is_empty() {
                        let source = extract_last_entity(before);
                        let target = extract_first_entity(after);

                        if !source.is_empty()
                            && !target.is_empty()
                            && source.len() < self.config.max_entity_length
                            && target.len() < self.config.max_entity_length
                        {
                            relationships.push(ExtractedRelationship {
                                source,
                                target,
                                relationship_type: marker.to_string(),
                                context: sentence.trim().to_string(),
                                confidence: 0.5,
                            });
                        }
                    }
                    break;
                }
            }
        }

        relationships.truncate(50);
        relationships
    }

    fn extract_events(&self, text: &str) -> Vec<ExtractedEvent> {
        let mut events = Vec::new();
        let event_markers = [
            "announced", "launched", "released", "published", "reported",
            "discovered", "invented", "created", "established", "founded",
            "acquired", "merged", "signed", "approved", "rejected",
            "celebrated", "occurred", "happened", "began", "started",
            "ended", "completed", "achieved", "won", "lost",
        ];

        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();

        for sentence in &sentences {
            let sentence_lower = sentence.to_lowercase();
            for marker in &event_markers {
                if sentence_lower.contains(marker) {
                    let participants = extract_participants(sentence);
                    let event_type = marker.to_string();

                    if sentence.len() > 10 && sentence.len() < 500 {
                        events.push(ExtractedEvent {
                            description: sentence.trim().to_string(),
                            event_type,
                            participants,
                            date: None,
                            location: None,
                            confidence: 0.4,
                        });
                    }
                    break;
                }
            }
        }

        events.truncate(30);
        events
    }

    fn extract_dates(&self, text: &str) -> Vec<ExtractedDate> {
        let mut dates = Vec::new();
        let months = [
            "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December",
        ];

        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');

            if clean.len() == 4 {
                if let Ok(year) = clean.parse::<i32>() {
                    if (1900..=2100).contains(&year) {
                        if i > 0 && i + 1 < words.len() {
                            let prev = words[i - 1].trim_matches(|c: char| !c.is_alphabetic());
                            if months.iter().any(|m| prev.eq_ignore_ascii_case(m)) {
                                let day_str = words[i + 1]
                                    .trim_matches(|c: char| !c.is_alphanumeric());
                                if let Ok(_day) = day_str.parse::<u32>() {
                                    dates.push(ExtractedDate {
                                        original_text: format!("{} {} {}", prev, clean, day_str),
                                        parsed_value: Some(format!("{}-{:02}-{:02}", clean, month_number(prev), day_str.parse::<u32>().unwrap_or(1))),
                                        date_type: "long_date".to_string(),
                                        confidence: 0.8,
                                    });
                                    continue;
                                }
                            }
                        }
                        dates.push(ExtractedDate {
                            original_text: clean.to_string(),
                            parsed_value: Some(format!("{}-01-01", clean)),
                            date_type: "year_only".to_string(),
                            confidence: 0.3,
                        });
                    }
                }
            }

            if clean.contains('/') && clean.matches('/').count() == 2 {
                let parts: Vec<&str> = clean.split('/').collect();
                if parts.len() == 3 {
                    dates.push(ExtractedDate {
                        original_text: clean.to_string(),
                        parsed_value: Some(clean.to_string()),
                        date_type: "us_date".to_string(),
                        confidence: 0.7,
                    });
                }
            }

            if clean.contains('-') && clean.matches('-').count() == 2 {
                let parts: Vec<&str> = clean.split('-').collect();
                if parts.len() == 3 {
                    if let (Ok(_y), Ok(_m), Ok(_d)) = (
                        parts[0].parse::<i32>(),
                        parts[1].parse::<u32>(),
                        parts[2].parse::<u32>(),
                    ) {
                        dates.push(ExtractedDate {
                            original_text: clean.to_string(),
                            parsed_value: Some(clean.to_string()),
                            date_type: "iso_date".to_string(),
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        dates.dedup_by(|a, b| a.original_text == b.original_text);
        dates.truncate(20);
        dates
    }

    fn extract_locations(&self, text: &str) -> Vec<ExtractedLocation> {
        let mut locations = Vec::new();
        let location_prepositions = ["in", "at", "near", "from", "to", "around", "within"];

        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();

        for sentence in &sentences {
            let words: Vec<&str> = sentence.split_whitespace().collect();
            for (i, word) in words.iter().enumerate() {
                if location_prepositions.contains(&word.to_lowercase().as_str()) {
                    if i + 1 < words.len() {
                        let candidate = words[i + 1]
                            .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ');
                        if !candidate.is_empty()
                            && candidate
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                            && candidate.len() > 2
                            && candidate.len() < 100
                        {
                            locations.push(ExtractedLocation {
                                name: candidate.to_string(),
                                location_type: "referenced".to_string(),
                                context: sentence.trim().to_string(),
                                confidence: 0.4,
                            });
                        }
                    }
                }
            }
        }

        locations.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
        locations.truncate(20);
        locations
    }

    fn extract_citations(&self, content: &FetchedContent) -> Vec<Citation> {
        let mut citations = Vec::new();

        citations.push(Citation {
            id: uuid::Uuid::new_v4(),
            source_url: Some(content.url.clone()),
            source_name: extract_domain(&content.url),
            title: content
                .metadata
                .get("title")
                .cloned()
                .or_else(|| extract_title_from_url(&content.url)),
            access_date: content.fetched_at,
            snippet: Some(
                content
                    .text_content
                    .chars()
                    .take(200)
                    .collect::<String>(),
            ),
            reliability_score: estimate_source_reliability(&content.url),
            citation_format: super::config::CitationFormat::Inline,
        });

        for token in text_tokens(&content.text_content) {
            if token.starts_with("http://") || token.starts_with("https://") {
                let url = token.trim_end_matches(|c: char| c == ')' || c == ']' || c == '>' || c == ',').to_string();
                if url != content.url && !url.contains(content.url.as_str()) {
                    citations.push(Citation {
                        id: uuid::Uuid::new_v4(),
                        source_url: Some(url.clone()),
                        source_name: extract_domain(&url),
                        title: None,
                        access_date: content.fetched_at,
                        snippet: None,
                        reliability_score: estimate_source_reliability(&url),
                        citation_format: super::config::CitationFormat::Inline,
                    });
                }
            }
        }

        citations.dedup_by(|a, b| a.source_url == b.source_url);
        citations.truncate(20);
        citations
    }

    fn extract_facts(&self, text: &str, source_url: &str) -> Vec<ExtractedFact> {
        let mut facts = Vec::new();

        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();

        for sentence in &sentences {
            let trimmed = sentence.trim();
            if trimmed.len() < 20 || trimmed.len() > self.config.max_fact_length {
                continue;
            }

            if let Some(fact) = extract_svo_fact(trimmed, source_url) {
                facts.push(fact);
            }
        }

        facts.truncate(30);
        facts
    }
}

fn finalize_entity(
    name: &str,
    entity_type: &str,
    text: &str,
    max_length: usize,
) -> Option<ExtractedEntity> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > max_length {
        return None;
    }

    let count = text.matches(&*name).count();
    if count == 0 {
        return None;
    }

    let inferred_type = infer_entity_type(&name, entity_type);

    Some(ExtractedEntity {
        name: name.clone(),
        entity_type: inferred_type,
        context: find_context(&name, text),
        confidence: (count as f32 * 0.1).min(1.0),
        mentions: count,
    })
}

fn infer_entity_type(name: &str, fallback: &str) -> String {
    let name_upper = name.to_uppercase();
    if name_upper.starts_with("MR.") || name_upper.starts_with("MRS.") || name_upper.starts_with("DR.") {
        "Person".to_string()
    } else if name_upper.contains("INC") || name_upper.contains("LLC") || name_upper.contains("CORP")
        || name_upper.contains("LTD") || name_upper.contains("AG") || name_upper.contains("GMBH")
    {
        "Organization".to_string()
    } else if name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        && !name.contains(' ')
        && name.len() < 20
    {
        "PossibleName".to_string()
    } else {
        fallback.to_string()
    }
}

fn find_context(entity_name: &str, text: &str) -> String {
    if let Some(pos) = text.find(entity_name) {
        let start = pos.saturating_sub(50);
        let end = (pos + entity_name.len() + 50).min(text.len());
        text[start..end].to_string()
    } else {
        String::new()
    }
}

fn extract_last_entity(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut entity_words = Vec::new();

    for word in words.iter().rev() {
        let word_clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'');
        if word_clean.is_empty() {
            break;
        }
        entity_words.insert(0, word_clean);
    }

    entity_words.join(" ")
}

fn extract_first_entity(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut entity_words = Vec::new();

    for word in &words {
        let word_clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'');
        if word_clean.is_empty() {
            continue;
        }
        let first_char = word_clean.chars().next().unwrap_or(' ');
        if first_char.is_uppercase() || !entity_words.is_empty() {
            entity_words.push(word_clean);
        }
        if entity_words.len() >= 4 {
            break;
        }
    }

    entity_words.join(" ")
}

fn extract_participants(sentence: &str) -> Vec<String> {
    let mut participants = Vec::new();
    let words: Vec<&str> = sentence.split_whitespace().collect();

    for window in words.windows(2) {
        if window[0].chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && window[1].chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        {
            participants.push(format!("{} {}", window[0], window[1]));
        }
    }

    participants.truncate(5);
    participants
}

fn extract_domain(url: &str) -> String {
    url.split('/')
        .nth(2)
        .unwrap_or("unknown")
        .to_string()
}

fn extract_title_from_url(url: &str) -> Option<String> {
    url.rsplit('/')
        .next()
        .map(|s| {
            s.replace('-', " ")
                .replace('_', " ")
                .split('.')
                .next()
                .unwrap_or(s)
                .to_string()
        })
}

fn estimate_source_reliability(url: &str) -> f32 {
    let domain = extract_domain(url);
    let domain_lower = domain.to_lowercase();

    if domain_lower.contains("gov") || domain_lower.contains("edu") {
        0.9
    } else if domain_lower.contains("org") || domain_lower.contains("ac.uk") {
        0.7
    } else if domain_lower.contains("wikipedia") || domain_lower.contains("britannica") {
        0.8
    } else if domain_lower.contains("news") || domain_lower.contains("reuters")
        || domain_lower.contains("ap") || domain_lower.contains("bbc")
    {
        0.7
    } else if domain_lower.contains("arxiv") || domain_lower.contains("pubmed")
        || domain_lower.contains("scholar")
    {
        0.85
    } else {
        0.4
    }
}

fn extract_svo_fact(sentence: &str, source_url: &str) -> Option<ExtractedFact> {
    let copula_markers = [" is ", " was ", " are ", " were ", " has ", " had "];

    for marker in &copula_markers {
        if let Some(pos) = sentence.find(marker) {
            let subject = sentence[..pos].trim();
            let rest = sentence[pos + marker.len()..].trim();

            if !subject.is_empty() && !rest.is_empty() {
                let predicate = marker.trim().to_string();
                let object = rest
                    .trim_end_matches('.')
                    .trim_end_matches(',')
                    .to_string();

                if !subject.is_empty()
                    && subject.len() < 100
                    && !object.is_empty()
                    && object.len() < 1024
                {
                    return Some(ExtractedFact {
                        subject: subject.to_string(),
                        predicate,
                        object,
                        confidence: 0.5,
                        source_url: Some(source_url.to_string()),
                        supporting_text: sentence.to_string(),
                    });
                }
            }
        }
    }

    None
}

fn text_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn month_number(month: &str) -> u32 {
    match month.to_lowercase().as_str() {
        "january" => 1,
        "february" => 2,
        "march" => 3,
        "april" => 4,
        "may" => 5,
        "june" => 6,
        "july" => 7,
        "august" => 8,
        "september" => 9,
        "october" => 10,
        "november" => 11,
        "december" => 12,
        _ => 1,
    }
}
