/**
 * RegelRecht File Upload Component with drag-drop support
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRFileUpload extends RRLocalBase {
  static componentName = 'rr-file-upload';

  static get observedAttributes() {
    return ['accept', 'disabled', 'multiple'];
  }

  constructor() {
    super();
    this._files = [];
  }

  get files() { return this._files; }
  get accept() { return this.getAttribute('accept') || '.pdf,.doc,.docx,.odt,.txt,.md'; }
  get disabled() { return this.getBooleanAttribute('disabled'); }
  get multiple() { return this.getBooleanAttribute('multiple'); }

  _getStyles() {
    return `
      :host { display: block; font-family: var(--rr-font-family-sans, system-ui, sans-serif); }
      .dropzone {
        border: 2px dashed #94a3b8;
        border-radius: 8px;
        padding: 32px;
        text-align: center;
        transition: all 0.2s;
        cursor: pointer;
        background: #f8fafc;
      }
      .dropzone:hover, .dropzone.dragover {
        border-color: var(--color-primary, #154273);
        background: #eff6ff;
      }
      .dropzone.disabled { opacity: 0.5; cursor: not-allowed; }
      .icon { font-size: 48px; margin-bottom: 16px; color: #64748b; }
      .title { font-weight: 600; color: #334155; margin-bottom: 8px; }
      .subtitle { font-size: 0.875rem; color: #64748b; }
      .file-input { display: none; }
      .file-list { margin-top: 16px; }
      .file-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 12px;
        background: #fff;
        border: 1px solid #e2e8f0;
        border-radius: 6px;
        margin-bottom: 8px;
      }
      .file-name { font-size: 0.875rem; color: #334155; }
      .file-size { font-size: 0.75rem; color: #64748b; margin-left: 8px; }
      .remove-btn {
        background: none;
        border: none;
        color: #ef4444;
        cursor: pointer;
        padding: 4px 8px;
        font-size: 0.875rem;
      }
      .remove-btn:hover { text-decoration: underline; }
    `;
  }

  render() {
    this.shadowRoot.innerHTML = `
      <div class="dropzone ${this.disabled ? 'disabled' : ''}">
        <div class="icon">📄</div>
        <div class="title">Sleep bestanden hierheen of klik om te uploaden</div>
        <div class="subtitle">PDF, Word, of tekstbestanden (max 50MB)</div>
        <input type="file" class="file-input" accept="${this.accept}" ${this.multiple ? 'multiple' : ''} ${this.disabled ? 'disabled' : ''}>
      </div>
      <div class="file-list"></div>
    `;

    this._setupListeners();
    this._renderFileList();
  }

  _setupListeners() {
    const dropzone = this.shadowRoot.querySelector('.dropzone');
    const input = this.shadowRoot.querySelector('.file-input');

    dropzone.addEventListener('click', () => !this.disabled && input.click());
    dropzone.addEventListener('dragover', (e) => {
      e.preventDefault();
      if (!this.disabled) dropzone.classList.add('dragover');
    });
    dropzone.addEventListener('dragleave', () => dropzone.classList.remove('dragover'));
    dropzone.addEventListener('drop', (e) => {
      e.preventDefault();
      dropzone.classList.remove('dragover');
      if (!this.disabled) this._handleFiles(e.dataTransfer.files);
    });
    input.addEventListener('change', (e) => this._handleFiles(e.target.files));
  }

  _handleFiles(fileList) {
    const newFiles = Array.from(fileList);
    if (this.multiple) {
      this._files = [...this._files, ...newFiles];
    } else {
      this._files = newFiles.slice(0, 1);
    }
    this._renderFileList();
    this.dispatchEvent(new CustomEvent('change', {
      detail: { files: this._files },
      bubbles: true,
      composed: true
    }));
  }

  _renderFileList() {
    const list = this.shadowRoot.querySelector('.file-list');
    if (!list) return;

    list.innerHTML = this._files.map((file, index) => `
      <div class="file-item" data-index="${index}">
        <span>
          <span class="file-name">${this.escapeHtml(file.name)}</span>
          <span class="file-size">(${this._formatSize(file.size)})</span>
        </span>
        <button class="remove-btn" data-index="${index}">Verwijderen</button>
      </div>
    `).join('');

    list.querySelectorAll('.remove-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.target.dataset.index);
        this._files.splice(index, 1);
        this._renderFileList();
        this.dispatchEvent(new CustomEvent('change', {
          detail: { files: this._files },
          bubbles: true,
          composed: true
        }));
      });
    });
  }

  _formatSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }

  clearFiles() {
    this._files = [];
    this._renderFileList();
  }
}

customElements.define('rr-file-upload', RRFileUpload);
