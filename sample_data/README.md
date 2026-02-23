# Sample Data for Smart Folder Ingestion Testing

This folder simulates a real user's Documents folder with a mix of personal data, media, config files, saved webpages, and binaries. It's designed to test the LLM-powered smart folder scanner's ability to classify files using directory context.

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
│   └── tax_receipt_2024.pdf        # PDF stub
├── health/
│   ├── doctor_visits.txt        # Medical visit notes
│   └── medications.json         # Prescription records
├── insurance/
│   ├── auto_policy.json         # Car insurance details
│   └── declarations_page.pdf    # PDF stub
├── journal/
│   ├── 2025-01-15.txt           # Daily journal entry
│   └── 2025-01-20.txt           # Daily journal entry
├── photos/
│   ├── profile_pic.png          # Image stub
│   ├── family/
│   │   ├── christmas_2024.jpg   # Image stub
│   │   └── thanksgiving_2024.jpg
│   └── vacation_2024/
│       ├── IMG_4521.jpg         # Image stubs
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
│   │   └── syllabus.pdf         # PDF stub
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
└── work/
    ├── expenses/
    │   └── jan_2025_expenses.csv # Expense report
    ├── presentations/
    │   └── team_retro_q4.md     # Team retrospective
    └── project_notes/
        └── q1_goals.json        # Quarterly goals
```

## Usage

In the UI (dev mode), click **"Try sample data"** on the Smart Folder tab, then click **Scan**.

Or via API:
```bash
curl -X POST http://localhost:9001/api/ingestion/smart-folder/scan \
  -H "Content-Type: application/json" \
  -H "X-User-Hash: test_user" \
  -d '{"folder_path": "sample_data", "max_files": 100}'
```

## What to expect

The LLM classifier should:
- **Recommend** personal data: finance, health, contacts, journal, travel bookings, taxes, insurance, recipes
- **Skip** config files (.bashrc, settings.json), binaries (.exe, .dll), font files (.woff2)
- **Skip** saved webpage scaffolding (CSS, GIFs inside `bank_of_america/`) while possibly recommending the HTML content
- **Classify** photos and PDFs as media (image stubs won't actually ingest, but the classifier should still see them)
