/**
 * Submission form logic - 5-step wizard with client-side file staging
 *
 * CRITICAL DATA PROTECTION:
 * - Files are staged client-side only during Steps 1-4
 * - NO server uploads until final submission in Step 5
 * - All data uploaded atomically with contact info
 */

// =============================================================================
// STATE
// =============================================================================

let currentStep = 1;
const totalSteps = 5;

// Client-side staged data (NOT uploaded to server until final submit)
const stagedData = {
  // Privacy consent
  consentPrivacy: false,
  consentDocuments: false,

  // Staged documents (stored as File objects, not uploaded)
  documents: [], // Array of { id, file, category, classification, previewUrl }
  formalLaws: [], // Array of { id, url, title }

  // Selected time slot
  selectedSlot: null, // { id, start, end }

  // Contact information
  contact: {
    name: '',
    email: '',
    organization: '',
    department: ''
  }
};

// Unique ID counter for staged items
let stagedIdCounter = 0;

// =============================================================================
// ELEMENTS
// =============================================================================

const stepPanels = {
  1: () => document.getElementById('step-1'),
  2: () => document.getElementById('step-2'),
  3: () => document.getElementById('step-3'),
  4: () => document.getElementById('step-4'),
  5: () => document.getElementById('step-5'),
  success: () => document.getElementById('step-success')
};

// =============================================================================
// INITIALIZATION
// =============================================================================

document.addEventListener('DOMContentLoaded', () => {
  setupEventListeners();
  loadAvailableSlots();
});

function setupEventListeners() {
  // Step 1: Privacy
  document.getElementById('btn-next-1')?.addEventListener('click', handleStep1Next);

  // Step 2: Documents
  document.getElementById('btn-back-2')?.addEventListener('click', (e) => { e.preventDefault(); goToStep(1); });
  document.getElementById('btn-next-2')?.addEventListener('click', () => goToStep(3));
  document.getElementById('btn-add-law')?.addEventListener('click', handleAddFormalLaw);
  document.getElementById('btn-add-doc')?.addEventListener('click', handleStageDocument);

  // Category and Classification toggle change
  document.getElementById('doc_category')?.addEventListener('change', handleFileChange);
  document.getElementById('doc_classification')?.addEventListener('change', handleClassificationChange);

  // File selection
  document.getElementById('doc_file')?.addEventListener('change', handleFileChange);

  // Step 3: Planning
  document.getElementById('btn-back-3')?.addEventListener('click', (e) => { e.preventDefault(); goToStep(2); });
  document.getElementById('btn-next-3')?.addEventListener('click', () => goToStep(4));

  // Step 4: Contact
  document.getElementById('btn-back-4')?.addEventListener('click', (e) => { e.preventDefault(); goToStep(3); });
  document.getElementById('btn-next-4')?.addEventListener('click', handleStep4Next);

  // Step 5: Summary
  document.getElementById('btn-back-5')?.addEventListener('click', (e) => { e.preventDefault(); goToStep(4); });
  document.getElementById('btn-submit')?.addEventListener('click', handleFinalSubmit);

  // Edit links in summary
  document.getElementById('edit-documents')?.addEventListener('click', () => goToStep(2));
  document.getElementById('edit-planning')?.addEventListener('click', () => goToStep(3));
  document.getElementById('edit-contact')?.addEventListener('click', () => goToStep(4));
}

// =============================================================================
// NAVIGATION
// =============================================================================

function goToStep(step) {
  currentStep = step;

  // Update progress bar
  const progressBar = document.getElementById('progress-bar');
  if (progressBar) {
    progressBar.setAttribute('current', step);
  }

  // Show/hide panels
  Object.entries(stepPanels).forEach(([key, getPanelFn]) => {
    const panel = getPanelFn();
    if (panel) {
      panel.style.display = (key == step) ? 'block' : 'none';
    }
  });

  // Step-specific actions
  if (step === 3) {
    loadAvailableSlots();
  } else if (step === 5) {
    renderSummary();
  }

  // Scroll to top
  window.scrollTo({ top: 0, behavior: 'smooth' });
}

