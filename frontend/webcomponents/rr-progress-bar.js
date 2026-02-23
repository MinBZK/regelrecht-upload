/**
 * RegelRecht Progress Bar Component
 *
 * Displays a multi-step progress indicator for the submission wizard.
 * Visual specification:
 * - Completed steps: checkmark icon, muted color
 * - Current step: inverted contrast (dark background, light text)
 * - Future steps: outline only, light appearance
 */
import { RRLocalBase } from './rr-local-base.js';

export class RRProgressBar extends RRLocalBase {
  static componentName = 'rr-progress-bar';

  static get observedAttributes() {
    return ['current', 'steps'];
  }

  get current() {
    return parseInt(this.getAttribute('current') || '1', 10);
  }

  get steps() {
    const attr = this.getAttribute('steps');
    if (!attr) {
      return [
        { label: 'Privacy', key: 'privacy' },
        { label: 'Documenten', key: 'documents' },
        { label: 'Planning', key: 'planning' },
        { label: 'Gegevens', key: 'contact' },
        { label: 'Bevestiging', key: 'summary' }
      ];
    }
    try {
      return JSON.parse(attr);
    } catch {
      return [];
    }
  }

  _getStyles() {
    return `
      :host {
        display: block;
        font-family: var(--rr-font-family-sans, 'RijksoverheidSans', system-ui, sans-serif);
      }

      .progress-bar {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        position: relative;
        padding: 0;
      }

      /* Connector line between steps */
      .progress-bar::before {
        content: '';
        position: absolute;
        top: 12px;
        left: 10%;
        right: 10%;
        height: 2px;
        background: var(--color-slate-200, #e2e8f0);
        z-index: 0;
      }

      .step {
        display: flex;
        flex-direction: column;
        align-items: center;
        position: relative;
        z-index: 1;
        flex: 1;
      }

      .step-indicator {
        width: 24px;
        height: 24px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 600;
        font-size: 0.75rem;
        transition: all 0.2s ease;
        background: var(--color-white, #fff);
      }

      /* Future/pending state - simple outline circle */
      .step.pending .step-indicator {
        border: 2px solid var(--color-slate-300, #cbd5e1);
        color: var(--color-slate-400, #94a3b8);
      }

      /* Current/active state - filled primary color */
      .step.active .step-indicator {
        background: var(--color-primary, #154273);
        border: 2px solid var(--color-primary, #154273);
        color: var(--color-white, #fff);
      }

      /* Completed state - green with checkmark */
      .step.completed .step-indicator {
        background: var(--color-secondary, #39870c);
        border: 2px solid var(--color-secondary, #39870c);
        color: var(--color-white, #fff);
      }

      .step-label {
        margin-top: 8px;
        font-size: 0.75rem;
        font-weight: 400;
        text-align: center;
        color: var(--color-slate-500, #64748b);
      }

      .step.active .step-label {
        color: var(--color-primary, #154273);
        font-weight: 600;
      }

      .step.completed .step-label {
        color: var(--color-slate-600, #475569);
      }

      /* Checkmark SVG */
      .checkmark {
        width: 12px;
        height: 12px;
        stroke: currentColor;
        stroke-width: 3;
        fill: none;
      }

      /* Responsive */
      @media (max-width: 500px) {
        .step-label {
          font-size: 0.625rem;
        }
        .step-indicator {
          width: 20px;
          height: 20px;
          font-size: 0.625rem;
        }
        .progress-bar::before {
          top: 10px;
        }
        .checkmark {
          width: 10px;
          height: 10px;
        }
      }
    `;
  }

  render() {
    const currentStep = this.current;
    const steps = this.steps;

    const stepsHtml = steps.map((step, index) => {
      const stepNum = index + 1;
      let state = 'pending';
      if (stepNum < currentStep) state = 'completed';
      if (stepNum === currentStep) state = 'active';

      const indicator = state === 'completed'
        ? `<svg class="checkmark" viewBox="0 0 24 24">
             <polyline points="20 6 9 17 4 12"></polyline>
           </svg>`
        : stepNum;

      return `
        <div class="step ${state}" data-step="${stepNum}" data-key="${step.key || ''}">
          <div class="step-indicator">${indicator}</div>
          <div class="step-label">${this.escapeHtml(step.label)}</div>
        </div>
      `;
    }).join('');

    this.shadowRoot.innerHTML = `
      <div class="progress-bar" role="navigation" aria-label="Voortgang">
        ${stepsHtml}
      </div>
    `;
  }
}

customElements.define('rr-progress-bar', RRProgressBar);
