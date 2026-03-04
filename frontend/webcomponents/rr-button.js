/**
 * RegelRecht Button Component
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRButton extends RRLocalBase {
  static componentName = 'rr-button';

  static get observedAttributes() {
    return ['variant', 'disabled', 'loading', 'type'];
  }

  get variant() { return this.getAttribute('variant') || 'primary'; }
  get disabled() { return this.getBooleanAttribute('disabled'); }
  get loading() { return this.getBooleanAttribute('loading'); }
  get type() { return this.getAttribute('type') || 'button'; }

  _getStyles() {
    return `
      :host { display: inline-block; font-family: var(--rr-font-family-sans, 'RijksoverheidSans', system-ui, sans-serif); }
      .btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: var(--spacing-2, 8px);
        padding: 10px 20px;
        border-radius: var(--border-radius-md, 6px);
        font-size: 1rem;
        font-weight: 500;
        font-family: inherit;
        cursor: pointer;
        transition: all 0.2s;
        border: 2px solid transparent;
      }
      .btn.primary {
        background: var(--color-primary, #154273);
        color: #fff;
        border-color: var(--color-primary, #154273);
      }
      .btn.primary:hover:not(:disabled) {
        background: var(--color-primary-hover, #1a5490);
        border-color: var(--color-primary-hover, #1a5490);
      }
      .btn.secondary {
        background: var(--color-slate-100, #f1f5f9);
        color: var(--color-slate-700, #334155);
        border-color: transparent;
      }
      .btn.secondary:hover:not(:disabled) {
        background: var(--color-slate-200, #e2e8f0);
      }
      .btn.danger {
        background: #dc2626;
        color: #fff;
        border-color: #dc2626;
      }
      .btn.danger:hover:not(:disabled) {
        background: #b91c1c;
        border-color: #b91c1c;
      }
      .btn:disabled { opacity: 0.5; cursor: not-allowed; }
      .btn:focus { outline: 2px solid var(--color-primary, #154273); outline-offset: 2px; }
      .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid currentColor;
        border-top-color: transparent;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }
      @keyframes spin { to { transform: rotate(360deg); } }
    `;
  }

  render() {
    const isDisabled = this.disabled || this.loading;
    this.shadowRoot.innerHTML = `
      <button class="btn ${this.variant}" type="${this.type}" ${isDisabled ? 'disabled' : ''}>
        ${this.loading ? '<span class="spinner"></span>' : ''}
        <slot></slot>
      </button>
    `;
    // Native click events are "composed" and automatically bubble through shadow DOM
    // No need to dispatch a custom event - that would cause duplicate events!
  }
}

customElements.define('rr-button', RRButton);
