/**
 * Base class for local RegelRecht Web Components
 *
 * Simplified version of the @minbzk/storybook base class for local components.
 * Tokens are inherited from the parent page (loaded via CSS).
 */

const stylesheetCache = new Map();

export class RRLocalBase extends HTMLElement {
  static get observedAttributes() {
    return [];
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._initialized = false;
  }

  async connectedCallback() {
    if (!this._initialized) {
      await this._initialize();
      this._initialized = true;
      this.render();
    } else {
      this.render();
    }
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue && this._initialized) {
      this.render();
    }
  }

  async _initialize() {
    const styles = await this._loadStyles();

    if ('adoptedStyleSheets' in this.shadowRoot) {
      this.shadowRoot.adoptedStyleSheets = styles;
    } else {
      const styleEl = document.createElement('style');
      for (const sheet of styles) {
        if (sheet.cssRules) {
          for (const rule of sheet.cssRules) {
            styleEl.textContent += rule.cssText + '\n';
          }
        }
      }
      this.shadowRoot.prepend(styleEl);
    }
  }

  async _loadStyles() {
    const componentName = this.constructor.componentName || this.tagName.toLowerCase();

    if (stylesheetCache.has(componentName)) {
      return stylesheetCache.get(componentName);
    }

    const sheet = new CSSStyleSheet();
    sheet.replaceSync(this._getStyles());

    const sheets = [sheet];
    stylesheetCache.set(componentName, sheets);

    return sheets;
  }

  _getStyles() {
    return '';
  }

  getBooleanAttribute(name) {
    return this.hasAttribute(name) && this.getAttribute(name) !== 'false';
  }

  escapeHtml(str) {
    if (str === null || str === undefined) return '';
    if (typeof str !== 'string') str = String(str);
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  render() {
    throw new Error('render() must be implemented by subclass');
  }
}
