//! Pre-populated canonical field registry.
//!
//! The schema service keeps a global registry of canonical field
//! names (e.g. `user_email`, `photo_caption`, `gps_latitude`) with a
//! description, type, and data classification. When a new schema is
//! proposed, each of its fields is matched against this registry via
//! semantic similarity so that semantically equivalent concepts
//! ("email_address" ↔ "user_email") are normalized to a single
//! canonical name across every schema on the network.
//!
//! Without pre-population, the first time the service sees a new field,
//! it runs an LLM classification call (Anthropic) to infer the field's
//! sensitivity + domain + interest category, and writes the resulting
//! `CanonicalField` to storage. That's a hot path:
//!
//! - **slow**: LLM round-trip + RMW persistence per new field
//! - **expensive**: each call is a billable API request
//! - **unreliable**: classification quality drifts across model versions
//!
//! This module pre-loads the registry with a curated set of ~150
//! common concepts covering identity, location, time, content, media,
//! communication, commerce, health, documents, and events. Every entry
//! carries its own authoritative description + classification so the
//! service can skip the LLM entirely for the cases that account for
//! the vast majority of real-world schema proposals.
//!
//! Idempotent: `seed()` is safe to call on every cold start. Entries
//! already present are silently skipped.
//!
//! The list is deliberately conservative:
//! - Every name is `snake_case` — matches the canonicalization
//!   convention used elsewhere in the service.
//! - Descriptions are written as "a $NOUN in/of $CONTEXT" so the
//!   semantic matcher has stable phrasing to compare against.
//! - Sensitivity levels bias toward `Restricted` (3) for anything
//!   that can identify a person, `Confidential` (2) for business
//!   content, `Internal` (1) for rarely-sensitive metadata, and
//!   `Public` (0) for things like public URLs or tag strings.
//! - `data_domain` picks from the existing vocabulary used elsewhere
//!   in the codebase: `general`, `identity`, `financial`, `medical`,
//!   `location`, `communication`, `content`, `temporal`, `commerce`,
//!   `media`, `social`, `document`.

use crate::error::FoldDbResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema_service::state::SchemaServiceState;
use crate::schema_service::types::CanonicalField;

/// A single pre-populated canonical field. Kept as a function that
/// constructs a `CanonicalField` so we can validate DataClassification
/// construction (which is fallible) at seed time rather than as a
/// static literal.
struct Entry {
    name: &'static str,
    description: &'static str,
    field_type: FieldValueType,
    sensitivity: u8,
    domain: &'static str,
    interest_category: Option<&'static str>,
}

impl Entry {
    fn build(&self) -> FoldDbResult<(String, CanonicalField)> {
        let classification =
            DataClassification::new(self.sensitivity, self.domain).map_err(|e| {
                crate::error::FoldDbError::Config(format!(
                    "Invalid classification for built-in canonical field '{}': {}",
                    self.name, e
                ))
            })?;
        Ok((
            self.name.to_string(),
            CanonicalField {
                description: self.description.to_string(),
                field_type: self.field_type.clone(),
                classification: Some(classification),
                interest_category: self.interest_category.map(String::from),
            },
        ))
    }
}

/// Seed the canonical field registry with the curated list. Called
/// from Lambda cold start alongside `builtin_schemas::seed`. Fails
/// loudly on any construction error (unknown domain, out-of-range
/// sensitivity) because a malformed built-in would be a code bug.
pub async fn seed(state: &SchemaServiceState) -> FoldDbResult<()> {
    let entries = all_entries();
    let count = entries.len();

    for entry in entries {
        let (name, canonical) = entry.build()?;
        // add_canonical_field is idempotent: returns Ok(()) whether
        // the field was already present or freshly inserted. Safe to
        // call on every cold start.
        state.add_canonical_field(&name, canonical).await?;
    }

    log_feature!(
        LogFeature::Schema,
        info,
        "Seeded {} built-in canonical fields",
        count
    );
    Ok(())
}

