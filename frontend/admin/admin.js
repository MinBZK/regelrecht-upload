/**
 * Admin Portal JavaScript
 */

let currentPage = 1;
let currentUser = null;

// Authentication
export async function checkAuth() {
  try {
    const response = await fetch('/api/admin/me', { credentials: 'include' });
    const result = await response.json();
    if (result.success) {
      currentUser = result.data;
      return result.data;
    }
  } catch (e) {
    console.error('Auth check failed:', e);
  }
  window.location.href = '/admin/';
  return null;
}

export async function logout(e) {
  e?.preventDefault();
  await fetch('/api/admin/logout', { method: 'POST', credentials: 'include' });
  window.location.href = '/admin/';
}

// Dashboard
export async function loadDashboard() {
  try {
    const response = await fetch('/api/admin/dashboard', { credentials: 'include' });
    const result = await response.json();
    if (result.success) {
      const stats = result.data.submissions_by_status || {};
      document.getElementById('stat-draft').textContent = stats.draft || 0;
      document.getElementById('stat-submitted').textContent = stats.submitted || 0;
      document.getElementById('stat-review').textContent = stats.under_review || 0;
      document.getElementById('stat-completed').textContent = (stats.completed || 0) + (stats.forwarded || 0);
      document.getElementById('stat-documents').textContent = result.data.total_documents || 0;
      document.getElementById('stat-slots').textContent = result.data.available_meeting_slots || 0;
    }
  } catch (e) {
    console.error('Failed to load dashboard:', e);
  }
}

// Submissions
export async function loadSubmissions(page = 1) {
  currentPage = page;
  const search = document.getElementById('search')?.value || '';
  const status = document.getElementById('filter-status')?.value || '';

  let url = `/api/admin/submissions?page=${page}&per_page=20`;
  if (search) url += `&search=${encodeURIComponent(search)}`;
  if (status) url += `&status=${status}`;

  try {
    const response = await fetch(url, { credentials: 'include' });
    const result = await response.json();

    if (result.success) {
      renderSubmissionsTable(result.data);
      renderPagination(result.data);
    }
  } catch (e) {
    console.error('Failed to load submissions:', e);
    document.getElementById('submissions-table').innerHTML = '<p>Kon inzendingen niet laden.</p>';
  }
}

function renderSubmissionsTable(data) {
  const container = document.getElementById('submissions-table');
  const statusLabels = {
    draft: 'Concept',
    submitted: 'Ingediend',
    under_review: 'In behandeling',
    approved: 'Goedgekeurd',
    rejected: 'Afgewezen',
    forwarded: 'Doorgestuurd',
    completed: 'Afgerond'
  };

  if (data.items.length === 0) {
    container.innerHTML = '<p>Geen inzendingen gevonden.</p>';
    return;
  }

  container.innerHTML = `
    <table class="data-table">
      <thead>
        <tr>
          <th>Referentie</th>
          <th>Naam</th>
          <th>Organisatie</th>
          <th>Status</th>
          <th>Documenten</th>
          <th>Datum</th>
          <th>Acties</th>
        </tr>
      </thead>
      <tbody>
        ${data.items.map(sub => `
          <tr>
            <td><code>${sub.slug}</code></td>
            <td>${escapeHtml(sub.submitter_name)}</td>
            <td>${escapeHtml(sub.organization)}</td>
            <td><span class="status-badge status-${sub.status}">${statusLabels[sub.status] || sub.status}</span></td>
            <td>${sub.documents?.length || 0}</td>
            <td>${new Date(sub.created_at).toLocaleDateString('nl-NL')}</td>
            <td>
              <button class="action-btn" onclick="openSubmissionModal('${sub.id}')">Bekijken</button>
            </td>
          </tr>
        `).join('')}
      </tbody>
    </table>
  `;
}

function renderPagination(data) {
  const container = document.getElementById('pagination');
  if (data.total_pages <= 1) {
    container.innerHTML = '';
    return;
  }

  let html = '';
  html += `<button class="pagination-btn" onclick="loadSubmissions(${currentPage - 1})" ${currentPage === 1 ? 'disabled' : ''}>&laquo;</button>`;

  for (let i = 1; i <= data.total_pages; i++) {
    if (i === 1 || i === data.total_pages || (i >= currentPage - 2 && i <= currentPage + 2)) {
      html += `<button class="pagination-btn ${i === currentPage ? 'active' : ''}" onclick="loadSubmissions(${i})">${i}</button>`;
    } else if (i === currentPage - 3 || i === currentPage + 3) {
      html += '<span>...</span>';
    }
  }

  html += `<button class="pagination-btn" onclick="loadSubmissions(${currentPage + 1})" ${currentPage === data.total_pages ? 'disabled' : ''}>&raquo;</button>`;
  container.innerHTML = html;
}

