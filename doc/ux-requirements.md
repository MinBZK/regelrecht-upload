# UX Requirements Document: RegelRecht Upload Portal

**Version:** 1.0
**Date:** 2026-02-23
**Status:** Draft - Pending Approval
**Source:** Early UX Designer Consultation

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State Overview](#2-current-state-overview)
3. [Requirements Overview](#3-requirements-overview)
4. [Landing Page Requirements](#4-landing-page-requirements)
5. [User Flow Restructuring](#5-user-flow-restructuring)
6. [Form Design Requirements](#6-form-design-requirements)
7. [Document Upload Page Requirements](#7-document-upload-page-requirements)
8. [Planning/Scheduling Page Requirements](#8-planningscheduling-page-requirements)
9. [Navigation & Progress Requirements](#9-navigation--progress-requirements)
10. [Visual Design Requirements](#10-visual-design-requirements)
11. [Accessibility Requirements](#11-accessibility-requirements)
12. [Implementation Priority](#12-implementation-priority)
13. [Affected Files](#13-affected-files)

---

## 1. Executive Summary

This document outlines comprehensive UX improvements for the RegelRecht Upload Portal based on early consultation with a UX Designer. The recommendations focus on:

- **Simplifying navigation** by removing unnecessary header elements
- **Restructuring the user flow** to prioritize user goals (documents first, then contact details)
- **Improving form usability** through better labeling, field grouping, and accessibility
- **Adding visual progress indicators** via a multi-page progress bar
- **Modernizing the visual design** with proper contrast and consistent styling

---

## 2. Current State Overview

### Current Architecture
- **Framework:** Vanilla HTML5 + Web Components
- **Pages:** Landing (`index.html`), Submit (`submit.html`), Status (`status.html`)
- **Flow:** 3-step wizard (Contact → Documents → Scheduling)
- **Styling:** CSS variables + RijksOverheid design system

### Current User Flow
```
Landing Page → Step 1: Contact Info → Step 2: Documents → Step 3: Planning → Success
```

### Issues Identified by UX Designer
1. FAQ not integrated into user flow
2. Navigation is cluttered (Privacy in header, unnecessary menu)
3. Landing page blocks look like buttons but aren't clickable
4. Form flow asks for contact info before understanding user's actual goal
5. Placeholder text in fields (accessibility issue)
6. Required field indicators using red asterisks (negative framing)
7. Links embedded in form labels (accessibility issue)
8. Dropdown menus reduce conversion rates

---

## 3. Requirements Overview

| Category | Requirements Count | Priority |
|----------|-------------------|----------|
| Landing Page | 6 | High |
| User Flow | 4 | Critical |
| Form Design | 6 | High |
| Document Upload | 7 | High |
| Planning Page | 2 | Medium |
| Navigation | 4 | High |
| Visual Design | 4 | Medium |
| Accessibility | 4 | Critical |

---

## 4. Landing Page Requirements

### REQ-LP-001: Remove "Welkom" Section
**Current:** Page opens with "Welkom" greeting
**Required:** Remove welcome text. Make the primary goal prominent as main header.

**Implementation:**
```
Before: "Welkom bij RegelRecht Upload"
After:  "Deel uw beleidsdocumenten" (as large H1 header with explanation)
```

### REQ-LP-002: Remove Privacy from Header Navigation
**Current:** Privacy link appears in main header navigation
**Required:** Remove Privacy from header. Keep only in footer legal bar.

### REQ-LP-003: Simplify Header Menu
**Current:** Full navigation menu in header
**Required:** Content should drive navigation. Remove menu from header. Let content guide users.

### REQ-LP-004: Transform Info Blocks to Multi-Page Progress Bar
**Current:** Visual blocks on landing page look like buttons but are not clickable
**Required:** Replace with a multi-page progress bar showing the submission steps.

**Visual Specification:**
```
[1. Privacy] → [2. Documenten] → [3. Planning] → [4. Gegevens] → [5. Bevestiging]
     ●              ○                 ○               ○               ○
```
- Active step: High contrast (inverted colors)
- Completed steps: Checkmark icon
- Future steps: Lower contrast outline

### REQ-LP-005: Add Document Type Information
**Current:** Users don't know upfront which documents can be submitted
**Required:** Clearly list acceptable document types on landing page.

### REQ-LP-006: Dual Call-to-Action Buttons
**Current:** Single entry point
**Required:** Two prominent buttons:
1. **Primary:** "Nieuwe inzending" (New submission)
2. **Secondary:** "Status opvragen / Inzending wijzigen" (Check status / Modify submission)

---

## 5. User Flow Restructuring

### REQ-UF-001: Reorder Form Steps (CRITICAL)
**Current Flow:**
```
Step 1: Contact Information → Step 2: Documents → Step 3: Planning
```

**Required Flow:**
```
Step 1: Privacy Consent → Step 2: Documents & Laws → Step 3: Planning → Step 4: Contact Information → Step 5: Summary
```

**Legal Constraint:** Privacy consent MUST be obtained in the first step before any data collection begins. This is a legal requirement and cannot be moved to a later step.

**Rationale (UX Designer):** After consent, capture documents first (user's primary intent), then supporting information. This aligns with user mental models while meeting legal requirements.

### REQ-UF-001a: Deferred Document Upload (CRITICAL - Data Protection)
**Current:** Documents are uploaded to the server immediately when added
**Required:** Documents must be staged client-side only until final submission

**Implementation:**
- Step 2 (Documents): Files are selected and validated client-side only
- Files are stored in browser memory (File objects / Blob URLs)
- Document metadata is collected but not transmitted
- NO API calls to upload endpoints during Steps 1-4
- Step 5 (Summary): User reviews all information
- Final "Rond inzending af" button triggers:
  1. Create submission with contact info
  2. Upload all staged documents
  3. Book meeting slot (if selected)

**Rationale (Legal):** Prevents storing documents without an identifiable owner. All data must be associated with a completed submission containing contact information.

**Technical Notes:**
- Use `FileReader` or `URL.createObjectURL()` for client-side file preview
- Store file references in JavaScript (not uploaded)
- Validate file types/sizes client-side before final submission
- Consider file size limits for browser memory (warn users if staging large files)

### REQ-UF-002: Group Related Input Fields
**Current:** Fields may be scattered
**Required:** Group all related fields together by category:
- Document information fields together
- Contact information fields together
- Planning/scheduling fields together

### REQ-UF-003: Add Summary Page Before Final Submission
**Current:** No overview before submission
**Required:** Final step shows complete summary of all entered information before submission.

**Summary Page Contents:**
- Uploaded documents list with categories
- Linked laws/regulations
- Selected meeting slot
- Contact information
- Edit buttons for each section

---

## 6. Form Design Requirements

### REQ-FD-001: Invert Required/Optional Field Indicators
**Current:** Required fields marked with red asterisks (*)
**Required:**
- Remove all red asterisks
- Mark only OPTIONAL fields with "(optioneel)" after the label
- Assume all unmarked fields are required

**Example:**
```
Before: Naam *
After:  Naam

Before: Afdeling
After:  Afdeling (optioneel)
```

### REQ-FD-002: Remove Placeholder Text from Input Fields
**Current:** Placeholder text inside input fields
**Required:**
- Move all placeholder/helper text to field labels or description text below label
- Leave input fields empty (better affordance + accessibility)

**Example:**
```
Before: [Enter your email address here...]
After:  E-mailadres
        Bijvoorbeeld: naam@organisatie.nl
        [                              ]
```

### REQ-FD-003: Remove Links from Form Labels
**Current:** Privacy policy link embedded in consent checkbox label
**Required:**
- Separate the information text from the checkbox
- Structure as: Title → Explanation text block → Checkbox with simple label

**Example:**
```
Before: ☐ Ik ga akkoord met de privacyverklaring (link)

After:  Privacyverklaring
        [Text block explaining privacy policy with link]

        ☐ Ik ga hiermee akkoord
```

### REQ-FD-004: Descriptive Button Labels
**Current:** Generic "Volgende" (Next) button
**Required:** Descriptive action buttons indicating the destination

**Examples:**
- "Verder naar uploaden" (Continue to upload)
- "Verder naar planning" (Continue to planning)
- "Verder naar gegevens" (Continue to details)
- "Rond inzending af" (Complete submission) — NOT "Inzending afronden"

### REQ-FD-005: Back Button Placement
**Current:** Back button at bottom alongside Next button
**Required:**
- Place Back button at TOP of form (above content)
- Keep only forward action button at bottom

### REQ-FD-006: Replace Dropdowns with Toggle Buttons
**Current:** Dropdown selects for document categories/classifications
**Required:** Replace dropdowns with visible toggle button groups where options ≤ 5

**Example (Document Type):**
```
Before: [Dropdown: Selecteer type ▼]

After:  Type document:
        [Circulaire] [Beleidsregel] [Instructie]
        (toggle buttons, one selected at a time)
```

**UX Designer Note:** Dropdowns reduce conversion compared to visible options. Toggle buttons improve discoverability and reduce interaction cost.

---

## 7. Document Upload Page Requirements

### REQ-DU-001: Rename Law Field Label
**Current:** "Naam van de wet" (Name of the law)
**Required:** "Titel van de wet" (Title of the law) — more accurate terminology

### REQ-DU-002: Action-First Button Labels
**Current:** "Link toevoegen" (Add link), "Document uploaden" (Upload document)
**Required:**
- "Voeg link toe" (Add link)
- "Upload document" (Upload document)

**Rationale:** Action verb first is more natural in Dutch for buttons.

### REQ-DU-003: Replace Category Dropdown with Toggle Buttons
**Current:** Dropdown for document category selection
**Required:** Toggle button group:
```
[Circulaire] [Beleidsregel] [Instructie]
```

### REQ-DU-004: Replace Classification Dropdown with Toggle Buttons
**Current:** Dropdown for document classification
**Required:** Toggle button group:
```
[Openbaar] [Claude-toegestaan] [Beperkt]
```

### REQ-DU-005: Add Delete Confirmation
**Current:** Single-click delete for uploaded documents
**Required:** Make deletion slightly harder:
- Option A: Confirmation dialog
- Option B: Undo toast notification (5 seconds)
- Option C: Two-step delete (click → confirm button appears)

### REQ-DU-006: Improve File Upload Zone
**Current:** Standard file upload component
**Required:** Ensure drag-drop zone has clear visual feedback and affordance.

### REQ-DU-007: Show Accepted File Types
**Current:** File type restrictions may not be immediately visible
**Required:** Display accepted file types and size limits clearly near upload zone.

---

## 8. Planning/Scheduling Page Requirements

### REQ-PL-001: Convert Warning to Informational Paragraph
**Current:** "Planning" section uses warning styling (yellow/orange alert)
**Required:** Display as normal informational paragraph, not as warning.

### REQ-PL-002: No Available Slots Message
**Current:** Behavior when no slots available unclear
**Required:** Display friendly message:
```
"Er zijn momenteel geen tijdsloten beschikbaar.
Wij nemen contact met u op om een afspraak in te plannen."
```

---

## 9. Navigation & Progress Requirements

### REQ-NV-001: Multi-Page Progress Bar Component
**Required:** Create reusable progress bar component showing:
- All steps in the submission flow
- Current step (inverted contrast/highlighted)
- Completed steps (checkmark icon)
- Future steps (outline only)

**Visual States:**
| State | Appearance |
|-------|------------|
| Completed | ✓ Checkmark, muted color |
| Current | Inverted contrast (dark background, light text) |
| Future | Outline only, light appearance |

### REQ-NV-002: Consistent Back Navigation
**Required:**
- Back button/link at TOP of each form step
- Text: "← Terug" or "← Terug naar [previous step]"
- Style: Text link, not button

### REQ-NV-003: Remove Custom Cursors
**Current:** Hand cursor may appear over non-interactive text
**Required:** Use default cursor for text. Only use pointer cursor for actual interactive elements.

### REQ-NV-004: Contextual Help Integration
**Current:** FAQ as separate page
**Required:** Scaffold help content into the user flow:
- Inline help text at relevant form sections
- Expandable help sections where needed
- Contact point readily available

---

## 10. Visual Design Requirements

### REQ-VD-001: Footer Redesign
**Current:** Blue prominent footer
**Required:**
- Subtle footer at bottom of screen
- Light gray text
- "Legal bar" style (minimal, unobtrusive)

### REQ-VD-002: Header Background
**Current:** May have colored background
**Required:** Standard/neutral background color for header. Keep it simple.

### REQ-VD-003: Consistent Color Usage
**Required:** Review contrast ratios:
- Progress bar: Invert contrast for current step
- Buttons: Clear primary/secondary distinction
- Form elements: Sufficient contrast for accessibility

### REQ-VD-004: Remove Decorative Elements
**Required:** Remove any decorative elements that don't serve functional purpose:
- Unnecessary icons
- Decorative borders
- Visual noise

---

## 11. Accessibility Requirements

### REQ-AC-001: Empty Input Fields
**Required:** Input fields should be visually empty (no placeholder text):
- Better affordance (users know to type there)
- Screen reader compatibility
- Placeholder text often has poor contrast

### REQ-AC-002: Label Association
**Required:** All form inputs must have properly associated labels:
- Use `<label for="id">` pattern
- No links inside labels
- Helper text as separate element with `aria-describedby`

### REQ-AC-003: Focus Indicators
**Required:** All interactive elements must have visible focus indicators for keyboard navigation.

### REQ-AC-004: Error Messaging
**Required:** Form validation errors must be:
- Announced to screen readers
- Visually associated with the relevant field
- Not rely on color alone

---

## 12. Implementation Priority

### Phase 1: Critical (User Flow & Data Protection)
| ID | Requirement | Effort |
|----|-------------|--------|
| REQ-UF-001 | Reorder form steps (Privacy first, 5 steps) | High |
| REQ-UF-001a | Deferred document upload (client-side staging) | High |
| REQ-NV-001 | Multi-page progress bar (5 steps) | Medium |
| REQ-LP-004 | Transform blocks to progress bar | Medium |

### Phase 2: High Priority (Forms & Usability)
| ID | Requirement | Effort |
|----|-------------|--------|
| REQ-FD-001 | Invert required/optional indicators | Low |
| REQ-FD-002 | Remove placeholder text | Low |
| REQ-FD-003 | Remove links from labels | Low |
| REQ-FD-004 | Descriptive button labels | Low |
| REQ-FD-005 | Back button placement | Low |
| REQ-FD-006 | Toggle buttons for dropdowns | Medium |
| REQ-DU-003 | Document category toggles | Medium |
| REQ-DU-004 | Classification toggles | Medium |

### Phase 3: Medium Priority (Visual & Navigation)
| ID | Requirement | Effort |
|----|-------------|--------|
| REQ-LP-001 | Remove Welkom section | Low |
| REQ-LP-002 | Remove Privacy from header | Low |
| REQ-LP-003 | Simplify header menu | Low |
| REQ-LP-006 | Dual CTA buttons | Low |
| REQ-VD-001 | Footer redesign | Medium |
| REQ-VD-002 | Header background | Low |
| REQ-UF-003 | Summary page | Medium |

### Phase 4: Polish
| ID | Requirement | Effort |
|----|-------------|--------|
| REQ-DU-001 | Rename law field | Low |
| REQ-DU-002 | Action-first buttons | Low |
| REQ-DU-005 | Delete confirmation | Low |
| REQ-PL-001 | Planning paragraph style | Low |
| REQ-PL-002 | No slots message | Low |
| REQ-NV-003 | Remove custom cursors | Low |

---

## 13. Affected Files

### HTML Pages (Major Changes)
| File | Changes Required |
|------|------------------|
| `frontend/index.html` | Landing page restructure, progress bar, dual CTAs |
| `frontend/submit.html` | Complete restructure of form steps, new step order |
| `frontend/status.html` | Minor styling updates |

### JavaScript (Logic Changes)
| File | Changes Required |
|------|------------------|
| `frontend/js/submit.js` | Reorder steps, new validation flow, summary page |
| `frontend/webcomponents/rr-select-field.js` | May be deprecated for toggle buttons |

### New Components Required
| Component | Purpose |
|-----------|---------|
| `rr-progress-bar.js` | Multi-page progress indicator |
| `rr-toggle-group.js` | Toggle button groups (replaces dropdowns) |
| `rr-summary-card.js` | Summary page display cards |

### CSS Updates
| File | Changes Required |
|------|------------------|
| `frontend/css/style.css` | Footer, header, progress bar, toggle buttons |

---

## Appendix A: Visual Mockups

### A.1 Progress Bar States
```
Step 1 (Privacy - Current):
[● Privacy] — [○ Documenten] — [○ Planning] — [○ Gegevens] — [○ Bevestiging]
   ▲ inverted

Step 2 (Documents):
[✓ Privacy] — [● Documenten] — [○ Planning] — [○ Gegevens] — [○ Bevestiging]
                  ▲ inverted

Step 3 (Planning):
[✓ Privacy] — [✓ Documenten] — [● Planning] — [○ Gegevens] — [○ Bevestiging]
                                   ▲ inverted

Step 4 (Contact Info):
[✓ Privacy] — [✓ Documenten] — [✓ Planning] — [● Gegevens] — [○ Bevestiging]
                                                  ▲ inverted

Step 5 (Final - all data uploaded here):
[✓ Privacy] — [✓ Documenten] — [✓ Planning] — [✓ Gegevens] — [● Bevestiging]
                                                                 ▲ inverted
```

### A.2 Toggle Button Group
```
Type document:
┌────────────┐ ┌────────────┐ ┌────────────┐
│ Circulaire │ │ Beleidsregel │ │ Instructie │  ← unselected: outline
└────────────┘ └────────────┘ └────────────┘

┌────────────┐ ┌════════════┐ ┌────────────┐
│ Circulaire │ ║ Beleidsregel ║ │ Instructie │  ← selected: filled
└────────────┘ └════════════┘ └────────────┘
```

### A.3 Form Field Pattern
```
E-mailadres
Bijvoorbeeld: naam@organisatie.nl
┌──────────────────────────────────────────────────────┐
│                                                      │  ← empty, no placeholder
└──────────────────────────────────────────────────────┘

Afdeling (optioneel)
┌──────────────────────────────────────────────────────┐
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

## Appendix B: New User Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        LANDING PAGE                              │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │         "Deel uw beleidsdocumenten"                      │    │
│  │         [Explanation text]                               │    │
│  │                                                          │    │
│  │  Progress: [1]───[2]───[3]───[4]───[5]                  │    │
│  │                                                          │    │
│  │  [████ Nieuwe inzending ████]  [Status opvragen]        │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 1: PRIVACY                               │
│  Progress: [●]───[○]───[○]───[○]───[○]                          │
│                                                                  │
│  Privacyverklaring                                              │
│                                                                  │
│  [Explanation text about data processing, storage duration,     │
│   and purpose. Link to full privacy statement.]                 │
│                                                                  │
│  ☐ Ik ga hiermee akkoord                                       │
│                                                                  │
│  [Verder naar documenten →]                                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 2: DOCUMENTEN                            │
│  Progress: [✓]───[●]───[○]───[○]───[○]                          │
│  ⚠ FILES STAGED CLIENT-SIDE ONLY (not uploaded yet)            │
│                                                                  │
│  ← Terug naar privacy                                           │
│                                                                  │
│  Titel van de wet                                               │
│  [                    ] [Voeg link toe]                         │
│                                                                  │
│  Upload document                                                │
│  ┌─ Drag & Drop Zone ─┐                                        │
│  │                    │  Type: [Circ] [Beleid] [Instr]         │
│  └────────────────────┘  Class: [Open] [Claude] [Beperkt]      │
│                                                                  │
│  Geselecteerde documenten: (lokaal opgeslagen)                  │
│  • Document1.pdf - Beleidsregel, Openbaar                       │
│                                                                  │
│  [Verder naar planning →]                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 3: PLANNING                              │
│  Progress: [✓]───[✓]───[●]───[○]───[○]                          │
│                                                                  │
│  ← Terug naar documenten                                        │
│                                                                  │
│  Selecteer een tijdslot voor uw gesprek:                        │
│  [Calendar with available slots]                                │
│                                                                  │
│  [Verder naar gegevens →]                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 4: GEGEVENS                              │
│  Progress: [✓]───[✓]───[✓]───[●]───[○]                          │
│                                                                  │
│  ← Terug naar planning                                          │
│                                                                  │
│  Naam                    E-mailadres                            │
│  [          ]            [              ]                       │
│                                                                  │
│  Organisatie             Afdeling (optioneel)                   │
│  [          ]            [              ]                       │
│                                                                  │
│  [Verder naar bevestiging →]                                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 5: BEVESTIGING                           │
│  Progress: [✓]───[✓]───[✓]───[✓]───[●]                          │
│  ⚠ ALL DATA UPLOADED ON FINAL SUBMIT                           │
│                                                                  │
│  ← Terug naar gegevens                                          │
│                                                                  │
│  Controleer uw inzending:                                       │
│  ┌─ Documenten ──────────────────────────── [Wijzig] ─┐        │
│  │ • Document1.pdf (Beleidsregel, Openbaar)           │        │
│  │ • Wet ABC (link)                                    │        │
│  └─────────────────────────────────────────────────────┘        │
│  ┌─ Planning ────────────────────────────── [Wijzig] ─┐        │
│  │ Dinsdag 25 feb 2026, 14:00                         │        │
│  └─────────────────────────────────────────────────────┘        │
│  ┌─ Contactgegevens ─────────────────────── [Wijzig] ─┐        │
│  │ Jan de Vries, jan@org.nl                           │        │
│  │ Ministerie van X, Afdeling Y                       │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                  │
│  [████ Rond inzending af ████]                                  │
│         ↓                                                        │
│  (Creates submission + uploads all documents + books slot)      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-23 | UX Designer Consultation | Initial requirements based on early UX design review |

---

*End of Document*