// =============================================================================
// STEP 1: PRIVACY CONSENT
// =============================================================================

function handleStep1Next() {
  const consentPrivacy = document.getElementById('consent_privacy').checked;
  const consentDocuments = document.getElementById('consent_documents').checked;

  if (!consentPrivacy || !consentDocuments) {
    showMessage('U moet akkoord gaan met beide voorwaarden om verder te gaan.', 'error');
    return;
  }

  // Store consent in staged data
  stagedData.consentPrivacy = consentPrivacy;
  stagedData.consentDocuments = consentDocuments;

  goToStep(2);
}

// =============================================================================
// STEP 2: DOCUMENTS (CLIENT-SIDE STAGING ONLY)
// =============================================================================

function handleAddFormalLaw() {
  const urlField = document.getElementById('formal_law_url');
  const titleField = document.getElementById('formal_law_title');
  const url = urlField.value.trim();
  const title = titleField.value.trim();

  if (!url) {
    showMessage('Vul een URL in.', 'error');
    return;
  }

  // Validate URL format
  if (!url.startsWith('https://wetten.overheid.nl/')) {
    showMessage('Gebruik een URL van wetten.overheid.nl', 'error');
    return;
  }

  // Stage the formal law (client-side only!)
  const formalLaw = {
    id: `law_${++stagedIdCounter}`,
    url,
    title: title || extractTitleFromUrl(url)
  };

  stagedData.formalLaws.push(formalLaw);
  renderStagedDocuments();

  // Clear inputs
  urlField.value = '';
  titleField.value = '';
  showMessage('Wet toegevoegd aan inzending.', 'success');
}

function extractTitleFromUrl(url) {
  // Try to extract a readable title from the URL
  const match = url.match(/BWBR\d+/);
  return match ? match[0] : 'Formele wet';
}

function handleClassificationChange(e) {
  const classification = e.detail?.value || '';
  const warning = document.getElementById('classification-warning');
  const addBtn = document.getElementById('btn-add-doc');
  const uploadArea = document.getElementById('upload-area');

  if (classification === 'restricted') {
    warning.style.display = 'block';
    addBtn.setAttribute('disabled', '');
    uploadArea.style.opacity = '0.5';
  } else {
    warning.style.display = 'none';
    uploadArea.style.opacity = '1';
    updateAddDocumentButton();
  }
}

function handleFileChange(e) {
  updateAddDocumentButton();
}

function updateAddDocumentButton() {
  const classification = document.getElementById('doc_classification')?.value;
  const category = document.getElementById('doc_category')?.value;
  const fileUpload = document.getElementById('doc_file');
  const files = fileUpload?.files;
  const addBtn = document.getElementById('btn-add-doc');

  if (files && files.length > 0 && classification && classification !== 'restricted' && category) {
    addBtn?.removeAttribute('disabled');
  } else {
    addBtn?.setAttribute('disabled', '');
  }
}

function handleStageDocument() {
  const categoryEl = document.getElementById('doc_category');
  const classificationEl = document.getElementById('doc_classification');
  const fileUpload = document.getElementById('doc_file');

  const category = categoryEl?.value;
  const classification = classificationEl?.value;
  const files = fileUpload?.files;

  if (!category || !classification || !files || files.length === 0) {
    showMessage('Selecteer categorie, classificatie en bestand.', 'error');
    return;
  }

  if (classification === 'restricted') {
    showMessage('Beperkte documenten kunnen niet worden ingediend.', 'error');
    return;
  }

  const file = files[0];

  // Validate file size (50MB max)
  const maxSize = 50 * 1024 * 1024;
  if (file.size > maxSize) {
    showMessage('Bestand is te groot. Maximum grootte is 50MB.', 'error');
    return;
  }

  // Stage the document (client-side only!)
  const stagedDoc = {
    id: `doc_${++stagedIdCounter}`,
    file,
    filename: file.name,
    size: file.size,
    category,
    classification,
    previewUrl: URL.createObjectURL(file) // For local preview only
  };

  stagedData.documents.push(stagedDoc);
  renderStagedDocuments();

  // Clear inputs
  fileUpload.clearFiles?.();
  categoryEl.value = '';
  classificationEl.value = '';
  document.getElementById('btn-add-doc')?.setAttribute('disabled', '');

  showMessage('Document toegevoegd aan inzending.', 'success');
}

