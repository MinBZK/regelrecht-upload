/**
 * RegelRecht Help Text Component - For warnings and info messages
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRHelpText extends RRLocalBase {
  static componentName = 'rr-help-text';

  static get observedAttributes() {
    return ['variant'];
  }

  get variant() { return this.getAttribute('variant') || 'info'; }

  _getStyles() {
    return `
      :host { display: block; font-family: var(--rr-font-family-sans, system-ui, sans-serif); }
      .help-text {
        padding: 12px 16px;
        border-radius: 6px;
        font-size: 0.875rem;
        line-height: 1.5;
        display: flex;
        gap: 12px;
        align-items: flex-start;
      }
      .help-text.info {
        background: #eff6ff;
        border: 1px solid #bfdbfe;
        color: #1e40af;
      }
      .help-text.warning {
        background: #fffbeb;
        border: 1px solid #fde68a;
        color: #92400e;
      }
      .help-text.error {
        background: #fef2f2;
        border: 1px solid #fecaca;
        color: #b91c1c;
      }
      .help-text.success {
        background: #f0fdf4;
        border: 1px solid #bbf7d0;
        color: #166534;
      }
      .icon { font-size: 1.25rem; flex-shrink: 0; }
    `;
  }

  render() {
    const icons = { info: 'ℹ️', warning: '⚠️', error: '❌', success: '✓' };
    this.shadowRoot.innerHTML = `
      <div class="help-text ${this.variant}">
        <span class="icon">${icons[this.variant] || icons.info}</span>
        <div><slot></slot></div>
      </div>
    `;
  }
}

customElements.define('rr-help-text', RRHelpText);
