/**
 * Uploader Dashboard Logic
 * Handles authenticated uploader session, document uploads, and management
 */

// State
let sessionData = null;
let submissionSlug = null;

// Status labels (Dutch)
const statusLabels = {
  draft: 'Concept',
  submitted: 'Ingediend',
  under_review: 'In behandeling',
  approved: 'Goedgekeurd',
  rejected: 'Afgewezen',
  forwarded: 'Doorgestuurd',
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

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  checkSession();
  setupEventListeners();
});

/**
 * Check if user is logged in, redirect to login if not
 */
async function checkSession() {
  try {
    const response = await fetch('/api/uploader/me', { credentials: 'include' });
    const result = await response.json();

    if (result.success) {
      sessionData = result.data;
      submissionSlug = result.data.slug;
      renderSessionInfo();
      renderDocumentList();
    } else {
      // Not logged in - redirect to login
      window.location.href = '/uploader-login.html';
    }
  } catch (e) {
    console.error('Session check failed:', e);
    window.location.href = '/uploader-login.html';
  }
}

/**
 * Setup event listeners for buttons
 */
function setupEventListeners() {
  // Logout
  document.getElementById('btn-logout').addEventListener('click', handleLogout);

  // Add formal law
  document.getElementById('btn-add-law').addEventListener('click', handleAddFormalLaw);

  // Upload document
  document.getElementById('btn-upload-doc').addEventListener('click', handleUploadDocument);

  // Classification change - show/hide warning
  const classificationSelect = document.getElementById('doc_classification');
  classificationSelect.addEventListener('change', handleClassificationChange);

  // File selection - enable upload button
  const fileUpload = document.getElementById('doc_file');
  fileUpload.addEventListener('change', handleFileChange);
}

/**
 * Render session info in the header bar
 */
function renderSessionInfo() {
  document.getElementById('display-slug').textContent = sessionData.slug;

  const statusBadge = document.getElementById('display-status');
  statusBadge.textContent = statusLabels[sessionData.status] || sessionData.status;
  statusBadge.className = `status-badge status-${sessionData.status}`;

  const expiresAt = new Date(sessionData.session_expires_at);
  document.getElementById('display-expires').textContent = expiresAt.toLocaleTimeString('nl-NL', {
    hour: '2-digit',
    minute: '2-digit'
  });
}

/**
 * Render document list
 */
function renderDocumentList() {
  const container = document.getElementById('document-list');
  const documents = sessionData.documents || [];

  if (documents.length === 0) {
    container.innerHTML = `
      <p style="color: var(--color-slate-500); text-align: center; padding: 20px;">
        Nog geen documenten toegevoegd
      </p>`;
    return;
  }

  container.innerHTML = documents.map(doc => `
    <div class="document-item" style="display: flex; justify-content: space-between; align-items: center; padding: 12px; border: 1px solid var(--color-slate-200); border-radius: 8px; margin-bottom: 8px;">
      <div class="document-info" style="display: flex; align-items: center; gap: 12px;">
        <div>
          <div class="document-name" style="font-weight: 500;">
            ${doc.external_url
              ? `<a href="${escapeHtml(doc.external_url)}" target="_blank" style="color: var(--color-primary);">${escapeHtml(doc.external_title || doc.external_url)}</a>`
              : escapeHtml(doc.filename) || 'Document'}
          </div>
          <div class="document-meta" style="font-size: 0.875rem; color: var(--color-slate-600);">
            ${categoryLabels[doc.category] || doc.category} · ${classificationLabels[doc.classification] || doc.classification}
            ${doc.file_size ? ` · ${formatFileSize(doc.file_size)}` : ''}
          </div>
        </div>
      </div>
      <button onclick="deleteDocument('${doc.id}')" style="background: none; border: none; color: #dc2626; cursor: pointer; padding: 8px;">
        Verwijderen
      </button>
    </div>
  `).join('');
}

/**
 * Handle logout
 */
async function handleLogout() {
  try {
    await fetch('/api/uploader/logout', {
      method: 'POST',
      credentials: 'include'
    });
  } catch (e) {
    // Continue even if logout request fails
  }
  window.location.href = '/uploader-login.html';
}

/**
 * Handle adding a formal law link
 */
async function handleAddFormalLaw() {
  const url = document.getElementById('formal_law_url').value.trim();
  const title = document.getElementById('formal_law_title').value.trim();

  if (!url) {
    showMessage('Vul een URL in.', 'error');
    return;
  }

  // Basic URL validation
  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    showMessage('Vul een geldige URL in (beginnend met http:// of https://).', 'error');
    return;
  }

  const btn = document.getElementById('btn-add-law');
  btn.setAttribute('loading', '');

  try {
    const response = await fetch(`/api/submissions/${submissionSlug}/formal-law`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        external_url: url,
        external_title: title || null
      }),
      credentials: 'include'
    });

    const result = await response.json();
    if (result.success) {
      // Add to local state and re-render
      sessionData.documents.push(result.data);
      renderDocumentList();
      // Clear inputs
      document.getElementById('formal_law_url').value = '';
      document.getElementById('formal_law_title').value = '';
      showMessage('Wet toegevoegd.', 'success');
    } else {
      showMessage(result.error || 'Kon wet niet toevoegen.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  } finally {
    btn.removeAttribute('loading');
  }
}