// Submission Modal
export async function openSubmissionModal(id) {
  const modal = document.getElementById('modal');
  const body = document.getElementById('modal-body');

  body.innerHTML = '<p>Laden...</p>';
  modal.style.display = 'flex';

  try {
    const response = await fetch(`/api/admin/submissions/${id}`, { credentials: 'include' });
    const result = await response.json();

    if (result.success) {
      renderSubmissionDetail(result.data);
    } else {
      body.innerHTML = '<p>Kon inzending niet laden.</p>';
    }
  } catch (e) {
    body.innerHTML = '<p>Fout bij laden.</p>';
  }
}

function renderSubmissionDetail(sub) {
  const body = document.getElementById('modal-body');
  const statusLabels = {
    draft: 'Concept', submitted: 'Ingediend', under_review: 'In behandeling',
    approved: 'Goedgekeurd', rejected: 'Afgewezen', forwarded: 'Doorgestuurd', completed: 'Afgerond'
  };
  const categoryLabels = {
    formal_law: 'Formele wet', circular: 'Circulaire',
    implementation_policy: 'Uitvoeringsbeleid', work_instruction: 'Werkinstructie'
  };

  body.innerHTML = `
    <h2 style="margin-bottom: 24px;">Inzending: ${escapeHtml(sub.slug)}</h2>

    <div class="detail-section">
      <h3>Contactgegevens</h3>
      <div class="detail-grid">
        <span class="detail-label">Naam:</span>
        <span class="detail-value">${escapeHtml(sub.submitter_name)}</span>
        <span class="detail-label">E-mail:</span>
        <span class="detail-value">${escapeHtml(sub.submitter_email) || '-'}</span>
        <span class="detail-label">Organisatie:</span>
        <span class="detail-value">${escapeHtml(sub.organization)}</span>
        <span class="detail-label">Afdeling:</span>
        <span class="detail-value">${escapeHtml(sub.organization_department) || '-'}</span>
      </div>
    </div>

    <div class="detail-section">
      <h3>Status</h3>
      <div class="detail-grid">
        <span class="detail-label">Huidige status:</span>
        <span class="detail-value"><span class="status-badge status-${sub.status}">${statusLabels[sub.status]}</span></span>
        <span class="detail-label">Aangemaakt:</span>
        <span class="detail-value">${new Date(sub.created_at).toLocaleString('nl-NL')}</span>
        <span class="detail-label">Ingediend:</span>
        <span class="detail-value">${sub.submitted_at ? new Date(sub.submitted_at).toLocaleString('nl-NL') : '-'}</span>
      </div>
    </div>

    <div class="detail-section">
      <h3>Documenten (${sub.documents?.length || 0})</h3>
      ${sub.documents?.length ? sub.documents.map(doc => `
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
                ${categoryLabels[doc.category] || doc.category} |
                <span class="badge badge-${doc.classification === 'public' ? 'public' : doc.classification === 'claude_allowed' ? 'claude' : 'restricted'}">${doc.classification}</span>
              </div>
            </div>
          </div>
        </div>
      `).join('') : '<p>Geen documenten</p>'}
    </div>

    <div class="detail-section">
      <h3>Acties</h3>
      <div style="display: flex; gap: 12px; flex-wrap: wrap;">
        <rr-select-field id="modal-status" style="width: 200px;">
          <option value="draft" ${sub.status === 'draft' ? 'selected' : ''}>Concept</option>
          <option value="submitted" ${sub.status === 'submitted' ? 'selected' : ''}>Ingediend</option>
          <option value="under_review" ${sub.status === 'under_review' ? 'selected' : ''}>In behandeling</option>
          <option value="approved" ${sub.status === 'approved' ? 'selected' : ''}>Goedgekeurd</option>
          <option value="rejected" ${sub.status === 'rejected' ? 'selected' : ''}>Afgewezen</option>
          <option value="forwarded" ${sub.status === 'forwarded' ? 'selected' : ''}>Doorgestuurd</option>
          <option value="completed" ${sub.status === 'completed' ? 'selected' : ''}>Afgerond</option>
        </rr-select-field>
        <rr-button variant="primary" onclick="updateStatus('${sub.id}')">Status bijwerken</rr-button>
        <rr-button variant="secondary" onclick="forwardSubmission('${sub.id}')">Doorsturen naar team</rr-button>
      </div>
    </div>
  `;
}

