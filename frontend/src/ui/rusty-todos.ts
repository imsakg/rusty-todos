import {BaseHTMLElement, customElement, getChild, html} from 'dom-native';

@customElement("rusty-todos")
class RustyTodos extends BaseHTMLElement { // extends HTMLElement
    init(): void {
        let htmlContent: DocumentFragment = html`
            <div class="box"></div>
            <h1>todos</h1>
            <todos-input></todos-input>
            <todo-list></todo-list>
        `;

        this.append(htmlContent);
    }
}

@customElement("todo-input")
class TodoInput extends BaseHTMLElement { // extends HTMLElement
    #inputEl!: HTMLInputElement;
    
    init(): void {
        let htmlContent = html`
            <input type="text" placeholder="What needs to be done?">
        `;
        
        this.#inputEl = getChild(htmlContent, 'input');
        this.append(htmlContent);
    }
}

// todo-input tag
declare global {
    interface HTMLElementTagNameMap {
        'todo-input' : TodoInput;
    }
}
