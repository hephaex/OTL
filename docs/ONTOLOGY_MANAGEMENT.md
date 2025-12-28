# Ontology Management & Schema Evolution

This document describes the ontology management system in OTL, which provides comprehensive lifecycle management for knowledge graph schemas.

## Overview

The ontology management system supports:
- **Schema Definition**: Define entity and relation types with properties
- **Versioning**: Semantic versioning (semver) for schema evolution
- **Validation**: Comprehensive schema validation and consistency checking
- **Migration**: Tools for migrating between schema versions
- **Compatibility**: Backward compatibility checking for safe schema updates
- **Import/Export**: Multiple formats (JSON, YAML, OWL, RDF)

## Architecture

### Core Components

1. **`otl-graph/src/ontology.rs`**: Core schema types and validation
2. **`otl-graph/src/ontology_io.rs`**: Import/export functionality
3. **`otl-graph/src/ontology_repository.rs`**: Database persistence
4. **`otl-api/src/handlers/ontology.rs`**: REST API endpoints

### Data Model

```rust
OntologySchema {
    id: Uuid,
    version: semver::Version,
    name: String,
    entities: HashMap<String, EntityTypeDef>,
    relations: HashMap<String, RelationTypeDef>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

EntityTypeDef {
    name: String,
    label: String,
    properties: HashMap<String, PropertyDef>,
    parent: Option<String>,  // For inheritance
}

RelationTypeDef {
    name: String,
    label: String,
    from_entity: String,
    to_entity: String,
    cardinality: RelationCardinality,
    properties: HashMap<String, PropertyDef>,
}

PropertyDef {
    name: String,
    data_type: PropertyType,
    required: bool,
    format: Option<String>,
    pattern: Option<String>,
    minimum: Option<f64>,
    maximum: Option<f64>,
}
```

## API Endpoints

### Get Current Schema

```http
GET /api/v1/ontology/schema
```

Returns the latest ontology schema version.

### Get Specific Version

```http
GET /api/v1/ontology/schema/{version}
```

Retrieve a specific schema version (e.g., `1.0.0`).

### List All Versions

```http
GET /api/v1/ontology/schema/versions
```

Returns list of all schema versions.

### Create New Schema

```http
POST /api/v1/ontology/schema
```

Create a new schema version (admin only).

**Request Body:**
```json
{
  "name": "HR Ontology",
  "version": "1.0.0",
  "description": "Human Resources knowledge graph schema",
  "entities": [
    {
      "name": "Employee",
      "label": "직원",
      "description": "An employee in the organization",
      "properties": [
        {
          "name": "name",
          "type": "string",
          "required": true
        },
        {
          "name": "email",
          "type": "string",
          "required": true,
          "format": "email"
        },
        {
          "name": "hireDate",
          "type": "date",
          "required": true
        }
      ]
    },
    {
      "name": "Department",
      "label": "부서",
      "properties": [
        {
          "name": "name",
          "type": "string",
          "required": true
        },
        {
          "name": "code",
          "type": "string",
          "required": true,
          "pattern": "^DEPT-[0-9]{4}$"
        }
      ]
    }
  ],
  "relations": [
    {
      "name": "WORKS_IN",
      "label": "소속",
      "from_entity": "Employee",
      "to_entity": "Department",
      "cardinality": "many_to_one"
    }
  ]
}
```

### Compare Schema Versions

```http
GET /api/v1/ontology/schema/diff?from=1.0.0&to=1.1.0
```

Returns detailed comparison between two versions including:
- Added/removed entities and relations
- Modified properties
- Backward compatibility status
- List of breaking changes

**Response:**
```json
{
  "added_entities": ["Organization"],
  "removed_entities": [],
  "modified_entities": {
    "Employee": {
      "label_changed": false,
      "added_properties": ["phoneNumber"],
      "removed_properties": [],
      "modified_properties": []
    }
  },
  "added_relations": ["MANAGES"],
  "removed_relations": [],
  "backward_compatible": true,
  "compatibility_issues": []
}
```

### Export Schema

```http
GET /api/v1/ontology/schema/export?format=json&version=1.0.0
```

