# Contributing to OTL

Thank you for your interest in contributing to OTL! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/hephaex/OTL/issues)
2. If not, create a new issue with:
   - Clear, descriptive title
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Rust version, etc.)

### Suggesting Features

1. Check existing issues for similar suggestions
2. Create a new issue with:
   - Clear description of the feature
   - Use cases and benefits
   - Possible implementation approach

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests and linting:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test --all-features
   ```
5. Commit with clear messages: `git commit -m "feat: add X feature"`
6. Push to your fork: `git push origin feature/my-feature`
7. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose
- Git

### Local Development

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/OTL.git
cd OTL

# Start infrastructure
docker compose up -d

# Copy environment file
cp .env.example .env
# Edit .env with your settings

# Build and test
cargo build
cargo test
```

### Running the API Server

```bash
# Development mode
cargo run -p otl-api

# With hot reload (install cargo-watch first)
cargo watch -x 'run -p otl-api'
```

## Code Style

### Rust Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Address all `clippy` warnings
- Write doc comments for public APIs

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Examples:
```
feat(parser): add PPTX support
fix(rag): correct ACL filtering logic
docs(readme): update installation instructions
```

### Attribution Rules

**Important**: All contributions must follow these attribution rules:

```
✅ DO:
- Use "Author: hephaex@gmail.com" in all files
- Use "Designed by author" or "Implemented by author"
- Reference issue numbers and PR numbers

❌ DO NOT:
- Mention third-party code generation tools in commits
- Include tool-generated markers in any text
- Reference external assistants in comments or documentation
```

### Testing

- Write unit tests for new functionality
- Aim for 80%+ test coverage
- Integration tests go in `tests/` directory

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test test_acl_filtering

# Run with output
cargo test -- --nocapture
```

## Project Structure

```
crates/
├── otl-core/      # Domain models (start here for new types)
├── otl-parser/    # Document parsing
├── otl-ocr/       # OCR integration
├── otl-graph/     # Graph database
├── otl-vector/    # Vector database
├── otl-extractor/ # NER/RE
├── otl-rag/       # RAG pipeline
├── otl-api/       # HTTP server
└── otl-cli/       # CLI tool
```

## Documentation

- Update docs when changing public APIs
- Add examples to doc comments
- Keep README.md current

```rust
/// Extracts text from a PDF document.
///
/// # Arguments
///
/// * `path` - Path to the PDF file
///
/// # Returns
///
/// A `ParsedDocument` containing the extracted text and metadata.
///
/// # Example
///
/// ```no_run
/// use otl_parser::PdfParser;
/// let doc = parser.parse("document.pdf")?;
/// ```
pub fn parse(&self, path: &Path) -> Result<ParsedDocument>
```

## Review Process

1. All PRs require at least one review
2. CI must pass (format, lint, test)
3. Breaking changes need discussion first
4. Keep PRs focused and reasonably sized

## Getting Help

- Open an issue for questions
- Tag maintainers for urgent matters
- Check existing issues and docs first

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.

---

Thank you for contributing to OTL! 🦀
