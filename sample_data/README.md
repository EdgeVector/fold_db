# Sample Data for Smart Folder Ingestion Testing

This folder simulates a real user's Documents folder with a mix of personal data, media, config files, saved webpages, and binaries. It's designed to test the LLM-powered smart folder scanner's ability to classify files using directory context.

## Quick Start

### 1. Set your OpenRouter API key

An [OpenRouter](https://openrouter.ai) API key is the only external requirement. Set it before starting the server:

```bash
export FOLD_OPENROUTER_API_KEY=your_key_here
```

### 2. Start the server

From the `fold_db/` directory:

```bash
# Recommended: local storage + production schema service
./run.sh --local

# Fully offline (no internet required after build):
./run.sh --local --local-schema
```

The `--local-schema` flag starts a local schema service on port 9002 (built from this repo, no separate setup needed).

### 3. Scan the sample data

Open http://localhost:5173, go to the Smart Folder tab, and click **"Try sample data"** to auto-fill the path, then click **Scan**.

Or via API:
```bash
curl -X POST http://localhost:9001/api/ingestion/smart-folder/scan \
  -H "Content-Type: application/json" \
  -H "X-User-Hash: test_user" \
  -d '{"folder_path": "sample_data", "max_files": 100}'
```

### 4. Ingest

Review the scan results and click **Proceed** to ingest the recommended files.

## Directory Structure

```
sample_data/
├── blog_posts.json              # Personal blog content
├── meeting_notes.txt            # Work meeting notes
├── products.csv                 # Product catalog
├── users.json                   # User records
├── contacts/
│   └── address_book.json        # Personal contacts
├── config/
│   ├── .bashrc                  # Shell config (should skip)
│   ├── settings.json            # Editor settings (should skip)
│   ├── old_backup.exe           # Binary (should skip)
│   └── helper_tool.dll          # Binary (should skip)
├── finance/
│   ├── bank_statement_jan2025.csv  # Bank transactions
│   ├── investments.json            # Portfolio holdings
│   └── tax_receipt_2024.pdf        # PDF with tax receipt text
├── health/
│   ├── doctor_visits.txt        # Medical visit notes
│   └── medications.json         # Prescription records
├── insurance/
│   ├── auto_policy.json         # Car insurance details
│   └── declarations_page.pdf    # PDF with insurance declarations
├── journal/
│   ├── 2025-01-15.txt           # Daily journal entry
│   └── 2025-01-20.txt           # Daily journal entry
├── photos/
│   ├── profile_pic.png          # 64x64 PNG image
│   ├── animals/                 # Animal photos (golden retriever, tabby cat, etc.)
│   ├── diagrams/                # SVG diagrams (architecture, ER diagram, flowchart)
│   ├── family/
│   │   ├── christmas_2024.jpg   # 64x64 JPEG image
│   │   └── thanksgiving_2024.jpg
│   ├── landscapes/              # Nature landscapes (mountain, ocean, desert, etc.)
│   ├── paintings/               # Famous paintings (Mona Lisa, Starry Night, etc.)
│   ├── profile/                 # Portrait photos (studio, outdoor, creative)
│   ├── screenshots/             # SVG screenshots (terminal, dashboard)
│   └── vacation_2024/
│       ├── IMG_4521.jpg         # 64x64 JPEG images
│       ├── IMG_4522.jpg
│       └── IMG_4523.jpg
├── recipes/
│   ├── grandmas_cookies.txt     # Family recipe
│   └── meal_plan.csv            # Weekly meal plan
├── saved_webpages/
│   └── bank_of_america/         # "Save as complete webpage"
│       ├── account_summary.html # The actual content
│       ├── css/
│       │   ├── styles.css       # Scaffolding (should skip)
│       │   └── icons.woff2      # Font file (should skip)
│       └── images/
│           ├── ajax-loader.gif  # Scaffolding (should skip)
│           ├── boa_logo.gif     # Scaffolding (should skip)
│           └── spacer.gif       # Scaffolding (should skip)
├── school/
│   ├── cs101/
│   │   ├── homework3.txt        # Graded homework
│   │   └── syllabus.pdf         # PDF with course syllabus
│   └── math201/
│       └── notes_linear_algebra.md  # Course notes
├── taxes_2024/
│   ├── w2_summary.json          # W-2 tax data
│   └── charitable_donations.csv # Donation records
├── travel/
│   ├── packing_list.txt         # Trip planning
│   ├── flights/
│   │   └── sfo_to_tokyo_2025.json  # Flight booking
│   └── hotels/
│       └── tokyo_hotel.json     # Hotel reservation
├── coding_projects/                 # Coding projects (should be auto-skipped)
│   ├── my_website/              # Node.js project (has package.json)
│   │   ├── package.json
│   │   ├── index.js
│   │   └── README.md
│   ├── rust_cli/                # Rust project (has Cargo.toml)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── data_analysis/           # Python project (has pyproject.toml)
│       ├── pyproject.toml
│       ├── analysis.py
│       └── requirements.txt
└── work/
    ├── expenses/
    │   └── jan_2025_expenses.csv # Expense report
    ├── presentations/
    │   └── team_retro_q4.md     # Team retrospective
    └── project_notes/
        └── q1_goals.json        # Quarterly goals
```

## What to expect

The LLM classifier should:
- **Recommend** personal data: finance, health, contacts, journal, travel bookings, taxes, insurance, recipes
- **Skip** config files (.bashrc, settings.json), binaries (.exe, .dll), font files (.woff2)
- **Skip** saved webpage scaffolding (CSS, GIFs inside `bank_of_america/`) while possibly recommending the HTML content
- **Recommend** photos and PDFs as media/personal data (these are valid files and will be processed via the vision model)
- **Auto-skip** coding projects (`coding_projects/`) — directories containing manifest files like `package.json`, `Cargo.toml`, or `pyproject.toml` are skipped entirely before LLM classification

## Dependencies

All dependencies are included in the fold_db repo:
- **Rust backend** — built by `run.sh`
- **React frontend** — `npm install` handled by `run.sh`
- **Local schema service** — built from `src/bin/schema_service.rs` (use `--local-schema` flag)
- **Sample files** — all images are valid 64x64 JPEG/PNG, all PDFs contain readable text

The only external requirement is an **OpenRouter API key** for AI-powered classification and ingestion.