function renderStagedDocuments() {
  const container = document.getElementById('document-list');
  if (!container) return;

  const allDocs = [
    ...stagedData.formalLaws.map(law => ({ ...law, type: 'law' })),
    ...stagedData.documents.map(doc => ({ ...doc, type: 'document' }))
  ];

  if (allDocs.length === 0) {
    container.innerHTML = '<p class="empty-state">Nog geen documenten toegevoegd</p>';
    return;
  }

  const categoryLabels = {
    formal_law: 'Wet',
    circular: 'Circulaire',
    implementation_policy: 'Beleidsregel',
    work_instruction: 'Instructie'
  };

  const classificationLabels = {
    public: 'Openbaar',
    claude_allowed: 'AI-verwerking',
    restricted: 'Beperkt'
  };

  container.innerHTML = allDocs.map(doc => {
    if (doc.type === 'law') {
      return `
        <div class="document-item">
          <div class="document-info">
            <div class="document-icon">🔗</div>
            <div>
              <div class="document-name">${escapeHtml(doc.title)}</div>
              <div class="document-meta">Formele wet</div>
            </div>
          </div>
          <button type="button" class="delete-btn" onclick="confirmDeleteItem('${doc.id}', 'law', this)">
            Verwijderen
          </button>
        </div>
      `;
    } else {
      return `
        <div class="document-item">
          <div class="document-info">
            <div class="document-icon">📄</div>
            <div>
              <div class="document-name">${escapeHtml(doc.filename)}</div>
              <div class="document-meta">
                ${categoryLabels[doc.category] || doc.category}
                <span class="badge badge-${doc.classification}">${classificationLabels[doc.classification]}</span>
              </div>
            </div>
          </div>
          <button type="button" class="delete-btn" onclick="confirmDeleteItem('${doc.id}', 'document', this)">
            Verwijderen
          </button>
        </div>
      `;
    }
  }).join('');
}

// Two-step delete: first click shows confirm, second click deletes
window.confirmDeleteItem = function(id, type, button) {
  // Show confirm state
  button.textContent = 'Bevestig';
  button.classList.add('confirm-delete');
  button.onclick = () => removeStagedItem(id, type);

  // Reset after 3 seconds if not confirmed
  setTimeout(() => {
    if (button.classList.contains('confirm-delete')) {
      button.textContent = 'Verwijderen';
      button.classList.remove('confirm-delete');
      button.onclick = () => confirmDeleteItem(id, type, button);
    }
  }, 3000);
};

function removeStagedItem(id, type) {
  if (type === 'law') {
    stagedData.formalLaws = stagedData.formalLaws.filter(l => l.id !== id);
  } else {
    const doc = stagedData.documents.find(d => d.id === id);
    if (doc?.previewUrl) {
      URL.revokeObjectURL(doc.previewUrl);
    }
    stagedData.documents = stagedData.documents.filter(d => d.id !== id);
  }
  renderStagedDocuments();
}

// =============================================================================
// STEP 3: PLANNING
// =============================================================================

