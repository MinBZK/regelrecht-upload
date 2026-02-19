# RegelRecht Upload Portal - Architecture

Dit document beschrijft de technische architectuur van het RegelRecht Upload Portal.

## Inhoudsopgave

1. [Systeem Overzicht](#systeem-overzicht)
2. [Component Architectuur](#component-architectuur)
3. [Database Schema](#database-schema)
4. [API Routes](#api-routes)
5. [Authentication Flows](#authentication-flows)
6. [Submission Workflow](#submission-workflow)
7. [File Upload Flow](#file-upload-flow)
8. [Directory Structuur](#directory-structuur)

---

## Systeem Overzicht

```mermaid
graph TB
    subgraph "Frontend Layer"
        A[Applicant Portal<br/>submit.html, status.html]
        B[Admin Portal<br/>admin/dashboard.html]
        C[Uploader Portal<br/>uploader/dashboard.html]
        WC[Web Components<br/>rr-button, rr-text-field, etc.]
    end

    subgraph "API Layer"
        R[Axum Router<br/>main.rs]
        MW[Middleware<br/>auth, security headers, rate limiting]
    end

    subgraph "Handler Layer"
        H1[submissions.rs<br/>CRUD operations]
        H2[auth.rs<br/>Admin authentication]
        H3[uploader_auth.rs<br/>Uploader self-service]
        H4[admin.rs<br/>Admin operations]
        H5[calendar.rs<br/>Meeting slots]
    end

    subgraph "Data Layer"
        DB[(PostgreSQL<br/>Database)]
        FS[File System<br/>/data/uploads]
    end

    A --> R
    B --> R
    C --> R
    WC -.-> A
    WC -.-> B
    WC -.-> C

    R --> MW
    MW --> H1
    MW --> H2
    MW --> H3
    MW --> H4
    MW --> H5

    H1 --> DB
    H1 --> FS
    H2 --> DB
    H3 --> DB
    H4 --> DB
    H4 --> FS
    H5 --> DB
```

---

## Component Architectuur

```mermaid
graph LR
    subgraph "Rust Backend"
        direction TB
        MAIN[main.rs<br/>Entry Point]
        CONFIG[config.rs<br/>Environment Config]

        subgraph "Handlers"
            AUTH[auth.rs]
            UAUTH[uploader_auth.rs]
            SUB[submissions.rs]
            ADM[admin.rs]
            CAL[calendar.rs]
            MID[middleware.rs]
        end

        subgraph "Database"
            POOL[pool.rs<br/>Connection Pool]
            MIG[migrations/<br/>SQL files]
        end

        MODELS[models/mod.rs<br/>Data Structures]
        VALID[validation/mod.rs<br/>Input Validation]
    end

    subgraph "Frontend"
        direction TB
        HTML[HTML Pages]
        JS[JavaScript<br/>submit.js, admin.js]
        CSS[Stylesheets<br/>style.css]
        COMP[Web Components<br/>rr-*.js]
    end

    MAIN --> CONFIG
    MAIN --> POOL
    POOL --> MIG
    MAIN --> AUTH
    MAIN --> UAUTH
    MAIN --> SUB
    MAIN --> ADM
    MAIN --> CAL

    AUTH --> MODELS
    SUB --> MODELS
    SUB --> VALID
    ADM --> MODELS

    HTML --> JS
    HTML --> CSS
    HTML --> COMP
```

---

## Database Schema

```mermaid
erDiagram
    submissions ||--o{ documents : contains
    submissions ||--o| calendar_slots : books
    submissions ||--o{ uploader_sessions : authenticates
    admin_users ||--o{ admin_sessions : has
    admin_users ||--o{ calendar_slots : creates

    submissions {
        uuid id PK
        string slug UK
        string submitter_name
        string submitter_email
        string organization
        string organization_department
        enum status
        text notes
        timestamp created_at
        timestamp updated_at
        timestamp submitted_at
        timestamp retention_expiry_date
    }

    documents {
        uuid id PK
        uuid submission_id FK
        enum category
        enum classification
        string external_url
        string external_title
        string filename
        string original_filename
        string file_path
        bigint file_size
        string mime_type
        text description
        timestamp created_at
    }

    admin_users {
        uuid id PK
        string username UK
        string email UK
        string password_hash
        string display_name
        boolean is_active
        timestamp created_at
        timestamp last_login_at
    }

    admin_sessions {
        uuid id PK
        uuid admin_user_id FK
        string token_hash
        timestamp expires_at
        timestamp created_at
        string ip_address
        string user_agent
    }

    uploader_sessions {
        uuid id PK
        uuid submission_id FK
        string email
        string token_hash
        timestamp expires_at
        timestamp created_at
        string ip_address
        string user_agent
    }

    calendar_slots {
        uuid id PK
        timestamp slot_start
        timestamp slot_end
        boolean is_available
        uuid booked_by_submission FK
        uuid created_by FK
        text notes
        timestamp created_at
    }

    audit_log {
        uuid id PK
        enum action
        string entity_type
        uuid entity_id
        string actor_type
        uuid actor_id
        string actor_ip
        jsonb details
        timestamp created_at
    }

    rate_limit_attempts {
        uuid id PK
        string ip_address
        string endpoint
        timestamp attempted_at
    }
```

### Enumeraties

```mermaid
graph LR
    subgraph "SubmissionStatus"
        S1[draft]
        S2[submitted]
        S3[under_review]
        S4[approved]
        S5[rejected]
        S6[forwarded]
        S7[completed]
    end

    subgraph "DocumentCategory"
        C1[formal_law]
        C2[circular]
        C3[implementation_policy]
        C4[work_instruction]
    end

    subgraph "DocumentClassification"
        CL1[public<br/>Mag gepubliceerd]
        CL2[claude_allowed<br/>Mag met AI]
        CL3[restricted<br/>Niet toegestaan]
    end
```

---

## API Routes

```mermaid
graph TB
    subgraph "Public Routes"
        P1["POST /api/submissions<br/>Create submission"]
        P2["GET /api/submissions/:slug<br/>Get submission"]
        P3["PUT /api/submissions/:slug<br/>Update submission"]
        P4["POST /api/submissions/:slug/submit<br/>Submit for review"]
        P5["POST /api/submissions/:slug/documents<br/>Upload document"]
        P6["POST /api/submissions/:slug/formal-law<br/>Add law link"]
        P7["DELETE /api/submissions/:slug/documents/:id<br/>Delete document"]
        P8["GET /api/calendar/available<br/>Available slots"]
        P9["POST /api/submissions/:slug/book-slot<br/>Book meeting"]
        P10["GET /api/faq<br/>FAQ content"]
    end

    subgraph "Auth Routes"
        A1["POST /api/admin/login<br/>Admin login"]
        A2["POST /api/admin/logout<br/>Admin logout"]
        A3["GET /api/admin/me<br/>Current admin"]
        A4["POST /api/uploader/login<br/>Uploader login"]
        A5["POST /api/uploader/logout<br/>Uploader logout"]
        A6["GET /api/uploader/me<br/>Current uploader"]
    end

    subgraph "Admin Routes (Protected)"
        AD1["GET /api/admin/submissions<br/>List all"]
        AD2["GET /api/admin/submissions/:id<br/>Get details"]
        AD3["PUT /api/admin/submissions/:id/status<br/>Update status"]
        AD4["POST /api/admin/submissions/:id/forward<br/>Forward"]
        AD5["GET /api/admin/submissions/:id/export<br/>Export JSON"]
        AD6["GET /api/admin/submissions/:id/export/files<br/>Export ZIP"]
        AD7["GET /api/admin/dashboard<br/>Statistics"]
        AD8["GET /api/admin/calendar/slots<br/>All slots"]
        AD9["POST /api/admin/calendar/slots<br/>Create slots"]
        AD10["DELETE /api/admin/calendar/slots/:id<br/>Delete slot"]
    end

    MW{Middleware<br/>require_admin}

    AD1 --> MW
    AD2 --> MW
    AD3 --> MW
    AD4 --> MW
    AD5 --> MW
    AD6 --> MW
    AD7 --> MW
    AD8 --> MW
    AD9 --> MW
    AD10 --> MW
```

---

## Authentication Flows

### Admin Login Flow

```mermaid
sequenceDiagram
    participant U as Admin User
    participant F as Frontend
    participant A as API Server
    participant DB as Database

    U->>F: Enter credentials
    F->>A: POST /api/admin/login

    A->>DB: Check rate limit
    alt Rate limit exceeded
        A-->>F: 429 Too Many Requests
        F-->>U: Show error
    end

    A->>DB: Record attempt
    A->>DB: Find user by username

    alt User not found
        A-->>F: 401 Unauthorized
        F-->>U: Invalid credentials
    end

    A->>A: Verify Argon2 password

    alt Password invalid
        A-->>F: 401 Unauthorized
        F-->>U: Invalid credentials
    end

    A->>A: Generate session token
    A->>A: Hash token (SHA256)
    A->>DB: Create admin_session
    A->>DB: Update last_login_at
    A->>DB: Log audit event

    A-->>F: 200 OK + Set-Cookie
    F->>F: Store cookie
    F-->>U: Redirect to dashboard
```

### Uploader Self-Service Login

```mermaid
sequenceDiagram
    participant U as Uploader
    participant F as Frontend
    participant A as API Server
    participant DB as Database

    U->>F: Enter slug + email
    F->>A: POST /api/uploader/login

    A->>DB: Check rate limit
    A->>DB: Record attempt

    A->>DB: Find submission by slug + email
    Note over A,DB: Case-insensitive match

    alt Not found
        A-->>F: 401 Unauthorized
        F-->>U: Invalid credentials
    end

    A->>A: Generate 4-hour session
    A->>DB: Create uploader_session
    A->>DB: Log audit event

    A-->>F: 200 OK + Set-Cookie
    Note over A,F: Returns submission info<br/>(no personal data)
    F-->>U: Redirect to dashboard
```

### Session Validation

```mermaid
flowchart TD
    A[Incoming Request] --> B{Has Cookie?}
    B -->|No| C[401 Unauthorized]
    B -->|Yes| D[Extract Token]

    D --> E[Hash Token SHA256]
    E --> F{Find Session<br/>in DB?}

    F -->|No| C
    F -->|Yes| G{Session<br/>Expired?}

    G -->|Yes| C
    G -->|No| H{User<br/>Active?}

    H -->|No| C
    H -->|Yes| I[Inject User<br/>into Request]

    I --> J[Continue to Handler]
```

---

## Submission Workflow

### Applicant Journey

```mermaid
stateDiagram-v2
    [*] --> CreateSubmission: POST /api/submissions

    state CreateSubmission {
        [*] --> GenerateSlug
        GenerateSlug --> SaveDraft
        SaveDraft --> [*]
    }

    CreateSubmission --> Draft: status = draft

    state Draft {
        [*] --> UpdateInfo
        UpdateInfo --> UploadDocs
        UploadDocs --> AddLaws
        AddLaws --> BookSlot
        BookSlot --> [*]
    }

    Draft --> Submitted: POST /submit

    state Submitted {
        [*] --> AwaitReview
        AwaitReview --> [*]
    }

    Submitted --> UnderReview: Admin action
    UnderReview --> Approved: Admin approves
    UnderReview --> Rejected: Admin rejects
    UnderReview --> Forwarded: Admin forwards

    Approved --> Completed: Process complete
    Forwarded --> Completed: External process done
    Rejected --> [*]
    Completed --> [*]
```

### Document Upload Flow

```mermaid
sequenceDiagram
    participant U as User
    participant F as Frontend
    participant A as API Server
    participant FS as File System
    participant DB as Database

    U->>F: Select file + category
    F->>F: Validate file size

    alt File too large
        F-->>U: Error: Max 50MB
    end

    F->>A: POST /api/submissions/:slug/documents
    Note over F,A: Multipart form data

    A->>A: Parse multipart
    A->>DB: Find submission

    alt Not in draft & no uploader session
        A-->>F: 403 Forbidden
    end

    A->>A: Validate MIME type
    A->>A: Check classification

    alt Classification = restricted
        A-->>F: 400 Bad Request
        Note over A,F: Restricted docs not allowed
    end

    A->>A: Generate UUID filename
    A->>FS: Write file to /data/:id/:filename
    A->>DB: Insert document record
    A->>DB: Log audit event

    A-->>F: 201 Created + document info
    F-->>U: Show uploaded file
```

---

## File Upload Flow

```mermaid
flowchart TD
    A[Upload Request] --> B{Submission<br/>Exists?}
    B -->|No| C[404 Not Found]
    B -->|Yes| D{Status Check}

    D -->|Draft| E[Allow Upload]
    D -->|Submitted| F{Has Uploader<br/>Session?}
    D -->|Other| G[403 Forbidden]

    F -->|Yes| E
    F -->|No| G

    E --> H{File Size<br/>≤ 50MB?}
    H -->|No| I[400 Too Large]
    H -->|Yes| J{Valid MIME<br/>Type?}

    J -->|No| K[400 Invalid Type]
    J -->|Yes| L{Classification?}

    L -->|Restricted| M[400 Not Allowed]
    L -->|Public/Claude| N[Generate Filename]

    N --> O[Write to /data/:id/:uuid]
    O --> P[Insert DB Record]
    P --> Q[Log Audit Event]
    Q --> R[201 Created]
```

### File Storage Structure

```mermaid
graph TD
    subgraph "/data (UPLOAD_DIR)"
        D1[submission-uuid-1/]
        D2[submission-uuid-2/]
        D3[submission-uuid-3/]
    end

    subgraph "submission-uuid-1/"
        F1[abc123-def456.pdf]
        F2[ghi789-jkl012.docx]
    end

    subgraph "submission-uuid-2/"
        F3[mno345-pqr678.pdf]
    end

    D1 --> F1
    D1 --> F2
    D2 --> F3
```

---

## Directory Structuur

```mermaid
graph TD
    ROOT[regelrecht-upload/]

    ROOT --> SRC[src/]
    ROOT --> FRONT[frontend/]
    ROOT --> DOC[doc/]
    ROOT --> GH[.github/]
    ROOT --> CONTAINER[Containerfile]
    ROOT --> COMPOSE[compose.yaml]

    subgraph "src/ - Rust Backend"
        SRC --> MAIN[main.rs]
        SRC --> CONFIG[config.rs]
        SRC --> DB[db/]
        SRC --> HANDLERS[handlers/]
        SRC --> MODELS[models/]
        SRC --> VALID[validation/]

        DB --> POOL[pool.rs]
        DB --> MIG[migrations/]

        HANDLERS --> AUTH[auth.rs]
        HANDLERS --> UAUTH[uploader_auth.rs]
        HANDLERS --> SUB[submissions.rs]
        HANDLERS --> ADM[admin.rs]
        HANDLERS --> CAL[calendar.rs]
        HANDLERS --> MID[middleware.rs]
    end

    subgraph "frontend/ - Web UI"
        FRONT --> INDEX[index.html]
        FRONT --> SUBMIT[submit.html]
        FRONT --> STATUS[status.html]
        FRONT --> FADMIN[admin/]
        FRONT --> FUPLOADER[uploader/]
        FRONT --> JS[js/]
        FRONT --> CSS[css/]
        FRONT --> WC[webcomponents/]
    end
```

---

## Deployment Architecture

```mermaid
graph TB
    subgraph "Container Runtime"
        subgraph "App Container"
            EP[entrypoint.sh]
            APP[regelrecht-upload binary]
            FE[/app/frontend/]
        end

        subgraph "Volumes"
            DATA[/data - uploads]
        end
    end

    subgraph "External Services"
        PG[(PostgreSQL 16)]
    end

    subgraph "CI/CD"
        GHA[GitHub Actions]
        GHCR[GitHub Container Registry]
    end

    EP --> APP
    APP --> FE
    APP --> DATA
    APP --> PG

    GHA --> GHCR
    GHCR --> APP
```

### Container Build Process

```mermaid
flowchart LR
    subgraph "Build Stage"
        B1[rust:1.85-bookworm]
        B2[Copy Cargo.toml]
        B3[Pin dependencies]
        B4[cargo build --release]
    end

    subgraph "Runtime Stage"
        R1[debian:bookworm-slim]
        R2[Install ca-certs, curl]
        R3[Copy binary]
        R4[Copy frontend]
        R5[Setup permissions]
    end

    B1 --> B2 --> B3 --> B4
    B4 --> R1
    R1 --> R2 --> R3 --> R4 --> R5
```

---

## Security Architecture

```mermaid
flowchart TD
    subgraph "Request Security"
        A[Incoming Request]
        B[Security Headers Middleware]
        C[Rate Limiting Check]
        D[Session Validation]
    end

    subgraph "Headers Applied"
        H1[X-Content-Type-Options: nosniff]
        H2[X-Frame-Options: DENY]
        H3[X-XSS-Protection: 1; mode=block]
        H4[Strict-Transport-Security]
        H5[Content-Security-Policy]
    end

    subgraph "Auth Security"
        S1[Argon2 Password Hashing]
        S2[SHA256 Token Hashing]
        S3[HttpOnly Cookies]
        S4[SameSite=Strict]
        S5[Secure flag in production]
    end

    subgraph "Rate Limiting"
        RL1[Admin login: 10/hour/IP]
        RL2[Uploader login: 10/hour/IP]
        RL3[Submission create: 20/hour/IP]
    end

    A --> B --> C --> D
    B --> H1
    B --> H2
    B --> H3
    B --> H4
    B --> H5

    D --> S1
    D --> S2
    D --> S3
    D --> S4
    D --> S5

    C --> RL1
    C --> RL2
    C --> RL3
```

---

## Environment Configuration

```mermaid
graph LR
    subgraph "Required"
        E1[DATABASE_URL]
        E2[ADMIN_USERNAME]
        E3[ADMIN_EMAIL]
        E4[ADMIN_PASSWORD_HASH<br/>or ADMIN_PASSWORD]
    end

    subgraph "Optional with Defaults"
        E5["HOST (0.0.0.0)"]
        E6["PORT (8080)"]
        E7["UPLOAD_DIR (/data)"]
        E8["FRONTEND_DIR (./frontend)"]
        E9["SESSION_EXPIRY_HOURS (8)"]
        E10["MAX_UPLOAD_SIZE (50MB)"]
        E11["ENVIRONMENT (development)"]
    end

    subgraph "Security"
        E12[CORS_ORIGINS]
        E13[TRUSTED_PROXIES]
    end

    CONFIG[config.rs] --> E1
    CONFIG --> E2
    CONFIG --> E3
    CONFIG --> E4
    CONFIG --> E5
    CONFIG --> E6
    CONFIG --> E7
    CONFIG --> E8
    CONFIG --> E9
    CONFIG --> E10
    CONFIG --> E11
    CONFIG --> E12
    CONFIG --> E13
```

---

## Periodic Tasks

```mermaid
flowchart TD
    subgraph "Hourly Cleanup Task"
        A[Spawn Async Task]
        B[Sleep 1 hour]
        C[Cleanup Rate Limits]
        D[Cleanup Admin Sessions]
        E[Cleanup Uploader Sessions]
        F[Log cleanup stats]
    end

    A --> B --> C --> D --> E --> F --> B

    C -->|DELETE| C1["rate_limit_attempts<br/>older than 1 hour"]
    D -->|DELETE| D1["admin_sessions<br/>where expires_at < NOW()"]
    E -->|DELETE| E1["uploader_sessions<br/>where expires_at < NOW()"]
```

---

## Tech Stack Summary

| Layer | Technology |
|-------|------------|
| **Backend** | Rust 1.85, Axum, Tokio |
| **Database** | PostgreSQL 16+ |
| **Auth** | Argon2, SHA256, Secure Cookies |
| **Frontend** | HTML5, Vanilla JS, Web Components |
| **Styling** | RijksOverheid Design System |
| **Container** | Podman/Docker, Multi-stage build |
| **CI/CD** | GitHub Actions, GHCR |

---

*Generated: 2024 | RegelRecht Upload Portal v1.0*
