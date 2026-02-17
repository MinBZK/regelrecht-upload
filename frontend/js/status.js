/**
 * Status lookup page logic
 */

// Elements
const lookupForm = document.getElementById('lookup-form');
const statusResult = document.getElementById('status-result');
const slugInput = document.getElementById('slug');

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  setupEventListeners();

  // Check for slug in URL query params
  const urlParams = new URLSearchParams(window.location.search);
  const slugParam = urlParams.get('slug');
  if (slugParam) {
    slugInput.value = slugParam;
    lookupSubmission(slugParam);
  }
});

function setupEventListeners() {
  document.getElementById('btn-lookup').addEventListener('click', handleLookup);
  document.getElementById('btn-new-lookup').addEventListener('click', resetForm);
  document.getElementById('btn-add-documents').addEventListener('click', handleAddDocuments);

  // Allow Enter key to submit
  slugInput.addEventListener('keyup', (e) => {
    if (e.key === 'Enter') handleLookup();
  });
}

function handleAddDocuments() {
  const slug = slugInput.value.trim();
  window.location.href = `/uploader-login.html?slug=${encodeURIComponent(slug)}`;
}

async function handleLookup() {
  const slug = slugInput.value.trim();

  if (!slug) {
    showMessage('Voer een referentiecode in.', 'error');
    return;
  }

  // Validate slug format (basic check)
  if (!slug.match(/^[a-z0-9-]+$/)) {
    showMessage('Ongeldige referentiecode. De code bevat alleen letters, cijfers en streepjes.', 'error');
    return;
  }

  await lookupSubmission(slug);
}

async function lookupSubmission(slug) {
  try {
    const response = await fetch(`/api/submissions/${encodeURIComponent(slug)}`);
    const result = await response.json();

    if (result.success) {
      renderSubmissionStatus(result.data);
      lookupForm.style.display = 'none';
      statusResult.style.display = 'block';

      // Update URL without reload
      const newUrl = `${window.location.pathname}?slug=${encodeURIComponent(slug)}`;
      window.history.pushState({ slug }, '', newUrl);
    } else {
      showMessage(result.error || 'Inzending niet gevonden. Controleer de referentiecode.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
}

function renderSubmissionStatus(sub) {
  const statusLabels = {
    draft: 'Concept',
    submitted: 'Ingediend',
    under_review: 'In behandeling',
    approved: 'Goedgekeurd',
    rejected: 'Afgewezen',
    forwarded: 'Doorgestuurd naar team',
    completed: 'Afgerond'
  };

  const categoryLabels = {
    formal_law: 'Formele wet',
    circular: 'Circulaire',
    implementation_policy: 'Uitvoeringsbeleid',
    work_instruction: 'Werkinstructie'
  };

  const classificationLabels = {
    public: 'Publiek',
    claude_allowed: 'Niet-EU AI toegestaan',
    restricted: 'Beperkt'
  };

  // Submission info
  document.getElementById('submission-info').innerHTML = `
    <span class="detail-label">Referentiecode:</span>
    <span class="detail-value"><code>${escapeHtml(sub.slug)}</code></span>
    <span class="detail-label">Naam:</span>
    <span class="detail-value">${escapeHtml(sub.submitter_name)}</span>
    <span class="detail-label">Organisatie:</span>
    <span class="detail-value">${escapeHtml(sub.organization)}${sub.organization_department ? ' - ' + escapeHtml(sub.organization_department) : ''}</span>
  `;

  // Status info
  document.getElementById('submission-status').innerHTML = `
    <span class="detail-label">Status:</span>
    <span class="detail-value"><span class="status-badge status-${sub.status}">${statusLabels[sub.status] || sub.status}</span></span>
    <span class="detail-label">Aangemaakt:</span>
    <span class="detail-value">${new Date(sub.created_at).toLocaleDateString('nl-NL', { day: 'numeric', month: 'long', year: 'numeric' })}</span>
    <span class="detail-label">Ingediend:</span>
    <span class="detail-value">${sub.submitted_at ? new Date(sub.submitted_at).toLocaleDateString('nl-NL', { day: 'numeric', month: 'long', year: 'numeric' }) : '-'}</span>
  `;

  // Show "Add Documents" button if submission has email and is not draft
  // (draft submissions can add documents without login via submit.html)
  const addDocsBtn = document.getElementById('btn-add-documents');
  if (sub.submitter_email && sub.status !== 'draft') {
    addDocsBtn.style.display = 'inline-flex';
  } else {
    addDocsBtn.style.display = 'none';
  }

  // Documents
  const docsContainer = document.getElementById('submission-documents');

  if (sub.documents && sub.documents.length > 0) {
    docsContainer.innerHTML = sub.documents.map(doc => `
      <div class="document-item" style="margin-bottom: 8px;">
        <div class="document-info">
          <div class="document-icon">${doc.category === 'formal_law' ? '🔗' : '📄'}</div>
          <div>
            <div class="document-name">
              ${doc.external_url
                ? `<a href="${escapeHtml(doc.external_url)}" target="_blank">${escapeHtml(doc.external_title || doc.external_url)}</a>`
                : escapeHtml(doc.filename) || 'Document'}
            </div>
            <div class="document-meta">
              ${categoryLabels[doc.category] || doc.category} | ${classificationLabels[doc.classification] || doc.classification}
            </div>
          </div>
        </div>
      </div>
    `).join('');
  } else {
    docsContainer.innerHTML = '<p style="color: var(--color-slate-500);">Geen documenten toegevoegd.</p>';
  }
}

function resetForm() {
  statusResult.style.display = 'none';
  lookupForm.style.display = 'block';
  slugInput.value = '';

  // Clear URL params
  window.history.pushState({}, '', window.location.pathname);
}

function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function showMessage(text, type) {
  const el = document.getElementById('status-message');
  el.textContent = text;
  el.className = `status-message ${type}`;
  el.style.display = 'block';
  setTimeout(() => { el.style.display = 'none'; }, 5000);
}
