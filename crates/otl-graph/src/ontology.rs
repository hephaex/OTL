//! Ontology schema management with versioning and migration
//!
//! This module provides comprehensive ontology lifecycle management:
//! - Schema definition with entity and relation types
//! - Semantic versioning for schema evolution
//! - Schema validation and compatibility checking
//! - Migration tools for schema changes
//! - Import/export in multiple formats (JSON, YAML, OWL, RDF)
//!
//! Author: hephaex@gmail.com

use chrono::{DateTime, Utc};
use otl_core::{OtlError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ============================================================================
// Core Ontology Schema Types
// ============================================================================

/// Complete ontology schema with versioning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OntologySchema {
    /// Schema identifier
    pub id: Uuid,

    /// Semantic version (e.g., "1.2.0")
    pub version: semver::Version,

    /// Schema name/title
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Entity type definitions
    pub entities: HashMap<String, EntityTypeDef>,

    /// Relation type definitions
    pub relations: HashMap<String, RelationTypeDef>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Author/creator
    pub author: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OntologySchema {
    /// Create a new ontology schema
    pub fn new(name: impl Into<String>, version: semver::Version) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            version,
            name: name.into(),
            description: None,
            entities: HashMap::new(),
            relations: HashMap::new(),
            created_at: now,
            updated_at: now,
            author: None,
            metadata: HashMap::new(),
        }
    }

    /// Add an entity type definition
    pub fn add_entity_type(&mut self, entity_type: EntityTypeDef) -> Result<()> {
        let name = entity_type.name.clone();

        // Check for duplicate
        if self.entities.contains_key(&name) {
            return Err(OtlError::InvalidOntology(format!(
                "Entity type '{}' already exists",
                name
            )));
        }

        self.entities.insert(name, entity_type);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a relation type definition
    pub fn add_relation_type(&mut self, relation_type: RelationTypeDef) -> Result<()> {
        let name = relation_type.name.clone();

        // Check for duplicate
        if self.relations.contains_key(&name) {
            return Err(OtlError::InvalidOntology(format!(
                "Relation type '{}' already exists",
                name
            )));
        }

        // Validate that from/to entity types exist
        if !self.entities.contains_key(&relation_type.from_entity) {
            return Err(OtlError::InvalidOntology(format!(
                "Source entity type '{}' not found",
                relation_type.from_entity
            )));
        }

        if !self.entities.contains_key(&relation_type.to_entity) {
            return Err(OtlError::InvalidOntology(format!(
                "Target entity type '{}' not found",
                relation_type.to_entity
            )));
        }

        self.relations.insert(name, relation_type);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Validate the entire schema for consistency
    pub fn validate(&self) -> Result<()> {
        // Check entity type validity
        for (name, entity) in &self.entities {
            if name != &entity.name {
                return Err(OtlError::InvalidOntology(format!(
                    "Entity key '{}' does not match entity name '{}'",
                    name, entity.name
                )));
            }

            entity.validate()?;
        }

        // Check relation type validity
        for (name, relation) in &self.relations {
            if name != &relation.name {
                return Err(OtlError::InvalidOntology(format!(
                    "Relation key '{}' does not match relation name '{}'",
                    name, relation.name
                )));
            }

            // Verify entity types exist
            if !self.entities.contains_key(&relation.from_entity) {
                return Err(OtlError::InvalidOntology(format!(
                    "Relation '{}' references unknown source entity type '{}'",
                    name, relation.from_entity
                )));
            }

            if !self.entities.contains_key(&relation.to_entity) {
                return Err(OtlError::InvalidOntology(format!(
                    "Relation '{}' references unknown target entity type '{}'",
                    name, relation.to_entity
                )));
            }

            relation.validate()?;
        }

        Ok(())
    }
}

/// Entity type definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityTypeDef {
    /// Entity type name (e.g., "Person", "Organization")
    pub name: String,

    /// Human-readable label
    pub label: String,

    /// Description
    pub description: Option<String>,

    /// Property definitions
    pub properties: HashMap<String, PropertyDef>,

    /// Parent entity type (for inheritance)
    pub parent: Option<String>,

    /// Additional constraints
    pub constraints: Vec<Constraint>,
}

