/**
 * RegelRecht Toggle Group Component
 *
 * Replaces dropdown selects with visible toggle button groups.
 * Improves discoverability and reduces interaction cost.
 *
 * Usage:
 * <rr-toggle-group name="category" required>
 *   <rr-toggle value="circular">Circulaire</rr-toggle>
 *   <rr-toggle value="policy">Beleidsregel</rr-toggle>
 *   <rr-toggle value="instruction">Instructie</rr-toggle>
 * </rr-toggle-group>
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRToggleGroup extends RRLocalBase {
  static componentName = 'rr-toggle-group';

  static get observedAttributes() {
    return ['name', 'value', 'required', 'disabled', 'options'];
  }

  get name() { return this.getAttribute('name') || ''; }
  get value() { return this.getAttribute('value') || ''; }
  set value(val) { this.setAttribute('value', val || ''); }
  get required() { return this.getBooleanAttribute('required'); }
  get disabled() { return this.getBooleanAttribute('disabled'); }

  get options() {
    const attr = this.getAttribute('options');
    if (attr) {
      try {
        return JSON.parse(attr);
      } catch {
        return [];
      }
    }
    return [];
  }

  _getStyles() {
    return `
      :host {
        display: block;
        font-family: var(--rr-font-family-sans, 'RijksoverheidSans', system-ui, sans-serif);
      }

      .toggle-group {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }

      .toggle-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 10px 16px;
        border: 2px solid var(--color-slate-300, #cbd5e1);
        border-radius: var(--border-radius-md, 6px);
        background: var(--color-white, #fff);
        color: var(--color-slate-700, #334155);
        font-size: 0.875rem;
        font-weight: 500;
        font-family: inherit;
        cursor: pointer;
        transition: all 0.2s ease;
        min-width: 100px;
      }

      .toggle-btn:hover:not(.selected):not(:disabled) {
        border-color: var(--color-primary, #154273);
        background: var(--color-slate-50, #f8fafc);
      }

      .toggle-btn:focus {
        outline: 2px solid var(--color-primary, #154273);
        outline-offset: 2px;
      }

      .toggle-btn.selected {
        background: var(--color-primary, #154273);
        border-color: var(--color-primary, #154273);
        color: var(--color-white, #fff);
      }

      .toggle-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .toggle-btn.warning {
        border-color: #f59e0b;
      }

      .toggle-btn.warning.selected {
        background: #f59e0b;
        border-color: #f59e0b;
      }

      .toggle-btn.danger {
        border-color: #dc2626;
      }

      .toggle-btn.danger.selected {
        background: #dc2626;
        border-color: #dc2626;
      }

      /* Responsive: stack on very small screens */
      @media (max-width: 400px) {
        .toggle-group {
          flex-direction: column;
        }
        .toggle-btn {
          width: 100%;
        }
      }
    `;
  }

  connectedCallback() {
    super.connectedCallback();
    this._setupEventDelegation();
  }

  _setupEventDelegation() {
    this.shadowRoot.addEventListener('click', (e) => {
      const btn = e.target.closest('.toggle-btn');
      if (!btn || btn.disabled) return;

      const value = btn.dataset.value;
      this.value = value;
      this.render();

      // Dispatch change event
      this.dispatchEvent(new CustomEvent('change', {
        detail: { value, name: this.name },
        bubbles: true,
        composed: true
      }));
    });
  }

  render() {
    const options = this.options;
    const currentValue = this.value;
    const isDisabled = this.disabled;

    const buttonsHtml = options.map(opt => {
      const value = typeof opt === 'string' ? opt : opt.value;
      const label = typeof opt === 'string' ? opt : opt.label;
      const variant = typeof opt === 'object' ? (opt.variant || '') : '';
      const isSelected = value === currentValue;

      return `
        <button
          type="button"
          class="toggle-btn ${isSelected ? 'selected' : ''} ${variant}"
          data-value="${this.escapeHtml(value)}"
          ${isDisabled ? 'disabled' : ''}
          aria-pressed="${isSelected}"
          role="radio"
        >
          ${this.escapeHtml(label)}
        </button>
      `;
    }).join('');

    this.shadowRoot.innerHTML = `
      <div class="toggle-group" role="radiogroup" aria-label="${this.escapeHtml(this.name)}">
        ${buttonsHtml}
      </div>
    `;
  }
}

customElements.define('rr-toggle-group', RRToggleGroup);
