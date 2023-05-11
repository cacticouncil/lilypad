import * as vscode from "vscode";

export class LilypadEditorProvider implements vscode.CustomTextEditorProvider {
    internalEdit = false;

    private static readonly viewType = "lilypad.frameBased";

    public static register(context: vscode.ExtensionContext): vscode.Disposable {
        const provider = new LilypadEditorProvider(context);
        const providerRegistration = vscode.window.registerCustomEditorProvider(LilypadEditorProvider.viewType, provider);
        return providerRegistration;
    }

    constructor(
        private readonly context: vscode.ExtensionContext
    ) { }

    public async resolveCustomTextEditor(
        document: vscode.TextDocument,
        webviewPanel: vscode.WebviewPanel,
        token: vscode.CancellationToken
    ): Promise<void> {
        // Create Webview
        webviewPanel.webview.options = {
            enableScripts: true,
        };
        webviewPanel.webview.html = this.getHtml(webviewPanel.webview);

        // Sync our editor to external changes
        const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
            if (e.document.uri.toString() === document.uri.toString()) {
                // sometimes there are random empty triggers,
                // don't change internal edits on those
                if (e.contentChanges.length === 0) {
                    return;
                }

                // don't notify of edit that the editor made itself
                // from what I can tell, edits are sent in order so this should work
                if (this.internalEdit) {
                    this.internalEdit = false;
                    return;
                }

                // notify
                for (const change of e.contentChanges) {
                    webviewPanel.webview.postMessage({
                        type: "apply_edit",
                        edit: change,
                    });
                }
            }
        });

        // Listen for new diagnostics
        const changeDiagnosticsSubscription = vscode.languages.onDidChangeDiagnostics(e => {
            if (e.uris.map(u => u.toString()).includes(document.uri.toString())) {
                webviewPanel.webview.postMessage({
                    type: "new_diagnostics",
                    diagnostics: vscode.languages.getDiagnostics(document.uri)
                });
            }
        });

        // Get rid of the listeners when our editor is closed.
        webviewPanel.onDidDispose(() => {
            changeDocumentSubscription.dispose();
            changeDiagnosticsSubscription.dispose();
        });

        // Receive message from the webview.
        webviewPanel.webview.onDidReceiveMessage(message => {
            switch (message.type) {
                case "started":
                    webviewPanel.webview.postMessage({
                        type: "set_text",
                        text: document.getText(),
                    });
                    break;
                case "edited":
                    const editedRange = new vscode.Range(
                        message.range.startLine,
                        message.range.startCol,
                        message.range.endLine,
                        message.range.endCol
                    );
                    this.updateTextDocument(document, message.text, editedRange);
                    break;
                case "set_clipboard":
                    vscode.env.clipboard.writeText(message.text);
                    break;
                case "get_quick_fixes":
                    const cursor = new vscode.Range(message.line, message.col,
                                                    message.line, message.col);
                    vscode.commands.executeCommand(
                        "vscode.executeCodeActionProvider",
                        document.uri,
                        cursor,
                        vscode.CodeActionKind.QuickFix.value,
                        5 // limit to receive
                    ).then((actions: any) => {
                        webviewPanel.webview.postMessage({
                            type: "return_quick_fixes",
                            actions: actions.map((action: any) => action.command)
                        });
                    });
                    break;
                case "execute_command":
                    vscode.commands.executeCommand(message.command, ...message.args);
            }
        });
    }

    private updateTextDocument(document: vscode.TextDocument, newText: string, range: vscode.Range) {
        const edit = new vscode.WorkspaceEdit();
        edit.replace(
            document.uri,
            range,
            newText
        );
        this.internalEdit = true;
        return vscode.workspace.applyEdit(edit);
    }

    private getHtml(webview: vscode.Webview): string {
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(
            this.context.extensionUri, "static", "run.js"));

        return `
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>Druid web example</title>
                <style>
                    html,
                    body,
                    canvas {
                        margin: 0px;
                        padding: 0px;
                        width: 100%;
                        height: 100%;
                        overflow: hidden;
                    }
                </style>
            </head>
            <body>
                <div style="text-align: center; margin: 0px; padding: 0px;">
                    <canvas id="canvas"></canvas>
                </div>
                <script type="module" src="${scriptUri}" type="application/javascript"></script>
            </body>
        </html>
        `;
    }

}