impl EntityTypeDef {
    /// Create a new entity type definition
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: None,
            properties: HashMap::new(),
            parent: None,
            constraints: Vec::new(),
        }
    }

    /// Add a property definition
    pub fn with_property(mut self, property: PropertyDef) -> Self {
        self.properties.insert(property.name.clone(), property);
        self
    }

    /// Validate the entity type definition
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(OtlError::ValidationError("Entity type name cannot be empty".to_string()));
        }

        if self.label.is_empty() {
            return Err(OtlError::ValidationError("Entity type label cannot be empty".to_string()));
        }

        // Validate properties
        for (key, prop) in &self.properties {
            if key != &prop.name {
                return Err(OtlError::ValidationError(format!(
                    "Property key '{}' does not match property name '{}'",
                    key, prop.name
                )));
            }
            prop.validate()?;
        }

        Ok(())
    }
}

/// Relation type definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationTypeDef {
    /// Relation type name (e.g., "WORKS_AT", "MANAGES")
    pub name: String,

    /// Human-readable label
    pub label: String,

    /// Description
    pub description: Option<String>,

    /// Source entity type
    pub from_entity: String,

    /// Target entity type
    pub to_entity: String,

    /// Property definitions for the relation
    pub properties: HashMap<String, PropertyDef>,

    /// Cardinality constraints
    pub cardinality: RelationCardinality,
}

impl RelationTypeDef {
    /// Create a new relation type definition
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: None,
            from_entity: from.into(),
            to_entity: to.into(),
            properties: HashMap::new(),
            cardinality: RelationCardinality::ManyToMany,
        }
    }

    /// Validate the relation type definition
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(OtlError::ValidationError("Relation type name cannot be empty".to_string()));
        }

        if self.label.is_empty() {
            return Err(OtlError::ValidationError("Relation type label cannot be empty".to_string()));
        }

        if self.from_entity.is_empty() {
            return Err(OtlError::ValidationError("From entity type cannot be empty".to_string()));
        }

        if self.to_entity.is_empty() {
            return Err(OtlError::ValidationError("To entity type cannot be empty".to_string()));
        }

        // Validate properties
        for (key, prop) in &self.properties {
            if key != &prop.name {
                return Err(OtlError::ValidationError(format!(
                    "Property key '{}' does not match property name '{}'",
                    key, prop.name
                )));
            }
            prop.validate()?;
        }

        Ok(())
    }
}

/// Property definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyDef {
    /// Property name
    pub name: String,

    /// Data type
    #[serde(rename = "type")]
    pub data_type: PropertyType,

    /// Is this property required?
    #[serde(default)]
    pub required: bool,

    /// Format constraint (e.g., "email", "uri", "date")
    pub format: Option<String>,

    /// Default value
    pub default: Option<serde_json::Value>,

    /// Description
    pub description: Option<String>,

    /// Validation pattern (regex)
    pub pattern: Option<String>,

    /// Minimum value (for numbers)
    pub minimum: Option<f64>,

    /// Maximum value (for numbers)
    pub maximum: Option<f64>,

    /// Enum values (allowed values)
    pub enum_values: Option<Vec<serde_json::Value>>,
}

impl PropertyDef {
    /// Create a new property definition
    pub fn new(name: impl Into<String>, data_type: PropertyType) -> Self {
        Self {
            name: name.into(),
            data_type,
            required: false,
            format: None,
            default: None,
            description: None,
            pattern: None,
            minimum: None,
            maximum: None,
            enum_values: None,
        }
    }

    /// Make this property required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Validate the property definition
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(OtlError::ValidationError("Property name cannot be empty".to_string()));
        }

        // Validate pattern if provided
        if let Some(pattern) = &self.pattern {
            regex::Regex::new(pattern).map_err(|e| {
                OtlError::ValidationError(format!("Invalid regex pattern '{}': {}", pattern, e))
            })?;
        }

        // Validate min/max constraints
        if let (Some(min), Some(max)) = (self.minimum, self.maximum) {
            if min > max {
                return Err(OtlError::ValidationError(format!(
                    "Minimum ({}) cannot be greater than maximum ({})",
                    min, max
                )));
            }
        }

        Ok(())
    }
}