async function loadAvailableSlots() {
  const container = document.getElementById('available-slots');
  if (!container) return;

  try {
    const response = await fetch('/api/calendar/available');
    const result = await response.json();

    if (result.success && result.data.length > 0) {
      container.innerHTML = `
        <div class="slots-grid">
          ${result.data.map(slot => {
            const start = new Date(slot.slot_start);
            const end = new Date(slot.slot_end);
            const dateStr = start.toLocaleDateString('nl-NL', { weekday: 'long', day: 'numeric', month: 'long' });
            const timeStr = `${start.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })} - ${end.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })}`;
            const isSelected = stagedData.selectedSlot?.id === slot.id;

            return `
              <div class="slot-option ${isSelected ? 'selected' : ''}" onclick="selectSlot('${slot.id}', '${slot.slot_start}', '${slot.slot_end}', this)">
                <input type="radio" name="slot" value="${slot.id}" ${isSelected ? 'checked' : ''}>
                <div class="slot-details">
                  <div class="slot-date">${dateStr}</div>
                  <div class="slot-time">${timeStr}</div>
                </div>
              </div>
            `;
          }).join('')}
        </div>
      `;
    } else {
      container.innerHTML = `
        <div class="info-box">
          <p>Er zijn momenteel geen tijdsloten beschikbaar.</p>
          <p>Wij nemen contact met u op om een afspraak in te plannen.</p>
        </div>
      `;
    }
  } catch (e) {
    container.innerHTML = `
      <div class="warning-box">
        <p>Kon beschikbare tijdsloten niet laden.</p>
        <p>U kunt uw inzending alsnog afronden zonder tijdslot.</p>
      </div>
    `;
  }
}

window.selectSlot = function(slotId, slotStart, slotEnd, element) {
  stagedData.selectedSlot = {
    id: slotId,
    start: new Date(slotStart),
    end: new Date(slotEnd)
  };

  // Update visual selection
  document.querySelectorAll('.slot-option').forEach(el => {
    el.classList.remove('selected');
  });
  element.classList.add('selected');
  element.querySelector('input[type="radio"]').checked = true;
};

// =============================================================================
// STEP 4: CONTACT INFORMATION
// =============================================================================

function handleStep4Next() {
  const name = document.getElementById('submitter_name')?.value?.trim();
  const email = document.getElementById('submitter_email')?.value?.trim();
  const org = document.getElementById('organization')?.value?.trim();
  const dept = document.getElementById('organization_department')?.value?.trim();

  if (!name || !email || !org) {
    showMessage('Vul alle verplichte velden in.', 'error');
    return;
  }

  // Validate email format
  if (!isValidEmail(email)) {
    showMessage('Vul een geldig e-mailadres in.', 'error');
    return;
  }

  // Store contact info (client-side only!)
  stagedData.contact = { name, email, organization: org, department: dept || '' };

  goToStep(5);
}

function isValidEmail(email) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

// =============================================================================
// STEP 5: SUMMARY & FINAL SUBMIT
// =============================================================================

