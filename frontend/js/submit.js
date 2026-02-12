/**
 * Submission form logic
 */

// State
let currentStep = 1;
let submissionSlug = null;
let submissionId = null;
let documents = [];
let selectedSlotId = null;

// Elements
const steps = document.querySelectorAll('.step');
const stepPanels = {
  1: document.getElementById('step-1'),
  2: document.getElementById('step-2'),
  3: document.getElementById('step-3'),
  success: document.getElementById('step-success')
};

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  setupEventListeners();
});

function setupEventListeners() {
  // Navigation buttons
  document.getElementById('btn-next-1').addEventListener('click', handleStep1Next);
  document.getElementById('btn-back-2').addEventListener('click', () => goToStep(1));
  document.getElementById('btn-next-2').addEventListener('click', () => goToStep(3));
  document.getElementById('btn-back-3').addEventListener('click', () => goToStep(2));
  document.getElementById('btn-submit').addEventListener('click', handleSubmit);

  // Document upload
  document.getElementById('btn-add-law').addEventListener('click', handleAddFormalLaw);
  document.getElementById('btn-upload-doc').addEventListener('click', handleUploadDocument);

  // Classification change
  const classificationSelect = document.getElementById('doc_classification');
  classificationSelect.addEventListener('change', handleClassificationChange);

  // File selection
  const fileUpload = document.getElementById('doc_file');
  fileUpload.addEventListener('change', handleFileChange);
}

function goToStep(step) {
  currentStep = step;

  // Update step indicators
  steps.forEach((s, i) => {
    const stepNum = i + 1;
    s.classList.remove('active', 'completed');
    if (stepNum < step) s.classList.add('completed');
    if (stepNum === step) s.classList.add('active');
  });

  // Show/hide panels
  Object.entries(stepPanels).forEach(([key, panel]) => {
    panel.style.display = (key == step) ? 'block' : 'none';
  });

  // Load data for step 3
  if (step === 3) {
    loadAvailableSlots();
  }
}