/// Property data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Date,
    Time,
    Duration,
    /// Array of specific type
    Array(Box<PropertyType>),
    /// Reference to another entity
    Reference(String),
    /// Object/map type
    Object,
    /// Any type
    Any,
}

/// Relation cardinality
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationCardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Schema constraint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// Unique constraint on specific properties
    Unique { properties: Vec<String> },

    /// Check constraint with expression
    Check { expression: String },

    /// Foreign key constraint
    ForeignKey {
        properties: Vec<String>,
        references: String,
        referenced_properties: Vec<String>,
    },
}

// ============================================================================
// Schema Versioning and Migration
// ============================================================================

/// Schema change operation for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SchemaChange {
    /// Add a new entity type
    AddEntityType { entity_type: EntityTypeDef },

    /// Remove an entity type
    RemoveEntityType { name: String },

    /// Modify an entity type
    ModifyEntityType {
        name: String,
        changes: EntityTypeChanges,
    },

    /// Add a new relation type
    AddRelationType { relation_type: RelationTypeDef },

    /// Remove a relation type
    RemoveRelationType { name: String },

    /// Modify a relation type
    ModifyRelationType {
        name: String,
        changes: RelationTypeChanges,
    },
}

/// Changes to an entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeChanges {
    /// New label
    pub label: Option<String>,

    /// New description
    pub description: Option<String>,

    /// Properties to add
    pub add_properties: HashMap<String, PropertyDef>,

    /// Properties to remove
    pub remove_properties: Vec<String>,

    /// Properties to modify
    pub modify_properties: HashMap<String, PropertyDef>,
}

/// Changes to a relation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationTypeChanges {
    /// New label
    pub label: Option<String>,

    /// New description
    pub description: Option<String>,

    /// Properties to add
    pub add_properties: HashMap<String, PropertyDef>,

    /// Properties to remove
    pub remove_properties: Vec<String>,

    /// Properties to modify
    pub modify_properties: HashMap<String, PropertyDef>,

    /// New cardinality
    pub cardinality: Option<RelationCardinality>,
}

/// Schema migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMigration {
    /// Migration ID
    pub id: Uuid,

    /// Source version
    pub from_version: semver::Version,

    /// Target version
    pub to_version: semver::Version,

    /// Changes to apply
    pub changes: Vec<SchemaChange>,

    /// Migration description
    pub description: String,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Author
    pub author: Option<String>,
}

