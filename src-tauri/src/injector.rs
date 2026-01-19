/// Returns script to send text to each AI service
pub fn get_send_script(service: &str, text: &str) -> String {
    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$");

    match service {
        "claude" => format!(
            r#"
            (function() {{
                const text = `{escaped_text}`;

                const pickEditor = () => {{
                    const candidates = Array.from(document.querySelectorAll('[contenteditable="true"]'))
                        .filter((el) => {{
                            const rect = el.getBoundingClientRect();
                            return rect.width > 0 && rect.height > 0;
                        }});

                    if (!candidates.length) return null;

                    const labeled = candidates.filter((el) => {{
                        const role = el.getAttribute('role') || '';
                        const aria = (el.getAttribute('aria-label') || '').toLowerCase();
                        return role === 'textbox' || aria.includes('message') || aria.includes('prompt');
                    }});

                    const list = labeled.length ? labeled : candidates;
                    return list.sort((a, b) => a.getBoundingClientRect().bottom - b.getBoundingClientRect().bottom)[list.length - 1];
                }};

                const editor = pickEditor();
                if (editor) {{
                    editor.focus();
                    document.execCommand('selectAll', false, null);
                    const inserted = document.execCommand('insertText', false, text);
                    if (!inserted) {{
                        editor.textContent = text;
                    }}

                    editor.dispatchEvent(new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'insertText',
                        data: text
                    }}));

                    setTimeout(() => {{
                        const form = editor.closest('form');
                        const buttons = Array.from((form || document).querySelectorAll('button'));
                        const sendBtn = buttons.find((button) => {{
                            const label = (button.getAttribute('aria-label') || '').toLowerCase();
                            if (!label) return false;
                            return label.includes('send')
                                || label.includes('送信')
                                || label.includes('メッセージを送信');
                        }});
                        if (sendBtn && !sendBtn.disabled) {{
                            sendBtn.click();
                        }}
                    }}, 100);
                }}
            }})();
            "#
        ),

        "chatgpt" => format!(
            r#"
            (function() {{
                const text = `{escaped_text}`;

                // ChatGPT uses #prompt-textarea
                const textarea = document.querySelector('#prompt-textarea');
                if (textarea) {{
                    textarea.focus();

                    // For contenteditable div (new ChatGPT UI)
                    if (textarea.contentEditable === 'true') {{
                        textarea.innerHTML = '<p>' + text + '</p>';
                        textarea.dispatchEvent(new InputEvent('input', {{
                            bubbles: true,
                            cancelable: true,
                            inputType: 'insertText',
                            data: text
                        }}));
                    }} else {{
                        // For regular textarea
                        textarea.value = text;
                        textarea.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    }}

                    // Find and click send button
                    setTimeout(() => {{
                        const sendBtn = document.querySelector('button[data-testid="send-button"]')
                            || document.querySelector('form button[type="submit"]')
                            || document.querySelector('button[aria-label*="Send"]');
                        if (sendBtn && !sendBtn.disabled) {{
                            sendBtn.click();
                        }}
                    }}, 100);
                }}
            }})();
            "#
        ),

        "gemini" => format!(
            r#"
            (function() {{
                const text = `{escaped_text}`;

                // Gemini uses rich-textarea or contenteditable
                const editor = document.querySelector('.ql-editor[contenteditable="true"]')
                    || document.querySelector('rich-textarea [contenteditable="true"]')
                    || document.querySelector('[contenteditable="true"]');

                if (editor) {{
                    editor.focus();

                    // Insert text
                    editor.innerHTML = '<p>' + text + '</p>';

                    // Trigger events
                    editor.dispatchEvent(new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'insertText',
                        data: text
                    }}));

                    // Find and click send button
                    setTimeout(() => {{
                        const sendBtn = document.querySelector('button[aria-label*="Send"]')
                            || document.querySelector('.send-button')
                            || document.querySelector('button[mattooltip*="Send"]');
                        if (sendBtn && !sendBtn.disabled) {{
                            sendBtn.click();
                        }}
                    }}, 100);
                }}
            }})();
            "#
        ),

        _ => String::new(),
    }
}

/// Returns script to trigger new chat via Cmd+Shift+O keyboard shortcut
pub fn get_new_chat_script() -> &'static str {
    r#"
    (function() {
        const event = new KeyboardEvent('keydown', {
            key: 'o',
            code: 'KeyO',
            keyCode: 79,
            which: 79,
            metaKey: true,
            shiftKey: true,
            bubbles: true,
            cancelable: true
        });
        document.dispatchEvent(event);
    })();
    "#
}
