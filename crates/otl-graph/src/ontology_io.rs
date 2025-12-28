//! Ontology import/export functionality
//!
//! Supports multiple formats:
//! - JSON (native format)
//! - YAML (human-readable)
//! - OWL (Web Ontology Language)
//! - RDF (Resource Description Framework)
//!
//! Author: hephaex@gmail.com

use crate::ontology::{OntologySchema, PropertyType};
use otl_core::{OtlError, Result};
use std::io::{Read, Write};

// ============================================================================
// JSON Import/Export
// ============================================================================

/// Export schema to JSON
pub fn export_json(schema: &OntologySchema) -> Result<String> {
    serde_json::to_string_pretty(schema)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("JSON serialization failed: {}", e)))
}

/// Export schema to JSON writer
pub fn export_json_writer<W: Write>(schema: &OntologySchema, writer: W) -> Result<()> {
    serde_json::to_writer_pretty(writer, schema)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("JSON serialization failed: {}", e)))
}

/// Import schema from JSON string
pub fn import_json(json: &str) -> Result<OntologySchema> {
    let schema: OntologySchema = serde_json::from_str(json)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("JSON deserialization failed: {}", e)))?;

    schema.validate()?;
    Ok(schema)
}

/// Import schema from JSON reader
pub fn import_json_reader<R: Read>(reader: R) -> Result<OntologySchema> {
    let schema: OntologySchema = serde_json::from_reader(reader)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("JSON deserialization failed: {}", e)))?;

    schema.validate()?;
    Ok(schema)
}

// ============================================================================
// YAML Import/Export
// ============================================================================

/// Export schema to YAML
pub fn export_yaml(schema: &OntologySchema) -> Result<String> {
    serde_yaml::to_string(schema)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("YAML serialization failed: {}", e)))
}

/// Export schema to YAML writer
pub fn export_yaml_writer<W: Write>(schema: &OntologySchema, writer: W) -> Result<()> {
    serde_yaml::to_writer(writer, schema)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("YAML serialization failed: {}", e)))
}

/// Import schema from YAML string
pub fn import_yaml(yaml: &str) -> Result<OntologySchema> {
    let schema: OntologySchema = serde_yaml::from_str(yaml)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("YAML deserialization failed: {}", e)))?;

    schema.validate()?;
    Ok(schema)
}

/// Import schema from YAML reader
pub fn import_yaml_reader<R: Read>(reader: R) -> Result<OntologySchema> {
    let schema: OntologySchema = serde_yaml::from_reader(reader)
        .map_err(|e| OtlError::Other(anyhow::anyhow!("YAML deserialization failed: {}", e)))?;

    schema.validate()?;
    Ok(schema)
}

// ============================================================================
// OWL Export (Web Ontology Language)
// ============================================================================