export function closeModal() {
  document.getElementById('modal').style.display = 'none';
}

export async function updateStatus(id) {
  const status = document.getElementById('modal-status').value;

  try {
    const response = await fetch(`/api/admin/submissions/${id}/status`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ status })
    });

    const result = await response.json();
    if (result.success) {
      closeModal();
      loadSubmissions(currentPage);
    } else {
      alert(result.error || 'Kon status niet bijwerken.');
    }
  } catch (e) {
    alert('Fout bij bijwerken status.');
  }
}

export async function forwardSubmission(id) {
  const forwardTo = prompt('Doorsturen naar (bijv. RegelRecht team):');
  if (!forwardTo) return;

  try {
    const response = await fetch(`/api/admin/submissions/${id}/forward`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ forward_to: forwardTo })
    });

    const result = await response.json();
    if (result.success) {
      closeModal();
      loadSubmissions(currentPage);
    } else {
      alert(result.error || 'Kon niet doorsturen.');
    }
  } catch (e) {
    alert('Fout bij doorsturen.');
  }
}

// Calendar
export async function loadCalendarSlots() {
  try {
    const response = await fetch('/api/admin/calendar/slots', { credentials: 'include' });
    const result = await response.json();

    if (result.success) {
      renderCalendarSlots(result.data);
    }
  } catch (e) {
    console.error('Failed to load slots:', e);
  }
}

function renderCalendarSlots(slots) {
  const container = document.getElementById('slots-list');

  if (slots.length === 0) {
    container.innerHTML = '<p>Geen tijdsloten gepland.</p>';
    return;
  }

  container.innerHTML = slots.map(slot => {
    const start = new Date(slot.slot_start);
    const end = new Date(slot.slot_end);
    const dateStr = start.toLocaleDateString('nl-NL', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
    const timeStr = `${start.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })} - ${end.toLocaleTimeString('nl-NL', { hour: '2-digit', minute: '2-digit' })}`;
    const isBooked = slot.booked_by_submission !== null;

    return `
      <div class="slot-item ${isBooked ? 'booked' : ''}">
        <div class="slot-info">
          <span class="slot-date">${dateStr}</span>
          <span class="slot-time">${timeStr}</span>
          <span class="slot-status">${isBooked ? 'Geboekt' : 'Beschikbaar'}</span>
          ${slot.notes ? `<span class="slot-status">${escapeHtml(slot.notes)}</span>` : ''}
        </div>
        ${!isBooked ? `<button class="action-btn danger" onclick="deleteSlot('${slot.id}')">Verwijderen</button>` : ''}
      </div>
    `;
  }).join('');
}

export async function addCalendarSlot() {
  const date = document.getElementById('slot-date').value;
  const startTime = document.getElementById('slot-start').value;
  const endTime = document.getElementById('slot-end').value;
  const notes = document.getElementById('slot-notes').value;

  if (!date || !startTime || !endTime) {
    alert('Vul datum en tijden in.');
    return;
  }

  const slotStart = new Date(`${date}T${startTime}:00`);
  const slotEnd = new Date(`${date}T${endTime}:00`);

  try {
    const response = await fetch('/api/admin/calendar/slots', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify([{
        slot_start: slotStart.toISOString(),
        slot_end: slotEnd.toISOString(),
        notes: notes || null
      }])
    });

    const result = await response.json();
    if (result.success) {
      document.getElementById('slot-notes').value = '';
      loadCalendarSlots();
    } else {
      alert(result.error || 'Kon tijdslot niet toevoegen.');
    }
  } catch (e) {
    alert('Fout bij toevoegen tijdslot.');
  }
}

export async function deleteSlot(id) {
  if (!confirm('Weet u zeker dat u dit tijdslot wilt verwijderen?')) return;

  try {
    const response = await fetch(`/api/admin/calendar/slots/${id}`, {
      method: 'DELETE',
      credentials: 'include'
    });

    const result = await response.json();
    if (result.success) {
      loadCalendarSlots();
    } else {
      alert(result.error || 'Kon tijdslot niet verwijderen.');
    }
  } catch (e) {
    alert('Fout bij verwijderen tijdslot.');
  }
}

// Helpers
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Make loadSubmissions globally available for pagination
window.loadSubmissions = loadSubmissions;