impl SchemaMigration {
    /// Create a new schema migration
    pub fn new(
        from: semver::Version,
        to: semver::Version,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_version: from,
            to_version: to,
            changes: Vec::new(),
            description: description.into(),
            created_at: Utc::now(),
            author: None,
        }
    }

    /// Add a change operation
    pub fn add_change(mut self, change: SchemaChange) -> Self {
        self.changes.push(change);
        self
    }

    /// Apply this migration to a schema
    pub fn apply(&self, schema: &mut OntologySchema) -> Result<()> {
        // Verify version compatibility
        if schema.version != self.from_version {
            return Err(OtlError::InvalidOntology(format!(
                "Schema version mismatch: expected {}, got {}",
                self.from_version, schema.version
            )));
        }

        // Apply each change
        for change in &self.changes {
            self.apply_change(schema, change)?;
        }

        // Update schema version
        schema.version = self.to_version.clone();
        schema.updated_at = Utc::now();

        Ok(())
    }

    /// Apply a single change
    fn apply_change(&self, schema: &mut OntologySchema, change: &SchemaChange) -> Result<()> {
        match change {
            SchemaChange::AddEntityType { entity_type } => {
                schema.add_entity_type(entity_type.clone())?;
            }
            SchemaChange::RemoveEntityType { name } => {
                // Check if any relations reference this entity
                for (rel_name, rel_type) in &schema.relations {
                    if &rel_type.from_entity == name || &rel_type.to_entity == name {
                        return Err(OtlError::InvalidOntology(format!(
                            "Cannot remove entity type '{}': referenced by relation '{}'",
                            name, rel_name
                        )));
                    }
                }

                schema.entities.remove(name);
            }
            SchemaChange::ModifyEntityType { name, changes } => {
                let entity = schema.entities.get_mut(name).ok_or_else(|| {
                    OtlError::NotFound(format!("Entity type '{}' not found", name))
                })?;

                if let Some(label) = &changes.label {
                    entity.label = label.clone();
                }

                if let Some(desc) = &changes.description {
                    entity.description = Some(desc.clone());
                }

                for (prop_name, prop_def) in &changes.add_properties {
                    entity.properties.insert(prop_name.clone(), prop_def.clone());
                }

                for prop_name in &changes.remove_properties {
                    entity.properties.remove(prop_name);
                }

                for (prop_name, prop_def) in &changes.modify_properties {
                    entity.properties.insert(prop_name.clone(), prop_def.clone());
                }
            }
            SchemaChange::AddRelationType { relation_type } => {
                schema.add_relation_type(relation_type.clone())?;
            }
            SchemaChange::RemoveRelationType { name } => {
                schema.relations.remove(name);
            }
            SchemaChange::ModifyRelationType { name, changes } => {
                let relation = schema.relations.get_mut(name).ok_or_else(|| {
                    OtlError::NotFound(format!("Relation type '{}' not found", name))
                })?;

                if let Some(label) = &changes.label {
                    relation.label = label.clone();
                }

                if let Some(desc) = &changes.description {
                    relation.description = Some(desc.clone());
                }

                for (prop_name, prop_def) in &changes.add_properties {
                    relation.properties.insert(prop_name.clone(), prop_def.clone());
                }

                for prop_name in &changes.remove_properties {
                    relation.properties.remove(prop_name);
                }

                for (prop_name, prop_def) in &changes.modify_properties {
                    relation.properties.insert(prop_name.clone(), prop_def.clone());
                }

                if let Some(cardinality) = &changes.cardinality {
                    relation.cardinality = cardinality.clone();
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Schema Comparison and Compatibility
// ============================================================================

/// Result of schema comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    /// Added entity types
    pub added_entities: Vec<String>,

    /// Removed entity types
    pub removed_entities: Vec<String>,

    /// Modified entity types
    pub modified_entities: HashMap<String, EntityTypeDiff>,

    /// Added relation types
    pub added_relations: Vec<String>,

    /// Removed relation types
    pub removed_relations: Vec<String>,

    /// Modified relation types
    pub modified_relations: HashMap<String, RelationTypeDiff>,

    /// Is this change backward compatible?
    pub backward_compatible: bool,

    /// Compatibility issues
    pub compatibility_issues: Vec<String>,
}

/// Entity type differences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeDiff {
    /// Label changed
    pub label_changed: bool,

    /// Description changed
    pub description_changed: bool,

    /// Added properties
    pub added_properties: Vec<String>,

    /// Removed properties
    pub removed_properties: Vec<String>,

    /// Modified properties
    pub modified_properties: Vec<String>,
}

/// Relation type differences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationTypeDiff {
    /// Label changed
    pub label_changed: bool,

    /// Description changed
    pub description_changed: bool,

    /// From entity changed
    pub from_changed: bool,

    /// To entity changed
    pub to_changed: bool,

    /// Cardinality changed
    pub cardinality_changed: bool,

    /// Added properties
    pub added_properties: Vec<String>,

    /// Removed properties
    pub removed_properties: Vec<String>,

    /// Modified properties
    pub modified_properties: Vec<String>,
}

impl SchemaDiff {
    /// Compare two schemas
    pub fn compare(old: &OntologySchema, new: &OntologySchema) -> Self {
        let mut diff = Self {
            added_entities: Vec::new(),
            removed_entities: Vec::new(),
            modified_entities: HashMap::new(),
            added_relations: Vec::new(),
            removed_relations: Vec::new(),
            modified_relations: HashMap::new(),
            backward_compatible: true,
            compatibility_issues: Vec::new(),
        };

        // Compare entities
        let old_entity_names: HashSet<_> = old.entities.keys().collect();
        let new_entity_names: HashSet<_> = new.entities.keys().collect();

        // Added entities (backward compatible)
        for name in new_entity_names.difference(&old_entity_names) {
            diff.added_entities.push((*name).clone());
        }

        // Removed entities (NOT backward compatible)
        for name in old_entity_names.difference(&new_entity_names) {
            diff.removed_entities.push((*name).clone());
            diff.backward_compatible = false;
            diff.compatibility_issues.push(format!(
                "Removed entity type '{}' breaks backward compatibility",
                name
            ));
        }

        // Modified entities
        for name in old_entity_names.intersection(&new_entity_names) {
            let old_entity = &old.entities[*name];
            let new_entity = &new.entities[*name];

            if let Some(entity_diff) = Self::compare_entity_types(old_entity, new_entity) {
                // Check for breaking changes
                if !entity_diff.removed_properties.is_empty() {
                    diff.backward_compatible = false;
                    diff.compatibility_issues.push(format!(
                        "Removed properties from entity '{}': {:?}",
                        name, entity_diff.removed_properties
                    ));
                }

                diff.modified_entities.insert((*name).clone(), entity_diff);
            }
        }

        // Compare relations
        let old_relation_names: HashSet<_> = old.relations.keys().collect();
        let new_relation_names: HashSet<_> = new.relations.keys().collect();

        // Added relations (backward compatible)
        for name in new_relation_names.difference(&old_relation_names) {
            diff.added_relations.push((*name).clone());
        }

        // Removed relations (NOT backward compatible)
        for name in old_relation_names.difference(&new_relation_names) {
            diff.removed_relations.push((*name).clone());
            diff.backward_compatible = false;
            diff.compatibility_issues.push(format!(
                "Removed relation type '{}' breaks backward compatibility",
                name
            ));
        }

        // Modified relations
        for name in old_relation_names.intersection(&new_relation_names) {
            let old_relation = &old.relations[*name];
            let new_relation = &new.relations[*name];

            if let Some(relation_diff) = Self::compare_relation_types(old_relation, new_relation) {
                // Check for breaking changes
                if relation_diff.from_changed || relation_diff.to_changed {
                    diff.backward_compatible = false;
                    diff.compatibility_issues.push(format!(
                        "Changed entity types for relation '{}'",
                        name
                    ));
                }

                if !relation_diff.removed_properties.is_empty() {
                    diff.backward_compatible = false;
                    diff.compatibility_issues.push(format!(
                        "Removed properties from relation '{}': {:?}",
                        name, relation_diff.removed_properties
                    ));
                }

                diff.modified_relations.insert((*name).clone(), relation_diff);
            }
        }

        diff
    }

    /// Compare two entity types
    fn compare_entity_types(old: &EntityTypeDef, new: &EntityTypeDef) -> Option<EntityTypeDiff> {
        let label_changed = old.label != new.label;
        let description_changed = old.description != new.description;

        let old_props: HashSet<_> = old.properties.keys().collect();
        let new_props: HashSet<_> = new.properties.keys().collect();

        let added_properties: Vec<_> = new_props
            .difference(&old_props)
            .map(|s| (*s).clone())
            .collect();
        let removed_properties: Vec<_> = old_props
            .difference(&new_props)
            .map(|s| (*s).clone())
            .collect();

        let mut modified_properties = Vec::new();
        for name in old_props.intersection(&new_props) {
            if old.properties[*name] != new.properties[*name] {
                modified_properties.push((*name).clone());
            }
        }

        if label_changed
            || description_changed
            || !added_properties.is_empty()
            || !removed_properties.is_empty()
            || !modified_properties.is_empty()
        {
            Some(EntityTypeDiff {
                label_changed,
                description_changed,
                added_properties,
                removed_properties,
                modified_properties,
            })
        } else {
            None
        }
    }

    /// Compare two relation types
    fn compare_relation_types(
        old: &RelationTypeDef,
        new: &RelationTypeDef,
    ) -> Option<RelationTypeDiff> {
        let label_changed = old.label != new.label;
        let description_changed = old.description != new.description;
        let from_changed = old.from_entity != new.from_entity;
        let to_changed = old.to_entity != new.to_entity;
        let cardinality_changed = old.cardinality != new.cardinality;

        let old_props: HashSet<_> = old.properties.keys().collect();
        let new_props: HashSet<_> = new.properties.keys().collect();

        let added_properties: Vec<_> = new_props
            .difference(&old_props)
            .map(|s| (*s).clone())
            .collect();
        let removed_properties: Vec<_> = old_props
            .difference(&new_props)
            .map(|s| (*s).clone())
            .collect();

        let mut modified_properties = Vec::new();
        for name in old_props.intersection(&new_props) {
            if old.properties[*name] != new.properties[*name] {
                modified_properties.push((*name).clone());
            }
        }

        if label_changed
            || description_changed
            || from_changed
            || to_changed
            || cardinality_changed
            || !added_properties.is_empty()
            || !removed_properties.is_empty()
            || !modified_properties.is_empty()
        {
            Some(RelationTypeDiff {
                label_changed,
                description_changed,
                from_changed,
                to_changed,
                cardinality_changed,
                added_properties,
                removed_properties,
                modified_properties,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_schema() {
        let schema = OntologySchema::new("HR Schema", semver::Version::new(1, 0, 0));
        assert_eq!(schema.name, "HR Schema");
        assert_eq!(schema.version, semver::Version::new(1, 0, 0));
    }

    #[test]
    fn test_add_entity_type() {
        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));

        let entity = EntityTypeDef::new("Person", "个人").with_property(
            PropertyDef::new("name", PropertyType::String).required(),
        );

        assert!(schema.add_entity_type(entity).is_ok());
        assert!(schema.entities.contains_key("Person"));
    }

    #[test]
    fn test_add_duplicate_entity_type() {
        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));

        let entity1 = EntityTypeDef::new("Person", "个人");
        let entity2 = EntityTypeDef::new("Person", "Person");

        assert!(schema.add_entity_type(entity1).is_ok());
        assert!(schema.add_entity_type(entity2).is_err());
    }

    #[test]
    fn test_add_relation_type() {
        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));

        schema
            .add_entity_type(EntityTypeDef::new("Person", "个人"))
            .unwrap();
        schema
            .add_entity_type(EntityTypeDef::new("Organization", "组织"))
            .unwrap();

        let relation = RelationTypeDef::new("WORKS_AT", "工作于", "Person", "Organization");

        assert!(schema.add_relation_type(relation).is_ok());
        assert!(schema.relations.contains_key("WORKS_AT"));
    }

    #[test]
    fn test_relation_invalid_entity_types() {
        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));

        let relation = RelationTypeDef::new("WORKS_AT", "工作于", "Person", "Organization");

        // Should fail because Person and Organization don't exist
        assert!(schema.add_relation_type(relation).is_err());
    }

    #[test]
    fn test_schema_validation() {
        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));

        schema
            .add_entity_type(
                EntityTypeDef::new("Person", "个人").with_property(
                    PropertyDef::new("email", PropertyType::String)
                        .required(),
                ),
            )
            .unwrap();

        assert!(schema.validate().is_ok());
    }

    #[test]
    fn test_schema_diff_backward_compatible() {
        let mut old_schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));
        old_schema
            .add_entity_type(EntityTypeDef::new("Person", "个人"))
            .unwrap();

        let mut new_schema = old_schema.clone();
        new_schema.version = semver::Version::new(1, 1, 0);
        new_schema
            .add_entity_type(EntityTypeDef::new("Organization", "组织"))
            .unwrap();

        let diff = SchemaDiff::compare(&old_schema, &new_schema);
        assert!(diff.backward_compatible);
        assert_eq!(diff.added_entities.len(), 1);
    }

    #[test]
    fn test_schema_diff_not_backward_compatible() {
        let mut old_schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));
        old_schema
            .add_entity_type(EntityTypeDef::new("Person", "个人"))
            .unwrap();
        old_schema
            .add_entity_type(EntityTypeDef::new("Organization", "组织"))
            .unwrap();

        let mut new_schema = old_schema.clone();
        new_schema.version = semver::Version::new(2, 0, 0);
        new_schema.entities.remove("Organization");

        let diff = SchemaDiff::compare(&old_schema, &new_schema);
        assert!(!diff.backward_compatible);
        assert_eq!(diff.removed_entities.len(), 1);
    }
}