/// Export schema to OWL 2 (RDF/XML format)
pub fn export_owl(schema: &OntologySchema) -> Result<String> {
    let mut owl = String::new();

    // XML header
    owl.push_str(r#"<?xml version="1.0"?>
<!DOCTYPE rdf:RDF [
    <!ENTITY owl "http://www.w3.org/2002/07/owl#" >
    <!ENTITY xsd "http://www.w3.org/2001/XMLSchema#" >
    <!ENTITY rdfs "http://www.w3.org/2000/01/rdf-schema#" >
    <!ENTITY rdf "http://www.w3.org/1999/02/22-rdf-syntax-ns#" >
]>
"#);

    // Ontology header
    let base_uri = format!("http://otl.ai/ontology/{}", schema.name.replace(' ', "_"));
    owl.push_str(&format!(
        r#"<rdf:RDF xmlns="{base_uri}#"
     xml:base="{base_uri}"
     xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
     xmlns:owl="http://www.w3.org/2002/07/owl#"
     xmlns:xsd="http://www.w3.org/2001/XMLSchema#"
     xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">

    <owl:Ontology rdf:about="{base_uri}">
"#
    ));

    if let Some(desc) = &schema.description {
        owl.push_str(&format!(
            "        <rdfs:comment>{}</rdfs:comment>\n",
            escape_xml(desc)
        ));
    }

    owl.push_str(&format!(
        "        <owl:versionInfo>{}</owl:versionInfo>\n",
        schema.version
    ));
    owl.push_str("    </owl:Ontology>\n\n");

    // Export entity types as OWL classes
    for (_, entity) in &schema.entities {
        owl.push_str(&format!(
            "    <owl:Class rdf:about=\"#{}\">\n",
            escape_xml(&entity.name)
        ));
        owl.push_str(&format!(
            "        <rdfs:label>{}</rdfs:label>\n",
            escape_xml(&entity.label)
        ));

        if let Some(desc) = &entity.description {
            owl.push_str(&format!(
                "        <rdfs:comment>{}</rdfs:comment>\n",
                escape_xml(desc)
            ));
        }

        if let Some(parent) = &entity.parent {
            owl.push_str(&format!(
                "        <rdfs:subClassOf rdf:resource=\"#{}\"/>\n",
                escape_xml(parent)
            ));
        }

        owl.push_str("    </owl:Class>\n\n");

        // Export properties as data properties or object properties
        for (_, prop) in &entity.properties {
            let prop_type = match &prop.data_type {
                PropertyType::Reference(_) => "owl:ObjectProperty",
                _ => "owl:DatatypeProperty",
            };

            owl.push_str(&format!(
                "    <{} rdf:about=\"#{}\">\n",
                prop_type,
                escape_xml(&prop.name)
            ));
            owl.push_str(&format!(
                "        <rdfs:domain rdf:resource=\"#{}\"/>\n",
                escape_xml(&entity.name)
            ));

            let range = match &prop.data_type {
                PropertyType::String => "xsd:string",
                PropertyType::Integer => "xsd:integer",
                PropertyType::Float => "xsd:double",
                PropertyType::Boolean => "xsd:boolean",
                PropertyType::DateTime => "xsd:dateTime",
                PropertyType::Date => "xsd:date",
                PropertyType::Time => "xsd:time",
                PropertyType::Reference(ref_type) => ref_type,
                _ => "xsd:string",
            };

            owl.push_str(&format!(
                "        <rdfs:range rdf:resource=\"{}\"/>\n",
                if range.starts_with("xsd:") {
                    range.to_string()
                } else {
                    format!("#{}", escape_xml(range))
                }
            ));

            if let Some(desc) = &prop.description {
                owl.push_str(&format!(
                    "        <rdfs:comment>{}</rdfs:comment>\n",
                    escape_xml(desc)
                ));
            }

            owl.push_str(&format!("    </{}>\n\n", prop_type));
        }
    }

    // Export relation types as object properties
    for (_, relation) in &schema.relations {
        owl.push_str(&format!(
            "    <owl:ObjectProperty rdf:about=\"#{}\">\n",
            escape_xml(&relation.name)
        ));
        owl.push_str(&format!(
            "        <rdfs:label>{}</rdfs:label>\n",
            escape_xml(&relation.label)
        ));

        if let Some(desc) = &relation.description {
            owl.push_str(&format!(
                "        <rdfs:comment>{}</rdfs:comment>\n",
                escape_xml(desc)
            ));
        }

        owl.push_str(&format!(
            "        <rdfs:domain rdf:resource=\"#{}\"/>\n",
            escape_xml(&relation.from_entity)
        ));
        owl.push_str(&format!(
            "        <rdfs:range rdf:resource=\"#{}\"/>\n",
            escape_xml(&relation.to_entity)
        ));

        owl.push_str("    </owl:ObjectProperty>\n\n");
    }

    owl.push_str("</rdf:RDF>\n");

    Ok(owl)
}

// ============================================================================
// RDF Export (Turtle format)
// ============================================================================

/// Export schema to RDF Turtle format
pub fn export_rdf_turtle(schema: &OntologySchema) -> Result<String> {
    let mut ttl = String::new();

    // Prefixes
    let base_uri = format!("http://otl.ai/ontology/{}", schema.name.replace(' ', "_"));
    ttl.push_str(&format!("@prefix : <{base_uri}#> .\n"));
    ttl.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
    ttl.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
    ttl.push_str("@prefix xml: <http://www.w3.org/XML/1998/namespace> .\n");
    ttl.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    ttl.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    ttl.push_str(&format!("@base <{base_uri}> .\n\n"));

    // Ontology declaration
    ttl.push_str(&format!("<{base_uri}> rdf:type owl:Ontology ;\n"));
    if let Some(desc) = &schema.description {
        ttl.push_str(&format!("    rdfs:comment \"{}\" ;\n", escape_turtle(desc)));
    }
    ttl.push_str(&format!("    owl:versionInfo \"{}\" .\n\n", schema.version));

    // Export entity types as classes
    for (_, entity) in &schema.entities {
        ttl.push_str(&format!(":{}  rdf:type owl:Class ;\n", entity.name));
        ttl.push_str(&format!("    rdfs:label \"{}\" ", escape_turtle(&entity.label)));

        if let Some(desc) = &entity.description {
            ttl.push_str(";\n");
            ttl.push_str(&format!(
                "    rdfs:comment \"{}\" ",
                escape_turtle(desc)
            ));
        }

        if let Some(parent) = &entity.parent {
            ttl.push_str(";\n");
            ttl.push_str(&format!("    rdfs:subClassOf :{} ", parent));
        }

        ttl.push_str(".\n\n");
    }

    // Export relation types as object properties
    for (_, relation) in &schema.relations {
        ttl.push_str(&format!(
            ":{} rdf:type owl:ObjectProperty ;\n",
            relation.name
        ));
        ttl.push_str(&format!(
            "    rdfs:label \"{}\" ;\n",
            escape_turtle(&relation.label)
        ));
        ttl.push_str(&format!("    rdfs:domain :{} ;\n", relation.from_entity));
        ttl.push_str(&format!("    rdfs:range :{} ", relation.to_entity));

        if let Some(desc) = &relation.description {
            ttl.push_str(";\n");
            ttl.push_str(&format!(
                "    rdfs:comment \"{}\" ",
                escape_turtle(desc)
            ));
        }

        ttl.push_str(".\n\n");
    }

    Ok(ttl)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape Turtle special characters
fn escape_turtle(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{EntityTypeDef, PropertyDef, PropertyType, RelationTypeDef};

    fn create_test_schema() -> OntologySchema {
        let mut schema = OntologySchema::new("Test Schema", semver::Version::new(1, 0, 0));
        schema.description = Some("A test schema".to_string());

        let person_entity = EntityTypeDef::new("Person", "个人")
            .with_property(PropertyDef::new("name", PropertyType::String).required())
            .with_property(PropertyDef::new("age", PropertyType::Integer));

        let org_entity = EntityTypeDef::new("Organization", "组织")
            .with_property(PropertyDef::new("name", PropertyType::String).required());

        schema.add_entity_type(person_entity).unwrap();
        schema.add_entity_type(org_entity).unwrap();

        let works_at = RelationTypeDef::new("WORKS_AT", "工作于", "Person", "Organization");
        schema.add_relation_type(works_at).unwrap();

        schema
    }

    #[test]
    fn test_json_export_import() {
        let schema = create_test_schema();

        let json = export_json(&schema).unwrap();
        assert!(json.contains("Test Schema"));

        let imported = import_json(&json).unwrap();
        assert_eq!(imported.name, schema.name);
        assert_eq!(imported.version, schema.version);
        assert_eq!(imported.entities.len(), schema.entities.len());
    }

    #[test]
    fn test_yaml_export_import() {
        let schema = create_test_schema();

        let yaml = export_yaml(&schema).unwrap();
        assert!(yaml.contains("Test Schema"));

        let imported = import_yaml(&yaml).unwrap();
        assert_eq!(imported.name, schema.name);
        assert_eq!(imported.version, schema.version);
    }

    #[test]
    fn test_owl_export() {
        let schema = create_test_schema();

        let owl = export_owl(&schema).unwrap();
        assert!(owl.contains("<?xml version=\"1.0\"?>"));
        assert!(owl.contains("owl:Ontology"));
        assert!(owl.contains("owl:Class"));
        assert!(owl.contains("Person"));
        assert!(owl.contains("Organization"));
    }

    #[test]
    fn test_rdf_turtle_export() {
        let schema = create_test_schema();

        let ttl = export_rdf_turtle(&schema).unwrap();
        assert!(ttl.contains("@prefix"));
        assert!(ttl.contains("owl:Ontology"));
        assert!(ttl.contains(":Person"));
        assert!(ttl.contains(":Organization"));
        assert!(ttl.contains(":WORKS_AT"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a<b>c&d\"e'f"), "a&lt;b&gt;c&amp;d&quot;e&apos;f");
    }

    #[test]
    fn test_escape_turtle() {
        assert_eq!(
            escape_turtle("line1\nline2\ttab"),
            "line1\\nline2\\ttab"
        );
    }
}