/// The curated list of canonical fields. Ordered by topical cluster
/// for readability, not for any functional reason.
#[rustfmt::skip]
fn all_entries() -> Vec<Entry> {
    use FieldValueType::*;
    vec![
        // ==================== Identity ====================
        Entry { name: "user_id",           description: "a stable opaque identifier for a user",                         field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "user_name",         description: "the display name of a user",                                    field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "first_name",        description: "a person's given name",                                         field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "last_name",         description: "a person's family or surname",                                  field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "full_name",         description: "a person's complete given and family name",                     field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "display_name",      description: "a human-readable label for a person or entity",                 field_type: String, sensitivity: 2, domain: "identity", interest_category: None },
        Entry { name: "handle",            description: "a short unique account identifier, like a social handle",       field_type: String, sensitivity: 2, domain: "identity", interest_category: None },
        Entry { name: "username",          description: "an account login identifier for a user",                        field_type: String, sensitivity: 2, domain: "identity", interest_category: None },
        Entry { name: "user_email",        description: "a user's email address",                                        field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "email_address",     description: "an email address belonging to a person or entity",              field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "phone_number",      description: "a telephone number belonging to a person or entity",            field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "date_of_birth",     description: "a person's date of birth",                                      field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "gender",            description: "a person's gender identity",                                    field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "pronouns",          description: "pronouns a person uses",                                        field_type: String, sensitivity: 2, domain: "identity", interest_category: None },
        Entry { name: "avatar_url",        description: "a URL pointing to a user's profile image",                      field_type: String, sensitivity: 1, domain: "identity", interest_category: None },
        Entry { name: "bio",               description: "a short biographical description of a person",                  field_type: String, sensitivity: 2, domain: "identity", interest_category: None },
        Entry { name: "identity_hash",     description: "a cryptographic hash uniquely identifying an entity",           field_type: String, sensitivity: 1, domain: "identity", interest_category: None },
        Entry { name: "public_key",        description: "a cryptographic public key",                                    field_type: String, sensitivity: 0, domain: "identity", interest_category: None },

        // ==================== Content / Generic ====================
        Entry { name: "id",                description: "a generic opaque primary identifier",                           field_type: String, sensitivity: 0, domain: "general",  interest_category: None },
        Entry { name: "name",              description: "a human-readable name for an entity",                           field_type: String, sensitivity: 0, domain: "general",  interest_category: None },
        Entry { name: "title",             description: "a title or headline for a piece of content",                    field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "description",       description: "a longer textual description of an entity or piece of content", field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "summary",           description: "a short summary of a larger piece of content",                  field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "body",              description: "the main textual body of a document or message",                field_type: String, sensitivity: 1, domain: "content",  interest_category: None },
        Entry { name: "content",           description: "the primary textual content of an item",                        field_type: String, sensitivity: 1, domain: "content",  interest_category: None },
        Entry { name: "tags",              description: "a list of tag labels applied to a piece of content",            field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "category",          description: "a category label classifying a piece of content",               field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "language",          description: "the natural language of a piece of content (ISO 639-1)",        field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "slug",              description: "a URL-friendly identifier derived from a title",                field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "url",               description: "a URL reference",                                               field_type: String, sensitivity: 0, domain: "content",  interest_category: None },
        Entry { name: "source_url",        description: "a URL indicating where content was originally retrieved from",  field_type: String, sensitivity: 0, domain: "content",  interest_category: None },

        // ==================== Time / Temporal ====================
        Entry { name: "created_at",        description: "the timestamp when an entity was created",                      field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "updated_at",        description: "the timestamp when an entity was last updated",                 field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "deleted_at",        description: "the timestamp when an entity was soft-deleted",                 field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "published_at",      description: "the timestamp when a piece of content was published",           field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "timestamp",         description: "a point in time for an observation or event",                   field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "start_time",        description: "the start timestamp of an interval or event",                   field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "end_time",          description: "the end timestamp of an interval or event",                     field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "duration_seconds",  description: "the duration of an interval, in seconds",                       field_type: Number, sensitivity: 0, domain: "temporal", interest_category: None },
        Entry { name: "expires_at",        description: "the timestamp at which an entity expires",                      field_type: String, sensitivity: 0, domain: "temporal", interest_category: None },

        // ==================== Location ====================
        Entry { name: "street_address",    description: "a street address line",                                         field_type: String, sensitivity: 3, domain: "location", interest_category: None },
        Entry { name: "city",              description: "the city of an address",                                        field_type: String, sensitivity: 2, domain: "location", interest_category: None },
        Entry { name: "state",             description: "the state or province of an address",                           field_type: String, sensitivity: 1, domain: "location", interest_category: None },
        Entry { name: "postal_code",       description: "the postal code of an address",                                 field_type: String, sensitivity: 2, domain: "location", interest_category: None },
        Entry { name: "country",           description: "the country of an address",                                     field_type: String, sensitivity: 1, domain: "location", interest_category: None },
        Entry { name: "gps_latitude",      description: "the latitude component of a geographic coordinate",             field_type: Number, sensitivity: 3, domain: "location", interest_category: None },
        Entry { name: "gps_longitude",     description: "the longitude component of a geographic coordinate",            field_type: Number, sensitivity: 3, domain: "location", interest_category: None },
        Entry { name: "altitude_meters",   description: "the altitude component of a geographic coordinate, in meters",  field_type: Number, sensitivity: 2, domain: "location", interest_category: None },
        Entry { name: "place_name",        description: "a human-readable place name",                                   field_type: String, sensitivity: 1, domain: "location", interest_category: Some("Travel") },
        Entry { name: "venue_name",        description: "the name of a venue hosting an event",                          field_type: String, sensitivity: 1, domain: "location", interest_category: Some("Events") },
        Entry { name: "timezone",          description: "a timezone identifier (IANA tz database)",                      field_type: String, sensitivity: 0, domain: "location", interest_category: None },

        // ==================== Media (images/video/audio) ====================
        Entry { name: "photo_url",         description: "a URL pointing to a photograph",                                field_type: String, sensitivity: 1, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "photo_caption",     description: "a caption describing the contents of a photograph",             field_type: String, sensitivity: 1, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "photo_taken_at",    description: "the timestamp when a photograph was captured",                  field_type: String, sensitivity: 1, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "image_width",       description: "the pixel width of an image",                                   field_type: Number, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "image_height",      description: "the pixel height of an image",                                  field_type: Number, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "video_url",         description: "a URL pointing to a video file or stream",                      field_type: String, sensitivity: 1, domain: "media",    interest_category: Some("Video") },
        Entry { name: "video_duration_seconds", description: "the duration of a video, in seconds",                     field_type: Number, sensitivity: 0, domain: "media",    interest_category: Some("Video") },
        Entry { name: "thumbnail_url",     description: "a URL pointing to a small preview image",                       field_type: String, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "audio_url",         description: "a URL pointing to an audio recording",                          field_type: String, sensitivity: 1, domain: "media",    interest_category: Some("Music") },
        Entry { name: "file_size_bytes",   description: "the size of a file in bytes",                                   field_type: Number, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "file_type",         description: "the MIME type of a file",                                       field_type: String, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "file_hash",         description: "a cryptographic hash of a file's contents",                     field_type: String, sensitivity: 0, domain: "media",    interest_category: None },
        Entry { name: "camera_make",       description: "the manufacturer of a camera that captured a photograph",       field_type: String, sensitivity: 0, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "camera_model",      description: "the model of a camera that captured a photograph",              field_type: String, sensitivity: 0, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "iso_speed",         description: "the ISO sensitivity setting used when capturing a photograph",  field_type: Number, sensitivity: 0, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "aperture",          description: "the aperture f-number used when capturing a photograph",        field_type: Number, sensitivity: 0, domain: "media",    interest_category: Some("Photography") },
        Entry { name: "shutter_speed",     description: "the shutter speed used when capturing a photograph",            field_type: String, sensitivity: 0, domain: "media",    interest_category: Some("Photography") },

        // ==================== Communication / Messaging ====================
        Entry { name: "message_id",        description: "a unique identifier for a message",                             field_type: String, sensitivity: 1, domain: "communication", interest_category: None },
        Entry { name: "message_text",      description: "the textual body of a message",                                 field_type: String, sensitivity: 3, domain: "communication", interest_category: None },
        Entry { name: "subject",           description: "the subject line of an email or message",                       field_type: String, sensitivity: 2, domain: "communication", interest_category: None },
        Entry { name: "sender",            description: "the sender of a message",                                       field_type: String, sensitivity: 3, domain: "communication", interest_category: None },
        Entry { name: "recipient",         description: "the intended recipient of a message",                           field_type: String, sensitivity: 3, domain: "communication", interest_category: None },
        Entry { name: "cc",                description: "carbon-copy recipients of an email",                            field_type: String, sensitivity: 3, domain: "communication", interest_category: None },
        Entry { name: "bcc",               description: "blind carbon-copy recipients of an email",                      field_type: String, sensitivity: 3, domain: "communication", interest_category: None },
        Entry { name: "reply_to",          description: "the message ID this message is replying to",                    field_type: String, sensitivity: 1, domain: "communication", interest_category: None },
        Entry { name: "thread_id",         description: "a unique identifier for a conversation thread",                 field_type: String, sensitivity: 1, domain: "communication", interest_category: None },
        Entry { name: "sent_at",           description: "the timestamp when a message was sent",                         field_type: String, sensitivity: 1, domain: "communication", interest_category: None },

        // ==================== Financial / Commerce ====================
        Entry { name: "amount",            description: "a monetary amount",                                             field_type: Number, sensitivity: 3, domain: "financial", interest_category: None },
        Entry { name: "currency",          description: "an ISO 4217 currency code",                                     field_type: String, sensitivity: 0, domain: "financial", interest_category: None },
        Entry { name: "transaction_id",    description: "a unique identifier for a financial transaction",               field_type: String, sensitivity: 3, domain: "financial", interest_category: None },
        Entry { name: "payment_method",    description: "the method used to settle a payment",                           field_type: String, sensitivity: 3, domain: "financial", interest_category: None },
        Entry { name: "merchant_name",     description: "the name of a merchant receiving a payment",                    field_type: String, sensitivity: 2, domain: "financial", interest_category: None },
        Entry { name: "vendor_id",         description: "a unique identifier for a vendor or supplier",                  field_type: String, sensitivity: 2, domain: "financial", interest_category: None },
        Entry { name: "account_number",    description: "a financial account number",                                    field_type: String, sensitivity: 4, domain: "financial", interest_category: None },
        Entry { name: "invoice_number",    description: "a unique identifier for an invoice",                            field_type: String, sensitivity: 2, domain: "financial", interest_category: None },
        Entry { name: "due_date",          description: "the date by which a payment or task is due",                    field_type: String, sensitivity: 1, domain: "financial", interest_category: None },
        Entry { name: "tax_amount",        description: "the tax component of a monetary amount",                        field_type: Number, sensitivity: 3, domain: "financial", interest_category: None },
        Entry { name: "product_id",        description: "a unique identifier for a product",                             field_type: String, sensitivity: 0, domain: "commerce",  interest_category: None },
        Entry { name: "product_name",      description: "the name of a product",                                         field_type: String, sensitivity: 0, domain: "commerce",  interest_category: None },
        Entry { name: "sku",               description: "a stock-keeping unit identifier",                                field_type: String, sensitivity: 0, domain: "commerce",  interest_category: None },
        Entry { name: "price",             description: "the listed price of a product",                                 field_type: Number, sensitivity: 0, domain: "commerce",  interest_category: None },
        Entry { name: "quantity",          description: "a count of items",                                              field_type: Number, sensitivity: 0, domain: "commerce",  interest_category: None },
        Entry { name: "order_id",          description: "a unique identifier for a purchase order",                      field_type: String, sensitivity: 2, domain: "commerce",  interest_category: None },
        Entry { name: "discount_amount",   description: "the monetary value of a discount applied to an order",          field_type: Number, sensitivity: 1, domain: "commerce",  interest_category: None },

        // ==================== Health / Medical ====================
        Entry { name: "diagnosis",         description: "a medical diagnosis",                                           field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "medication",        description: "a prescribed or administered medication",                       field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "dosage",            description: "the dosage of a medication",                                    field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "provider_name",     description: "the name of a healthcare provider",                             field_type: String, sensitivity: 3, domain: "medical", interest_category: None },
        Entry { name: "appointment_date",  description: "the date and time of a healthcare appointment",                 field_type: String, sensitivity: 3, domain: "medical", interest_category: None },
        Entry { name: "symptom",           description: "a reported medical symptom",                                    field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "allergy",           description: "a known allergy",                                               field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "blood_type",        description: "a person's blood type",                                         field_type: String, sensitivity: 4, domain: "medical", interest_category: None },
        Entry { name: "height_cm",         description: "a person's height in centimeters",                              field_type: Number, sensitivity: 3, domain: "medical", interest_category: None },
        Entry { name: "weight_kg",         description: "a person's weight in kilograms",                                field_type: Number, sensitivity: 3, domain: "medical", interest_category: None },

        // ==================== Documents ====================
        Entry { name: "document_title",    description: "the title of a document",                                       field_type: String, sensitivity: 0, domain: "document", interest_category: None },
        Entry { name: "document_author",   description: "the author of a document",                                      field_type: String, sensitivity: 1, domain: "document", interest_category: None },
        Entry { name: "publisher",         description: "the publisher of a document",                                   field_type: String, sensitivity: 0, domain: "document", interest_category: None },
        Entry { name: "publication_date",  description: "the date a document was published",                             field_type: String, sensitivity: 0, domain: "document", interest_category: None },
        Entry { name: "isbn",              description: "an International Standard Book Number",                         field_type: String, sensitivity: 0, domain: "document", interest_category: Some("Reading") },
        Entry { name: "doi",               description: "a Digital Object Identifier",                                   field_type: String, sensitivity: 0, domain: "document", interest_category: None },
        Entry { name: "page_count",        description: "the number of pages in a document",                             field_type: Number, sensitivity: 0, domain: "document", interest_category: None },
        Entry { name: "chapter",           description: "a chapter heading within a document",                           field_type: String, sensitivity: 0, domain: "document", interest_category: None },

        // ==================== Events / Calendar ====================
        Entry { name: "event_name",        description: "the name of a calendar event",                                  field_type: String, sensitivity: 2, domain: "general",  interest_category: Some("Events") },
        Entry { name: "event_type",        description: "a category for a calendar event",                               field_type: String, sensitivity: 1, domain: "general",  interest_category: Some("Events") },
        Entry { name: "organizer",         description: "the organizer of an event",                                     field_type: String, sensitivity: 2, domain: "general",  interest_category: Some("Events") },
        Entry { name: "attendees",         description: "a list of people attending an event",                           field_type: String, sensitivity: 3, domain: "general",  interest_category: Some("Events") },
        Entry { name: "rsvp_status",       description: "an attendee's RSVP response to an event invitation",             field_type: String, sensitivity: 2, domain: "general",  interest_category: Some("Events") },
        Entry { name: "location",          description: "a location for an event or activity",                           field_type: String, sensitivity: 2, domain: "location", interest_category: None },

        // ==================== Social ====================
        Entry { name: "post_id",           description: "a unique identifier for a social post",                         field_type: String, sensitivity: 0, domain: "social",   interest_category: None },
        Entry { name: "post_text",         description: "the text body of a social post",                                field_type: String, sensitivity: 1, domain: "social",   interest_category: None },
        Entry { name: "like_count",        description: "the number of likes on a social post",                          field_type: Number, sensitivity: 0, domain: "social",   interest_category: None },
        Entry { name: "comment_text",      description: "the text body of a comment",                                    field_type: String, sensitivity: 1, domain: "social",   interest_category: None },
        Entry { name: "follower_count",    description: "the number of followers a user has",                            field_type: Number, sensitivity: 0, domain: "social",   interest_category: None },
        Entry { name: "following_count",   description: "the number of accounts a user follows",                         field_type: Number, sensitivity: 0, domain: "social",   interest_category: None },
        Entry { name: "share_count",       description: "the number of shares a post has received",                      field_type: Number, sensitivity: 0, domain: "social",   interest_category: None },

        // ==================== Web / Links ====================
        Entry { name: "domain_name",       description: "an Internet domain name",                                       field_type: String, sensitivity: 0, domain: "general",  interest_category: None },
        Entry { name: "ip_address",        description: "an Internet Protocol address",                                  field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "user_agent",        description: "a User-Agent HTTP header string",                                field_type: String, sensitivity: 2, domain: "general",  interest_category: None },
        Entry { name: "referrer",          description: "a Referer HTTP header URL",                                     field_type: String, sensitivity: 1, domain: "general",  interest_category: None },

        // ==================== Recipes / Cooking ====================
        Entry { name: "recipe_name",       description: "the name of a recipe",                                          field_type: String, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },
        Entry { name: "ingredients",       description: "a list of ingredients used in a recipe",                        field_type: String, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },
        Entry { name: "instructions",      description: "the step-by-step preparation instructions for a recipe",        field_type: String, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },
        Entry { name: "servings",          description: "the number of servings a recipe yields",                        field_type: Number, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },
        Entry { name: "prep_time_minutes", description: "the preparation time for a recipe, in minutes",                 field_type: Number, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },
        Entry { name: "cook_time_minutes", description: "the cook time for a recipe, in minutes",                        field_type: Number, sensitivity: 0, domain: "content",  interest_category: Some("Cooking") },

        // ==================== Fitness ====================
        Entry { name: "activity_type",     description: "the type of physical activity or exercise",                     field_type: String, sensitivity: 1, domain: "general",  interest_category: Some("Fitness") },
        Entry { name: "distance_km",       description: "a distance measured in kilometers",                             field_type: Number, sensitivity: 1, domain: "general",  interest_category: Some("Fitness") },
        Entry { name: "calories_burned",   description: "the number of calories burned during an activity",              field_type: Number, sensitivity: 2, domain: "medical", interest_category: Some("Fitness") },
        Entry { name: "heart_rate_bpm",    description: "a heart rate measurement in beats per minute",                  field_type: Number, sensitivity: 3, domain: "medical", interest_category: Some("Fitness") },
        Entry { name: "steps_count",       description: "a count of steps taken",                                        field_type: Number, sensitivity: 1, domain: "general",  interest_category: Some("Fitness") },

        // ==================== Notes / Journaling ====================
        Entry { name: "note_text",         description: "the textual body of a personal note",                           field_type: String, sensitivity: 3, domain: "content",  interest_category: None },
        Entry { name: "journal_entry",     description: "a personal journal entry",                                      field_type: String, sensitivity: 3, domain: "content",  interest_category: None },
        Entry { name: "mood",              description: "a self-reported mood or emotional state",                       field_type: String, sensitivity: 3, domain: "content",  interest_category: None },

        // ==================== Fingerprints / Identity subsystem (Phase 1) ====================
        Entry { name: "fingerprint_id",    description: "a unique identifier for a fingerprint observation",             field_type: String, sensitivity: 2, domain: "general",  interest_category: None },
        Entry { name: "fingerprint_kind",  description: "the kind of a fingerprint (face, name, voice, etc.)",           field_type: String, sensitivity: 1, domain: "general",  interest_category: None },
        Entry { name: "fingerprint_value", description: "the raw observed value of a fingerprint",                       field_type: String, sensitivity: 3, domain: "general",  interest_category: None },
        Entry { name: "confidence",        description: "a confidence score in the range [0,1]",                         field_type: Number, sensitivity: 0, domain: "general",  interest_category: None },
        Entry { name: "persona_id",        description: "a unique identifier for a persona cluster",                     field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "relationship",      description: "the relationship between a persona and the node owner",        field_type: String, sensitivity: 3, domain: "identity", interest_category: None },
        Entry { name: "trust_tier",        description: "an access-control trust tier (0=Public to 4=Owner)",            field_type: Number, sensitivity: 1, domain: "general",  interest_category: None },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_constructs_valid_classification() {
        // Catches typos in data_domain or out-of-range sensitivity
        // in the hardcoded list.
        for entry in all_entries() {
            entry.build().unwrap_or_else(|e| {
                panic!("Failed to build built-in field '{}': {}", entry.name, e)
            });
        }
    }

    #[test]
    fn every_name_is_snake_case_and_unique() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for entry in all_entries() {
            assert!(
                entry
                    .name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "Non-snake_case name: '{}'",
                entry.name
            );
            assert!(
                seen.insert(entry.name),
                "Duplicate built-in canonical field: '{}'",
                entry.name
            );
        }
    }

    #[test]
    fn reasonable_entry_count() {
        assert!(
            all_entries().len() >= 100,
            "Expected at least 100 built-in canonical fields, got {}",
            all_entries().len()
        );
    }
}