Export schema in various formats:
- `json`: Native JSON format
- `yaml`: Human-readable YAML
- `owl`: Web Ontology Language (RDF/XML)
- `rdf` or `turtle`: RDF Turtle format

### Import Schema

```http
POST /api/v1/ontology/schema/import?format=json
Content-Type: application/json
```

Import schema from JSON or YAML file (admin only).

### Validate Schema

```http
POST /api/v1/ontology/schema/validate
```

Validate a schema definition without storing it.

## Schema Versioning

### Semantic Versioning

Schemas follow semantic versioning (semver):
- **MAJOR**: Incompatible changes (removed entities/properties)
- **MINOR**: Backward-compatible additions (new entities/properties)
- **PATCH**: Bug fixes and documentation updates

### Version Examples

```
1.0.0 → 1.1.0: Added new entity type (backward compatible)
1.1.0 → 2.0.0: Removed entity type (breaking change)
1.0.0 → 1.0.1: Fixed property validation (patch)
```

## Schema Migration

### Creating a Migration

```rust
use otl_graph::{SchemaMigration, SchemaChange, EntityTypeDef, PropertyDef, PropertyType};

let migration = SchemaMigration::new(
    semver::Version::new(1, 0, 0),
    semver::Version::new(1, 1, 0),
    "Add phoneNumber to Employee"
)
.add_change(SchemaChange::ModifyEntityType {
    name: "Employee".to_string(),
    changes: EntityTypeChanges {
        add_properties: HashMap::from([(
            "phoneNumber".to_string(),
            PropertyDef::new("phoneNumber", PropertyType::String)
        )]),
        ..Default::default()
    }
});

// Apply migration
migration.apply(&mut schema)?;
```

### Backward Compatibility

The system automatically checks for backward compatibility:

**Compatible Changes:**
- Adding new entity types
- Adding new relation types
- Adding optional properties
- Adding new constraints (if they don't affect existing data)

**Incompatible Changes:**
- Removing entity types
- Removing relation types
- Removing properties
- Changing property types
- Making optional properties required
- Changing relation cardinality

## Import/Export Formats

### JSON Format

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "name": "HR Ontology",
  "description": "Human Resources knowledge graph",
  "entities": {
    "Employee": {
      "name": "Employee",
      "label": "직원",
      "properties": {
        "name": {
          "name": "name",
          "type": "string",
          "required": true
        }
      }
    }
  },
  "relations": {
    "WORKS_IN": {
      "name": "WORKS_IN",
      "label": "소속",
      "from_entity": "Employee",
      "to_entity": "Department",
      "cardinality": "many_to_one"
    }
  }
}
```

### YAML Format

```yaml
version: "1.0.0"
name: HR Ontology
description: Human Resources knowledge graph
entities:
  Employee:
    name: Employee
    label: 직원
    properties:
      name:
        name: name
        type: string
        required: true
  Department:
    name: Department
    label: 부서
    properties:
      name:
        name: name
        type: string
        required: true
relations:
  WORKS_IN:
    name: WORKS_IN
    label: 소속
    from_entity: Employee
    to_entity: Department
    cardinality: many_to_one
```

### OWL Format

Exports to Web Ontology Language (OWL 2) in RDF/XML format:

```xml
<?xml version="1.0"?>
<rdf:RDF xmlns="http://otl.ai/ontology/HR_Ontology#"
     xml:base="http://otl.ai/ontology/HR_Ontology"
     xmlns:owl="http://www.w3.org/2002/07/owl#"
     xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
     xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
     xmlns:xsd="http://www.w3.org/2001/XMLSchema#">

    <owl:Ontology rdf:about="http://otl.ai/ontology/HR_Ontology">
        <owl:versionInfo>1.0.0</owl:versionInfo>
    </owl:Ontology>

    <owl:Class rdf:about="#Employee">
        <rdfs:label>직원</rdfs:label>
    </owl:Class>

    <owl:ObjectProperty rdf:about="#WORKS_IN">
        <rdfs:label>소속</rdfs:label>
        <rdfs:domain rdf:resource="#Employee"/>
        <rdfs:range rdf:resource="#Department"/>
    </owl:ObjectProperty>