function renderSummary() {
  // Render documents summary
  const docsContainer = document.getElementById('summary-documents');
  if (docsContainer) {
    const allDocs = [...stagedData.formalLaws, ...stagedData.documents];
    if (allDocs.length === 0) {
      docsContainer.innerHTML = '<p class="empty-state">Geen documenten toegevoegd</p>';
    } else {
      docsContainer.innerHTML = `
        <ul class="summary-list">
          ${stagedData.formalLaws.map(law => `
            <li>🔗 ${escapeHtml(law.title)} <span class="meta">(Formele wet)</span></li>
          `).join('')}
          ${stagedData.documents.map(doc => `
            <li>📄 ${escapeHtml(doc.filename)} <span class="meta">(${getCategoryLabel(doc.category)}, ${getClassificationLabel(doc.classification)})</span></li>
          `).join('')}
        </ul>
      `;
    }
  }

  // Render planning summary
  const planningContainer = document.getElementById('summary-planning');
  if (planningContainer) {
    if (stagedData.selectedSlot) {
      const slot = stagedData.selectedSlot;
      const dateStr = slot.start.toLocaleDateString('nl-NL', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
      const timeStr = `${slot.start.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })} - ${slot.end.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })}`;
      planningContainer.innerHTML = `<p>${dateStr}<br>${timeStr}</p>`;
    } else {
      planningContainer.innerHTML = '<p class="empty-state">Geen tijdslot geselecteerd - wij nemen contact met u op</p>';
    }
  }

  // Render contact summary
  const contactContainer = document.getElementById('summary-contact');
  if (contactContainer) {
    const c = stagedData.contact;
    if (c.name) {
      contactContainer.innerHTML = `
        <p><strong>${escapeHtml(c.name)}</strong></p>
        <p>${escapeHtml(c.email)}</p>
        <p>${escapeHtml(c.organization)}${c.department ? `, ${escapeHtml(c.department)}` : ''}</p>
      `;
    } else {
      contactContainer.innerHTML = '<p class="empty-state">Geen gegevens ingevuld</p>';
    }
  }
}

function getCategoryLabel(category) {
  const labels = {
    circular: 'Circulaire',
    implementation_policy: 'Beleidsregel',
    work_instruction: 'Instructie'
  };
  return labels[category] || category;
}

function getClassificationLabel(classification) {
  const labels = {
    public: 'Openbaar',
    claude_allowed: 'AI-verwerking',
    restricted: 'Beperkt'
  };
  return labels[classification] || classification;
}

async function handleFinalSubmit() {
  const submitBtn = document.getElementById('btn-submit');

  // Validate we have minimum required data
  if (!stagedData.consentPrivacy || !stagedData.consentDocuments) {
    showMessage('Privacy toestemming is vereist.', 'error');
    return;
  }

  if (!stagedData.contact.name || !stagedData.contact.email || !stagedData.contact.organization) {
    showMessage('Contactgegevens zijn niet volledig.', 'error');
    return;
  }

  // Disable submit button
  if (submitBtn) {
    submitBtn.setAttribute('loading', '');
    submitBtn.setAttribute('disabled', '');
  }

  try {
    // STEP 1: Create submission with contact info
    const submissionResponse = await fetch('/api/submissions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        submitter_name: stagedData.contact.name,
        submitter_email: stagedData.contact.email,
        organization: stagedData.contact.organization,
        organization_department: stagedData.contact.department || null
      })
    });

    const submissionResult = await submissionResponse.json();
    if (!submissionResult.success) {
      throw new Error(submissionResult.error || 'Kon inzending niet aanmaken.');
    }

    const slug = submissionResult.data.slug;

    // STEP 2: Upload all staged formal laws
    for (const law of stagedData.formalLaws) {
      await fetch(`/api/submissions/${slug}/formal-law`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          external_url: law.url,
          external_title: law.title || null
        })
      });
    }

    // STEP 3: Upload all staged documents
    for (const doc of stagedData.documents) {
      const formData = new FormData();
      formData.append('file', doc.file);

      const url = `/api/submissions/${slug}/documents?category=${doc.category}&classification=${doc.classification}`;
      await fetch(url, {
        method: 'POST',
        body: formData
      });
    }

    // STEP 4: Book slot if selected
    if (stagedData.selectedSlot) {
      await fetch(`/api/submissions/${slug}/book-slot`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ slot_id: stagedData.selectedSlot.id })
      });
    }

    // STEP 5: Submit the submission
    const finalResponse = await fetch(`/api/submissions/${slug}/submit`, {
      method: 'POST'
    });

    const finalResult = await finalResponse.json();
    if (!finalResult.success) {
      throw new Error(finalResult.error || 'Kon inzending niet afronden.');
    }

    // Success!
    document.getElementById('submission-slug').textContent = slug;

    // Update status link with the slug
    const statusLink = document.getElementById('link-status');
    if (statusLink) {
      statusLink.href = `/status.html?slug=${encodeURIComponent(slug)}`;
    }

    // Clean up preview URLs
    stagedData.documents.forEach(doc => {
      if (doc.previewUrl) URL.revokeObjectURL(doc.previewUrl);
    });

    goToStep('success');

  } catch (e) {
    showMessage(e.message || 'Er ging iets mis bij het indienen.', 'error');
  } finally {
    if (submitBtn) {
      submitBtn.removeAttribute('loading');
      submitBtn.removeAttribute('disabled');
    }
  }
}

// =============================================================================
// UTILITIES
// =============================================================================

function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function showMessage(text, type) {
  const el = document.getElementById('status-message');
  if (!el) return;

  el.textContent = text;
  el.className = `status-message ${type}`;
  el.style.display = 'block';

  // Scroll to message
  el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });

  setTimeout(() => { el.style.display = 'none'; }, 5000);
}
