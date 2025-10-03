# rust-gpt

A Rust implementation for training / running GPT models from scratch.

## Data Preprocessing

### Input Data Format

This pipeline works with Wikipedia data from the [Stanford Oval Wikipedia dataset](https://huggingface.co/datasets/stanford-oval/wikipedia). Stanford Oval has already handled the challenging task of cleaning Wikipedia text, which requires numerous hand-crafted rules.

<details>
<summary>
    Cleaning Wikipedia dumps is hard.
</summary>

There's no direct converter from Wikitext (Wikipedia's markdown format used in their data dumps) to markdown. I tried creating a python script using [Aaron Halfaker's](https://github.com/halfak) [MediaWiki XML](https://github.com/mediawiki-utilities/python-mwxml) but ended up needing to code heuristics like "remove 'References'", etc. to extract the main content. It was brittle. Lots of garbage was still left in my markdown. Gave up.
</details>

The dataset is in Apache Parquet format. This format is interesting in that it allows processing tabular data with lots of columns. You only need to load the columns of interest from a batch of rows at once.


Each row of the `collection.parquet` represents a section of a Wikipedia article. We need to group multiple consecutive rows with the same `document_title` together to form complete articles.

**Example input data** (other columns not shown):

| document_title      | section_title | content                                    |
|---------------------|---------------|--------------------------------------------|
| "Anarchism"         | ""            | "Anarchism is a political philosophy..."   |
| "Anarchism"         | "Etymology"   | "The word anarchism is derived from..."    |
| "Anarchism"         | "History"     | "The history of anarchism dates back..."   |
| "Albert Einstein"   | ""            | "Albert Einstein was a German-born..."     |


### What the Preprocessor Does

The `preprocess` binary (`src/bin/preprocess.rs`) performs the following operations:

1. **Parallel Batch Processing**
   - Reads the Parquet file in batches (batch size = CPU threads × 64)
   - Uses Rayon for efficient parallel processing

2. **Text Cleaning & Normalization**
   - Removes control characters (except newlines)
   - Removes HTML table tags (`<Table>...</Table>`)
   - Applies Unicode NFC (Normalization Form C) for consistent character representation
   - Filters out empty documents and content

3. **Article Assembly**
   - Groups consecutive rows by `document_title`
   - Assembles sections into complete articles
   - Preserves section hierarchy

4. **Markdown Conversion**
   - Document title → `# Title` (H1 heading)
   - Section titles → `## Section` (H2 headings)
   - Normalizes markdown formatting using [comrak](https://github.com/kivikakk/comrak)

5. **Compression & Sharding**
   - Writes articles to Zstandard (`.zstd`) compressed files
   - Creates one shard file per CPU thread for parallel writing
   - Each article is separated by the end-of-text control character (`\u{3}`)
   - Compression level: 19 (maximum)

### Output Format

**Output files:**
```
output-dir/
  ├── shard-0.md.zstd
  ├── shard-1.md.zstd
  ├── shard-2.md.zstd
  └── ...
```

To view:
```sh
FILE=/var/lib/rust-gpt/training-data/stanford-oval/wikipedia/20250320/en/preprocessed/shard-0.md.zstd

# Get compressed file size in bytes
SIZE=$(stat -c%s $FILE)  # Linux
# SIZE=$(stat -f%z $FILE)  # macOS

OFFSET=$((RANDOM % SIZE))

# Zstd doesn't support seeking in compressed data without decompressing first.
# So we decompress the whole thing (cheap; shards files are ~200MiB)
# but pipe it to dd (data duplicator) with block size of 1 byte.
# dd will then duplicate the data from stdin to stdout but skip $OFFSET chunks,
# each chunk being 1 byte.
zstdcat $FILE | dd bs=1 skip=$OFFSET | head -100
```

Each shard contains multiple articles in this format:
```markdown
# Article Title

## Section 1

Content for section 1...

## Section 2

Content for section 2...
<end-of-text character>
# Next Article

...
```

### Usage

```bash
just run preprocess \
  --input-file /path/to/collection.parquet \
  --output-dir /path/to/output
```

### Performance

- **Parallel Processing**: Utilizes all CPU cores via Rayon
- **Memory Efficient**: Streams articles through a bounded channel (avoids loading millions of articles into RAM)
- **Compression**: Zstandard level 19 provides excellent compression ratios while maintaining fast decompression

### Architecture

```
Parquet File → Producer (Rayon task)
                 ↓
              Channel (bounded)
                 ↓
         Consumer (par_bridge)
                 ↓
    Thread-local zstd Encoders → Shard Files
```

The producer reads and processes parquet row batches in parallel, sends assembled articles through a channel, and consumers (Rayon workers; one per logical CPU core) write to thread-local shard files in parallel.

## Tokenizer Training

*(Work in Progress)*

## Pre-Training

*(Work in Progress)*

## Inference

*(Work in Progress)*