</rdf:RDF>
```

### RDF Turtle Format

```turtle
@prefix : <http://otl.ai/ontology/HR_Ontology#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<http://otl.ai/ontology/HR_Ontology> rdf:type owl:Ontology ;
    owl:versionInfo "1.0.0" .

:Employee rdf:type owl:Class ;
    rdfs:label "직원" .

:Department rdf:type owl:Class ;
    rdfs:label "부서" .

:WORKS_IN rdf:type owl:ObjectProperty ;
    rdfs:label "소속" ;
    rdfs:domain :Employee ;
    rdfs:range :Department .
```

## Property Types

Supported property types:
- `string`: Text values
- `integer`: Whole numbers
- `float`: Decimal numbers
- `boolean`: True/false values
- `datetime`: Date and time (ISO 8601)
- `date`: Date only
- `time`: Time only
- `duration`: Time duration
- `array:<type>`: Array of specific type (e.g., `array:string`)
- `reference:<EntityType>`: Reference to another entity
- `object`: Nested object/map
- `any`: Any type

## Property Validation

Properties support various validation constraints:

```json
{
  "name": "email",
  "type": "string",
  "required": true,
  "format": "email",
  "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
}
```

```json
{
  "name": "age",
  "type": "integer",
  "minimum": 0,
  "maximum": 150
}
```

```json
{
  "name": "status",
  "type": "string",
  "enum_values": ["active", "inactive", "pending"]
}
```

## Best Practices

### 1. Version Incrementing

- Use PATCH for documentation or non-functional changes
- Use MINOR for backward-compatible additions
- Use MAJOR for breaking changes

### 2. Schema Design

- Keep entity types focused and cohesive
- Use descriptive names and labels
- Add descriptions for complex properties
- Define validation constraints upfront

### 3. Migration Strategy

- Test migrations on non-production data first
- Create migrations incrementally
- Document breaking changes clearly
- Provide data migration scripts for breaking changes

### 4. Backward Compatibility

- Avoid removing entities or properties when possible
- Mark deprecated items instead of removing them
- Provide migration paths for breaking changes

### 5. Documentation

- Document schema changes in migration descriptions
- Maintain a CHANGELOG for schema versions
- Include examples for complex property types

## Example: Complete Workflow

### 1. Create Initial Schema (v1.0.0)

```bash
curl -X POST http://localhost:8080/api/v1/ontology/schema \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @schema-1.0.0.json
```

### 2. Export for Review

```bash
curl http://localhost:8080/api/v1/ontology/schema/export?format=yaml \
  -H "Authorization: Bearer $TOKEN" \
  > schema-1.0.0.yaml
```

### 3. Create New Version (v1.1.0)

Add new entity type and property:

```bash
curl -X POST http://localhost:8080/api/v1/ontology/schema \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @schema-1.1.0.json
```

### 4. Check Compatibility

```bash
curl "http://localhost:8080/api/v1/ontology/schema/diff?from=1.0.0&to=1.1.0" \
  -H "Authorization: Bearer $TOKEN"
```

### 5. Export as OWL

```bash
curl "http://localhost:8080/api/v1/ontology/schema/export?format=owl&version=1.1.0" \
  -H "Authorization: Bearer $TOKEN" \
  > schema-1.1.0.owl
```

## Integration with Graph Database

Ontology schemas are stored in SurrealDB alongside the knowledge graph data. The repository layer provides:

- Schema versioning and history
- Migration tracking
- Schema-data consistency checks

## Security

- Schema creation/modification requires admin role
- Schema viewing requires authentication
- All operations are logged for audit

## Future Enhancements

- [ ] Automatic schema inference from data
- [ ] Schema validation for graph data
- [ ] Visual schema editor
- [ ] Automated data migration for schema changes
- [ ] Schema diff visualization
- [ ] Integration with external ontology registries
- [ ] Support for SHACL constraints
- [ ] JSON-LD context generation

## References

- [Semantic Versioning](https://semver.org/)
- [OWL 2 Web Ontology Language](https://www.w3.org/TR/owl2-overview/)
- [RDF 1.1 Turtle](https://www.w3.org/TR/turtle/)
- [JSON Schema](https://json-schema.org/)
