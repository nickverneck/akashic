# Akashic - File Ingestion Service

A robust file ingestion service built with Rust and Loco that processes various document formats (PDF, DOC, TXT, MD, EPUB) and ingests them into Vector (ChromaDB) and Graph (FalkorDB, Neo4j) databases.

## Features

- **Multiple File Format Support**: PDF, DOC/DOCX, TXT, MD, EPUB
- **OCR Fallback**: Automatic OCR processing for unreadable PDFs using Tesseract
- **Dual Database Support**:
  - **Vector**: ChromaDB for semantic search
  - **Graph**: Neo4j, FalkorDB, or Graphiti for knowledge graphs
- **Web API**: RESTful API for file uploads and status tracking
- **CLI Interface**: Command-line tool with stdin support for automation
- **Background Processing**: Async job queue for handling multiple files
- **Progress Tracking**: Real-time status and progress monitoring

## Installation

### Prerequisites

- Rust 1.70+
- SQLite (or PostgreSQL)
- Tesseract OCR (optional, for OCR fallback)
- ChromaDB instance (optional)
- Neo4j/FalkorDB instance (optional)

### Build

```bash
cargo build --release
```

### Optional: Enable Graphiti Support

```bash
cargo build --release --features graphiti
```

## Configuration

Set environment variables for database connections:

```bash
# ChromaDB
export CHROMA_URL="http://localhost:8000"

# Neo4j
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="password"

# FalkorDB
export FALKORDB_URI="redis://localhost:6379"
export FALKORDB_GRAPH="akashic"
```

## Usage

### Web Server

Start the web server:

```bash
cargo loco start
```

#### API Endpoints

**Upload File**
```bash
curl -X POST http://localhost:5150/api/ingest/file \
  -F "file=@document.pdf" \
  -F "target=both" \
  -F "graph_db=neo4j"
```

Parameters:
- `file`: The file to ingest
- `target`: `vector`, `graph`, or `both`
- `graph_db`: `neo4j`, `falkordb`, or `graphiti` (optional, required if target includes graph)

**Ingest Text**
```bash
curl -X POST http://localhost:5150/api/ingest/text \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Your text content here",
    "target": "both",
    "graph_db": "neo4j"
  }'
```

**Check Status**
```bash
curl http://localhost:5150/api/ingest/status/1
```

Response:
```json
{
  "document_id": 1,
  "filename": "document.pdf",
  "status": "completed",
  "progress": 100,
  "error_message": null
}
```

### CLI

**Ingest a file:**
```bash
cargo loco task ingest --file samples/ideas.md --target both --graph-db neo4j
```

**Ingest from stdin:**
```bash
cat samples/ideas.md | cargo loco task ingest --stdin --target vector
```

**Pipe from automation:**
```bash
echo "Important text to ingest" | cargo loco task ingest --stdin --target both --graph-db falkordb
```

## Architecture

### Components

1. **Extractors** (`src/ingestion/extractors.rs`)
   - PDF: Native extraction with OCR fallback
   - Markdown/Text: Direct file reading
   - EPUB: Chapter-by-chapter extraction
   - DOC/DOCX: OCR-based (can be extended)

2. **Stores** (`src/ingestion/stores.rs`)
   - ChromaDB: HTTP API for vector storage
   - Neo4j: Cypher queries for graph storage
   - FalkorDB: Redis protocol for graph storage
   - Graphiti: PyO3 integration (optional)

3. **Pipeline** (`src/ingestion/pipeline.rs`)
   - Orchestrates extraction and ingestion
   - Updates document status and progress
   - Handles errors gracefully

4. **API Controllers** (`src/controllers/ingest.rs`)
   - File upload endpoint
   - Text ingestion endpoint
   - Status tracking endpoint

5. **Background Workers** (`src/workers/ingest.rs`)
   - Async processing of ingestion jobs
   - Configurable database connections

6. **CLI Tasks** (`src/tasks/ingest.rs`)
   - Direct file ingestion
   - Stdin support for piping

## Database Schema

### Documents Table

| Column | Type | Description |
|--------|------|-------------|
| id | Integer | Primary key |
| filename | String | Original filename or "stdin" |
| status | String | `queued`, `processing`, `completed`, `failed` |
| ingestion_type | String | `Vector`, `Graph`, or `Both` |
| graph_db | String | Graph database type (if applicable) |
| progress | Integer | 0-100 percentage |
| metadata | Text | JSON metadata |
| error_message | Text | Error details (if failed) |
| created_at | Timestamp | Creation time |
| updated_at | Timestamp | Last update time |

## Development

### Run Migrations

```bash
cargo loco db migrate
```

### Run Tests

```bash
cargo test
```

### Generate New Components

```bash
# Generate a new controller
cargo loco generate controller --api mycontroller

# Generate a new model
cargo loco generate model mymodel field1:string field2:int

# Generate a new worker
cargo loco generate worker myworker
```

## Extending

### Adding New File Formats

1. Create a new extractor in `src/ingestion/extractors.rs`
2. Implement the `Extractor` trait
3. Add to the `get_extractor` factory function

### Adding New Graph Databases

1. Create a new store in `src/ingestion/stores.rs`
2. Implement the `GraphStore` trait
3. Add to `GraphDbType` enum and `create_graph_store` factory

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.
