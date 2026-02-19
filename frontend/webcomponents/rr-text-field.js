/**
 * RegelRecht Text Field Component
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRTextField extends RRLocalBase {
  static componentName = 'rr-text-field';

  static get observedAttributes() {
    return ['value', 'placeholder', 'disabled', 'type', 'name', 'required'];
  }

  constructor() {
    super();
    this._onInput = this._onInput.bind(this);
    this._onChange = this._onChange.bind(this);
  }

  connectedCallback() {
    super.connectedCallback();
    requestAnimationFrame(() => this._attachListeners());
  }

  disconnectedCallback() {
    this._detachListeners();
  }

  _attachListeners() {
    const input = this.shadowRoot.querySelector('input');
    if (input) {
      input.addEventListener('input', this._onInput);
      input.addEventListener('change', this._onChange);
    }
  }

  _detachListeners() {
    const input = this.shadowRoot.querySelector('input');
    if (input) {
      input.removeEventListener('input', this._onInput);
      input.removeEventListener('change', this._onChange);
    }
  }

  _onInput(event) {
    this.dispatchEvent(new CustomEvent('input', {
      detail: { value: event.target.value },
      bubbles: true,
      composed: true
    }));
  }

  _onChange(event) {
    this.setAttribute('value', event.target.value);
    this.dispatchEvent(new CustomEvent('change', {
      detail: { value: event.target.value },
      bubbles: true,
      composed: true
    }));
  }

  get value() { return this.getAttribute('value') || ''; }
  set value(val) {
    this.setAttribute('value', val);
    const input = this.shadowRoot?.querySelector('input');
    if (input) input.value = val;
  }

  get name() { return this.getAttribute('name') || ''; }
  get placeholder() { return this.getAttribute('placeholder') || ''; }
  get disabled() { return this.getBooleanAttribute('disabled'); }
  get required() { return this.getBooleanAttribute('required'); }
  get type() { return this.getAttribute('type') || 'text'; }

  _getStyles() {
    return `
      :host { display: block; font-family: var(--rr-font-family-sans, 'RijksoverheidSans', system-ui, sans-serif); }
      :host([hidden]) { display: none; }
      .input {
        width: 100%;
        padding: var(--spacing-2, 8px) var(--spacing-3, 12px);
        border: 2px solid var(--color-border, #e2e8f0);
        border-radius: var(--border-radius-md, 6px);
        font-size: 0.875rem;
        font-family: inherit;
        background: var(--color-white, #fff);
        color: var(--color-text-primary, #0f172a);
        box-sizing: border-box;
      }
      .input:focus { outline: 2px solid var(--color-primary, #154273); outline-offset: -2px; }
      .input:disabled { opacity: 0.6; cursor: not-allowed; background: var(--color-slate-100, #f1f5f9); }
      .input::placeholder { color: var(--color-slate-400, #94a3b8); }
    `;
  }

  render() {
    this.shadowRoot.innerHTML = `
      <input class="input" type="${this.type}" value="${this.escapeHtml(this.value)}"
        placeholder="${this.escapeHtml(this.placeholder)}" name="${this.name}"
        ${this.disabled ? 'disabled' : ''} ${this.required ? 'required' : ''}>
    `;
    this._attachListeners();
  }
}

customElements.define('rr-text-field', RRTextField);
