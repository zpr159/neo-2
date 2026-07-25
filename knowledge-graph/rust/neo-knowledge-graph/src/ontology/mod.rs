pub mod types;
pub mod taxonomy;
pub mod schema_validation;

pub use types::{Ontology, EntityTypeDefinition, RelationTypeDefinition, PropertyDefinition};
pub use taxonomy::{TaxonomyNode, TaxonomyTree, TaxonomyPath};
pub use schema_validation::{OntologyValidator, ValidationResult, ValidationViolation};
