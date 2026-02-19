/**
 * RegelRecht Select Field Component
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRSelectField extends RRLocalBase {
  static componentName = 'rr-select-field';

  static get observedAttributes() {
    return ['value', 'disabled', 'name', 'required'];
  }

  constructor() {
    super();
    this._onChange = this._onChange.bind(this);
  }

  connectedCallback() {
    super.connectedCallback();
    requestAnimationFrame(() => {
      this._setupSelect();
      this._syncOptions();
    });
    this._mutationObserver = new MutationObserver(() => this._syncOptions());
    this._mutationObserver.observe(this, { childList: true, subtree: true, characterData: true });
  }

  disconnectedCallback() {
    const select = this.shadowRoot.querySelector('select');
    if (select) select.removeEventListener('change', this._onChange);
    if (this._mutationObserver) this._mutationObserver.disconnect();
  }

  _setupSelect() {
    const select = this.shadowRoot.querySelector('select');
    if (select) select.addEventListener('change', this._onChange);
  }

  _syncOptions() {
    const select = this.shadowRoot.querySelector('select');
    if (!select) return;

    const lightOptions = this.querySelectorAll('option');
    select.innerHTML = '';
    lightOptions.forEach(opt => {
      const newOpt = document.createElement('option');
      newOpt.value = opt.value;
      newOpt.textContent = opt.textContent;
      newOpt.selected = opt.selected || opt.hasAttribute('selected');
      newOpt.disabled = opt.disabled;
      select.appendChild(newOpt);
    });

    if (this.value) select.value = this.value;
  }

  _onChange(event) {
    // Stop native event - we'll dispatch our own with detail.value
    // This prevents duplicate events since native 'change' is composed
    event.stopPropagation();
    this.setAttribute('value', event.target.value);
    this.dispatchEvent(new CustomEvent('change', {
      detail: { value: event.target.value },
      bubbles: true,
      composed: true
    }));
  }

  get value() { return this.getAttribute('value') || ''; }
  set value(val) { this.setAttribute('value', val); }
  get name() { return this.getAttribute('name') || ''; }
  get disabled() { return this.getBooleanAttribute('disabled'); }
  get required() { return this.getBooleanAttribute('required'); }

  _getStyles() {
    return `
      :host { display: block; font-family: var(--rr-font-family-sans, 'RijksoverheidSans', system-ui, sans-serif); }
      :host([hidden]) { display: none; }
      .select-wrapper { position: relative; width: 100%; }
      .select {
        width: 100%;
        height: 44px;
        padding: var(--spacing-2, 8px) 40px var(--spacing-2, 8px) var(--spacing-3, 12px);
        border: 2px solid var(--color-border, #e2e8f0);
        border-radius: var(--border-radius-md, 6px);
        font-size: 1rem;
        font-family: inherit;
        background-color: var(--color-white, #fff);
        color: var(--color-text-primary, #0f172a);
        cursor: pointer;
        appearance: none;
        -webkit-appearance: none;
      }
      .select:focus { outline: 2px solid var(--color-primary, #154273); outline-offset: -2px; }
      .select:disabled { opacity: 0.6; cursor: not-allowed; }
      .chevron { position: absolute; right: var(--spacing-3, 12px); top: 50%; transform: translateY(-50%); pointer-events: none; }
    `;
  }

  render() {
    this.shadowRoot.innerHTML = `
      <div class="select-wrapper">
        <select class="select" name="${this.name}" ${this.disabled ? 'disabled' : ''} ${this.required ? 'required' : ''}></select>
        <span class="chevron">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 6L8 10L12 6" stroke="#334155" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
      </div>
      <slot style="display: none;"></slot>
    `;
    this._setupSelect();
    this._syncOptions();
  }
}

customElements.define('rr-select-field', RRSelectField);