async function handleStep1Next() {
  const name = document.getElementById('submitter_name').value.trim();
  const email = document.getElementById('submitter_email').value.trim();
  const org = document.getElementById('organization').value.trim();
  const dept = document.getElementById('organization_department').value.trim();

  if (!name || !org) {
    showMessage('Vul alle verplichte velden in.', 'error');
    return;
  }

  // Create submission
  try {
    const response = await fetch('/api/submissions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        submitter_name: name,
        submitter_email: email || null,
        organization: org,
        organization_department: dept || null
      })
    });

    const result = await response.json();
    if (result.success) {
      submissionSlug = result.data.slug;
      submissionId = result.data.id;
      goToStep(2);
    } else {
      showMessage(result.error || 'Er ging iets mis.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
}

async function handleAddFormalLaw() {
  const url = document.getElementById('formal_law_url').value.trim();
  const title = document.getElementById('formal_law_title').value.trim();

  if (!url) {
    showMessage('Vul een URL in.', 'error');
    return;
  }

  try {
    const response = await fetch(`/api/submissions/${submissionSlug}/formal-law`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        external_url: url,
        external_title: title || null
      })
    });

    const result = await response.json();
    if (result.success) {
      documents.push(result.data);
      renderDocumentList();
      document.getElementById('formal_law_url').value = '';
      document.getElementById('formal_law_title').value = '';
      showMessage('Wet toegevoegd.', 'success');
    } else {
      showMessage(result.error || 'Kon wet niet toevoegen.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
}

function handleClassificationChange(e) {
  const classification = e.detail?.value || e.target?.value;
  const warning = document.getElementById('classification-warning');
  const uploadBtn = document.getElementById('btn-upload-doc');
  const uploadArea = document.getElementById('upload-area');

  if (classification === 'restricted') {
    warning.style.display = 'block';
    uploadBtn.setAttribute('disabled', '');
    uploadArea.style.opacity = '0.5';
  } else {
    warning.style.display = 'none';
    uploadArea.style.opacity = '1';
    // Check if file is selected
    const fileUpload = document.getElementById('doc_file');
    if (fileUpload.files && fileUpload.files.length > 0 && classification) {
      uploadBtn.removeAttribute('disabled');
    }
  }
}

function handleFileChange(e) {
  const files = e.detail?.files || [];
  const classification = document.getElementById('doc_classification').value;
  const uploadBtn = document.getElementById('btn-upload-doc');

  if (files.length > 0 && classification && classification !== 'restricted') {
    uploadBtn.removeAttribute('disabled');
  } else {
    uploadBtn.setAttribute('disabled', '');
  }
}

async function handleUploadDocument() {
  const category = document.getElementById('doc_category').value;
  const classification = document.getElementById('doc_classification').value;
  const fileUpload = document.getElementById('doc_file');
  const files = fileUpload.files;

  if (!category || !classification || !files || files.length === 0) {
    showMessage('Selecteer categorie, classificatie en bestand.', 'error');
    return;
  }

  if (classification === 'restricted') {
    showMessage('Beperkte documenten kunnen niet worden geupload.', 'error');
    return;
  }

  const formData = new FormData();
  formData.append('file', files[0]);

  try {
    const url = `/api/submissions/${submissionSlug}/documents?category=${category}&classification=${classification}`;
    const response = await fetch(url, {
      method: 'POST',
      body: formData
    });

    const result = await response.json();
    if (result.success) {
      documents.push(result.data);
      renderDocumentList();
      fileUpload.clearFiles();
      document.getElementById('doc_category').value = '';
      document.getElementById('doc_classification').value = '';
      document.getElementById('btn-upload-doc').setAttribute('disabled', '');
      showMessage('Document geupload.', 'success');
    } else {
      showMessage(result.error || 'Kon document niet uploaden.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
}

function renderDocumentList() {
  const container = document.getElementById('document-list');

  if (documents.length === 0) {
    container.innerHTML = `
      <p style="color: var(--color-slate-500); text-align: center; padding: 20px;">
        Nog geen documenten toegevoegd
      </p>`;
    return;
  }

  const categoryLabels = {
    formal_law: 'Wet',
    circular: 'Circulaire',
    implementation_policy: 'Uitvoeringsbeleid',
    work_instruction: 'Werkinstructie'
  };

  const classificationBadges = {
    public: 'badge-public',
    claude_allowed: 'badge-claude',
    restricted: 'badge-restricted'
  };

  container.innerHTML = documents.map(doc => `
    <div class="document-item">
      <div class="document-info">
        <div class="document-icon">${doc.category === 'formal_law' ? '🔗' : '📄'}</div>
        <div>
          <div class="document-name">${escapeHtml(doc.external_title || doc.filename || 'Document')}</div>
          <div class="document-meta">
            ${escapeHtml(categoryLabels[doc.category] || doc.category)}
            <span class="badge ${classificationBadges[doc.classification] || ''}">${escapeHtml(doc.classification)}</span>
          </div>
        </div>
      </div>
      <button onclick="removeDocument('${escapeHtml(doc.id)}')" style="background: none; border: none; color: #dc2626; cursor: pointer;">
        Verwijderen
      </button>
    </div>
  `).join('');
}

window.removeDocument = async function(docId) {
  try {
    const response = await fetch(`/api/submissions/${submissionSlug}/documents/${docId}`, {
      method: 'DELETE'
    });

    const result = await response.json();
    if (result.success) {
      documents = documents.filter(d => d.id !== docId);
      renderDocumentList();
    } else {
      showMessage(result.error || 'Kon document niet verwijderen.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
};

async function loadAvailableSlots() {
  const container = document.getElementById('available-slots');

  try {
    const response = await fetch('/api/calendar/available');
    const result = await response.json();

    if (result.success && result.data.length > 0) {
      container.innerHTML = `
        <p style="margin-bottom: 16px;">Selecteer een beschikbaar tijdslot:</p>
        <div style="display: flex; flex-direction: column; gap: 8px;">
          ${result.data.map(slot => {
            const start = new Date(slot.slot_start);
            const end = new Date(slot.slot_end);
            const dateStr = start.toLocaleDateString('nl-NL', { weekday: 'long', day: 'numeric', month: 'long' });
            const timeStr = `${start.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })} - ${end.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })}`;
            return `
              <label style="display: flex; align-items: center; padding: 12px; border: 2px solid var(--color-slate-200); border-radius: 8px; cursor: pointer;"
                     onclick="selectSlot('${slot.id}', this)">
                <input type="radio" name="slot" value="${slot.id}" style="margin-right: 12px;">
                <div>
                  <div style="font-weight: 500;">${dateStr}</div>
                  <div style="font-size: 0.875rem; color: var(--color-slate-600);">${timeStr}</div>
                </div>
              </label>
            `;
          }).join('')}
        </div>
      `;
    } else {
      container.innerHTML = `
        <rr-help-text variant="info">
          Er zijn momenteel geen tijdsloten beschikbaar. U kunt uw inzending alsnog afronden en wij nemen contact met u op.
        </rr-help-text>
      `;
    }
  } catch (e) {
    container.innerHTML = `
      <rr-help-text variant="warning">
        Kon beschikbare tijdsloten niet laden. U kunt uw inzending alsnog afronden.
      </rr-help-text>
    `;
  }
}

window.selectSlot = function(slotId, element) {
  selectedSlotId = slotId;
  document.querySelectorAll('#available-slots label').forEach(el => {
    el.style.borderColor = 'var(--color-slate-200)';
  });
  element.style.borderColor = 'var(--color-primary)';
};

async function handleSubmit() {
  // Book slot if selected
  if (selectedSlotId) {
    try {
      await fetch(`/api/submissions/${submissionSlug}/book-slot`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ slot_id: selectedSlotId })
      });
    } catch (e) {
      // Continue even if slot booking fails
    }
  }

  // Submit the submission
  try {
    const response = await fetch(`/api/submissions/${submissionSlug}/submit`, {
      method: 'POST'
    });

    const result = await response.json();
    if (result.success) {
      document.getElementById('submission-slug').textContent = submissionSlug;
      goToStep('success');
    } else {
      showMessage(result.error || 'Kon inzending niet afronden.', 'error');
    }
  } catch (e) {
    showMessage('Kon geen verbinding maken met de server.', 'error');
  }
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