/**
 * Handle classification selection change
 */
function handleClassificationChange(e) {
  const classification = e.detail?.value || e.target?.value;
  const warning = document.getElementById('classification-warning');
  const uploadBtn = document.getElementById('btn-upload-doc');
  const uploadArea = document.getElementById('upload-area');

  if (classification === 'restricted') {
    warning.classList.add('show');
    uploadBtn.setAttribute('disabled', '');
    uploadArea.style.opacity = '0.5';
  } else {
    warning.classList.remove('show');
    uploadArea.style.opacity = '1';
    // Check if file is selected
    const fileUpload = document.getElementById('doc_file');
    const category = document.getElementById('doc_category').value;
    if (fileUpload.files && fileUpload.files.length > 0 && classification && category) {
      uploadBtn.removeAttribute('disabled');
    }
  }
}

/**
 * Handle file selection
 */
function handleFileChange(e) {
  const files = e.detail?.files || [];
  const classification = document.getElementById('doc_classification').value;
  const category = document.getElementById('doc_category').value;
  const uploadBtn = document.getElementById('btn-upload-doc');

  if (files.length > 0 && classification && classification !== 'restricted' && category) {
    uploadBtn.removeAttribute('disabled');
  } else {
    uploadBtn.setAttribute('disabled', '');
  }
}

/**
 * Handle document upload
 */
async function handleUploadDocument() {
  const category = document.getElementById('doc_category').value;
  const classification = document.getElementById('doc_classification').value;
  const fileUpload = document.getElementById('doc_file');
  const files = fileUpload.files;
  const uploadBtn = document.getElementById('btn-upload-doc');

  if (!category || !classification || !files || files.length === 0) {
    showMessage('Selecteer categorie, classificatie en bestand.', 'error');
    return;
  }

  if (classification === 'restricted') {
    showMessage('Beperkte documenten kunnen niet worden geüpload.', 'error');
    return;
  }

  const formData = new FormData();
  formData.append('file', files[0]);

  // Setup abort controller with 2 minute timeout
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 120000);

  uploadBtn.setAttribute('loading', '');
  uploadBtn.setAttribute('disabled', '');

  try {
    const url = `/api/submissions/${submissionSlug}/documents?category=${category}&classification=${classification}`;
    const response = await fetch(url, {
      method: 'POST',
      body: formData,
      credentials: 'include',
      signal: controller.signal
    });

    clearTimeout(timeoutId);

    // Handle specific HTTP status codes
    if (response.status === 413) {
      showMessage('Bestand is te groot. Maximum grootte is 50MB.', 'error');
      return;
    }

    if (response.status === 401) {
      showMessage('Sessie verlopen. Log opnieuw in.', 'error');
      setTimeout(() => { window.location.href = '/uploader-login.html'; }, 2000);
      return;
    }

    if (response.status === 429) {
      showMessage('Te veel uploads. Probeer het later opnieuw.', 'error');
      return;
    }

    const result = await response.json();
    if (result.success) {
      // Add to local state and re-render
      sessionData.documents.push(result.data);
      renderDocumentList();
      // Clear inputs
      fileUpload.clearFiles();
      document.getElementById('doc_category').value = '';
      document.getElementById('doc_classification').value = '';
      document.getElementById('classification-warning').classList.remove('show');
      showMessage('Document geüpload.', 'success');
    } else {
      showMessage(result.error || 'Kon document niet uploaden.', 'error');
    }
  } catch (e) {
    clearTimeout(timeoutId);
    if (e.name === 'AbortError') {
      showMessage('Upload timeout. Probeer het opnieuw met een kleiner bestand.', 'error');
    } else {
      showMessage('Upload mislukt. Probeer het opnieuw.', 'error');
    }
  } finally {
    uploadBtn.removeAttribute('loading');
    // Don't re-enable - form is cleared
  }
}

/**
 * Delete a document
 */
window.deleteDocument = async function(docId) {
  if (!confirm('Weet u zeker dat u dit document wilt verwijderen?')) {
    return;
  }

  try {
    const response = await fetch(`/api/submissions/${submissionSlug}/documents/${docId}`, {
      method: 'DELETE',
      credentials: 'include'
    });

    if (response.status === 401) {
      showMessage('Sessie verlopen. Log opnieuw in.', 'error');
      setTimeout(() => { window.location.href = '/uploader-login.html'; }, 2000);
      return;
    }

    const result = await response.json();
    if (result.success) {
      // Remove from local state and re-render
      sessionData.documents = sessionData.documents.filter(d => d.id !== docId);
      renderDocumentList();
      showMessage('Document verwijderd.', 'success');
    } else {
      showMessage(result.error || 'Kon document niet verwijderen.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
};

/**
 * Utility: Escape HTML to prevent XSS
 */
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

/**
 * Utility: Format file size
 */
function formatFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

/**
 * Utility: Show status message
 */
function showMessage(text, type) {
  const el = document.getElementById('status-message');
  el.textContent = text;
  el.className = `status-message ${type}`;
  el.style.display = 'block';
  setTimeout(() => { el.style.display = 'none'; }, 5000);
}
