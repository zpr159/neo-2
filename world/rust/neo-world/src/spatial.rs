use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{
    AttributeValue, Confidence, EntityId, EnvironmentType, LocationId,
};

/// 3D coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Coordinates {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coordinates {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }

    pub fn midpoint(&self, other: &Self) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
            z: (self.z + other.z) / 2.0,
        }
    }
}

impl Default for Coordinates {
    fn default() -> Self {
        Self::ORIGIN
    }
}

/// A named location in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub name: String,
    pub environment_type: EnvironmentType,
    pub coordinates: Option<Coordinates>,
    pub parent_id: Option<LocationId>,
    pub properties: HashMap<String, AttributeValue>,
    pub description: String,
    pub occupants: Vec<EntityId>,
    pub confidence: Confidence,
    pub tags: Vec<String>,
    pub recorded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Location {
    pub fn new(name: impl Into<String>, environment_type: EnvironmentType) -> Self {
        let now = Utc::now();
        Self {
            id: LocationId::random(),
            name: name.into(),
            environment_type,
            coordinates: None,
            parent_id: None,
            properties: HashMap::new(),
            description: String::new(),
            occupants: Vec::new(),
            confidence: Confidence::MEDIUM,
            tags: Vec::new(),
            recorded_at: now,
            updated_at: now,
        }
    }

    pub fn add_occupant(&mut self, entity_id: EntityId) {
        if !self.occupants.contains(&entity_id) {
            self.occupants.push(entity_id);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_occupant(&mut self, entity_id: &EntityId) {
        self.occupants.retain(|o| o != entity_id);
        self.updated_at = Utc::now();
    }
}

/// Types of spatial relationships.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpatialRelationType {
    Contains,
    AdjacentTo,
    Near,
    Far,
    Inside,
    Outside,
    Above,
    Below,
    ConnectedTo,
    Custom(String),
}

impl std::fmt::Display for SpatialRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contains => write!(f, "contains"),
            Self::AdjacentTo => write!(f, "adjacent_to"),
            Self::Near => write!(f, "near"),
            Self::Far => write!(f, "far"),
            Self::Inside => write!(f, "inside"),
            Self::Outside => write!(f, "outside"),
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::ConnectedTo => write!(f, "connected_to"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// A spatial relationship between two locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialRelation {
    pub from: LocationId,
    pub to: LocationId,
    pub relation_type: SpatialRelationType,
    pub distance: Option<f64>,
    pub confidence: Confidence,
    pub properties: HashMap<String, AttributeValue>,
}

/// A region defined by bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialRegion {
    pub min: Coordinates,
    pub max: Coordinates,
    pub name: String,
    pub region_type: String,
}

impl SpatialRegion {
    pub fn contains(&self, coords: &Coordinates) -> bool {
        coords.x >= self.min.x && coords.x <= self.max.x
            && coords.y >= self.min.y && coords.y <= self.max.y
            && coords.z >= self.min.z && coords.z <= self.max.z
    }

    pub fn intersects(&self, other: &SpatialRegion) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x
            && self.min.y <= other.max.y && self.max.y >= other.min.y
            && self.min.z <= other.max.z && self.max.z >= other.min.z
    }
}

/// Simple spatial index for proximity queries.
struct SpatialIndexEntry {
    location_id: LocationId,
    coords: Coordinates,
}

/// Manages spatial knowledge.
pub struct SpatialModel {
    locations: dashmap::DashMap<LocationId, Location>,
    relations: Vec<SpatialRelation>,
    index: Vec<SpatialIndexEntry>,
    regions: Vec<SpatialRegion>,
}

impl SpatialModel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            locations: dashmap::DashMap::new(),
            relations: Vec::new(),
            index: Vec::new(),
            regions: Vec::new(),
        }
    }

    pub fn add_location(&mut self, location: Location) -> LocationId {
        let id = location.id.clone();
        if let Some(coords) = location.coordinates {
            self.index.push(SpatialIndexEntry {
                location_id: id.clone(),
                coords,
            });
        }
        self.locations.insert(id.clone(), location);
        id
    }

    pub fn get_location(&self, id: &LocationId) -> Option<Location> {
        self.locations.get(id).map(|l| l.value().clone())
    }

    pub fn find_locations(&self, name: &str) -> Vec<Location> {
        let lower = name.to_lowercase();
        self.locations
            .iter()
            .filter(|l| l.value().name.to_lowercase().contains(&lower))
            .map(|l| l.value().clone())
            .collect()
    }

    pub fn locations_by_environment(&self, env_type: &EnvironmentType) -> Vec<Location> {
        self.locations
            .iter()
            .filter(|l| &l.value().environment_type == env_type)
            .map(|l| l.value().clone())
            .collect()
    }

    pub fn children_of(&self, parent_id: &LocationId) -> Vec<Location> {
        self.locations
            .iter()
            .filter(|l| l.value().parent_id.as_ref() == Some(parent_id))
            .map(|l| l.value().clone())
            .collect()
    }

    pub fn entity_moved(&self, entity_id: &EntityId, from: Option<&LocationId>, to: &LocationId) {
        if let Some(from_id) = from {
            if let Some(mut loc) = self.locations.get_mut(from_id) {
                loc.remove_occupant(entity_id);
            }
        }
        if let Some(mut loc) = self.locations.get_mut(to) {
            loc.add_occupant(entity_id.clone());
        }
    }

    pub fn occupants_at(&self, location_id: &LocationId) -> Vec<EntityId> {
        self.locations
            .get(location_id)
            .map(|l| l.value().occupants.clone())
            .unwrap_or_default()
    }

    pub fn nearby(&self, coords: &Coordinates, radius: f64) -> Vec<(Location, f64)> {
        let mut result: Vec<(Location, f64)> = self
            .index
            .iter()
            .filter_map(|entry| {
                let dist = coords.distance_to(&entry.coords);
                if dist <= radius {
                    self.locations.get(&entry.location_id).map(|l| (l.value().clone(), dist))
                } else {
                    None
                }
            })
            .collect();
        result.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    pub fn add_relation(&mut self, relation: SpatialRelation) {
        self.relations.push(relation);
    }

    pub fn add_region(&mut self, region: SpatialRegion) {
        self.regions.push(region);
    }

    pub fn find_containing_region(&self, coords: &Coordinates) -> Option<&SpatialRegion> {
        self.regions.iter().find(|r| r.contains(coords))
    }

    pub fn count(&self) -> usize {
        self.locations.len()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

impl Default for SpatialModel {
    fn default() -> Self {
        Self::new()
    }
}
